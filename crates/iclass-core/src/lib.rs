//! Core login, query, schedule-selection, and attendance orchestration logic.

use std::collections::{BTreeMap, HashSet};

use chrono::{Local, NaiveDate, NaiveDateTime};
use iclass_domain::{
    CheckInAttempt, CheckInAvailability, CheckInMode, Credentials, ScheduleEntry, Semester, Session,
};
use iclass_session::{SessionClient, SessionError, SessionErrorKind};
use thiserror::Error;

pub use iclass_domain::{CheckInMethod, CheckInReceipt, Course};

/// Stable classification of business-facing core failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreErrorKind {
    /// Authentication is required or the session expired.
    Authentication,
    /// User credentials were rejected.
    InvalidCredentials,
    /// No session refresh credentials were available.
    MissingCredentials,
    /// Persistent local storage failed.
    Store,
    /// Network or transport failure occurred.
    Transport,
    /// Response parsing or normalization failed.
    Parse,
    /// No schedule could be found for the target date.
    NoSchedule,
    /// Attendance QR is invalid or expired.
    QrExpired,
    /// Requested check-in mode is incompatible with the selected schedule.
    UnsupportedCheckInMode,
    /// The selected class has not yet reached the check-in window.
    CheckInTooEarly,
    /// The selected class has already ended.
    CheckInClosed,
    /// Request parameters were rejected by the server.
    Parameter,
    /// Other business-level error.
    Business,
}

/// Errors exposed by the business-oriented core layer.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Wrapped session- or API-layer failure.
    #[error(transparent)]
    Session(#[from] SessionError),
    /// No suitable schedule row could be selected for the requested date.
    #[error("no schedule could be selected for {date}")]
    NoScheduleAvailable {
        /// Date for which schedule selection failed.
        date: NaiveDate,
    },
    /// The chosen check-in mode is incompatible with the selected schedule row.
    #[error("schedule {schedule_id} does not support {mode:?} check-in")]
    UnsupportedCheckInMode {
        /// Requested attendance strategy.
        mode: CheckInMode,
        /// Identifier of the schedule that could not satisfy the request.
        schedule_id: String,
    },
    /// The selected class has not yet reached the allowed check-in time.
    #[error("{course_name} is not open for check-in yet; opens at {opens_at}")]
    CheckInNotOpenYet {
        /// Identifier of the schedule that is not yet open.
        schedule_id: String,
        /// Human-readable course name for display.
        course_name: String,
        /// Local datetime when check-in becomes available.
        opens_at: NaiveDateTime,
    },
    /// The selected class has already ended and can no longer be checked in.
    #[error("{course_name} has already ended at {ended_at}")]
    CheckInClosed {
        /// Identifier of the closed schedule.
        schedule_id: String,
        /// Human-readable course name for display.
        course_name: String,
        /// Local datetime when the class ended.
        ended_at: NaiveDateTime,
    },
}

impl CoreError {
    /// Returns the stable classification of this core-layer error.
    pub fn kind(&self) -> CoreErrorKind {
        match self {
            Self::Session(error) => match error.kind() {
                SessionErrorKind::Authentication => CoreErrorKind::Authentication,
                SessionErrorKind::InvalidCredentials => CoreErrorKind::InvalidCredentials,
                SessionErrorKind::MissingCredentials => CoreErrorKind::MissingCredentials,
                SessionErrorKind::Store => CoreErrorKind::Store,
                SessionErrorKind::Transport => CoreErrorKind::Transport,
                SessionErrorKind::Parse => CoreErrorKind::Parse,
                SessionErrorKind::EmptySchedule => CoreErrorKind::NoSchedule,
                SessionErrorKind::QrExpired => CoreErrorKind::QrExpired,
                SessionErrorKind::Parameter => CoreErrorKind::Parameter,
                SessionErrorKind::Business => CoreErrorKind::Business,
            },
            Self::NoScheduleAvailable { .. } => CoreErrorKind::NoSchedule,
            Self::UnsupportedCheckInMode { .. } => CoreErrorKind::UnsupportedCheckInMode,
            Self::CheckInNotOpenYet { .. } => CoreErrorKind::CheckInTooEarly,
            Self::CheckInClosed { .. } => CoreErrorKind::CheckInClosed,
        }
    }

