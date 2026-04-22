//! Persistence for desktop-specific GUI preferences.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::models::CheckInModePayload;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Persisted desktop preference snapshot.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub(crate) struct PersistedDesktopSettings {
    /// Whether the app should register itself for system autostart.
    pub(crate) autostart_enabled: bool,
    /// Whether closing the main window should hide the app to the tray.
    pub(crate) close_to_tray: bool,
}

/// Errors produced by desktop settings persistence.
#[derive(Debug, Error)]
pub(crate) enum DesktopSettingsStoreError {
    /// Reading or writing the backing file failed.
    #[error("desktop settings store error at {path}: {message}")]
    Store {
        /// File or directory path involved in the failure.
        path: PathBuf,
        /// Human-readable error message.
        message: String,
    },
}

/// Persisted automation preference snapshot.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct PersistedAutomationSettings {
    /// Whether background auto check-in is enabled.
    pub(crate) auto_check_in_enabled: bool,
    /// Preferred check-in mode for background attempts.
    pub(crate) auto_check_in_mode: CheckInModePayload,
    /// Polling interval in seconds used by the background loop.
    pub(crate) auto_check_interval_seconds: u64,
}

impl Default for PersistedAutomationSettings {
    fn default() -> Self {
        Self {
            auto_check_in_enabled: false,
            auto_check_in_mode: CheckInModePayload::Auto,
            auto_check_interval_seconds: 30,
        }
    }
}

/// Errors produced by automation settings persistence.
#[derive(Debug, Error)]
pub(crate) enum AutomationSettingsStoreError {
    /// Reading or writing the backing file failed.
    #[error("automation settings store error at {path}: {message}")]
    Store {
        /// File or directory path involved in the failure.
        path: PathBuf,
        /// Human-readable error message.
        message: String,
    },
}

/// JSON-backed store for desktop preferences.
#[derive(Debug, Clone)]
pub(crate) struct DesktopSettingsStore {
    path: PathBuf,
}

impl Default for DesktopSettingsStore {
    fn default() -> Self {
        Self::new(default_store_path())
    }
}

impl DesktopSettingsStore {
    /// Creates a store backed by the given file path.
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Returns the backing file path used by this store.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Loads persisted desktop settings, returning defaults when the file does not exist.
    pub(crate) fn load(&self) -> Result<PersistedDesktopSettings, DesktopSettingsStoreError> {
        read_json_or_default(&self.path).map_err(|message| DesktopSettingsStoreError::Store {
            path: self.path.clone(),
            message,
        })
    }

    /// Saves desktop settings to disk, creating parent directories when needed.
    pub(crate) fn save(
        &self,
        settings: &PersistedDesktopSettings,
    ) -> Result<(), DesktopSettingsStoreError> {
        write_json_pretty(&self.path, settings).map_err(|message| {
            DesktopSettingsStoreError::Store {
                path: self.path.clone(),
                message,
            }
        })
    }
}

/// JSON-backed store for automation preferences.
#[derive(Debug, Clone)]
pub(crate) struct AutomationSettingsStore {
    path: PathBuf,
}

impl AutomationSettingsStore {
    /// Creates a store backed by the given file path.
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Loads persisted automation settings, returning defaults when the file does not exist.
    pub(crate) fn load(&self) -> Result<PersistedAutomationSettings, AutomationSettingsStoreError> {
        read_json_or_default(&self.path).map_err(|message| AutomationSettingsStoreError::Store {
            path: self.path.clone(),
            message,
        })
    }

    /// Saves automation settings to disk.
    pub(crate) fn save(
        &self,
        settings: &PersistedAutomationSettings,
    ) -> Result<(), AutomationSettingsStoreError> {
        write_json_pretty(&self.path, settings).map_err(|message| {
            AutomationSettingsStoreError::Store {
                path: self.path.clone(),
                message,
            }
        })
    }
}

/// Reads a JSON file into `T`, returning `T::default()` when the file does not exist.
fn read_json_or_default<T>(path: &Path) -> Result<T, String>
where
    T: for<'de> Deserialize<'de> + Default,
{
    if !path.exists() {
        return Ok(T::default());
    }

    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

/// Writes `value` as pretty JSON, creating parent directories when needed.
fn write_json_pretty<T>(path: &Path, value: &T) -> Result<(), String>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let content = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

/// Returns the default location for persisted desktop settings.
fn default_store_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("ucas-iclass-checkin")
        .join("desktop-settings.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_loads_default_settings() {
        let path = std::env::temp_dir().join(format!(
            "iclass-desktop-settings-missing-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        let store = DesktopSettingsStore::new(&path);

        let loaded = store.load().expect("missing file should yield defaults");

        assert!(!loaded.autostart_enabled);
        assert!(!loaded.close_to_tray);
    }

    #[test]
    fn store_round_trip_works() {
        let path = std::env::temp_dir().join(format!(
            "iclass-desktop-settings-{}.json",
            std::process::id()
        ));
        let store = DesktopSettingsStore::new(&path);
        let settings = PersistedDesktopSettings {
            autostart_enabled: true,
            close_to_tray: true,
        };

        store.save(&settings).expect("settings should save");
        let loaded = store.load().expect("settings should load");

        assert!(loaded.autostart_enabled);
        assert!(loaded.close_to_tray);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn automation_settings_round_trip_works() {
        let path = std::env::temp_dir().join(format!(
            "iclass-automation-settings-{}.json",
            std::process::id()
        ));
        let store = AutomationSettingsStore::new(&path);
        let settings = PersistedAutomationSettings {
            auto_check_in_enabled: true,
            auto_check_in_mode: CheckInModePayload::Id,
            auto_check_interval_seconds: 45,
        };

        store.save(&settings).expect("settings should save");
        let loaded = store.load().expect("settings should load");

        assert!(loaded.auto_check_in_enabled);
        assert_eq!(loaded.auto_check_in_mode, CheckInModePayload::Id);
        assert_eq!(loaded.auto_check_interval_seconds, 45);

        let _ = fs::remove_file(path);
    }
}
