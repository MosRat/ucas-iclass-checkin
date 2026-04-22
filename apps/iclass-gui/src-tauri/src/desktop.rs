//! Desktop integration helpers for tray, autostart, and window events.

use iclass_gui::GuiErrorPayload;

#[cfg(all(
    feature = "desktop-autostart",
    any(target_os = "macos", target_os = "linux", windows)
))]
use iclass_gui::GuiErrorCode;
use tauri::{AppHandle, Emitter};
use tracing::warn;

#[cfg(desktop)]
use tauri::{Manager, WebviewWindow, WindowEvent};

#[cfg(all(desktop, feature = "desktop-tray"))]
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

#[cfg(all(
    feature = "desktop-autostart",
    any(target_os = "macos", target_os = "linux", windows)
))]
use tauri_plugin_autostart::ManagerExt;

use crate::settings::PersistedDesktopSettings;
use crate::state::AppState;

/// Loads persisted desktop settings from disk, falling back to defaults when the store is unreadable.
pub(crate) fn load_persisted_settings(state: &AppState) -> PersistedDesktopSettings {
    match state.desktop_settings_store.load() {
        Ok(settings) => settings,
        Err(error) => {
            warn!(
                store = %state.desktop_settings_store.path().display(),
                error = %error,
                "failed to load persisted desktop settings; using defaults"
            );
            PersistedDesktopSettings::default()
        }
    }
}

/// Restores persisted desktop preferences into runtime state and platform integrations.
pub(crate) fn restore_desktop_settings(app: &AppHandle, state: &AppState) {
    let persisted = load_persisted_settings(state);
    state.set_close_to_tray(cfg!(feature = "desktop-tray") && persisted.close_to_tray);

    if let Err(error) = write_autostart_enabled(app, persisted.autostart_enabled) {
        warn!(error = %error.message, "failed to restore autostart preference");
    }
}

/// Reads the current autostart state from the platform integration plugin.
pub(crate) fn read_autostart_enabled(app: &AppHandle) -> Result<bool, GuiErrorPayload> {
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
pub(crate) fn write_autostart_enabled(
    app: &AppHandle,
    enabled: bool,
) -> Result<(), GuiErrorPayload> {
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
#[cfg(all(
    feature = "desktop-autostart",
    any(target_os = "macos", target_os = "linux", windows)
))]
fn is_missing_autostart_entry_error(message: &str) -> bool {
    message.contains("os error 2")
        || message.contains("The system cannot find the file specified")
        || message.contains("cannot find the file specified")
}

/// Shows the main window and restores focus from the system tray.
#[cfg(desktop)]
fn show_main_window(window: &WebviewWindow) {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

/// Hides the main window and keeps the process alive in the tray.
#[cfg(desktop)]
fn hide_main_window(window: &WebviewWindow) {
    let _ = window.hide();
}

/// Emits a frontend event hinting that the app was hidden to the tray.
#[cfg(desktop)]
fn emit_tray_hidden(app: &AppHandle) {
    let _ = app.emit("desktop://tray-hidden", ());
}

/// Builds the tray icon and its basic menu actions.
#[cfg(all(desktop, feature = "desktop-tray"))]
pub(crate) fn setup_tray(app: &AppHandle, state: &AppState) -> tauri::Result<()> {
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
#[cfg(not(all(desktop, feature = "desktop-tray")))]
pub(crate) fn setup_tray(_app: &AppHandle, _state: &AppState) -> tauri::Result<()> {
    Ok(())
}

/// Hooks window events so close requests can minimize to the tray instead of terminating.
#[cfg(desktop)]
pub(crate) fn setup_main_window(window: &WebviewWindow, state: AppState) {
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
