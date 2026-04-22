//! Tauri command handlers for GUI actions.

use chrono::{Local, NaiveDate};
use iclass_domain::Credentials;
use iclass_gui::{
    CheckInViewModel, DashboardSnapshot, GuiErrorCode, GuiErrorPayload, WeeklyScheduleSnapshot,
    load_dashboard_for, load_week_schedule_for,
};
use tauri::{AppHandle, State};
use tracing::{debug, info};

use crate::{
    desktop::{load_persisted_settings, read_autostart_enabled, write_autostart_enabled},
    models::{
        CheckInModePayload, CheckInRequest, DesktopSettingsPayload, LoginRequest,
        UpdateDesktopSettingsRequest,
    },
    settings::PersistedDesktopSettings,
    state::AppState,
};

/// Authenticates the user and returns the freshly loaded dashboard.
#[tauri::command]
pub(crate) async fn login(
    state: State<'_, AppState>,
    request: LoginRequest,
) -> Result<DashboardSnapshot, GuiErrorPayload> {
    info!(account = %request.account, "processing GUI login request");
    let credentials = Credentials {
        account: request.account,
        password: request.password,
    };

    state
        .core
        .login(&credentials, request.remember_password)
        .await
        .map_err(map_core_error)?;

    load_dashboard_for(&state.core, Local::now().date_naive())
        .await
        .map_err(map_gui_error)
}

/// Loads dashboard data for the requested date, defaulting to the current local day.
#[tauri::command]
pub(crate) async fn load_dashboard(
    state: State<'_, AppState>,
    date: Option<String>,
) -> Result<DashboardSnapshot, GuiErrorPayload> {
    let date = parse_date(date)?;
    debug!(%date, "loading dashboard snapshot");
    load_dashboard_for(&state.core, date)
        .await
        .map_err(map_gui_error)
}

/// Loads weekly schedule cards anchored at the requested date.
#[tauri::command]
pub(crate) async fn load_week_schedule(
    state: State<'_, AppState>,
    date: Option<String>,
) -> Result<WeeklyScheduleSnapshot, GuiErrorPayload> {
    let date = parse_date(date)?;
    debug!(%date, "loading weekly schedule snapshot");
    load_week_schedule_for(&state.core, date)
        .await
        .map_err(map_gui_error)
}

/// Attempts attendance for a selected schedule row.
#[tauri::command]
pub(crate) async fn check_in(
    state: State<'_, AppState>,
    request: CheckInRequest,
) -> Result<CheckInViewModel, GuiErrorPayload> {
    info!(
        schedule_id = %request.schedule.schedule_id,
        "processing GUI check-in request"
    );
    state
        .core
        .check_in_for_schedule(
            request.schedule,
            request.mode.unwrap_or(CheckInModePayload::Auto).into(),
            Local::now().timestamp(),
        )
        .await
        .map(|attempt| CheckInViewModel {
            schedule: attempt.schedule,
            receipt: attempt.receipt,
        })
        .map_err(map_core_error)
}

/// Returns current desktop integration settings such as tray-close behavior and autostart.
#[tauri::command]
pub(crate) async fn get_desktop_settings(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DesktopSettingsPayload, GuiErrorPayload> {
    debug!("reading desktop integration settings");
    let persisted = load_persisted_settings(&state);
    state.set_close_to_tray(cfg!(feature = "desktop-tray") && persisted.close_to_tray);

    Ok(DesktopSettingsPayload {
        autostart_enabled: read_autostart_enabled(&app)?,
        close_to_tray: state.close_to_tray(),
        autostart_available: cfg!(feature = "desktop-autostart"),
        close_to_tray_available: cfg!(feature = "desktop-tray"),
    })
}

/// Updates desktop integration settings and applies autostart changes immediately.
#[tauri::command]
pub(crate) async fn update_desktop_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    request: UpdateDesktopSettingsRequest,
) -> Result<DesktopSettingsPayload, GuiErrorPayload> {
    let close_to_tray = cfg!(feature = "desktop-tray") && request.close_to_tray;
    let autostart_enabled = cfg!(feature = "desktop-autostart") && request.autostart_enabled;

    info!(
        autostart_enabled,
        close_to_tray, "updating desktop integration settings"
    );
    write_autostart_enabled(&app, autostart_enabled)?;
    state.set_close_to_tray(close_to_tray);
    state
        .desktop_settings_store
        .save(&PersistedDesktopSettings {
            autostart_enabled,
            close_to_tray,
        })
        .map_err(|error| GuiErrorPayload::new(GuiErrorCode::Storage, error.to_string(), false))?;

    Ok(DesktopSettingsPayload {
        autostart_enabled: read_autostart_enabled(&app)?,
        close_to_tray: state.close_to_tray(),
        autostart_available: cfg!(feature = "desktop-autostart"),
        close_to_tray_available: cfg!(feature = "desktop-tray"),
    })
}

/// Clears the locally persisted session token.
#[tauri::command]
pub(crate) async fn logout(state: State<'_, AppState>) -> Result<(), GuiErrorPayload> {
    info!("clearing persisted GUI session");
    state
        .core
        .session_client()
        .store()
        .clear_session()
        .map_err(|error| GuiErrorPayload::new(GuiErrorCode::Storage, error.to_string(), false))
}

/// Parses an optional frontend date string.
fn parse_date(date: Option<String>) -> Result<NaiveDate, GuiErrorPayload> {
    match date {
        Some(raw) => NaiveDate::parse_from_str(&raw, "%Y-%m-%d").map_err(|_| {
            GuiErrorPayload::new(
                GuiErrorCode::Parameter,
                format!("无效日期 `{raw}`，请使用 YYYY-MM-DD 格式。"),
                false,
            )
        }),
        None => Ok(Local::now().date_naive()),
    }
}

/// Maps a GUI bridge error into the serialized frontend payload.
fn map_gui_error(error: iclass_gui::GuiBridgeError) -> GuiErrorPayload {
    error.payload()
}

/// Maps a core-layer error into the serialized frontend payload.
fn map_core_error(error: iclass_core::CoreError) -> GuiErrorPayload {
    iclass_gui::GuiBridgeError::Core(error).payload()
}
