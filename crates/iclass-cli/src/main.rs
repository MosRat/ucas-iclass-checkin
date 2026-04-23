//! Command-line entry point for exercising the iCLASS core functionality.

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use chrono::{Local, NaiveDate};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use iclass_api::IClassApiClient;
use iclass_core::{CoreError, IClassCore};
use iclass_domain::{CheckInMode, Credentials, ScheduleEntry, UCAS_DEFAULT_BASE_URL};
use iclass_session::{SessionClient, SessionError, SessionStore};
#[cfg(feature = "allocator-mimalloc")]
use mimalloc::MiMalloc;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Global allocator selection for the CLI binary.
#[cfg(feature = "allocator-mimalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

/// Top-level CLI arguments.
#[derive(Debug, Parser)]
#[command(
    name = "iclass-cli",
    version,
    about = "UCAS iCLASS sign-in automation CLI"
)]
struct Cli {
    /// Override for the UCAS iCLASS base URL.
    #[arg(long, env = "UCAS_ICLASS_BASE_URL", default_value = UCAS_DEFAULT_BASE_URL)]
    base_url: String,
    /// Login account, typically supplied via environment variable.
    #[arg(long, env = "UCAS_ICLASS_ACCOUNT")]
    account: Option<String>,
    /// Login password, typically supplied via environment variable.
    #[arg(long, env = "UCAS_ICLASS_PASSWORD")]
    password: Option<String>,
    /// Custom path for persisted session state.
    #[arg(long, env = "UCAS_ICLASS_SESSION_PATH")]
    session_path: Option<PathBuf>,
    /// Logging filter passed to the tracing subscriber.
    #[arg(long, env = "RUST_LOG", default_value = "info")]
    log_level: String,
    /// Selected subcommand.
    #[command(subcommand)]
    command: Command,
}

/// Supported CLI subcommands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Authenticate and persist the resulting session.
    Login {
        /// Whether to persist the provided password for future refresh.
        #[arg(long, default_value_t = true, action = ArgAction::Set)]
        remember_password: bool,
    },
    /// Inspect or clear the persisted session state.
    Session {
        /// Whether to clear the saved session token.
        #[arg(long, default_value_t = false)]
        clear: bool,
    },
    /// List available semester metadata.
    Semesters,
    /// List current course metadata.
    Courses,
    /// Show schedule rows for today or a specified date.
    Today {
        /// Target date in `YYYY-MM-DD` format.
        #[arg(long)]
        date: Option<String>,
    },
    /// Attempt attendance for today or a specified date.
    Checkin {
        /// Requested attendance strategy.
        #[arg(long, value_enum, default_value_t = CheckInModeArg::Auto)]
        mode: CheckInModeArg,
        /// Target date in `YYYY-MM-DD` format.
        #[arg(long)]
        date: Option<String>,
    },
}

/// CLI-level representation of check-in strategy choices.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum CheckInModeArg {
    /// Prefer UUID-based attendance and fall back to ID-based attendance.
    Auto,
    /// Force UUID-based attendance.
    Uuid,
    /// Force ID-based attendance.
    Id,
}

impl From<CheckInModeArg> for CheckInMode {
    fn from(value: CheckInModeArg) -> Self {
        match value {
            CheckInModeArg::Auto => Self::Auto,
            CheckInModeArg::Uuid => Self::ByUuid,
            CheckInModeArg::Id => Self::ById,
        }
    }
}

/// Program entry point.
#[tokio::main]
async fn main() {
    let exit_code = match run().await {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("Error: {}", render_error(&error));
            1
        }
    };
    std::process::exit(exit_code);
}