    /// Returns whether the failure is the observed "QR expired / invalid time window" case.
    pub fn is_qr_expired(&self) -> bool {
        self.kind() == CoreErrorKind::QrExpired
    }

    /// Returns whether the failure indicates invalid login credentials.
    pub fn is_invalid_credentials(&self) -> bool {
        self.kind() == CoreErrorKind::InvalidCredentials
    }

    /// Returns whether the failure indicates authentication/session expiry.
    pub fn is_authentication_error(&self) -> bool {
        matches!(
            self.kind(),
            CoreErrorKind::Authentication | CoreErrorKind::InvalidCredentials
        )
    }
}

/// Main business facade used by CLI and future GUI entry points.
#[derive(Debug, Clone)]
pub struct IClassCore {
    session_client: SessionClient,
}

impl IClassCore {
    /// Creates a new core facade using the provided session-aware client.
    pub fn new(session_client: SessionClient) -> Self {
        Self { session_client }
    }

    /// Exposes the underlying session client for advanced callers.
    pub fn session_client(&self) -> &SessionClient {
        &self.session_client
    }

    /// Logs in with explicit credentials and returns the normalized session.
    pub async fn login(
        &self,
        credentials: &Credentials,
        remember_password: bool,
    ) -> Result<Session, CoreError> {
        Ok(self
            .session_client
            .login(credentials, remember_password)
            .await?)
    }

    /// Returns the current valid session, refreshing it when necessary.
    pub async fn current_session(&self) -> Result<Session, CoreError> {
        Ok(self.session_client.ensure_session().await?)
    }

    /// Returns semester metadata for the current user.
    pub async fn semesters(&self) -> Result<Vec<Semester>, CoreError> {
        Ok(dedupe_semesters(self.session_client.get_semesters().await?))
    }

    /// Returns the current user's courses.
    pub async fn courses(&self) -> Result<Vec<Course>, CoreError> {
        Ok(self.session_client.get_my_courses().await?)
    }

    /// Returns schedule rows for a given day.
    pub async fn daily_schedule(&self, date: NaiveDate) -> Result<Vec<ScheduleEntry>, CoreError> {
        Ok(normalize_schedule_entries(
            self.session_client.get_daily_schedule(date).await?,
        ))
    }

    /// Returns flattened schedule rows from the weekly schedule view.
    pub async fn weekly_schedule(&self, date: NaiveDate) -> Result<Vec<ScheduleEntry>, CoreError> {
        Ok(normalize_schedule_entries(
            self.session_client.get_weekly_schedule(date).await?,
        ))
    }

    /// Selects the most appropriate schedule row for the given local moment.
    ///
    /// The daily schedule is preferred. When it is empty, the weekly view is used as a fallback.
    pub async fn best_schedule_for(
        &self,
        moment: NaiveDateTime,
    ) -> Result<ScheduleEntry, CoreError> {
        let date = moment.date();
        let mut schedules = self.daily_schedule(date).await?;
        if schedules.is_empty() {
            schedules = self
                .weekly_schedule(date)
                .await?
                .into_iter()
                .filter(|schedule| schedule.teach_date == date)
                .collect();
        }

        select_best_schedule(&schedules, moment).ok_or(CoreError::NoScheduleAvailable { date })
    }

    /// Attempts attendance for a specific schedule using the requested check-in mode.
    pub async fn check_in_for_schedule(
        &self,
        schedule: ScheduleEntry,
        mode: CheckInMode,
        timestamp: i64,
    ) -> Result<CheckInAttempt, CoreError> {
        self.check_in_for_schedule_at(schedule, mode, Local::now().naive_local(), timestamp)
            .await
    }

