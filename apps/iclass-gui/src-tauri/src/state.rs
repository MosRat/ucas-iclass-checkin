//! Shared Tauri application state.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use iclass_api::IClassApiClient;
use iclass_core::IClassCore;
use iclass_domain::ScheduleEntry;
use iclass_session::{SessionClient, SessionStore};

use crate::settings::{AutomationSettingsStore, DesktopSettingsStore};

#[derive(Debug, Clone, Copy)]
struct AutoCheckRecord {
    last_attempt_timestamp: i64,
    succeeded: bool,
}

/// Application-wide shared state for Tauri commands.
#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) core: IClassCore,
    pub(crate) desktop_settings_store: DesktopSettingsStore,
    pub(crate) automation_settings_store: AutomationSettingsStore,
    close_to_tray: Arc<AtomicBool>,
    #[cfg(desktop)]
    allow_exit: Arc<AtomicBool>,
    auto_check_records: Arc<Mutex<HashMap<String, AutoCheckRecord>>>,
}

impl AppState {
    /// Creates the shared GUI application state.
    pub(crate) fn new(
        session_store: SessionStore,
        desktop_settings_store: DesktopSettingsStore,
        automation_settings_store: AutomationSettingsStore,
    ) -> Self {
        let api = IClassApiClient::default();
        let session_client = SessionClient::new(api, session_store);
        let core = IClassCore::new(session_client);
        Self {
            core,
            desktop_settings_store,
            automation_settings_store,
            close_to_tray: Arc::new(AtomicBool::new(false)),
            #[cfg(desktop)]
            allow_exit: Arc::new(AtomicBool::new(false)),
            auto_check_records: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Returns whether close requests should minimize the app to the tray.
    pub(crate) fn close_to_tray(&self) -> bool {
        self.close_to_tray.load(Ordering::Relaxed)
    }

    /// Updates whether close requests should minimize the app to the tray.
    pub(crate) fn set_close_to_tray(&self, value: bool) {
        self.close_to_tray.store(value, Ordering::Relaxed);
    }

    /// Marks the app as allowed to fully exit.
    #[cfg(desktop)]
    pub(crate) fn allow_exit(&self) {
        self.allow_exit.store(true, Ordering::Relaxed);
    }

    /// Returns whether a full exit has been explicitly requested.
    #[cfg(desktop)]
    pub(crate) fn exit_allowed(&self) -> bool {
        self.allow_exit.load(Ordering::Relaxed)
    }

    /// Returns whether the given schedule should be retried by the auto check-in worker.
    pub(crate) fn should_attempt_auto_check(
        &self,
        schedule: &ScheduleEntry,
        now_timestamp: i64,
        retry_after_seconds: u64,
    ) -> bool {
        let key = auto_check_key(schedule);
        let records = self
            .auto_check_records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        match records.get(&key) {
            Some(record) if record.succeeded => false,
            Some(record) => {
                let retry_after_seconds = i64::try_from(retry_after_seconds).unwrap_or(i64::MAX);
                now_timestamp.saturating_sub(record.last_attempt_timestamp) >= retry_after_seconds
            }
            None => true,
        }
    }

    /// Records the outcome of one background auto check-in attempt.
    pub(crate) fn record_auto_check_attempt(
        &self,
        schedule: &ScheduleEntry,
        now_timestamp: i64,
        succeeded: bool,
    ) {
        let key = auto_check_key(schedule);
        let mut records = self
            .auto_check_records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        records.insert(
            key,
            AutoCheckRecord {
                last_attempt_timestamp: now_timestamp,
                succeeded,
            },
        );
    }
}

fn auto_check_key(schedule: &ScheduleEntry) -> String {
    format!("{}:{}", schedule.teach_date, schedule.schedule_id)
}
