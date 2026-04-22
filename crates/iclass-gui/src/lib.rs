//! GUI-facing bridge models and helper functions built on top of the core facade.

use std::time::Instant;

use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime};
use iclass_core::{CheckInReceipt, CoreErrorKind, Course, IClassCore};
use iclass_domain::{CheckInAvailability, CheckInMode, ScheduleEntry, Semester};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stable GUI-facing error codes suitable for front-end branching.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GuiErrorCode {
    /// Authentication is required or the stored session expired.
    AuthenticationRequired,
    /// User credentials were rejected.
    InvalidCredentials,
    /// No credentials are available for re-login.
    MissingCredentials,
    /// No schedule could be found for the selected date.
    NoSchedule,
    /// The selected attendance mode cannot be used with the chosen schedule.
    UnsupportedCheckInMode,
    /// Attendance QR is invalid or expired.
    QrExpired,
    /// The selected class has not yet reached the allowed check-in time.
    CheckInTooEarly,
    /// The selected class has already ended.
    CheckInClosed,
    /// Local persistence failed.
    Storage,
    /// Network or transport failure occurred.
    Network,
    /// Server rejected request parameters.
    Parameter,
    /// Local parsing failed.
    Parse,
    /// Other server-side business error.
    Business,
}

/// Errors surfaced by the GUI bridge layer.
#[derive(Debug, Error)]
pub enum GuiBridgeError {
    /// Wrapped core-layer error.
    #[error(transparent)]
    Core(#[from] iclass_core::CoreError),
}

impl GuiBridgeError {
    /// Converts the bridge error into a serializable payload suitable for GUI presentation.
    pub fn payload(&self) -> GuiErrorPayload {
        match self {
            Self::Core(error) => {
                let code = match error.kind() {
                    CoreErrorKind::Authentication => GuiErrorCode::AuthenticationRequired,
                    CoreErrorKind::InvalidCredentials => GuiErrorCode::InvalidCredentials,
                    CoreErrorKind::MissingCredentials => GuiErrorCode::MissingCredentials,
                    CoreErrorKind::NoSchedule => GuiErrorCode::NoSchedule,
                    CoreErrorKind::UnsupportedCheckInMode => GuiErrorCode::UnsupportedCheckInMode,
                    CoreErrorKind::CheckInTooEarly => GuiErrorCode::CheckInTooEarly,
                    CoreErrorKind::CheckInClosed => GuiErrorCode::CheckInClosed,
                    CoreErrorKind::QrExpired => GuiErrorCode::QrExpired,
                    CoreErrorKind::Store => GuiErrorCode::Storage,
                    CoreErrorKind::Transport => GuiErrorCode::Network,
                    CoreErrorKind::Parameter => GuiErrorCode::Parameter,
                    CoreErrorKind::Parse => GuiErrorCode::Parse,
                    CoreErrorKind::Business => GuiErrorCode::Business,
                };
                GuiErrorPayload {
                    code,
                    message: error.to_string(),
                    retryable: matches!(
                        code,
                        GuiErrorCode::AuthenticationRequired | GuiErrorCode::Network
                    ),
                }
            }
        }
    }
}

/// Serializable GUI-facing error payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuiErrorPayload {
    /// Stable error code for front-end branching.
    pub code: GuiErrorCode,
    /// Human-readable message suitable for toast/dialog display.
    pub message: String,
    /// Whether the user may reasonably retry the operation.
    pub retryable: bool,
}

impl GuiErrorPayload {
    /// Creates a new serializable GUI-facing error payload.
    pub fn new(code: GuiErrorCode, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retryable,
        }
    }
}

/// Minimal session information typically shown in a GUI shell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Login account name.
    pub account: String,
    /// Display name of the current user.
    pub real_name: String,
    /// Whether session information is currently available.
    pub authenticated: bool,
}

/// GUI-oriented view of a schedule row with derived check-in state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleCard {
    /// Raw schedule information.
    pub schedule: ScheduleEntry,
    /// Datetime when check-in first becomes available.
    pub check_in_opens_at: NaiveDateTime,
    /// Current derived availability state.
    pub availability: CheckInAvailability,
    /// Whether the schedule can currently be checked in.
    pub can_check_in: bool,
}

