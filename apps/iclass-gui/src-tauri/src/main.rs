//! Tauri desktop backend for the UCAS iCLASS GUI shell.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use chrono::{Local, NaiveDate};
use iclass_api::IClassApiClient;
use iclass_core::IClassCore;
use iclass_domain::{CheckInMode, Credentials, ScheduleEntry};
use iclass_gui::{
    CheckInViewModel, DashboardSnapshot, GuiErrorCode, GuiErrorPayload, WeeklyScheduleSnapshot,
    load_dashboard_for, load_week_schedule_for,
};
use iclass_session::{SessionClient, SessionStore};
#[cfg(feature = "allocator-mimalloc")]
use mimalloc::MiMalloc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow, WindowEvent};
#[cfg(feature = "desktop-tray")]
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
#[cfg(all(
    feature = "desktop-autostart",
    any(target_os = "macos", target_os = "linux", windows)
))]
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

/// Global allocator selection for the desktop GUI binary.
#[cfg(feature = "allocator-mimalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

/// Application-wide shared state for Tauri commands.
#[derive(Clone)]
struct AppState {
    core: IClassCore,
    close_to_tray: Arc<AtomicBool>,
    allow_exit: Arc<AtomicBool>,
}

impl AppState {
    /// Creates the shared GUI application state.
    fn new() -> Self {
        let api = IClassApiClient::default();
        let store = SessionStore::default();
        let session_client = SessionClient::new(api, store);
        let core = IClassCore::new(session_client);
        Self {
            core,
            close_to_tray: Arc::new(AtomicBool::new(false)),
            allow_exit: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns whether close requests should minimize the app to the tray.
    fn close_to_tray(&self) -> bool {
        self.close_to_tray.load(Ordering::Relaxed)
    }

    /// Updates whether close requests should minimize the app to the tray.
    fn set_close_to_tray(&self, value: bool) {
        self.close_to_tray.store(value, Ordering::Relaxed);
    }

    /// Marks the app as allowed to fully exit.
    fn allow_exit(&self) {
        self.allow_exit.store(true, Ordering::Relaxed);
    }

    /// Returns whether a full exit has been explicitly requested.
    fn exit_allowed(&self) -> bool {
        self.allow_exit.load(Ordering::Relaxed)
    }
}

/// Login form payload sent by the frontend.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginRequest {
    /// UCAS iCLASS account.
    account: String,
    /// UCAS iCLASS password.
    password: String,
    /// Whether the password should be persisted for auto login.
    remember_password: bool,
}

/// Check-in request sent by the frontend.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckInRequest {
    /// Selected schedule row.
    schedule: ScheduleEntry,
    /// Requested check-in mode.
    mode: Option<CheckInModePayload>,
}

/// GUI-facing attendance mode representation.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum CheckInModePayload {
    /// Prefer UUID mode and fall back to ID mode.
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

/// Serialized desktop integration state exposed to the frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopSettingsPayload {
    /// Whether the app is currently registered for autostart.
    autostart_enabled: bool,
    /// Whether closing the main window should hide it to the system tray.
    close_to_tray: bool,
    /// Whether autostart support is compiled into this build.
    autostart_available: bool,
    /// Whether tray-close behavior is compiled into this build.
    close_to_tray_available: bool,
}

/// Frontend request for updating desktop integration behavior.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateDesktopSettingsRequest {
    /// Desired autostart state.
    autostart_enabled: bool,
    /// Whether close actions should hide the window to tray.
    close_to_tray: bool,
}

/// Authenticates the user and returns the freshly loaded dashboard.
#[tauri::command]
async fn login(
    state: State<'_, AppState>,
    request: LoginRequest,
) -> Result<DashboardSnapshot, GuiErrorPayload> {
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
async fn load_dashboard(
    state: State<'_, AppState>,
    date: Option<String>,
) -> Result<DashboardSnapshot, GuiErrorPayload> {
    let date = parse_date(date)?;
    load_dashboard_for(&state.core, date)
        .await
        .map_err(map_gui_error)
}

/// Loads weekly schedule cards anchored at the requested date.
#[tauri::command]
async fn load_week_schedule(
    state: State<'_, AppState>,
    date: Option<String>,
) -> Result<WeeklyScheduleSnapshot, GuiErrorPayload> {
    let date = parse_date(date)?;
    load_week_schedule_for(&state.core, date)
        .await
        .map_err(map_gui_error)
}

/// Attempts attendance for a selected schedule row.
#[tauri::command]
async fn check_in(
    state: State<'_, AppState>,
    request: CheckInRequest,
) -> Result<CheckInViewModel, GuiErrorPayload> {
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
async fn get_desktop_settings(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DesktopSettingsPayload, GuiErrorPayload> {
    Ok(DesktopSettingsPayload {
        autostart_enabled: read_autostart_enabled(&app)?,
        close_to_tray: state.close_to_tray(),
        autostart_available: cfg!(feature = "desktop-autostart"),
        close_to_tray_available: cfg!(feature = "desktop-tray"),
    })
}

/// Updates desktop integration settings and applies autostart changes immediately.
#[tauri::command]
async fn update_desktop_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    request: UpdateDesktopSettingsRequest,
) -> Result<DesktopSettingsPayload, GuiErrorPayload> {
    state.set_close_to_tray(cfg!(feature = "desktop-tray") && request.close_to_tray);
    write_autostart_enabled(&app, request.autostart_enabled)?;

    Ok(DesktopSettingsPayload {
        autostart_enabled: read_autostart_enabled(&app)?,
        close_to_tray: state.close_to_tray(),
        autostart_available: cfg!(feature = "desktop-autostart"),
        close_to_tray_available: cfg!(feature = "desktop-tray"),
    })
}

/// Clears the locally persisted session token.
#[tauri::command]
async fn logout(state: State<'_, AppState>) -> Result<(), GuiErrorPayload> {
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

/// Reads the current autostart state from the platform integration plugin.
fn read_autostart_enabled(app: &AppHandle) -> Result<bool, GuiErrorPayload> {
    #[cfg(all(
        feature = "desktop-autostart",
        any(target_os = "macos", target_os = "linux", windows)
    ))]
    {
        app.autolaunch().is_enabled().map_err(|error| {
            GuiErrorPayload::new(
                GuiErrorCode::Business,
                format!("读取开机自启状态失败：{error}"),
                false,
            )
        })
    }

    #[cfg(not(all(
        feature = "desktop-autostart",
        any(target_os = "macos", target_os = "linux", windows)
    )))]
    {
        let _ = app;
        Ok(false)
    }
}

