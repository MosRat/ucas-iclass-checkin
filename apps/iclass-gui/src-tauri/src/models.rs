//! Request and response payloads used by the Tauri GUI commands.

use iclass_domain::{CheckInMode, ScheduleEntry};
use serde::{Deserialize, Serialize};

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