/// Executes the CLI command and returns any structured failure.
async fn run() -> Result<()> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();
    init_tracing(&cli.log_level)?;

    let store = SessionStore::new(
        cli.session_path
            .clone()
            .unwrap_or_else(|| SessionStore::default().path().to_path_buf()),
    );
    let api = IClassApiClient::new(&cli.base_url)?;
    let runtime_credentials = credentials_from_cli(&cli);
    let session_client =
        SessionClient::new(api, store).with_runtime_credentials(runtime_credentials.clone());
    let core = IClassCore::new(session_client.clone());

    match cli.command {
        Command::Login { remember_password } => {
            let credentials = runtime_credentials
                .ok_or_else(|| anyhow!("missing credentials, set UCAS_ICLASS_ACCOUNT and UCAS_ICLASS_PASSWORD or pass --account/--password"))?;
            let session = core.login(&credentials, remember_password).await?;
            println!(
                "logged in: {} ({}) session={} refreshed_at={}",
                session.real_name, session.account, session.session_id, session.refreshed_at
            );
        }
        Command::Session { clear } => {
            if clear {
                session_client.store().clear_session()?;
                println!(
                    "cleared saved session at {}",
                    session_client.store().path().display()
                );
            } else {
                let state = session_client.load_state()?;
                println!(
                    "store: {}\nupdated_at: {}\nhas_session: {}\nhas_credentials: {}",
                    session_client.store().path().display(),
                    state.updated_at,
                    state.session.is_some(),
                    state.credentials.is_some()
                );
                if let Some(session) = state.session {
                    println!(
                        "user: {} ({}) session={} refreshed_at={}",
                        session.real_name,
                        session.account,
                        session.session_id,
                        session.refreshed_at
                    );
                }
            }
        }
        Command::Semesters => {
            for semester in core.semesters().await? {
                println!(
                    "{} {} [{} - {}] current={}",
                    semester.code,
                    semester.name,
                    semester.begin_date,
                    semester.end_date,
                    semester.current
                );
            }
        }
        Command::Courses => {
            for course in core.courses().await? {
                println!(
                    "{} | {} | teacher={} | classroom={} | pending={}",
                    course.id,
                    course.name,
                    course.teacher_name.as_deref().unwrap_or("-"),
                    course.classroom_name.as_deref().unwrap_or("-"),
                    course
                        .pending_checkins
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "-".into())
                );
            }
        }
        Command::Today { date } => {
            let date = parse_date(date.as_deref())?;
            let schedules = core.daily_schedule(date).await?;
            print_schedules(date, &schedules);
        }
        Command::Checkin { mode, date } => {
            let target_date = parse_date(date.as_deref())?;
            let result = if target_date == Local::now().date_naive() {
                core.check_in_now(mode.into()).await?
            } else {
                let schedules = core.daily_schedule(target_date).await?;
                let moment = target_date
                    .and_hms_opt(12, 0, 0)
                    .expect("midday should always be valid");
                let schedule = iclass_core::select_best_schedule(&schedules, moment)
                    .ok_or(CoreError::NoScheduleAvailable { date: target_date })?;
                core.check_in_for_schedule_at(schedule, mode.into(), moment)
                    .await?
            };

            println!(
                "check-in result: course={} method={:?} signed_in={} record_id={}",
                result.schedule.course_name,
                result.receipt.method,
                result.receipt.signed_in,
                result.receipt.record_id.as_deref().unwrap_or("-")
            );
        }
    }

    Ok(())
}

/// Initializes tracing/logging based on the provided filter string.
fn init_tracing(log_level: &str) -> Result<()> {
    let filter = EnvFilter::try_new(log_level)
        .or_else(|_| EnvFilter::try_new("info"))
        .context("failed to initialize log filter")?;
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
    Ok(())
}

/// Builds runtime credentials from CLI arguments when both account and password are present.
fn credentials_from_cli(cli: &Cli) -> Option<Credentials> {
    match (&cli.account, &cli.password) {
        (Some(account), Some(password)) => Some(Credentials {
            account: account.clone(),
            password: password.clone(),
        }),
        _ => None,
    }
}

/// Parses a CLI date string or falls back to the current local date.
fn parse_date(value: Option<&str>) -> Result<NaiveDate> {
    match value {
        Some(raw) => NaiveDate::parse_from_str(raw, "%Y-%m-%d")
            .with_context(|| format!("invalid date `{raw}`, expected YYYY-MM-DD")),
        None => Ok(Local::now().date_naive()),
    }
}

/// Prints a schedule list in a compact human-readable CLI format.
fn print_schedules(date: NaiveDate, schedules: &[ScheduleEntry]) {
    if schedules.is_empty() {
        println!("{date} 没有课表。");
        return;
    }

    for schedule in schedules {
        println!(
            "{} | {} | teacher={} | room={} | {} - {} | id={} | uuid={}",
            schedule.teach_date,
            schedule.course_name,
            schedule.teacher_name.as_deref().unwrap_or("-"),
            schedule.classroom_name.as_deref().unwrap_or("-"),
            schedule.begins_at.time(),
            schedule.ends_at.time(),
            schedule.schedule_id,
            schedule.schedule_uuid.as_deref().unwrap_or("-")
        );
    }
}

/// Renders a compact, user-facing CLI error message from the structured error chain.
fn render_error(error: &anyhow::Error) -> String {
    if let Some(core_error) = find_cause::<CoreError>(error) {
        return render_core_error(core_error);
    }
    if let Some(session_error) = find_cause::<SessionError>(error) {
        return render_session_error(session_error);
    }
    if let Some(api_error) = find_cause::<iclass_api::ApiError>(error) {
        return render_api_error(api_error);
    }
    error.to_string()
}