/// Updates the platform autostart registration to match the requested state.
fn write_autostart_enabled(app: &AppHandle, enabled: bool) -> Result<(), GuiErrorPayload> {
    #[cfg(all(
        feature = "desktop-autostart",
        any(target_os = "macos", target_os = "linux", windows)
    ))]
    {
        let manager = app.autolaunch();
        let currently_enabled = manager.is_enabled().map_err(|error| {
            GuiErrorPayload::new(
                GuiErrorCode::Business,
                format!("读取开机自启状态失败：{error}"),
                false,
            )
        })?;

        if currently_enabled == enabled {
            return Ok(());
        }

        let result = if enabled {
            manager.enable()
        } else {
            manager.disable()
        };

        match result {
            Ok(()) => Ok(()),
            Err(error) if !enabled && is_missing_autostart_entry_error(&error.to_string()) => {
                Ok(())
            }
            Err(error) => Err(GuiErrorPayload::new(
                GuiErrorCode::Business,
                format!("更新开机自启状态失败：{error}"),
                false,
            )),
        }
    }

    #[cfg(not(all(
        feature = "desktop-autostart",
        any(target_os = "macos", target_os = "linux", windows)
    )))]
    {
        let _ = (app, enabled);
        Ok(())
    }
}

/// Returns whether the autostart backend reported that no existing registration was found.
fn is_missing_autostart_entry_error(message: &str) -> bool {
    message.contains("os error 2")
        || message.contains("The system cannot find the file specified")
        || message.contains("cannot find the file specified")
}

/// Shows the main window and restores focus from the system tray.
fn show_main_window(window: &WebviewWindow) {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

/// Hides the main window and keeps the process alive in the tray.
fn hide_main_window(window: &WebviewWindow) {
    let _ = window.hide();
}

/// Emits a frontend event hinting that the app was hidden to the tray.
fn emit_tray_hidden(app: &AppHandle) {
    let _ = app.emit("desktop://tray-hidden", ());
}

/// Builds the tray icon and its basic menu actions.
#[cfg(feature = "desktop-tray")]
fn setup_tray(app: &AppHandle, state: &AppState) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, "hide", "隐藏到托盘", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &hide_item, &quit_item])?;
    let state = state.clone();

    TrayIconBuilder::with_id("main-tray")
        .icon(
            app.default_window_icon()
                .expect("default tray icon should be available")
                .clone(),
        )
        .tooltip("UCAS iCLASS")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            if let Some(window) = app.get_webview_window("main") {
                match event.id.as_ref() {
                    "show" => show_main_window(&window),
                    "hide" => {
                        hide_main_window(&window);
                        emit_tray_hidden(app);
                    }
                    "quit" => {
                        state.allow_exit();
                        app.exit(0);
                    }
                    _ => {}
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let Some(window) = tray.app_handle().get_webview_window("main") {
                match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
                    | TrayIconEvent::DoubleClick {
                        button: MouseButton::Left,
                        ..
                    } => show_main_window(&window),
                    _ => {}
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// Builds the tray icon and its menu actions when tray support is disabled.
#[cfg(not(feature = "desktop-tray"))]
fn setup_tray(_app: &AppHandle, _state: &AppState) -> tauri::Result<()> {
    Ok(())
}

/// Hooks window events so close requests can minimize to the tray instead of terminating.
fn setup_main_window(window: &WebviewWindow, state: AppState) {
    let app = window.app_handle().clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event
            && state.close_to_tray()
            && !state.exit_allowed()
        {
            api.prevent_close();
            if let Some(window) = app.get_webview_window("main") {
                hide_main_window(&window);
                emit_tray_hidden(&app);
            }
        }
    });
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let state = AppState::new();
    let mut builder = tauri::Builder::default();

    #[cfg(all(
        feature = "desktop-autostart",
        any(target_os = "macos", target_os = "linux", windows)
    ))]
    {
        builder = builder.plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ));
    }

    builder
        .setup({
            let state = state.clone();
            move |app| {
                setup_tray(app.handle(), &state)?;

                if let Some(window) = app.get_webview_window("main") {
                    setup_main_window(&window, state.clone());
                }

                Ok(())
            }
        })
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            login,
            load_dashboard,
            load_week_schedule,
            check_in,
            get_desktop_settings,
            update_desktop_settings,
            logout
        ])
        .run(tauri::generate_context!())
        .expect("failed to run tauri application");
}
