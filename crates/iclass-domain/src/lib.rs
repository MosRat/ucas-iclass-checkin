//! Shared domain models used across the iCLASS API, session, and core layers.

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

/// Default UCAS iCLASS service base URL.
pub const UCAS_DEFAULT_BASE_URL: &str = "https://iclass.ucas.edu.cn:8181";

/// Compact date format expected by schedule-related API parameters.
pub const API_DATE_FORMAT: &str = "%Y%m%d";

/// Calendar day format returned by several API payload fields.
pub const API_DAY_FORMAT: &str = "%Y-%m-%d";

/// Local datetime format returned by schedule payload fields.
pub const API_DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// Raw login credentials used to obtain or refresh an authenticated session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credentials {
    /// Student account / phone field submitted to the login endpoint.
    pub account: String,

    /// Plain-text password submitted to the login endpoint.
    pub password: String,
}

/// Authenticated session material extracted from the login response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// Stable UCAS iCLASS user identifier.
    pub user_id: String,

    /// Session token that must be sent as the `sessionId` header.
    pub session_id: String,

    /// Login account name returned by the service.
    pub account: String,

    /// User-friendly display name for CLI or GUI presentation.
    pub real_name: String,

    /// Optional class identifier returned by login.
    pub class_id: Option<String>,

    /// Optional class display name returned by login.
    pub class_name: Option<String>,

    /// Optional class UUID returned by login.
    pub class_uuid: Option<String>,

    /// Optional avatar URL returned by login.
    pub avatar_url: Option<String>,

    /// Timestamp recording when this session was last refreshed locally.
    pub refreshed_at: DateTime<Utc>,
}

/// Simplified semester metadata used by higher-level query flows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Semester {
    /// Semester code used by the upstream system.
    pub code: String,

    /// Human-readable semester name.
    pub name: String,

    /// Semester start date.
    pub begin_date: NaiveDate,

    /// Semester end date.
    pub end_date: NaiveDate,

    /// Whether the semester is marked as current by the upstream service.
    pub current: bool,
}

/// Minimal course information needed by the current CLI and future GUI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Course {
    /// Course identifier.
    pub id: String,

    /// Display name of the course.
    pub name: String,

    /// Optional course number / catalog identifier.
    pub course_num: Option<String>,

    /// Optional teacher name associated with the course.
    pub teacher_name: Option<String>,

    /// Optional classroom name.
    pub classroom_name: Option<String>,

    /// Optional first teaching date.
    pub begin_date: Option<NaiveDate>,

    /// Optional last teaching date.
    pub end_date: Option<NaiveDate>,

    /// Optional count of pending sign-ins reported by the service.
    pub pending_checkins: Option<u32>,
}

/// A single teaching schedule row that may be used for attendance check-in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleEntry {
    /// Numeric schedule identifier used by ID-based check-in.
    pub schedule_id: String,

    /// UUID-style schedule identifier used by UUID-based check-in.
    pub schedule_uuid: Option<String>,

    /// Related course identifier when present.
    pub course_id: Option<String>,

    /// Display name of the course.
    pub course_name: String,

    /// Optional teacher name.
    pub teacher_name: Option<String>,

    /// Optional classroom name.
    pub classroom_name: Option<String>,

    /// Teaching day for the row.
    pub teach_date: NaiveDate,

    /// Inclusive start time of the class session.
    pub begins_at: NaiveDateTime,

    /// Inclusive end time of the class session.
    pub ends_at: NaiveDateTime,

    /// Number of class periods represented by this logical lesson row.
    pub lesson_units: u16,

    /// Raw sign status string returned by the API.
    pub sign_status: Option<String>,
}

/// Derived availability state for a schedule's check-in window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckInAvailability {
    /// It is too early to attempt attendance.
    NotOpenYet,
    /// Attendance may be attempted now.
    Open,
    /// The class session has already ended.
    Closed,
}

impl ScheduleEntry {
    /// Returns `true` when the upstream schedule row already indicates a completed sign-in.
    pub fn is_signed_in(&self) -> bool {
        self.sign_status.as_deref() == Some("1")
    }

    /// Returns the local datetime when attendance becomes available for this schedule.
    ///
    /// The current GUI policy allows check-in beginning 30 minutes before the class starts.
    pub fn check_in_opens_at(&self) -> NaiveDateTime {
        self.begins_at - Duration::minutes(30)
    }

    /// Returns `true` when the row contains a UUID usable for check-in.
    pub fn supports_uuid_checkin(&self) -> bool {
        self.schedule_uuid.is_some()
    }

    /// Returns `true` when the row contains an ID usable for check-in.
    pub fn supports_id_checkin(&self) -> bool {
        !self.schedule_id.is_empty()
    }

    /// Returns whether `now` is within the schedule's inclusive class window.
    pub fn is_active_at(&self, now: NaiveDateTime) -> bool {
        self.begins_at <= now && now <= self.ends_at
    }

    /// Returns the current check-in availability state for `now`.
    pub fn check_in_availability(&self, now: NaiveDateTime) -> CheckInAvailability {
        if now < self.check_in_opens_at() {
            CheckInAvailability::NotOpenYet
        } else if now > self.ends_at {
            CheckInAvailability::Closed
        } else {
            CheckInAvailability::Open
        }
    }

    /// Returns whether attendance may be attempted at `now`.
    pub fn can_check_in_at(&self, now: NaiveDateTime) -> bool {
        self.check_in_availability(now) == CheckInAvailability::Open
    }

    /// Returns the absolute distance in seconds between `now` and the active class window.
    ///
    /// When `now` is inside the window, the distance is zero.
    pub fn distance_seconds(&self, now: NaiveDateTime) -> i64 {
        if self.is_active_at(now) {
            0
        } else if now < self.begins_at {
            (self.begins_at - now).num_seconds()
        } else {
            (now - self.ends_at).num_seconds()
        }
    }
}

/// Preferred strategy for choosing which upstream check-in parameter to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckInMode {
    /// Prefer UUID-based check-in and fall back to ID-based check-in.
    Auto,

    /// Require UUID-based check-in.
    ByUuid,

    /// Require ID-based check-in.
    ById,
}

/// The concrete check-in method that was actually used for a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckInMethod {
    /// The `timeTableId` / UUID variant.
    Uuid,

    /// The `courseSchedId` / numeric ID variant.
    Id,
}

/// Business-level result of a check-in request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckInReceipt {
    /// Actual method used by the request.
    pub method: CheckInMethod,

    /// Optional upstream attendance record identifier.
    pub record_id: Option<String>,

    /// Whether the service marked the check-in as successful.
    pub signed_in: bool,

    /// Upstream status code preserved for display or logging.
    pub status_code: String,

    /// Whether a follow-up schedule refresh confirmed the row as signed in.
    pub verified_signed_in: Option<bool>,

    /// Observed schedule sign status from the verification refresh, when available.
    pub observed_sign_status: Option<String>,
}

/// Combined result of choosing a schedule row and attempting attendance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckInAttempt {
    /// Schedule row used for the attempt.
    pub schedule: ScheduleEntry,

    /// Result returned by the attendance endpoint.
    pub receipt: CheckInReceipt,
}

/// Formats a date using the compact UCAS API request representation.
pub fn format_api_date(date: NaiveDate) -> String {
    date.format(API_DATE_FORMAT).to_string()
}