/// Walks the error chain and returns the first cause of the requested concrete type.
fn find_cause<T>(error: &anyhow::Error) -> Option<&T>
where
    T: std::error::Error + Send + Sync + 'static,
{
    error.chain().find_map(|cause| cause.downcast_ref::<T>())
}

/// Formats a core-layer error for CLI presentation.
fn render_core_error(error: &CoreError) -> String {
    match error {
        CoreError::NoScheduleAvailable { date } => {
            format!(
                "{date} 没有可用课表，因此无法打卡。可以先运行 `today --date {date}` 查看当天课程。"
            )
        }
        CoreError::UnsupportedCheckInMode { mode, schedule_id } => {
            format!("课程 {schedule_id} 不支持 {mode:?} 打卡模式，请改用其他模式后重试。")
        }
        CoreError::CheckInNotOpenYet {
            course_name,
            opens_at,
            ..
        } => {
            format!("{course_name} 还未到打卡时间，将在 {opens_at} 开放。")
        }
        CoreError::CheckInClosed {
            course_name,
            ended_at,
            ..
        } => {
            format!("{course_name} 已于 {ended_at} 结束，当前无法继续打卡。")
        }
        CoreError::Session(session_error) => render_session_error(session_error),
    }
}

/// Formats a session-layer error for CLI presentation.
fn render_session_error(error: &SessionError) -> String {
    match error.kind() {
        iclass_session::SessionErrorKind::Authentication => {
            "当前登录状态已失效，请重新执行 `login` 或重新提供账号密码。".into()
        }
        iclass_session::SessionErrorKind::InvalidCredentials => {
            "账号或密码错误，请确认 `--account` 和 `--password` 是否正确。".into()
        }
        iclass_session::SessionErrorKind::MissingCredentials => {
            "缺少可用于自动登录的凭证，请先执行 `login`，或传入 `--account` 与 `--password`。"
                .into()
        }
        iclass_session::SessionErrorKind::Store => {
            format!("本地 session 存储失败：{error}")
        }
        _ => match error {
            SessionError::Api(api_error) => render_api_error(api_error),
            SessionError::Store { .. } | SessionError::MissingCredentials => error.to_string(),
        },
    }
}

/// Formats an API-layer error for CLI presentation.
fn render_api_error(error: &iclass_api::ApiError) -> String {
    match error.kind() {
        iclass_api::ApiErrorKind::Transport | iclass_api::ApiErrorKind::Url => {
            format!("网络请求失败：{error}")
        }
        iclass_api::ApiErrorKind::Authentication => "上游会话已失效，请重新登录后重试。".into(),
        iclass_api::ApiErrorKind::InvalidCredentials => "账号或密码错误。".into(),
        iclass_api::ApiErrorKind::QrExpired => {
            "当前不在有效打卡时间窗口内，或签到二维码已经失效。".into()
        }
        iclass_api::ApiErrorKind::EmptySchedule => {
            "当前日期没有课表，或上游接口未返回可用的打卡课程。".into()
        }
        iclass_api::ApiErrorKind::Parameter => {
            let detail = error.business_message().unwrap_or("参数错误");
            format!("请求参数被上游拒绝：{detail}")
        }
        iclass_api::ApiErrorKind::Business => {
            let code = error.business_code().unwrap_or("unknown");
            let detail = error.business_message().unwrap_or("未知业务错误");
            format!("上游接口返回业务错误 {code}：{detail}")
        }
        iclass_api::ApiErrorKind::Parse => {
            format!("上游响应解析失败：{error}")
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Error;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
    use iclass_api::ApiError;
    use iclass_core::CoreError;

    use super::render_error;

    #[test]
    fn renders_no_schedule_with_actionable_hint() {
        let error = Error::new(CoreError::NoScheduleAvailable {
            date: NaiveDate::from_ymd_opt(2026, 4, 22).expect("valid date"),
        });

        let rendered = render_error(&error);
        assert!(rendered.contains("2026-04-22"));
        assert!(rendered.contains("today --date 2026-04-22"));
    }

    #[test]
    fn renders_too_early_check_in_clearly() {
        let error = Error::new(CoreError::CheckInNotOpenYet {
            schedule_id: "1166475".into(),
            course_name: "智能计算系统".into(),
            opens_at: NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2026, 4, 23).expect("valid date"),
                NaiveTime::from_hms_opt(13, 0, 0).expect("valid time"),
            ),
        });

        let rendered = render_error(&error);
        assert!(rendered.contains("智能计算系统"));
        assert!(rendered.contains("开放"));
    }

    #[test]
    fn renders_invalid_credentials_without_raw_business_wrapper() {
        let error = Error::new(ApiError::Business {
            code: "107".into(),
            message: "密码错误！".into(),
        });

        let rendered = render_error(&error);
        assert_eq!(rendered, "账号或密码错误。");
    }
}