    /// Attempts attendance for a specific schedule at an explicit local moment.
    pub async fn check_in_for_schedule_at(
        &self,
        schedule: ScheduleEntry,
        mode: CheckInMode,
        moment: NaiveDateTime,
        timestamp: i64,
    ) -> Result<CheckInAttempt, CoreError> {
        validate_check_in_window(&schedule, moment)?;

        let receipt = match mode {
            CheckInMode::Auto => {
                if let Some(schedule_uuid) = schedule.schedule_uuid.as_deref() {
                    self.session_client
                        .check_in_by_uuid(schedule_uuid, timestamp)
                        .await?
                } else if schedule.supports_id_checkin() {
                    self.session_client
                        .check_in_by_id(&schedule.schedule_id, timestamp)
                        .await?
                } else {
                    return Err(CoreError::UnsupportedCheckInMode {
                        mode,
                        schedule_id: schedule.schedule_id.clone(),
                    });
                }
            }
            CheckInMode::ByUuid => {
                if let Some(schedule_uuid) = schedule.schedule_uuid.as_deref() {
                    self.session_client
                        .check_in_by_uuid(schedule_uuid, timestamp)
                        .await?
                } else {
                    return Err(CoreError::UnsupportedCheckInMode {
                        mode,
                        schedule_id: schedule.schedule_id.clone(),
                    });
                }
            }
            CheckInMode::ById => {
                if schedule.supports_id_checkin() {
                    self.session_client
                        .check_in_by_id(&schedule.schedule_id, timestamp)
                        .await?
                } else {
                    return Err(CoreError::UnsupportedCheckInMode {
                        mode,
                        schedule_id: schedule.schedule_id.clone(),
                    });
                }
            }
        };

        Ok(CheckInAttempt { schedule, receipt })
    }

    /// Selects the best schedule for `moment` and immediately attempts attendance.
    pub async fn check_in_at(
        &self,
        moment: NaiveDateTime,
        mode: CheckInMode,
        timestamp: i64,
    ) -> Result<CheckInAttempt, CoreError> {
        let schedule = self.best_schedule_for(moment).await?;
        self.check_in_for_schedule_at(schedule, mode, moment, timestamp)
            .await
    }

    /// Convenience wrapper that checks in using the current local time and current Unix timestamp.
    pub async fn check_in_now(&self, mode: CheckInMode) -> Result<CheckInAttempt, CoreError> {
        let now = Local::now().naive_local();
        self.check_in_at(now, mode, Local::now().timestamp()).await
    }
}

/// Ranks schedule rows and returns the best candidate for the given moment.
///
/// Active classes win over future classes, and future classes win over classes that have already ended.
pub fn select_best_schedule(
    schedules: &[ScheduleEntry],
    moment: NaiveDateTime,
) -> Option<ScheduleEntry> {
    let mut ranked = schedules.to_vec();
    ranked.sort_by_key(|schedule| {
        let timing_rank = if schedule.is_active_at(moment) {
            0
        } else if schedule.begins_at > moment {
            1
        } else {
            2
        };
        (
            timing_rank,
            schedule.distance_seconds(moment),
            schedule.begins_at,
        )
    });
    ranked.into_iter().next()
}

/// Removes duplicate semester rows while preserving the first occurrence order.
pub fn dedupe_semesters(semesters: Vec<Semester>) -> Vec<Semester> {
    let mut seen = HashSet::new();
    semesters
        .into_iter()
        .filter(|semester| seen.insert(semester.code.clone()))
        .collect()
}

