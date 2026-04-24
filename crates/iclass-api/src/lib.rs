//! Low-level HTTP client and DTO mapping for the UCAS iCLASS service.

use chrono::{NaiveDate, NaiveDateTime, Utc};
use iclass_domain::{
    API_DATETIME_FORMAT, API_DAY_FORMAT, CheckInMethod, CheckInReceipt, Course, Credentials,
    ScheduleEntry, Semester, Session, UCAS_DEFAULT_BASE_URL, format_api_date,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use reqwest::{Client, Url, multipart};
use serde::Deserialize;
use thiserror::Error;
use tracing::debug;

const APPLY_TIMESTAMP_OFFSET_THRESHOLD_MS: i64 = 1_500;
const MAX_CLOCK_SYNC_RTT_MS: i64 = 10_000;

fn current_timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before UNIX_EPOCH")
        .as_millis() as i64
}

/// Stable classification of low-level API failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiErrorKind {
    /// HTTP transport, TLS, timeout, or JSON decoding failure.
    Transport,
    /// Invalid base URL or endpoint construction failure.
    Url,
    /// Authentication material is missing, expired, or rejected by the service.
    Authentication,
    /// Login credentials are invalid.
    InvalidCredentials,
    /// Attendance QR code is invalid, expired, or outside the valid time window.
    QrExpired,
    /// The requested schedule collection is empty.
    EmptySchedule,
    /// The server rejected request parameters.
    Parameter,
    /// Other business-level error returned by the service.
    Business,
    /// Local normalization or payload-shape parsing failure.
    Parse,
}

/// Errors produced while talking to the upstream iCLASS HTTP API.
#[derive(Debug, Error)]
pub enum ApiError {
    /// HTTP transport or response decoding error from `reqwest`.
    #[error("network request failed: {source}")]
    Transport {
        /// Original `reqwest` failure.
        #[source]
        source: reqwest::Error,
        /// Optional request summary that omits sensitive headers and credentials.
        request_summary: Option<String>,
    },
    /// Base URL or endpoint URL construction error.
    #[error("url error: {0}")]
    Url(#[from] url::ParseError),
    /// Business-level error returned by the upstream service.
    #[error("api returned business error {code}: {message}")]
    Business {
        /// Upstream business error code, such as `101`.
        code: String,
        /// Upstream business error message.
        message: String,
        /// Optional request summary that omits sensitive headers and credentials.
        request_summary: Option<String>,
    },
    /// Local parse or shape mismatch while normalizing API payloads.
    #[error("api payload parse error: {message}")]
    Parse {
        /// Human-readable parse failure detail.
        message: String,
        /// Optional request summary that omits sensitive headers and credentials.
        request_summary: Option<String>,
    },
}

impl ApiError {
    /// Returns the stable classification of this API error.
    pub fn kind(&self) -> ApiErrorKind {
        match self {
            Self::Transport { .. } => ApiErrorKind::Transport,
            Self::Url(_) => ApiErrorKind::Url,
            Self::Parse { .. } => ApiErrorKind::Parse,
            Self::Business { code, message, .. } => {
                let message = message.to_ascii_lowercase();
                if matches!(code.as_str(), "100" | "PARAMETER") || message.contains("参数") {
                    ApiErrorKind::Parameter
                } else if matches!(code.as_str(), "101") {
                    ApiErrorKind::QrExpired
                } else if matches!(code.as_str(), "2") {
                    ApiErrorKind::EmptySchedule
                } else if matches!(code.as_str(), "401" | "403")
                    || message.contains("密码")
                    || message.contains("账号")
                    || message.contains("用户名")
                {
                    ApiErrorKind::InvalidCredentials
                } else if message.contains("session")
                    || message.contains("登录")
                    || message.contains("失效")
                {
                    ApiErrorKind::Authentication
                } else {
                    ApiErrorKind::Business
                }
            }
        }
    }

    /// Returns the upstream business error code, if this error came from a non-success API payload.
    pub fn business_code(&self) -> Option<&str> {
        match self {
            Self::Business { code, .. } => Some(code.as_str()),
            _ => None,
        }
    }

    /// Returns the upstream business error message, if available.
    pub fn business_message(&self) -> Option<&str> {
        match self {
            Self::Business { message, .. } => Some(message.as_str()),
            _ => None,
        }
    }

