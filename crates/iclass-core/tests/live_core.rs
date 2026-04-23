//! Optional live integration tests for core login, query, and check-in behavior.

use std::{env, path::PathBuf};

use anyhow::Result;
use chrono::{Local, NaiveDate};
use iclass_api::IClassApiClient;
use iclass_core::{IClassCore, select_best_schedule};
use iclass_domain::{CheckInMode, Credentials, UCAS_DEFAULT_BASE_URL};
use iclass_session::{SessionClient, SessionStore};
use tempfile::TempDir;

fn live_tests_enabled() -> bool {
    env::var("ICLASS_RUN_LIVE_TESTS").is_ok_and(|value| value == "1")
}

fn live_checkin_tests_enabled() -> bool {
    env::var("ICLASS_RUN_LIVE_CHECKIN_TESTS").is_ok_and(|value| value == "1")
}

fn test_store_path(temp_dir: &TempDir) -> PathBuf {
    temp_dir.path().join("live-session.json")
}

fn load_credentials() -> Option<Credentials> {
    let _ = dotenvy::dotenv();
    let account = env::var("UCAS_ICLASS_ACCOUNT").ok()?;
    let password = env::var("UCAS_ICLASS_PASSWORD").ok()?;
    Some(Credentials { account, password })
}

fn build_live_core(temp_dir: &TempDir, credentials: Credentials) -> Result<IClassCore> {
    let base_url =
        env::var("UCAS_ICLASS_BASE_URL").unwrap_or_else(|_| UCAS_DEFAULT_BASE_URL.into());
    let api = IClassApiClient::new(base_url)?;
    let session_client = SessionClient::new(api, SessionStore::new(test_store_path(temp_dir)))
        .with_runtime_credentials(Some(credentials));
    Ok(IClassCore::new(session_client))
}

#[tokio::test]
async fn live_login_and_fetch_reference_data() -> Result<()> {
    if !live_tests_enabled() {
        return Ok(());
    }

    let Some(credentials) = load_credentials() else {
        return Ok(());
    };

    let temp_dir = TempDir::new()?;
    let core = build_live_core(&temp_dir, credentials.clone())?;

    let session = core.login(&credentials, false).await?;
    assert!(!session.user_id.is_empty());
    assert!(!session.session_id.is_empty());

    let semesters = core.semesters().await?;
    assert!(!semesters.is_empty());

    let courses = core.courses().await?;
    assert!(!courses.is_empty());

    let today_schedules = core.daily_schedule(Local::now().date_naive()).await?;
    assert!(today_schedules.len() <= 32);

    Ok(())
}

#[tokio::test]
async fn live_checkin_reports_qr_expired_when_enabled() -> Result<()> {
    if !live_tests_enabled() || !live_checkin_tests_enabled() {
        return Ok(());
    }

    let Some(credentials) = load_credentials() else {
        return Ok(());
    };

    let Some(date) = env::var("UCAS_ICLASS_TEST_DATE")
        .ok()
        .and_then(|value| NaiveDate::parse_from_str(&value, "%Y-%m-%d").ok())
    else {
        return Ok(());
    };

    let temp_dir = TempDir::new()?;
    let core = build_live_core(&temp_dir, credentials.clone())?;
    core.login(&credentials, false).await?;

    let schedules = core.daily_schedule(date).await?;
    if schedules.is_empty() {
        return Ok(());
    }

    let moment = date.and_hms_opt(12, 0, 0).expect("midday should be valid");
    let schedule = select_best_schedule(&schedules, moment).expect("non-empty schedules");
    let result = core
        .check_in_for_schedule(schedule, CheckInMode::Auto)
        .await;

    let error = result.expect_err("mock account should not complete live check-in");
    assert!(
        error.is_qr_expired(),
        "unexpected live check-in error: {error}"
    );
    Ok(())
}
