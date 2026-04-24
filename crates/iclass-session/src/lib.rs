//! Session persistence and auto-refresh facade built on top of the raw API client.

use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use iclass_api::{ApiError, IClassApiClient};
use iclass_domain::{CheckInReceipt, Course, Credentials, ScheduleEntry, Semester, Session};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::warn;

/// Stable classification of session-layer failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionErrorKind {
    /// Wrapped low-level transport or URL failure.
    Transport,
    /// Authentication material is missing or invalid.
    Authentication,
    /// The provided credentials are invalid.
    InvalidCredentials,
    /// Session persistence failed.
    Store,
    /// The requested schedule collection is empty.
    EmptySchedule,
    /// Attendance QR code is invalid or expired.
    QrExpired,
    /// Request parameters were rejected by the server.
    Parameter,
    /// Local parsing or normalization failed.
    Parse,
    /// Other business-level server error.
    Business,
    /// No credentials were available for refresh.
    MissingCredentials,
}

/// Errors produced by session persistence, session refresh, or wrapped API calls.
#[derive(Debug, Error)]
pub enum SessionError {
    /// Wrapped API-layer error.
    #[error(transparent)]
    Api(#[from] ApiError),
    /// Failure while reading or writing the persistent session store.
    #[error("session store error at {path}: {message}")]
    Store {
        /// File or directory path involved in the storage failure.
        path: PathBuf,
        /// Human-readable error message.
        message: String,
    },
    /// No runtime or persisted credentials were available for re-login.
    #[error("no saved credentials are available for auto login")]
    MissingCredentials,
}

impl SessionError {
    /// Returns the stable classification of this session-layer error.
    pub fn kind(&self) -> SessionErrorKind {
        match self {
            Self::Api(error) => match error.kind() {
                iclass_api::ApiErrorKind::Transport | iclass_api::ApiErrorKind::Url => {
                    SessionErrorKind::Transport
                }
                iclass_api::ApiErrorKind::Authentication => SessionErrorKind::Authentication,
                iclass_api::ApiErrorKind::InvalidCredentials => {
                    SessionErrorKind::InvalidCredentials
                }
                iclass_api::ApiErrorKind::QrExpired => SessionErrorKind::QrExpired,
                iclass_api::ApiErrorKind::EmptySchedule => SessionErrorKind::EmptySchedule,
                iclass_api::ApiErrorKind::Parameter => SessionErrorKind::Parameter,
                iclass_api::ApiErrorKind::Business => SessionErrorKind::Business,
                iclass_api::ApiErrorKind::Parse => SessionErrorKind::Parse,
            },
            Self::Store { .. } => SessionErrorKind::Store,
            Self::MissingCredentials => SessionErrorKind::MissingCredentials,
        }
    }

    /// Returns whether this error represents an expired or invalid QR/sign-in window.
    pub fn is_qr_expired(&self) -> bool {
        self.kind() == SessionErrorKind::QrExpired
    }

    /// Returns whether this error indicates invalid login credentials.
    pub fn is_invalid_credentials(&self) -> bool {
        self.kind() == SessionErrorKind::InvalidCredentials
    }

    /// Returns whether this error represents a recoverable authentication/session-expiry issue.
    pub fn is_authentication_error(&self) -> bool {
        matches!(
            self.kind(),
            SessionErrorKind::Authentication | SessionErrorKind::InvalidCredentials
        )
    }

    /// Returns whether retrying after a forced re-login is likely to help.
    pub fn should_retry_with_relogin(&self) -> bool {
        self.kind() == SessionErrorKind::Authentication
    }
}

/// JSON-backed storage for persisted session and credential state.
#[derive(Debug, Clone)]
pub struct SessionStore {
    path: PathBuf,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new(default_store_path())
    }
}

impl SessionStore {
    /// Creates a store backed by the given file path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Returns the backing file path used by this store.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Loads persisted state from disk, returning default state when the file does not exist.
    pub fn load(&self) -> Result<PersistedState, SessionError> {
        if !self.path.exists() {
            return Ok(PersistedState::default());
        }

        let content = fs::read_to_string(&self.path).map_err(|error| SessionError::Store {
            path: self.path.clone(),
            message: error.to_string(),
        })?;
        serde_json::from_str(&content).map_err(|error| SessionError::Store {
            path: self.path.clone(),
            message: error.to_string(),
        })
    }