/// Normalizes raw schedule rows into logical lessons and aggregates lesson-unit counts.
pub fn normalize_schedule_entries(schedules: Vec<ScheduleEntry>) -> Vec<ScheduleEntry> {
    type ScheduleKey = (NaiveDate, NaiveDateTime, NaiveDateTime, String);

    let mut groups: BTreeMap<ScheduleKey, Vec<ScheduleEntry>> = BTreeMap::new();
    for schedule in schedules {
        let key = (
            schedule.teach_date,
            schedule.begins_at,
            schedule.ends_at,
            schedule.course_name.clone(),
        );
        groups.entry(key).or_default().push(schedule);
    }

    groups
        .into_values()
        .filter_map(|entries| {
            let lesson_units = u16::try_from(entries.len()).ok()?;
            let min_schedule_id = entries
                .iter()
                .map(|entry| entry.schedule_id.as_str())
                .min_by(|left, right| compare_schedule_ids(left, right))?
                .to_owned();
            let mut schedule = entries.into_iter().max_by_key(|entry| {
                (
                    entry.supports_uuid_checkin(),
                    entry.supports_id_checkin(),
                    entry.sign_status.as_deref() == Some("1"),
                )
            })?;
            schedule.schedule_id = min_schedule_id;
            schedule.lesson_units = lesson_units;
            Some(schedule)
        })
        .collect()
}

/// Compares schedule identifiers, preferring numeric ordering when both IDs are numeric.
fn compare_schedule_ids(left: &str, right: &str) -> std::cmp::Ordering {
    match (left.parse::<u64>(), right.parse::<u64>()) {
        (Ok(left_num), Ok(right_num)) => left_num.cmp(&right_num),
        _ => left.cmp(right),
    }
}