    /// Returns the sanitized request summary associated with this error, if available.
    pub fn request_summary(&self) -> Option<&str> {
        match self {
            Self::Transport {
                request_summary, ..
            }
            | Self::Business {
                request_summary, ..
            }
            | Self::Parse {
                request_summary, ..
            } => request_summary.as_deref(),
            Self::Url(_) => None,
        }
    }

    /// Returns whether this error is a business error with the given code.
    pub fn is_business_code(&self, code: &str) -> bool {
        self.business_code() == Some(code)
    }

    /// Returns whether this error indicates authentication failure or session expiry.
    pub fn is_authentication_error(&self) -> bool {
        matches!(
            self.kind(),
            ApiErrorKind::Authentication | ApiErrorKind::InvalidCredentials
        )
    }

    /// Returns whether this error indicates the provided credentials were rejected.
    pub fn is_invalid_credentials(&self) -> bool {
        self.kind() == ApiErrorKind::InvalidCredentials
    }

    /// Returns whether this error represents the observed "QR expired / invalid window" failure.
    pub fn is_qr_expired(&self) -> bool {
        self.kind() == ApiErrorKind::QrExpired
    }

    /// Returns whether this error represents the "no schedule" business response.
    pub fn is_empty_schedule(&self) -> bool {
        self.kind() == ApiErrorKind::EmptySchedule
    }

    /// Returns whether this error represents request parameter validation failure.
    pub fn is_parameter_error(&self) -> bool {
        self.kind() == ApiErrorKind::Parameter
    }

    /// Returns whether the request is worth retrying after forcing a fresh login.
    pub fn should_retry_with_relogin(&self) -> bool {
        matches!(self.kind(), ApiErrorKind::Authentication)
    }

    fn transport_with_request(source: reqwest::Error, request_summary: impl Into<String>) -> Self {
        Self::Transport {
            source,
            request_summary: Some(request_summary.into()),
        }
    }

    fn business_with_request(
        code: String,
        message: String,
        request_summary: impl Into<String>,
    ) -> Self {
        Self::Business {
            code,
            message,
            request_summary: Some(request_summary.into()),
        }
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(source: reqwest::Error) -> Self {
        Self::Transport {
            source,
            request_summary: None,
        }
    }
}

/// Thin async client around the UCAS iCLASS HTTP API.
#[derive(Debug, Clone)]
pub struct IClassApiClient {
    base_url: Url,
    http: Client,
    timestamp_offset_ms: Arc<AtomicI64>,
}

impl Default for IClassApiClient {
    fn default() -> Self {
        Self::new(UCAS_DEFAULT_BASE_URL).expect("default UCAS iCLASS base url is valid")
    }
}

impl IClassApiClient {
    /// Builds a client for the given service base URL.
    pub fn new(base_url: impl AsRef<str>) -> Result<Self, ApiError> {
        let http = Client::builder()
            .user_agent("ucas-iclass-checkin/0.1")
            .http1_only()
            .build()?;
        Ok(Self {
            base_url: Url::parse(base_url.as_ref())?,
            http,
            timestamp_offset_ms: Arc::new(AtomicI64::new(0)),
        })
    }

    /// Authenticates with the upstream service and returns normalized session data.
    pub async fn login(&self, credentials: &Credentials) -> Result<Session, ApiError> {
        let url = self.endpoint("/app/user/login.action")?;
        let request_summary = build_request_summary(
            "POST",
            "/app/user/login.action",
            &[
                ("phone", credentials.account.as_str()),
                ("password", "<redacted>"),
            ],
        );
        let form = multipart::Form::new()
            .text("phone", credentials.account.clone())
            .text("password", credentials.password.clone());
        let response = self
            .http
            .post(url)
            .multipart(form)
            .send()
            .await
            .map_err(|source| ApiError::transport_with_request(source, request_summary.clone()))?;
        let payload: Envelope<LoginResultDto> = response
            .json()
            .await
            .map_err(|source| ApiError::transport_with_request(source, request_summary.clone()))?;
        let body = payload.into_result_with_request(Some(request_summary.clone()))?;

        Ok(Session {
            user_id: body.id,
            session_id: body.session_id,
            account: body.user_name,
            real_name: body.real_name.or(body.nick_name).unwrap_or_default(),
            class_id: empty_to_none(body.class_id),
            class_name: empty_to_none(body.class_info_name),
            class_uuid: empty_to_none(body.class_uuid),
            avatar_url: empty_to_none(body.pic_url),
            refreshed_at: Utc::now(),
        })
    }