    /// Writes the provided state to disk, creating parent directories if needed.
    pub fn save(&self, state: &PersistedState) -> Result<(), SessionError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| SessionError::Store {
                path: parent.to_path_buf(),
                message: error.to_string(),
            })?;
        }

        let content = serde_json::to_string_pretty(state).map_err(|error| SessionError::Store {
            path: self.path.clone(),
            message: error.to_string(),
        })?;
        fs::write(&self.path, content).map_err(|error| SessionError::Store {
            path: self.path.clone(),
            message: error.to_string(),
        })
    }

    /// Clears only the saved session token while preserving other persisted state.
    pub fn clear_session(&self) -> Result<(), SessionError> {
        let mut state = self.load()?;
        state.session = None;
        state.updated_at = Utc::now();
        self.save(&state)
    }
}

/// Serializable snapshot of the locally persisted authentication state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersistedState {
    /// Last known authenticated session.
    pub session: Option<Session>,

    /// Saved credentials used for future auto-login, when allowed.
    pub credentials: Option<Credentials>,

    /// Local timestamp for the last write to this state file.
    pub updated_at: DateTime<Utc>,
}

/// Higher-level client that persists sessions and transparently refreshes them when needed.
#[derive(Debug, Clone)]
pub struct SessionClient {
    api: IClassApiClient,
    store: SessionStore,
    runtime_credentials: Option<Credentials>,
}

impl SessionClient {
    /// Creates a new session client using the provided API client and store.
    pub fn new(api: IClassApiClient, store: SessionStore) -> Self {
        Self {
            api,
            store,
            runtime_credentials: None,
        }
    }

    /// Attaches runtime-only credentials that may be used for refresh without persisting them.
    pub fn with_runtime_credentials(mut self, credentials: Option<Credentials>) -> Self {
        self.runtime_credentials = credentials;
        self
    }

    /// Returns the underlying persistent store.
    pub fn store(&self) -> &SessionStore {
        &self.store
    }

    /// Loads the raw persisted state from disk.
    pub fn load_state(&self) -> Result<PersistedState, SessionError> {
        self.store.load()
    }

    /// Logs in with explicit credentials and updates persisted state.
    ///
    /// When `remember_password` is `true`, the credentials are stored for future refreshes.
    pub async fn login(
        &self,
        credentials: &Credentials,
        remember_password: bool,
    ) -> Result<Session, SessionError> {
        let session = self.api.login(credentials).await?;
        self.synchronize_timestamp_offset(&session).await;
        let mut state = self.store.load()?;
        state.session = Some(session.clone());
        if remember_password {
            state.credentials = Some(credentials.clone());
        } else if state
            .credentials
            .as_ref()
            .is_some_and(|saved| saved.account == credentials.account)
        {
            state.credentials = None;
        }
        state.updated_at = Utc::now();
        self.store.save(&state)?;
        Ok(session)
    }

    /// Returns a currently usable session, refreshing it when no cached session exists.
    pub async fn ensure_session(&self) -> Result<Session, SessionError> {
        let state = self.store.load()?;
        if let Some(session) = state.session {
            return Ok(session);
        }

        self.refresh_session().await
    }

    /// Forces a fresh login using runtime or persisted credentials and stores the new session.
    pub async fn refresh_session(&self) -> Result<Session, SessionError> {
        let credentials = self.resolve_credentials()?;
        let session = self.api.login(&credentials).await?;
        self.synchronize_timestamp_offset(&session).await;
        let mut state = self.store.load()?;
        state.session = Some(session.clone());
        if state.credentials.is_none() && self.runtime_credentials.is_some() {
            state.credentials = self.runtime_credentials.clone();
        }
        state.updated_at = Utc::now();
        self.store.save(&state)?;
        Ok(session)
    }

