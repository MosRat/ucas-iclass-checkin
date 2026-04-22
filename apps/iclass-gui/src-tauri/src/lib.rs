//! Tauri desktop backend library for the UCAS iCLASS GUI shell.

mod automation;
mod commands;
mod desktop;
mod models;
mod settings;
mod state;
mod tracing_setup;

use iclass_session::SessionStore;
use settings::DesktopSettingsStore;
use state::AppState;
use tauri::Manager;

/// Runs the Tauri desktop application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tracing_setup::plugin());

    #[cfg(all(
        feature = "desktop-autostart",
        any(target_os = "macos", target_os = "linux", windows)
    ))]
    let builder = builder.plugin(tauri_plugin_autostart::init(
        tauri_plugin_autostart::MacosLauncher::LaunchAgent,
        None::<Vec<&str>>,
    ));

    builder
        .setup({
            move |app| {
                let config_dir = app.path().app_config_dir()?;
                let session_store = SessionStore::new(config_dir.join("session.json"));
                let desktop_settings_store =
                    DesktopSettingsStore::new(config_dir.join("desktop-settings.json"));
                let automation_settings_store = settings::AutomationSettingsStore::new(
                    config_dir.join("automation-settings.json"),
                );
                let state = AppState::new(
                    session_store,
                    desktop_settings_store,
                    automation_settings_store,
                );

                app.manage(state.clone());
                desktop::restore_desktop_settings(app.handle(), &state);
                desktop::setup_tray(app.handle(), &state)?;
                automation::spawn_auto_check_loop(state.clone());

                #[cfg(desktop)]
                if let Some(window) = app.get_webview_window("main") {
                    desktop::setup_main_window(&window, state.clone());
                }

                Ok(())
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::login,
            commands::load_dashboard,
            commands::load_week_schedule,
            commands::check_in,
            commands::check_in_custom,
            commands::get_desktop_settings,
            commands::update_desktop_settings,
            commands::get_automation_settings,
            commands::update_automation_settings,
            commands::logout
        ])
        .run(tauri::generate_context!())
        .expect("failed to run tauri application");
}
