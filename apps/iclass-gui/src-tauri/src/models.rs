//! Request and response payloads used by the Tauri GUI commands.

use chrono::Local;
use iclass_domain::CheckInAvailability;
use iclass_domain::{CheckInMode, ScheduleEntry};
use serde::{Deserialize, Serialize};

use crate::{
    settings::PersistedAutomationSettings,
    state::{AutoCheckLastAction, AutoCheckStatus, AutoCheckStatusKind},
};

/// Serialized summary of the latest background auto check-in action.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutoCheckLastActionPayload {
    /// Local timestamp when the action was attempted.
    pub(crate) attempted_at: String,
    /// Schedule identifier targeted by the background worker.
    pub(crate) schedule_id: String,
    /// Human-readable course name.
    pub(crate) course_name: String,
    /// Whether the latest action completed successfully.
    pub(crate) succeeded: bool,
    /// Human-readable result message for the latest action.
    pub(crate) message: String,
}

/// Stable GUI-facing state of the background auto check-in worker.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AutoCheckStatusKindPayload {
    /// Worker is enabled but currently has no actionable course.
    Idle,
    /// A candidate course exists, but its check-in window is not open yet.
    WaitingWindow,
    /// A candidate course is in range and ready for a future attempt.
    Ready,
    /// The worker is currently submitting a check-in attempt.
    Attempting,
    /// The most recent automatic attempt completed successfully.
    Success,
    /// The most recent automatic attempt failed.
    Error,
}

impl From<AutoCheckStatusKind> for AutoCheckStatusKindPayload {
    fn from(value: AutoCheckStatusKind) -> Self {
        match value {
            AutoCheckStatusKind::Idle => Self::Idle,
            AutoCheckStatusKind::WaitingWindow => Self::WaitingWindow,
            AutoCheckStatusKind::Ready => Self::Ready,
            AutoCheckStatusKind::Attempting => Self::Attempting,
            AutoCheckStatusKind::Success => Self::Success,
            AutoCheckStatusKind::Error => Self::Error,
        }
    }
}

/// Serialized snapshot of the current auto check-in candidate and worker state.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutoCheckCurrentStatusPayload {
    /// Local timestamp when the snapshot was refreshed.
    pub(crate) updated_at: String,
    /// Stable worker state for frontend rendering.
    pub(crate) status: AutoCheckStatusKindPayload,
    /// Human-readable status message.
    pub(crate) message: String,
    /// Candidate schedule currently being observed by the worker, if any.
    pub(crate) schedule: Option<ScheduleEntry>,
    /// Derived candidate availability, if a schedule exists.
    pub(crate) availability: Option<CheckInAvailability>,
    /// When the worker believes check-in first becomes available.
    pub(crate) check_in_opens_at: Option<String>,
    /// Whether the candidate is currently eligible for check-in.
    pub(crate) can_check_in: bool,
    /// Whether the candidate is already marked as signed in.
    pub(crate) is_signed_in: bool,
}

/// Login form payload sent by the frontend.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoginRequest {
    /// UCAS iCLASS account.
    pub(crate) account: String,
    /// UCAS iCLASS password.
    pub(crate) password: String,
    /// Whether the password should be persisted for auto login.
    pub(crate) remember_password: bool,
}

/// Check-in request sent by the frontend.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CheckInRequest {
    /// Selected schedule row.
    pub(crate) schedule: ScheduleEntry,
    /// Requested check-in mode.
    pub(crate) mode: Option<CheckInModePayload>,
}

/// GUI-facing attendance mode representation.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CheckInModePayload {
    /// Prefer UUID mode and fall back to ID mode.
    #[default]
    Auto,
    /// Require UUID mode.
    Uuid,
    /// Require ID mode.
    Id,
}

impl From<CheckInModePayload> for CheckInMode {
    fn from(value: CheckInModePayload) -> Self {
        match value {
            CheckInModePayload::Auto => Self::Auto,
            CheckInModePayload::Uuid => Self::ByUuid,
            CheckInModePayload::Id => Self::ById,
        }
    }
}