    /// Fetches semester metadata for the currently authenticated user.
    pub async fn get_semesters(&self, session: &Session) -> Result<Vec<Semester>, ApiError> {
        let url = self.endpoint("/app/course/get_base_school_year.action")?;
        let request_summary = build_request_summary(
            "POST",
            "/app/course/get_base_school_year.action",
            &[("userId", session.user_id.as_str())],
        );
        let form = multipart::Form::new().text("userId", session.user_id.clone());
        let response = self
            .http
            .post(url)
            .multipart(form)
            .send()
            .await
            .map_err(|source| ApiError::transport_with_request(source, request_summary.clone()))?;
        let payload: Envelope<Vec<SemesterDto>> = response
            .json()
            .await
            .map_err(|source| ApiError::transport_with_request(source, request_summary.clone()))?;
        payload
            .into_result_with_request(Some(request_summary.clone()))?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    /// Fetches the current user's course list.
    pub async fn get_my_courses(&self, session: &Session) -> Result<Vec<Course>, ApiError> {
        let url = self.endpoint("/app/my/get_my_course.action")?;
        let request_summary = build_request_summary(
            "POST",
            "/app/my/get_my_course.action",
            &[("id", session.user_id.as_str())],
        );
        let form = multipart::Form::new().text("id", session.user_id.clone());
        let response = self
            .http
            .post(url)
            .header("sessionId", &session.session_id)
            .multipart(form)
            .send()
            .await
            .map_err(|source| ApiError::transport_with_request(source, request_summary.clone()))?;
        let payload: Envelope<Vec<CourseDto>> = response
            .json()
            .await
            .map_err(|source| ApiError::transport_with_request(source, request_summary.clone()))?;
        payload
            .into_result_with_request(Some(request_summary.clone()))?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    /// Fetches the schedule rows for a specific calendar day.
    ///
    /// The upstream service may respond with business code `2` when no schedule exists.
    /// This method normalizes that case into an empty vector.
    pub async fn get_daily_schedule(
        &self,
        session: &Session,
        date: NaiveDate,
    ) -> Result<Vec<ScheduleEntry>, ApiError> {
        let url = self.endpoint("/app/course/get_stu_course_sched.action")?;
        let date_str = format_api_date(date);
        let request_summary = build_request_summary(
            "POST",
            "/app/course/get_stu_course_sched.action",
            &[
                ("id", session.user_id.as_str()),
                ("dateStr", date_str.as_str()),
            ],
        );
        let form = multipart::Form::new()
            .text("id", session.user_id.clone())
            .text("dateStr", date_str);
        let response = self
            .http
            .post(url)
            .header("sessionId", &session.session_id)
            .multipart(form)
            .send()
            .await
            .map_err(|source| ApiError::transport_with_request(source, request_summary.clone()))?;
        let payload: Envelope<Vec<ScheduleDto>> = response
            .json()
            .await
            .map_err(|source| ApiError::transport_with_request(source, request_summary.clone()))?;
        map_schedule_collection(payload, &request_summary)?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    /// Fetches the weekly schedule view anchored at `date` and flattens it into rows.
    ///
    /// The upstream service may respond with business code `2` when no schedule exists.
    /// This method normalizes that case into an empty vector.
    pub async fn get_weekly_schedule(
        &self,
        session: &Session,
        date: NaiveDate,
    ) -> Result<Vec<ScheduleEntry>, ApiError> {
        let url = self.endpoint("/app/course/get_stu_course_sched_week.action")?;
        let date_str = format_api_date(date);
        let request_summary = build_request_summary(
            "POST",
            "/app/course/get_stu_course_sched_week.action",
            &[
                ("id", session.user_id.as_str()),
                ("dateStr", date_str.as_str()),
            ],
        );
        let form = multipart::Form::new()
            .text("id", session.user_id.clone())
            .text("dateStr", date_str);
        let response = self
            .http
            .post(url)
            .header("sessionId", &session.session_id)
            .multipart(form)
            .send()
            .await
            .map_err(|source| ApiError::transport_with_request(source, request_summary.clone()))?;
        let payload: Envelope<Vec<WeeklyDayDto>> = response
            .json()
            .await
            .map_err(|source| ApiError::transport_with_request(source, request_summary.clone()))?;
        let mut schedules = Vec::new();
        for day in map_schedule_collection(payload, &request_summary)? {
            for schedule in day.sched_data {
                schedules.push(schedule.try_into()?);
            }
        }

        Ok(schedules)
    }

    /// Attempts attendance using the UUID-style `timeTableId` parameter.
    pub async fn check_in_by_uuid(
        &self,
        session: &Session,
        schedule_uuid: &str,
    ) -> Result<CheckInReceipt, ApiError> {
        let timestamp = self.adjusted_timestamp_ms().to_string();
        let mut url = self.endpoint("/app/course/stu_scan_sign.action")?;
        url.query_pairs_mut()
            .append_pair("id", &session.user_id)
            .append_pair("timeTableId", schedule_uuid)
            .append_pair("timestamp", &timestamp);
        let request_summary = build_request_summary(
            "GET",
            "/app/course/stu_scan_sign.action",
            &[
                ("id", session.user_id.as_str()),
                ("timeTableId", schedule_uuid),
                ("timestamp", timestamp.as_str()),
            ],
        );
        self.check_in(session, url, CheckInMethod::Uuid, &request_summary)
            .await
    }

    /// Attempts attendance using the numeric `courseSchedId` parameter.
    pub async fn check_in_by_id(
        &self,
        session: &Session,
        schedule_id: &str,
    ) -> Result<CheckInReceipt, ApiError> {
        let timestamp = self.adjusted_timestamp_ms().to_string();
        let mut url = self.endpoint("/app/course/stu_scan_sign.action")?;
        url.query_pairs_mut()
            .append_pair("id", &session.user_id)
            .append_pair("courseSchedId", schedule_id)
            .append_pair("timestamp", &timestamp);
        let request_summary = build_request_summary(
            "GET",
            "/app/course/stu_scan_sign.action",
            &[
                ("id", session.user_id.as_str()),
                ("courseSchedId", schedule_id),
                ("timestamp", timestamp.as_str()),
            ],
        );
        self.check_in(session, url, CheckInMethod::Id, &request_summary)
            .await
    }

    /// Samples the upstream server clock and updates the local timestamp offset used for check-in.
    pub async fn synchronize_timestamp_offset(&self, session: &Session) -> Result<i64, ApiError> {
        let local_sent_at_ms = current_timestamp_ms();
        let server_timestamp_ms = self.get_server_timestamp_ms(session).await?;
        let local_received_at_ms = current_timestamp_ms();
        let offset_ms = estimate_timestamp_offset_ms(
            local_sent_at_ms,
            local_received_at_ms,
            server_timestamp_ms,
        )
        .unwrap_or(0);
        self.timestamp_offset_ms.store(offset_ms, Ordering::Relaxed);
        debug!(
            local_sent_at_ms,
            local_received_at_ms, server_timestamp_ms, offset_ms, "updated iCLASS timestamp offset"
        );
        Ok(offset_ms)
    }

    /// Returns the currently applied local timestamp offset in milliseconds.
    pub fn timestamp_offset_ms(&self) -> i64 {
        self.timestamp_offset_ms.load(Ordering::Relaxed)
    }

    /// Resolves an API-relative path against the configured base URL.
    fn endpoint(&self, path: &str) -> Result<Url, ApiError> {
        Ok(self.base_url.join(path)?)
    }

    async fn get_server_timestamp_ms(&self, session: &Session) -> Result<i64, ApiError> {
        let url = self.endpoint("/app/common/get_timestamp.do")?;
        let request_summary = build_request_summary(
            "POST",
            "/app/common/get_timestamp.do",
            &[("id", session.user_id.as_str())],
        );
        let form = multipart::Form::new().text("id", session.user_id.clone());
        let response = self
            .http
            .post(url)
            .header("sessionId", &session.session_id)
            .multipart(form)
            .send()
            .await
            .map_err(|source| ApiError::transport_with_request(source, request_summary.clone()))?;
        let payload: TimestampEnvelope = response
            .json()
            .await
            .map_err(|source| ApiError::transport_with_request(source, request_summary.clone()))?;
        payload.into_timestamp_with_request(Some(request_summary.clone()))
    }

    fn adjusted_timestamp_ms(&self) -> i64 {
        current_timestamp_ms().saturating_add(self.timestamp_offset_ms())
    }

    /// Shared implementation for both attendance request variants.
    async fn check_in(
        &self,
        session: &Session,
        url: Url,
        method: CheckInMethod,
        request_summary: &str,
    ) -> Result<CheckInReceipt, ApiError> {
        let response = self
            .http
            .get(url)
            .header("sessionId", &session.session_id)
            .send()
            .await
            .map_err(|source| {
                ApiError::transport_with_request(source, request_summary.to_owned())
            })?;
        let payload: Envelope<CheckInResultDto> = response.json().await.map_err(|source| {
            ApiError::transport_with_request(source, request_summary.to_owned())
        })?;
        let result = payload.into_result_with_request(Some(request_summary.to_owned()))?;
        Ok(CheckInReceipt {
            method,
            record_id: result.stu_sign_id,
            signed_in: result.stu_sign_status.as_deref() == Some("1"),
            status_code: "0".into(),
            verified_signed_in: None,
            observed_sign_status: result.stu_sign_status,
        })
    }
}

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    #[serde(rename = "STATUS")]
    status: String,
    #[serde(rename = "ERRCODE")]
    error_code: Option<String>,
    #[serde(rename = "ERRMSG")]
    error_message: Option<String>,
    result: Option<T>,
}

impl<T> Envelope<T> {
    fn into_result_with_request(self, request_summary: Option<String>) -> Result<T, ApiError> {
        if self.status == "0" {
            self.result.ok_or_else(|| ApiError::Parse {
                message: "missing result field".into(),
                request_summary,
            })
        } else {
            let code = self.error_code.unwrap_or_else(|| self.status.clone());
            let message = self
                .error_message
                .unwrap_or_else(|| "unknown business error".into());
            match request_summary {
                Some(request_summary) => Err(ApiError::business_with_request(
                    code,
                    message,
                    request_summary,
                )),
                None => Err(ApiError::Business {
                    code,
                    message,
                    request_summary: None,
                }),
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct LoginResultDto {
    id: String,
    #[serde(rename = "sessionId")]
    session_id: String,
    #[serde(rename = "userName")]
    user_name: String,
    #[serde(rename = "realName")]
    real_name: Option<String>,
    #[serde(rename = "nickName")]
    nick_name: Option<String>,
    #[serde(rename = "classId")]
    class_id: Option<String>,
    #[serde(rename = "classInfoName")]
    class_info_name: Option<String>,
    #[serde(rename = "classUUID")]
    class_uuid: Option<String>,
    #[serde(rename = "picUrl")]
    pic_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SemesterDto {
    code: String,
    name: String,
    #[serde(rename = "beginDate")]
    begin_date: String,
    #[serde(rename = "endDate")]
    end_date: String,
    #[serde(rename = "yearStatus")]
    year_status: String,
}

impl TryFrom<SemesterDto> for Semester {
    type Error = ApiError;

    fn try_from(value: SemesterDto) -> Result<Self, Self::Error> {
        Ok(Self {
            code: value.code,
            name: value.name,
            begin_date: parse_date(&value.begin_date, API_DAY_FORMAT)?,
            end_date: parse_date(&value.end_date, API_DAY_FORMAT)?,
            current: value.year_status == "1",
        })
    }
}

#[derive(Debug, Deserialize)]
struct CourseDto {
    id: String,
    #[serde(rename = "courseName")]
    course_name: String,
    #[serde(rename = "courseNum")]
    course_num: Option<String>,
    #[serde(rename = "teacherName")]
    teacher_name: Option<String>,
    #[serde(rename = "classroomName")]
    classroom_name: Option<String>,
    #[serde(rename = "beginDate")]
    begin_date: Option<String>,
    #[serde(rename = "endDate")]
    end_date: Option<String>,
    #[serde(rename = "myNoSignNum")]
    my_no_sign_num: Option<String>,
}

impl TryFrom<CourseDto> for Course {
    type Error = ApiError;

    fn try_from(value: CourseDto) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            name: value.course_name,
            course_num: empty_to_none(value.course_num),
            teacher_name: empty_to_none(value.teacher_name),
            classroom_name: empty_to_none(value.classroom_name),
            begin_date: parse_optional_date(value.begin_date, "%Y%m%d")?,
            end_date: parse_optional_date(value.end_date, "%Y%m%d")?,
            pending_checkins: value.my_no_sign_num.and_then(|count| count.parse().ok()),
        })
    }
}

#[derive(Debug, Deserialize)]
struct ScheduleDto {
    id: String,
    uuid: Option<String>,
    #[serde(rename = "courseId")]
    course_id: Option<String>,
    #[serde(rename = "courseName")]
    course_name: String,
    #[serde(rename = "teacherName")]
    teacher_name: Option<String>,
    #[serde(rename = "classroomName")]
    classroom_name: Option<String>,
    #[serde(rename = "teachTime")]
    teach_time: String,
    #[serde(rename = "classBeginTime")]
    class_begin_time: String,
    #[serde(rename = "classEndTime")]
    class_end_time: String,
    #[serde(rename = "signStatus")]
    sign_status: Option<String>,
}

impl TryFrom<ScheduleDto> for ScheduleEntry {
    type Error = ApiError;

    fn try_from(value: ScheduleDto) -> Result<Self, Self::Error> {
        Ok(Self {
            schedule_id: value.id,
            schedule_uuid: empty_to_none(value.uuid),
            course_id: empty_to_none(value.course_id),
            course_name: value.course_name,
            teacher_name: empty_to_none(value.teacher_name),
            classroom_name: empty_to_none(value.classroom_name),
            teach_date: parse_date(&value.teach_time, API_DAY_FORMAT)?,
            begins_at: parse_datetime(&value.class_begin_time)?,
            ends_at: parse_datetime(&value.class_end_time)?,
            lesson_units: 1,
            sign_status: empty_to_none(value.sign_status),
        })
    }
}

#[derive(Debug, Deserialize)]
struct WeeklyDayDto {
    #[serde(rename = "schedData", default)]
    sched_data: Vec<ScheduleDto>,
}

#[derive(Debug, Deserialize)]
struct CheckInResultDto {
    #[serde(rename = "stuSignId")]
    stu_sign_id: Option<String>,
    #[serde(rename = "stuSignStatus")]
    stu_sign_status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TimestampEnvelope {
    #[serde(rename = "STATUS")]
    status: String,
    #[serde(rename = "ERRCODE")]
    error_code: Option<String>,
    #[serde(rename = "ERRMSG")]
    error_message: Option<String>,
    timestamp: Option<i64>,
}

impl TimestampEnvelope {
    fn into_timestamp_with_request(self, request_summary: Option<String>) -> Result<i64, ApiError> {
        if self.status == "0" {
            self.timestamp.ok_or_else(|| ApiError::Parse {
                message: "missing timestamp field".into(),
                request_summary,
            })
        } else {
            let code = self.error_code.unwrap_or(self.status);
            let message = self
                .error_message
                .unwrap_or_else(|| "unknown business error".into());
            match request_summary {
                Some(request_summary) => Err(ApiError::business_with_request(
                    code,
                    message,
                    request_summary,
                )),
                None => Err(ApiError::Business {
                    code,
                    message,
                    request_summary: None,
                }),
            }
        }
    }
}

/// Converts empty strings and stringified `null` values into `None`.
fn empty_to_none(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim().trim_matches('"').to_string();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
            None
        } else {
            Some(trimmed)
        }
    })
}

/// Parses an optional date string using the given format.
fn parse_optional_date(value: Option<String>, format: &str) -> Result<Option<NaiveDate>, ApiError> {
    value.map(|raw| parse_date(&raw, format)).transpose()
}

/// Parses a required date string using the given format.
fn parse_date(value: &str, format: &str) -> Result<NaiveDate, ApiError> {
    NaiveDate::parse_from_str(value, format).map_err(|error| ApiError::Parse {
        message: format!("failed to parse date `{value}`: {error}"),
        request_summary: None,
    })
}

/// Parses a required datetime string using the UCAS schedule datetime format.
fn parse_datetime(value: &str) -> Result<NaiveDateTime, ApiError> {
    NaiveDateTime::parse_from_str(value, API_DATETIME_FORMAT).map_err(|error| ApiError::Parse {
        message: format!("failed to parse datetime `{value}`: {error}"),
        request_summary: None,
    })
}

/// Normalizes the "no schedule" business response into an empty collection.
fn map_schedule_collection<T>(
    payload: Envelope<Vec<T>>,
    request_summary: &str,
) -> Result<Vec<T>, ApiError> {
    match payload.into_result_with_request(Some(request_summary.to_owned())) {
        Ok(value) => Ok(value),
        Err(error) if error.is_empty_schedule() => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

fn build_request_summary(method: &str, path: &str, params: &[(&str, &str)]) -> String {
    let params = params
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("&");
    format!("request.method={method}\nrequest.path={path}\nrequest.params={params}")
}

fn estimate_timestamp_offset_ms(
    local_sent_at_ms: i64,
    local_received_at_ms: i64,
    server_timestamp_ms: i64,
) -> Option<i64> {
    if local_received_at_ms < local_sent_at_ms {
        return None;
    }

    let round_trip_ms = local_received_at_ms - local_sent_at_ms;
    if round_trip_ms > MAX_CLOCK_SYNC_RTT_MS {
        return Some(0);
    }

    let local_midpoint_ms = local_sent_at_ms + round_trip_ms / 2;
    let offset_ms = server_timestamp_ms - local_midpoint_ms;
    if offset_ms.abs() < APPLY_TIMESTAMP_OFFSET_THRESHOLD_MS {
        Some(0)
    } else {
        Some(offset_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_schedule_payload_into_domain_model() {
        let dto = ScheduleDto {
            id: "1166475".into(),
            uuid: Some("4EE358AAE647417AAE9FA7939342BBDE".into()),
            course_id: Some("54632".into()),
            course_name: "中国马克思主义与当代".into(),
            teacher_name: Some("田曦".into()),
            classroom_name: Some("教一楼002".into()),
            teach_time: "2026-04-23".into(),
            class_begin_time: "2026-04-23 13:30:00".into(),
            class_end_time: "2026-04-23 17:00:00".into(),
            sign_status: Some("0".into()),
        };

        let schedule: ScheduleEntry = dto.try_into().expect("schedule should parse");
        assert_eq!(schedule.schedule_id, "1166475");
        assert_eq!(
            schedule.schedule_uuid.as_deref(),
            Some("4EE358AAE647417AAE9FA7939342BBDE")
        );
        assert_eq!(schedule.course_name, "中国马克思主义与当代");
    }

    #[test]
    fn treats_business_code_2_as_empty_schedule_collection() {
        let payload: Envelope<Vec<ScheduleDto>> =
            serde_json::from_str(r#"{"STATUS":"2","ERRCODE":"2","ERRMSG":"暂无课表"}"#)
                .expect("payload should deserialize");

        let schedules = map_schedule_collection(payload, "request.method=POST")
            .expect("empty schedules should normalize");
        assert!(schedules.is_empty());
    }

    #[test]
    fn recognizes_qr_expired_business_error() {
        let error = ApiError::Business {
            code: "101".into(),
            message: "二维码已失效！".into(),
            request_summary: None,
        };

        assert!(error.is_qr_expired());
        assert_eq!(error.kind(), ApiErrorKind::QrExpired);
        assert_eq!(error.business_message(), Some("二维码已失效！"));
    }

    #[test]
    fn recognizes_parameter_business_error() {
        let error = ApiError::Business {
            code: "100".into(),
            message: "参数错误!".into(),
            request_summary: None,
        };

        assert!(error.is_parameter_error());
        assert_eq!(error.kind(), ApiErrorKind::Parameter);
    }

    #[test]
    fn estimate_timestamp_offset_uses_midpoint_and_threshold() {
        let offset = estimate_timestamp_offset_ms(1_000, 1_200, 3_100).expect("valid offset");
        assert_eq!(offset, 2_000);

        let tiny_offset = estimate_timestamp_offset_ms(1_000, 1_200, 2_250).expect("valid offset");
        assert_eq!(tiny_offset, 0);
    }

    #[test]
    fn estimate_timestamp_offset_ignores_untrusted_round_trip() {
        let offset = estimate_timestamp_offset_ms(1_000, 20_500, 3_100).expect("valid offset");
        assert_eq!(offset, 0);
    }
}
