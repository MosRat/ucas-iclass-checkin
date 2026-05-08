//! Session persistence and auto-refresh facade built on top of the raw API client.

use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::{DateTime, Utc};
use iclass_api::{ApiError, IClassApiClient, TimestampAdjustment, TimestampFeedback};
use iclass_domain::{CheckInReceipt, Course, Credentials, ScheduleEntry, Semester, Session};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, warn};

const MAX_CHECK_IN_ATTEMPTS: usize = 6;
const CHECK_IN_RETRY_DELAY_MS: u64 = 250;
const CONCURRENT_TIMESTAMP_OFFSETS_MS: &[i64] =
    &[0, -500, 500, -1_000, 1_000, -2_000, 2_000, -4_000, 4_000];

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
    /// The upstream accepted a request, but none of the attempted timestamps produced a sign-in.
    #[error("check-in was not confirmed after trying timestamps: {attempted_timestamps:?}")]
    CheckInNotConfirmed {
        /// Timestamps submitted during the exhausted recovery attempts.
        attempted_timestamps: Vec<i64>,
    },
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
            Self::CheckInNotConfirmed { .. } => SessionErrorKind::Business,
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

    /// Returns whether retrying once after refreshing the timestamp offset is likely to help.
    pub fn should_retry_with_timestamp_sync(&self) -> bool {
        matches!(
            self.kind(),
            SessionErrorKind::QrExpired | SessionErrorKind::Parameter
        )
    }

    /// Returns the timestamp feedback implied by this error, when known.
    pub fn timestamp_feedback(&self) -> Option<TimestampFeedback> {
        match self {
            Self::Api(error) => error.timestamp_feedback().or_else(|| match self.kind() {
                SessionErrorKind::Parameter => Some(TimestampFeedback::TooFarAhead),
                SessionErrorKind::QrExpired => Some(TimestampFeedback::TooFarBehind),
                _ => None,
            }),
            Self::CheckInNotConfirmed { .. } | Self::MissingCredentials | Self::Store { .. } => {
                None
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum CheckInTarget<'a> {
    Uuid(&'a str),
    Id(&'a str),
}

#[derive(Debug, Clone)]
enum CheckInTargetOwned {
    Uuid(String),
    Id(String),
}

impl CheckInTarget<'_> {
    fn to_owned(self) -> CheckInTargetOwned {
        match self {
            CheckInTarget::Uuid(value) => CheckInTargetOwned::Uuid(value.to_owned()),
            CheckInTarget::Id(value) => CheckInTargetOwned::Id(value.to_owned()),
        }
    }
}

#[derive(Debug)]
struct CheckInBatchResult {
    receipt: Option<CheckInReceipt>,
    errors: Vec<ApiError>,
    attempted_timestamps: Vec<i64>,
    saw_unconfirmed_response: bool,
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

    /// Returns the current check-in timestamp adjustment estimate.
    pub fn timestamp_adjustment(&self) -> TimestampAdjustment {
        self.api.timestamp_adjustment()
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

    /// Attempts UUID-based check-in, retrying after timestamp, network, or session recovery.
    pub async fn check_in_by_uuid(
        &self,
        schedule_uuid: &str,
    ) -> Result<CheckInReceipt, SessionError> {
        self.check_in_with_retries(CheckInTarget::Uuid(schedule_uuid))
            .await
    }

    /// Attempts ID-based check-in, retrying after timestamp, network, or session recovery.
    pub async fn check_in_by_id(&self, schedule_id: &str) -> Result<CheckInReceipt, SessionError> {
        self.check_in_with_retries(CheckInTarget::Id(schedule_id))
            .await
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

    async fn check_in_with_retries(
        &self,
        target: CheckInTarget<'_>,
    ) -> Result<CheckInReceipt, SessionError> {
        let mut session = self.ensure_session().await?;
        self.synchronize_timestamp_offset(&session).await;
        let mut all_attempted_timestamps = Vec::new();

        for attempt_index in 0..MAX_CHECK_IN_ATTEMPTS {
            let batch = self.send_check_in_batch(&session, target).await;
            all_attempted_timestamps.extend(batch.attempted_timestamps.iter().copied());
            if let Some(mut receipt) = batch.receipt {
                receipt.attempted_timestamps = all_attempted_timestamps;
                return Ok(receipt);
            }

            let mut retry_error = batch.errors.into_iter().next().map(SessionError::from);
            if retry_error.is_none() && batch.saw_unconfirmed_response {
                retry_error = Some(SessionError::CheckInNotConfirmed {
                    attempted_timestamps: all_attempted_timestamps.clone(),
                });
            }

            if let Some(error) = retry_error {
                let has_more_attempts = attempt_index + 1 < MAX_CHECK_IN_ATTEMPTS;
                if !has_more_attempts {
                    return Err(error);
                }

                if error.should_retry_with_relogin() {
                    debug!(
                        attempt = attempt_index + 1,
                        ?target,
                        "check-in failed with session error; refreshing session before retry"
                    );
                    session = self.refresh_session().await?;
                    self.synchronize_timestamp_offset(&session).await;
                    pause_before_check_in_retry().await;
                    continue;
                }

                if let Some(feedback) = error.timestamp_feedback() {
                    debug!(
                        attempt = attempt_index + 1,
                        ?target,
                        ?feedback,
                        "check-in failed with timestamp-like business error; adjusting before retry"
                    );
                    self.synchronize_timestamp_offset(&session).await;
                    self.api.apply_timestamp_feedback(feedback);
                    pause_before_check_in_retry().await;
                    continue;
                }

                if matches!(error.kind(), SessionErrorKind::Transport) {
                    debug!(
                        attempt = attempt_index + 1,
                        ?target,
                        "check-in failed with transport error; retrying"
                    );
                    pause_before_check_in_retry().await;
                    continue;
                }

                return Err(error);
            } else {
                return Err(SessionError::CheckInNotConfirmed {
                    attempted_timestamps: all_attempted_timestamps,
                });
            }
        }

        unreachable!("check-in retry loop either returns a receipt or the last error");
    }

    async fn send_check_in_batch(
        &self,
        session: &Session,
        target: CheckInTarget<'_>,
    ) -> CheckInBatchResult {
        let target = target.to_owned();
        let mut attempted_timestamps = CONCURRENT_TIMESTAMP_OFFSETS_MS
            .iter()
            .map(|offset| self.api.adjusted_timestamp_with_extra_offset_ms(*offset))
            .collect::<Vec<_>>();
        attempted_timestamps.sort_unstable();
        attempted_timestamps.dedup();

        debug!(
            ?target,
            ?attempted_timestamps,
            "sending concurrent timestamp check-in attempts"
        );

        let mut tasks = tokio::task::JoinSet::new();
        for timestamp in attempted_timestamps.iter().copied() {
            let api = self.api.clone();
            let session = session.clone();
            let target = target.clone();
            tasks.spawn(async move {
                let result = match target {
                    CheckInTargetOwned::Uuid(schedule_uuid) => {
                        api.check_in_by_uuid_at_timestamp(&session, &schedule_uuid, timestamp)
                            .await
                    }
                    CheckInTargetOwned::Id(schedule_id) => {
                        api.check_in_by_id_at_timestamp(&session, &schedule_id, timestamp)
                            .await
                    }
                };
                (timestamp, result)
            });
        }

        let mut errors = Vec::new();
        let mut saw_unconfirmed_response = false;
        while let Some(joined) = tasks.join_next().await {
            match joined {
                Ok((_, Ok(mut receipt))) if receipt.signed_in => {
                    receipt.attempted_timestamps = attempted_timestamps.clone();
                    tasks.abort_all();
                    return CheckInBatchResult {
                        receipt: Some(receipt),
                        errors,
                        attempted_timestamps,
                        saw_unconfirmed_response,
                    };
                }
                Ok((_, Ok(_receipt))) => {
                    saw_unconfirmed_response = true;
                }
                Ok((_, Err(error))) => errors.push(error),
                Err(error) => warn!(error = %error, "concurrent check-in task failed"),
            }
        }

        CheckInBatchResult {
            receipt: None,
            errors,
            attempted_timestamps,
            saw_unconfirmed_response,
        }
    }
}

async fn pause_before_check_in_retry() {
    tokio::time::sleep(Duration::from_millis(CHECK_IN_RETRY_DELAY_MS)).await;
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
