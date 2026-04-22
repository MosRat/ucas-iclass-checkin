//! Tauri desktop backend library for the UCAS iCLASS GUI shell.

mod commands;
mod desktop;
mod models;
mod state;
mod tracing_setup;

use state::AppState;
use tauri::Manager;

/// Runs the Tauri desktop application.
pub fn run() {
    tracing_setup::init();

    let state = AppState::new();
    let mut builder = tauri::Builder::default();

    #[cfg(all(
        feature = "desktop-autostart",
        any(target_os = "macos", target_os = "linux", windows)
    ))]
    {
        builder = builder.plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ));
    }

    builder
        .setup({
            let state = state.clone();
            move |app| {
                desktop::setup_tray(app.handle(), &state)?;

                if let Some(window) = app.get_webview_window("main") {
                    desktop::setup_main_window(&window, state.clone());
                }

                Ok(())
            }
        })
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::login,
            commands::load_dashboard,
            commands::load_week_schedule,
            commands::check_in,
            commands::get_desktop_settings,
            commands::update_desktop_settings,
            commands::logout
        ])
        .run(tauri::generate_context!())
        .expect("failed to run tauri application");
}