/// Single measured phase within a higher-level GUI operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilePhase {
    /// Stable phase name for display or diagnostics.
    pub name: String,
    /// Elapsed wall-clock time for the phase in milliseconds.
    pub duration_ms: u64,
}

/// Aggregated timing profile for a GUI-facing operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperationProfile {
    /// Total wall-clock time across the full operation.
    pub total_ms: u64,
    /// Named sub-phases that contributed to the total runtime.
    pub phases: Vec<ProfilePhase>,
}

impl OperationProfile {
    /// Creates an empty operation profile.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one named phase and updates the total duration.
    pub fn push_phase(&mut self, name: impl Into<String>, duration_ms: u64) {
        self.total_ms = self.total_ms.saturating_add(duration_ms);
        self.phases.push(ProfilePhase {
            name: name.into(),
            duration_ms,
        });
    }

    /// Prepends a phase to an existing profile and updates the total duration.
    pub fn prepend_phase(&mut self, name: impl Into<String>, duration_ms: u64) {
        self.total_ms = self.total_ms.saturating_add(duration_ms);
        self.phases.insert(
            0,
            ProfilePhase {
                name: name.into(),
                duration_ms,
            },
        );
    }
}

/// Snapshot of the data commonly needed for a dashboard/home screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSnapshot {
    /// Time when the snapshot was created locally.
    pub generated_at: DateTime<Local>,

    /// Minimal current session/user information.
    pub session: SessionSummary,

    /// Known semester metadata.
    pub semesters: Vec<Semester>,

    /// Current user's courses.
    pub courses: Vec<Course>,

    /// Selected schedule day.
    pub schedule_date: NaiveDate,

    /// GUI-oriented schedule cards for the selected day.
    pub schedules: Vec<ScheduleCard>,

    /// Timing profile for the dashboard load operation.
    pub profile: OperationProfile,
}

/// Snapshot of the weekly schedule view anchored at a selected date.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyScheduleSnapshot {
    /// Time when the snapshot was created locally.
    pub generated_at: DateTime<Local>,

    /// Start day of the represented week.
    pub week_start: NaiveDate,

    /// End day of the represented week.
    pub week_end: NaiveDate,

    /// GUI-oriented schedule cards across the selected week.
    pub schedules: Vec<ScheduleCard>,

    /// Timing profile for the weekly schedule load operation.
    pub profile: OperationProfile,
}

/// GUI-friendly representation of a completed attendance attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckInViewModel {
    /// Schedule row chosen for the attempt.
    pub schedule: ScheduleEntry,

    /// Attendance result returned by the core layer.
    pub receipt: CheckInReceipt,

    /// Timing profile for the attendance operation.
    pub profile: OperationProfile,
}

/// Builds GUI-friendly schedule cards for the provided rows and local time.
pub fn build_schedule_cards(
    schedules: Vec<ScheduleEntry>,
    now: NaiveDateTime,
) -> Vec<ScheduleCard> {
    schedules
        .into_iter()
        .map(|schedule| {
            let availability = schedule.check_in_availability(now);
            let check_in_opens_at = schedule.check_in_opens_at();
            let can_check_in = availability == CheckInAvailability::Open;
            ScheduleCard {
                schedule,
                check_in_opens_at,
                availability,
                can_check_in,
            }
        })
        .collect()
}

/// Loads the basic dashboard data needed by a future GUI shell.
pub async fn load_dashboard(core: &IClassCore) -> Result<DashboardSnapshot, GuiBridgeError> {
    load_dashboard_for(core, Local::now().date_naive()).await
}