/// Custom/manual check-in request sent by the frontend.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CustomCheckInRequest {
    /// Caller-supplied schedule ID or UUID.
    pub(crate) identifier: String,
    /// Explicit attendance mode to use for the identifier.
    pub(crate) mode: CheckInModePayload,
}

/// Serialized desktop integration state exposed to the frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopSettingsPayload {
    /// Whether the app is currently registered for autostart.
    pub(crate) autostart_enabled: bool,
    /// Whether closing the main window should hide it to the system tray.
    pub(crate) close_to_tray: bool,
    /// Whether autostart support is compiled into this build.
    pub(crate) autostart_available: bool,
    /// Whether tray-close behavior is compiled into this build.
    pub(crate) close_to_tray_available: bool,
}

/// Frontend request for updating desktop integration behavior.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateDesktopSettingsRequest {
    /// Desired autostart state.
    pub(crate) autostart_enabled: bool,
    /// Whether close actions should hide the window to tray.
    pub(crate) close_to_tray: bool,
}

/// Serialized automation settings exposed to the frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutomationSettingsPayload {
    /// Whether background auto check-in is enabled.
    pub(crate) auto_check_in_enabled: bool,
    /// Polling interval in seconds for background auto check-in.
    pub(crate) auto_check_interval_seconds: u64,
    /// Preferred check-in mode for background attempts.
    pub(crate) auto_check_in_mode: CheckInModePayload,
    /// Most recent background auto check-in action, if any.
    pub(crate) last_auto_check_action: Option<AutoCheckLastActionPayload>,
    /// Current worker/candidate status exposed to the frontend.
    pub(crate) current_status: AutoCheckCurrentStatusPayload,
}

/// Frontend request for updating background automation behavior.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateAutomationSettingsRequest {
    /// Whether background auto check-in is enabled.
    pub(crate) auto_check_in_enabled: bool,
    /// Polling interval in seconds for background auto check-in.
    pub(crate) auto_check_interval_seconds: u64,
    /// Preferred check-in mode for background attempts.
    pub(crate) auto_check_in_mode: CheckInModePayload,
}

/// Builds the GUI payload returned by automation-related commands and events.
pub(crate) fn build_automation_settings_payload(
    settings: PersistedAutomationSettings,
    last_action: Option<AutoCheckLastAction>,
    current_status: AutoCheckStatus,
) -> AutomationSettingsPayload {
    AutomationSettingsPayload {
        auto_check_in_enabled: settings.auto_check_in_enabled,
        auto_check_interval_seconds: settings.auto_check_interval_seconds,
        auto_check_in_mode: settings.auto_check_in_mode,
        last_auto_check_action: last_action.map(|action| AutoCheckLastActionPayload {
            attempted_at: action.attempted_at.to_rfc3339(),
            schedule_id: action.schedule_id,
            course_name: action.course_name,
            succeeded: action.succeeded,
            message: action.message,
        }),
        current_status: build_auto_check_current_status_payload(current_status),
    }
}

fn build_auto_check_current_status_payload(
    current_status: AutoCheckStatus,
) -> AutoCheckCurrentStatusPayload {
    let current_schedule = current_status.schedule;
    let reference_time = current_status.updated_at.naive_local();
    let availability = current_schedule
        .as_ref()
        .map(|schedule| schedule.check_in_availability(reference_time));
    let can_check_in = current_schedule.as_ref().is_some_and(|schedule| {
        schedule.can_check_in_at(reference_time) && !schedule.is_signed_in()
    });
    let is_signed_in = current_schedule
        .as_ref()
        .is_some_and(ScheduleEntry::is_signed_in);
    let check_in_opens_at = current_schedule
        .as_ref()
        .and_then(|schedule| {
            schedule
                .check_in_opens_at()
                .and_local_timezone(Local)
                .single()
        })
        .map(|value| value.to_rfc3339());

    AutoCheckCurrentStatusPayload {
        updated_at: current_status.updated_at.to_rfc3339(),
        status: current_status.kind.into(),
        message: current_status.message,
        schedule: current_schedule,
        availability,
        check_in_opens_at,
        can_check_in,
        is_signed_in,
    }
}
