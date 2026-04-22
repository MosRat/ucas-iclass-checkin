//! Tauri command handlers for GUI actions.

use std::time::Instant;

use chrono::{Local, NaiveDate};
use iclass_domain::Credentials;
use iclass_gui::{
    CheckInViewModel, DashboardSnapshot, GuiErrorCode, GuiErrorPayload, OperationProfile,
    ProfilePhase, WeeklyScheduleSnapshot, load_dashboard_for, load_week_schedule_for,
};
use tauri::{AppHandle, State};
use tracing::{debug, info};

use crate::{
    desktop::{load_persisted_settings, read_autostart_enabled, write_autostart_enabled},
    models::{
        AutomationSettingsPayload, CheckInModePayload, CheckInRequest, CustomCheckInRequest,
        DesktopSettingsPayload, LoginRequest, UpdateAutomationSettingsRequest,
        UpdateDesktopSettingsRequest,
    },
    settings::{PersistedAutomationSettings, PersistedDesktopSettings},
    state::AppState,
};

static DASHBOARD_REQUEST_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
static WEEK_SCHEDULE_REQUEST_ID: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1);

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
    let login_started = Instant::now();

    state
        .core
        .login(&credentials, request.remember_password)
        .await
        .map_err(map_core_error)?;
    let login_duration_ms = elapsed_ms(login_started);

    let mut dashboard = load_dashboard_for(&state.core, Local::now().date_naive())
        .await
        .map_err(map_gui_error)?;
    dashboard.profile.prepend_phase("login", login_duration_ms);
    info!(
        account = %dashboard.session.account,
        total_ms = dashboard.profile.total_ms,
        phases = %format_profile_phases(&dashboard.profile),
        "GUI login and dashboard sync finished"
    );
    Ok(dashboard)
}

/// Loads dashboard data for the requested date, defaulting to the current local day.
#[tauri::command]
pub(crate) async fn load_dashboard(
    state: State<'_, AppState>,
    date: Option<String>,
) -> Result<DashboardSnapshot, GuiErrorPayload> {
    let date = parse_date(date)?;
    let request_id = DASHBOARD_REQUEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    debug!(request_id, %date, "loading dashboard snapshot");
    let snapshot = load_dashboard_for(&state.core, date)
        .await
        .map_err(map_gui_error)?;
    info!(
        request_id,
        %date,
        total_ms = snapshot.profile.total_ms,
        phases = %format_profile_phases(&snapshot.profile),
        "dashboard snapshot loaded"
    );
    Ok(snapshot)
}

/// Loads weekly schedule cards anchored at the requested date.
#[tauri::command]
pub(crate) async fn load_week_schedule(
    state: State<'_, AppState>,
    date: Option<String>,
) -> Result<WeeklyScheduleSnapshot, GuiErrorPayload> {
    let date = parse_date(date)?;
    let request_id = WEEK_SCHEDULE_REQUEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    debug!(request_id, %date, "loading weekly schedule snapshot");
    let snapshot = load_week_schedule_for(&state.core, date)
        .await
        .map_err(map_gui_error)?;
    info!(
        request_id,
        %date,
        total_ms = snapshot.profile.total_ms,
        phases = %format_profile_phases(&snapshot.profile),
        "weekly schedule snapshot loaded"
    );
    Ok(snapshot)
}

/// Attempts attendance for a selected schedule row.
#[tauri::command]
pub(crate) async fn check_in(
    state: State<'_, AppState>,
    request: CheckInRequest,
) -> Result<CheckInViewModel, GuiErrorPayload> {
    let started = Instant::now();
    info!(
        schedule_id = %request.schedule.schedule_id,
        "processing GUI check-in request"
    );
    let mut view_model = state
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
            profile: OperationProfile::new(),
        })
        .map_err(map_core_error)?;
    let duration_ms = elapsed_ms(started);
    view_model.profile = OperationProfile {
        total_ms: duration_ms,
        phases: vec![ProfilePhase {
            name: "check_in".into(),
            duration_ms,
        }],
    };
    Ok(view_model)
}

/// Attempts attendance for a caller-supplied schedule ID or UUID.
#[tauri::command]
pub(crate) async fn check_in_custom(
    state: State<'_, AppState>,
    request: CustomCheckInRequest,
) -> Result<CheckInViewModel, GuiErrorPayload> {
    let started = Instant::now();
    info!(
        identifier = %request.identifier,
        mode = ?request.mode,
        "processing GUI custom check-in request"
    );
    let result = state
        .core
        .check_in_with_identifier(
            request.identifier.trim(),
            request.mode.into(),
            Local::now().timestamp(),
        )
        .await
        .map_err(map_core_error)?;
    let duration_ms = elapsed_ms(started);
    Ok(CheckInViewModel {
        schedule: result.schedule,
        receipt: result.receipt,
        profile: OperationProfile {
            total_ms: duration_ms,
            phases: vec![ProfilePhase {
                name: "custom_check_in".into(),
                duration_ms,
            }],
        },
    })
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

/// Returns current automation settings such as background auto check-in mode and interval.
#[tauri::command]
pub(crate) async fn get_automation_settings(
    state: State<'_, AppState>,
) -> Result<AutomationSettingsPayload, GuiErrorPayload> {
    let persisted = state
        .automation_settings_store
        .load()
        .map_err(|error| GuiErrorPayload::new(GuiErrorCode::Storage, error.to_string(), false))?;
    Ok(AutomationSettingsPayload {
        auto_check_in_enabled: persisted.auto_check_in_enabled,
        auto_check_interval_seconds: persisted.auto_check_interval_seconds,
        auto_check_in_mode: persisted.auto_check_in_mode,
    })
}

/// Updates background automation behavior.
#[tauri::command]
pub(crate) async fn update_automation_settings(
    state: State<'_, AppState>,
    request: UpdateAutomationSettingsRequest,
) -> Result<AutomationSettingsPayload, GuiErrorPayload> {
    let normalized = PersistedAutomationSettings {
        auto_check_in_enabled: request.auto_check_in_enabled,
        auto_check_in_mode: request.auto_check_in_mode,
        auto_check_interval_seconds: request.auto_check_interval_seconds.clamp(15, 300),
    };

    state
        .automation_settings_store
        .save(&normalized)
        .map_err(|error| GuiErrorPayload::new(GuiErrorCode::Storage, error.to_string(), false))?;

    Ok(AutomationSettingsPayload {
        auto_check_in_enabled: normalized.auto_check_in_enabled,
        auto_check_interval_seconds: normalized.auto_check_interval_seconds,
        auto_check_in_mode: normalized.auto_check_in_mode,
    })
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

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn format_profile_phases(profile: &OperationProfile) -> String {
    profile
        .phases
        .iter()
        .map(|phase| format!("{}={}ms", phase.name, phase.duration_ms))
        .collect::<Vec<_>>()
        .join(", ")
}