/// Validates whether a schedule may be checked in at the given local moment.
pub fn validate_check_in_window(
    schedule: &ScheduleEntry,
    moment: NaiveDateTime,
) -> Result<(), CoreError> {
    match schedule.check_in_availability(moment) {
        CheckInAvailability::Open => Ok(()),
        CheckInAvailability::NotOpenYet => Err(CoreError::CheckInNotOpenYet {
            schedule_id: schedule.schedule_id.clone(),
            course_name: schedule.course_name.clone(),
            opens_at: schedule.check_in_opens_at(),
        }),
        CheckInAvailability::Closed => Err(CoreError::CheckInClosed {
            schedule_id: schedule.schedule_id.clone(),
            course_name: schedule.course_name.clone(),
            ended_at: schedule.ends_at,
        }),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
    use iclass_domain::{ScheduleEntry, Semester};

    use super::{
        CoreErrorKind, dedupe_semesters, normalize_schedule_entries, select_best_schedule,
        validate_check_in_window,
    };

    fn schedule(id: &str, begin_hour: u32, end_hour: u32) -> ScheduleEntry {
        let date = NaiveDate::from_ymd_opt(2026, 4, 23).expect("valid test date");
        ScheduleEntry {
            schedule_id: id.into(),
            schedule_uuid: Some(format!("uuid-{id}")),
            course_id: None,
            course_name: format!("course-{id}"),
            teacher_name: None,
            classroom_name: None,
            teach_date: date,
            begins_at: NaiveDateTime::new(
                date,
                NaiveTime::from_hms_opt(begin_hour, 0, 0).expect("valid test time"),
            ),
            ends_at: NaiveDateTime::new(
                date,
                NaiveTime::from_hms_opt(end_hour, 0, 0).expect("valid test time"),
            ),
            lesson_units: 1,
            sign_status: None,
        }
    }

    #[test]
    fn prefers_active_schedule() {
        let now = NaiveDate::from_ymd_opt(2026, 4, 23)
            .expect("valid test date")
            .and_hms_opt(13, 30, 0)
            .expect("valid test datetime");
        let schedules = vec![schedule("a", 8, 10), schedule("b", 13, 15)];

        let picked = select_best_schedule(&schedules, now).expect("schedule should exist");
        assert_eq!(picked.schedule_id, "b");
    }

    #[test]
    fn falls_back_to_closest_future_schedule() {
        let now = NaiveDate::from_ymd_opt(2026, 4, 23)
            .expect("valid test date")
            .and_hms_opt(11, 0, 0)
            .expect("valid test datetime");
        let schedules = vec![schedule("a", 8, 10), schedule("b", 13, 15)];

        let picked = select_best_schedule(&schedules, now).expect("schedule should exist");
        assert_eq!(picked.schedule_id, "b");
    }

    #[test]
    fn prefers_future_schedule_over_recently_ended_schedule() {
        let now = NaiveDate::from_ymd_opt(2026, 4, 23)
            .expect("valid test date")
            .and_hms_opt(11, 0, 0)
            .expect("valid test datetime");
        let schedules = vec![schedule("past", 9, 10), schedule("future", 12, 13)];

        let picked = select_best_schedule(&schedules, now).expect("schedule should exist");
        assert_eq!(picked.schedule_id, "future");
    }

    #[test]
    fn rejects_check_in_before_open_window() {
        let schedule = schedule("future", 13, 15);
        let moment = NaiveDate::from_ymd_opt(2026, 4, 23)
            .expect("valid test date")
            .and_hms_opt(12, 0, 0)
            .expect("valid test datetime");

        let error = validate_check_in_window(&schedule, moment).expect_err("should reject");
        assert_eq!(error.kind(), CoreErrorKind::CheckInTooEarly);
    }

    #[test]
    fn rejects_check_in_after_class_end() {
        let schedule = schedule("past", 13, 15);
        let moment = NaiveDate::from_ymd_opt(2026, 4, 23)
            .expect("valid test date")
            .and_hms_opt(15, 30, 0)
            .expect("valid test datetime");

        let error = validate_check_in_window(&schedule, moment).expect_err("should reject");
        assert_eq!(error.kind(), CoreErrorKind::CheckInClosed);
    }

    #[test]
    fn deduplicates_semesters_by_code() {
        let semesters = vec![
            Semester {
                code: "2025202602".into(),
                name: "2025-2026学年春季学期".into(),
                begin_date: NaiveDate::from_ymd_opt(2026, 2, 23).expect("valid date"),
                end_date: NaiveDate::from_ymd_opt(2026, 7, 5).expect("valid date"),
                current: true,
            },
            Semester {
                code: "2025202602".into(),
                name: "2025-2026学年春季学期".into(),
                begin_date: NaiveDate::from_ymd_opt(2026, 2, 23).expect("valid date"),
                end_date: NaiveDate::from_ymd_opt(2026, 7, 5).expect("valid date"),
                current: true,
            },
        ];

        let normalized = dedupe_semesters(semesters);
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].code, "2025202602");
    }

    #[test]
    fn normalizes_duplicate_schedule_rows_into_lesson_units() {
        let first = schedule("12", 8, 10);
        let mut second = schedule("34", 8, 10);
        second.course_name = first.course_name.clone();
        second.teacher_name = first.teacher_name.clone();
        second.classroom_name = first.classroom_name.clone();
        second.schedule_uuid = Some("uuid-b".into());

        let normalized = normalize_schedule_entries(vec![first, second]);
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].lesson_units, 2);
        assert_eq!(normalized[0].schedule_id, "12");
    }

    #[test]
    fn normalizes_same_name_and_time_even_if_teacher_differs() {
        let mut first = schedule("8", 8, 10);
        first.course_name = "智能计算系统".into();
        first.teacher_name = Some("教师甲".into());

        let mut second = schedule("3", 8, 10);
        second.course_name = "智能计算系统".into();
        second.teacher_name = Some("教师乙".into());
        second.schedule_uuid = Some("uuid-3".into());

        let normalized = normalize_schedule_entries(vec![first, second]);
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].lesson_units, 2);
        assert_eq!(normalized[0].schedule_id, "3");
    }
}
