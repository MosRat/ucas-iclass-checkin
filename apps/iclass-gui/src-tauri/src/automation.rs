//! Background automation helpers such as periodic auto check-in.

use std::time::Duration;

use chrono::Local;
use tauri::{AppHandle, Emitter};
use tracing::{debug, info, warn};

use crate::{
    models::build_automation_settings_payload,
    settings::{MIN_AUTO_CHECK_INTERVAL_SECONDS, PersistedAutomationSettings},
    state::{AppState, AutoCheckLastAction, AutoCheckStatus, AutoCheckStatusKind},
};

/// Starts the background auto check-in loop for the current application process.
pub(crate) fn spawn_auto_check_loop(app: AppHandle, state: AppState) {
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
                run_auto_check_iteration(&app, &state, settings).await;
            } else {
                update_auto_check_status(
                    &app,
                    &state,
                    settings,
                    AutoCheckStatus {
                        updated_at: Local::now(),
                        kind: AutoCheckStatusKind::Idle,
                        message: "自动打卡已关闭。".into(),
                        schedule: None,
                    },
                );
            }

            let sleep_seconds = settings.auto_check_interval_seconds;
            tokio::time::sleep(Duration::from_secs(sleep_seconds)).await;
        }
    });
}

async fn run_auto_check_iteration(
    app: &AppHandle,
    state: &AppState,
    settings: PersistedAutomationSettings,
) {
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
            update_auto_check_status(
                app,
                state,
                settings,
                AutoCheckStatus {
                    updated_at: now,
                    kind: AutoCheckStatusKind::Idle,
                    message: "当前没有处于自动打卡观察范围内的课程。".into(),
                    schedule: None,
                },
            );
            return;
        }
    };

    if !schedule.can_check_in_at(now_local) {
        debug!(
            schedule_id = %schedule.schedule_id,
            course_name = %schedule.course_name,
            "nearest schedule is not open for check-in yet"
        );
        update_auto_check_status(
            app,
            state,
            settings,
            AutoCheckStatus {
                updated_at: now,
                kind: AutoCheckStatusKind::WaitingWindow,
                message: "已刷新当前课程状态，等待打卡时间窗口开放。".into(),
                schedule: Some(schedule),
            },
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
        update_auto_check_status(
            app,
            state,
            settings,
            AutoCheckStatus {
                updated_at: now,
                kind: AutoCheckStatusKind::Ready,
                message: "当前课程可打卡，但仍在自动重试冷却时间内。".into(),
                schedule: Some(schedule),
            },
        );
        return;
    }

    let schedule_id = schedule.schedule_id.clone();
    let course_name = schedule.course_name.clone();
    update_auto_check_status(
        app,
        state,
        settings,
        AutoCheckStatus {
            updated_at: now,
            kind: AutoCheckStatusKind::Attempting,
            message: "已刷新当前课程状态，正在发起自动打卡。".into(),
            schedule: Some(schedule.clone()),
        },
    );
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
            let verification_message = match result.receipt.verified_signed_in {
                Some(true) => "课表复核显示已打卡".to_string(),
                Some(false) => "接口返回成功，但课表复核尚未显示已打卡".to_string(),
                None => "接口返回成功，暂未完成课表复核".to_string(),
            };
            state.set_auto_check_last_action(AutoCheckLastAction {
                attempted_at: now,
                schedule_id: schedule_id.clone(),
                course_name: course_name.clone(),
                succeeded: result
                    .receipt
                    .verified_signed_in
                    .unwrap_or(result.receipt.signed_in),
                message: verification_message.clone(),
            });
            update_auto_check_status(
                app,
                state,
                settings,
                AutoCheckStatus {
                    updated_at: Local::now(),
                    kind: AutoCheckStatusKind::Success,
                    message: verification_message,
                    schedule: Some(result.schedule),
                },
            );
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
            state.set_auto_check_last_action(AutoCheckLastAction {
                attempted_at: now,
                schedule_id: schedule_id.clone(),
                course_name: course_name.clone(),
                succeeded: false,
                message: error.to_string(),
            });
            update_auto_check_status(
                app,
                state,
                settings,
                AutoCheckStatus {
                    updated_at: Local::now(),
                    kind: AutoCheckStatusKind::Error,
                    message: error.to_string(),
                    schedule: Some(schedule),
                },
            );
            warn!(
                schedule_id = %schedule_id,
                course_name = %course_name,
                error = %error,
                "background auto check-in attempt failed"
            );
        }
    }
}

fn update_auto_check_status(
    app: &AppHandle,
    state: &AppState,
    settings: PersistedAutomationSettings,
    status: AutoCheckStatus,
) {
    state.set_auto_check_status(status);
    let payload = build_automation_settings_payload(
        settings,
        state.auto_check_last_action(),
        state.auto_check_status(),
    );
    if let Err(error) = app.emit("automation://status-updated", payload) {
        warn!(error = %error, "failed to emit automation status update");
    }
}
