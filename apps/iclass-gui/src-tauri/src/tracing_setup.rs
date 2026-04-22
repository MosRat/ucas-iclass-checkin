//! Tracing integration for the desktop backend.

use tauri_plugin_tracing::{Builder, LevelFilter};

/// Builds the tracing plugin with a default subscriber and rotating file logging.
pub(crate) fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let max_level = if cfg!(debug_assertions) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    Builder::new()
        .with_max_level(max_level)
        .with_file_logging()
        .with_default_subscriber()
        .build()
}
