//! Background automation helpers such as periodic auto check-in.

use std::time::Duration;

use chrono::Local;
use tracing::{debug, info, warn};

use crate::{
    settings::{MIN_AUTO_CHECK_INTERVAL_SECONDS, PersistedAutomationSettings},
    state::AppState,
};

/// Starts the background auto check-in loop for the current application process.
pub(crate) fn spawn_auto_check_loop(state: AppState) {
    tauri::async_runtime::spawn(async move {
        loop {
            let settings = match state.automation_settings_store.load() {
                Ok(settings) => settings,
                Err(error) => {
                    warn!(error = %error, "failed to load automation settings");
                    PersistedAutomationSettings::default()
                }
            };

            if settings.auto_check_in_enabled {
                run_auto_check_iteration(&state, settings).await;
            }

            let sleep_seconds = settings.auto_check_interval_seconds;
            tokio::time::sleep(Duration::from_secs(sleep_seconds)).await;
        }
    });
}

async fn run_auto_check_iteration(state: &AppState, settings: PersistedAutomationSettings) {
    let now = Local::now();
    let now_local = now.naive_local();
    let now_timestamp = now.timestamp();

    debug!(
        interval_seconds = settings.auto_check_interval_seconds,
        mode = ?settings.auto_check_in_mode,
        "running auto check-in iteration"
    );

    let schedule = match state.core.best_schedule_for(now_local).await {
        Ok(schedule) => schedule,
        Err(error) => {
            debug!(error = %error, "auto check-in skipped because no eligible schedule was found");
            return;
        }
    };

    if !schedule.can_check_in_at(now_local) {
        debug!(
            schedule_id = %schedule.schedule_id,
            course_name = %schedule.course_name,
            "nearest schedule is not open for check-in yet"
        );
        return;
    }

    if !state.should_attempt_auto_check(
        &schedule,
        now_timestamp,
        settings
            .auto_check_interval_seconds
            .max(MIN_AUTO_CHECK_INTERVAL_SECONDS),
    ) {
        debug!(
            schedule_id = %schedule.schedule_id,
            "auto check-in skipped because the retry window has not elapsed"
        );
        return;
    }

    let schedule_id = schedule.schedule_id.clone();
    let course_name = schedule.course_name.clone();
    match state
        .core
        .check_in_for_schedule_at(
            schedule.clone(),
            settings.auto_check_in_mode.into(),
            now_local,
        )
        .await
    {
        Ok(result) => {
            state.record_auto_check_attempt(&schedule, now_timestamp, true);
            info!(
                schedule_id = %schedule_id,
                course_name = %course_name,
                method = ?result.receipt.method,
                signed_in = result.receipt.signed_in,
                "background auto check-in attempt finished"
            );
        }
        Err(error) => {
            state.record_auto_check_attempt(&schedule, now_timestamp, false);
            warn!(
                schedule_id = %schedule_id,
                course_name = %course_name,
                error = %error,
                "background auto check-in attempt failed"
            );
        }
    }
}
