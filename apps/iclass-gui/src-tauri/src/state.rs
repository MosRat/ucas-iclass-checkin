//! Shared Tauri application state.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use iclass_api::IClassApiClient;
use iclass_core::IClassCore;
use iclass_session::{SessionClient, SessionStore};

/// Application-wide shared state for Tauri commands.
#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) core: IClassCore,
    close_to_tray: Arc<AtomicBool>,
    allow_exit: Arc<AtomicBool>,
}

impl AppState {
    /// Creates the shared GUI application state.
    pub(crate) fn new() -> Self {
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
    pub(crate) fn close_to_tray(&self) -> bool {
        self.close_to_tray.load(Ordering::Relaxed)
    }

    /// Updates whether close requests should minimize the app to the tray.
    pub(crate) fn set_close_to_tray(&self, value: bool) {
        self.close_to_tray.store(value, Ordering::Relaxed);
    }

    /// Marks the app as allowed to fully exit.
    pub(crate) fn allow_exit(&self) {
        self.allow_exit.store(true, Ordering::Relaxed);
    }

    /// Returns whether a full exit has been explicitly requested.
    pub(crate) fn exit_allowed(&self) -> bool {
        self.allow_exit.load(Ordering::Relaxed)
    }
}