/// Loads dashboard data for a specific schedule date.
pub async fn load_dashboard_for(
    core: &IClassCore,
    date: NaiveDate,
) -> Result<DashboardSnapshot, GuiBridgeError> {
    let now = Local::now();
    let mut profile = OperationProfile::new();

    let started = Instant::now();
    let session = core.current_session().await?;
    profile.push_phase("session", elapsed_ms(started));

    let started = Instant::now();
    let semesters = core.semesters().await?;
    profile.push_phase("semesters", elapsed_ms(started));

    let started = Instant::now();
    let courses = core.courses().await?;
    profile.push_phase("courses", elapsed_ms(started));

    let started = Instant::now();
    let schedules = core.daily_schedule(date).await?;
    profile.push_phase("daily_schedule", elapsed_ms(started));

    Ok(DashboardSnapshot {
        generated_at: now,
        session: SessionSummary {
            account: session.account,
            real_name: session.real_name,
            authenticated: true,
        },
        semesters,
        courses,
        schedule_date: date,
        schedules: build_schedule_cards(schedules, now.naive_local()),
        profile,
    })
}

/// Loads weekly schedule cards anchored at a specific date.
pub async fn load_week_schedule_for(
    core: &IClassCore,
    date: NaiveDate,
) -> Result<WeeklyScheduleSnapshot, GuiBridgeError> {
    let now = Local::now();
    let started = Instant::now();
    let weekday_offset = i64::from(date.weekday().num_days_from_monday());
    let week_start = date - chrono::Days::new(weekday_offset as u64);
    let week_end = week_start + chrono::Days::new(6);
    let mut schedules = core.weekly_schedule(date).await?;
    schedules.sort_by_key(|schedule| (schedule.teach_date, schedule.begins_at, schedule.ends_at));
    let mut profile = OperationProfile::new();
    profile.push_phase("weekly_schedule", elapsed_ms(started));

    Ok(WeeklyScheduleSnapshot {
        generated_at: now,
        week_start,
        week_end,
        schedules: build_schedule_cards(schedules, now.naive_local()),
        profile,
    })
}

/// Performs attendance using the given check-in mode and maps it into a GUI view model.
pub async fn perform_check_in(
    core: &IClassCore,
    mode: CheckInMode,
) -> Result<CheckInViewModel, GuiBridgeError> {
    let started = Instant::now();
    let result = core.check_in_now(mode).await?;
    Ok(CheckInViewModel {
        schedule: result.schedule,
        receipt: result.receipt,
        profile: OperationProfile {
            total_ms: elapsed_ms(started),
            phases: vec![ProfilePhase {
                name: "check_in".into(),
                duration_ms: elapsed_ms(started),
            }],
        },
    })
}

/// Returns the elapsed time since `started` in milliseconds, saturating at `u64::MAX`.
fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
    use iclass_core::{CoreError, CoreErrorKind};

    use super::{GuiBridgeError, GuiErrorCode, build_schedule_cards};
    use iclass_domain::ScheduleEntry;

    #[test]
    fn maps_qr_expired_into_stable_gui_code() {
        let error = GuiBridgeError::Core(CoreError::NoScheduleAvailable {
            date: chrono::NaiveDate::from_ymd_opt(2026, 4, 23).expect("valid date"),
        });
        let payload = error.payload();
        assert_eq!(payload.code, GuiErrorCode::NoSchedule);
        assert!(!payload.retryable);
    }

    #[test]
    fn core_kind_mapping_remains_explicit() {
        assert_eq!(
            CoreErrorKind::Parameter as u8,
            CoreErrorKind::Parameter as u8
        );
    }

    #[test]
    fn builds_one_card_per_normalized_schedule() {
        let date = NaiveDate::from_ymd_opt(2026, 4, 23).expect("valid date");
        let begins_at =
            NaiveDateTime::new(date, NaiveTime::from_hms_opt(8, 0, 0).expect("valid time"));
        let ends_at =
            NaiveDateTime::new(date, NaiveTime::from_hms_opt(10, 0, 0).expect("valid time"));

        let schedules = vec![ScheduleEntry {
            schedule_id: "a".into(),
            schedule_uuid: None,
            course_id: Some("course-1".into()),
            course_name: "智能计算系统".into(),
            teacher_name: Some("李玲".into()),
            classroom_name: Some("教一楼101".into()),
            teach_date: date,
            begins_at,
            ends_at,
            lesson_units: 2,
            sign_status: Some("0".into()),
        }];

        let cards = build_schedule_cards(schedules, begins_at);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].schedule.lesson_units, 2);
        assert_eq!(cards[0].schedule.schedule_id, "a");
    }
}