    /// Fetches semester data, retrying once after re-login when appropriate.
    pub async fn get_semesters(&self) -> Result<Vec<Semester>, SessionError> {
        let session = self.ensure_session().await?;
        match self.api.get_semesters(&session).await {
            Ok(value) => Ok(value),
            Err(error) if error.should_retry_with_relogin() => {
                let session = self.refresh_session().await?;
                Ok(self.api.get_semesters(&session).await?)
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Fetches the current user's courses, retrying once after re-login when appropriate.
    pub async fn get_my_courses(&self) -> Result<Vec<Course>, SessionError> {
        let session = self.ensure_session().await?;
        match self.api.get_my_courses(&session).await {
            Ok(value) => Ok(value),
            Err(error) if error.should_retry_with_relogin() => {
                let session = self.refresh_session().await?;
                Ok(self.api.get_my_courses(&session).await?)
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Fetches daily schedule rows, retrying once after re-login when appropriate.
    pub async fn get_daily_schedule(
        &self,
        date: chrono::NaiveDate,
    ) -> Result<Vec<ScheduleEntry>, SessionError> {
        let session = self.ensure_session().await?;
        match self.api.get_daily_schedule(&session, date).await {
            Ok(value) => Ok(value),
            Err(error) if error.should_retry_with_relogin() => {
                let session = self.refresh_session().await?;
                Ok(self.api.get_daily_schedule(&session, date).await?)
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Fetches weekly schedule rows, retrying once after re-login when appropriate.
    pub async fn get_weekly_schedule(
        &self,
        date: chrono::NaiveDate,
    ) -> Result<Vec<ScheduleEntry>, SessionError> {
        let session = self.ensure_session().await?;
        match self.api.get_weekly_schedule(&session, date).await {
            Ok(value) => Ok(value),
            Err(error) if error.should_retry_with_relogin() => {
                let session = self.refresh_session().await?;
                Ok(self.api.get_weekly_schedule(&session, date).await?)
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Attempts UUID-based check-in, retrying once after re-login when appropriate.
    pub async fn check_in_by_uuid(
        &self,
        schedule_uuid: &str,
    ) -> Result<CheckInReceipt, SessionError> {
        let session = self.ensure_session().await?;
        self.synchronize_timestamp_offset(&session).await;
        match self.api.check_in_by_uuid(&session, schedule_uuid).await {
            Ok(value) => Ok(value),
            Err(error) if error.should_retry_with_relogin() => {
                let session = self.refresh_session().await?;
                self.synchronize_timestamp_offset(&session).await;
                Ok(self.api.check_in_by_uuid(&session, schedule_uuid).await?)
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Attempts ID-based check-in, retrying once after re-login when appropriate.
    pub async fn check_in_by_id(&self, schedule_id: &str) -> Result<CheckInReceipt, SessionError> {
        let session = self.ensure_session().await?;
        self.synchronize_timestamp_offset(&session).await;
        match self.api.check_in_by_id(&session, schedule_id).await {
            Ok(value) => Ok(value),
            Err(error) if error.should_retry_with_relogin() => {
                let session = self.refresh_session().await?;
                self.synchronize_timestamp_offset(&session).await;
                Ok(self.api.check_in_by_id(&session, schedule_id).await?)
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Resolves the best available credentials source for a forced refresh.
    fn resolve_credentials(&self) -> Result<Credentials, SessionError> {
        if let Some(credentials) = &self.runtime_credentials {
            return Ok(credentials.clone());
        }

        self.store
            .load()?
            .credentials
            .ok_or(SessionError::MissingCredentials)
    }

    async fn synchronize_timestamp_offset(&self, session: &Session) {
        if let Err(error) = self.api.synchronize_timestamp_offset(session).await {
            warn!(error = %error, "failed to synchronize iCLASS timestamp offset");
        }
    }
}

/// Returns the default location for persisted session state.
fn default_store_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("ucas-iclass-checkin").join("session.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_round_trip_works() {
        let temp = std::env::temp_dir().join(format!("iclass-session-{}.json", std::process::id()));
        let store = SessionStore::new(&temp);
        let state = PersistedState {
            session: Some(Session {
                user_id: "1".into(),
                session_id: "session".into(),
                account: "2025".into(),
                real_name: "Mock".into(),
                class_id: None,
                class_name: None,
                class_uuid: None,
                avatar_url: None,
                refreshed_at: Utc::now(),
            }),
            credentials: Some(Credentials {
                account: "2025".into(),
                password: "secret".into(),
            }),
            updated_at: Utc::now(),
        };

        store.save(&state).expect("state should save");
        let loaded = store.load().expect("state should load");
        assert_eq!(
            loaded.session.as_ref().map(|s| s.user_id.as_str()),
            Some("1")
        );
        let _ = fs::remove_file(temp);
    }
}
