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
                        next_retry_at: None,
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
                    next_retry_at: None,
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
                next_retry_at: None,
            },
        );
        return;
    }

    let retry_after_seconds = settings
        .auto_check_interval_seconds
        .max(MIN_AUTO_CHECK_INTERVAL_SECONDS);
    if !state.should_attempt_auto_check(&schedule, now_timestamp, retry_after_seconds) {
        debug!(
            schedule_id = %schedule.schedule_id,
            "auto check-in skipped because the retry window has not elapsed"
        );
        let next_retry_at = state.next_auto_check_retry_at(&schedule, retry_after_seconds);
        update_auto_check_status(
            app,
            state,
            settings,
            AutoCheckStatus {
                updated_at: now,
                kind: AutoCheckStatusKind::Ready,
                message: "当前课程可打卡，但仍在自动重试冷却时间内。".into(),
                schedule: Some(schedule),
                next_retry_at,
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
            next_retry_at: None,
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
            let timestamp_message = format_receipt_timestamp_message(&result.receipt);
            let action_message = format!("{verification_message}；{timestamp_message}");
            state.set_auto_check_last_action(AutoCheckLastAction {
                attempted_at: now,
                schedule_id: schedule_id.clone(),
                course_name: course_name.clone(),
                succeeded: result
                    .receipt
                    .verified_signed_in
                    .unwrap_or(result.receipt.signed_in),
                message: action_message.clone(),
                timestamp_attempts: result.receipt.timestamp_attempts.clone(),
            });
            update_auto_check_status(
                app,
                state,
                settings,
                AutoCheckStatus {
                    updated_at: Local::now(),
                    kind: AutoCheckStatusKind::Success,
                    message: action_message,
                    schedule: Some(result.schedule),
                    next_retry_at: None,
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
            let timestamp_attempts = error_timestamp_attempts(&error);
            let error_message = error.to_string();
            state.record_auto_check_attempt(&schedule, now_timestamp, false);
            state.set_auto_check_last_action(AutoCheckLastAction {
                attempted_at: now,
                schedule_id: schedule_id.clone(),
                course_name: course_name.clone(),
                succeeded: false,
                message: error_message.clone(),
                timestamp_attempts,
            });
            update_auto_check_status(
                app,
                state,
                settings,
                AutoCheckStatus {
                    updated_at: Local::now(),
                    kind: AutoCheckStatusKind::Error,
                    message: error_message,
                    schedule: Some(schedule),
                    next_retry_at: None,
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

fn error_timestamp_attempts(
    error: &iclass_core::CoreError,
) -> Vec<iclass_domain::CheckInTimestampAttempt> {
    match error {
        iclass_core::CoreError::Session(session_error) => session_error
            .timestamp_attempts()
            .map(|attempts| attempts.to_vec())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn format_receipt_timestamp_message(receipt: &iclass_core::CheckInReceipt) -> String {
    if !receipt.timestamp_attempts.is_empty() {
        let attempted = receipt
            .timestamp_attempts
            .iter()
            .take(12)
            .map(|attempt| {
                let status = if attempt.signed_in {
                    "成功"
                } else {
                    "失败"
                };
                let detail = [attempt.status_code.as_deref(), attempt.message.as_deref()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" ");
                if detail.is_empty() {
                    format!("{} {}", attempt.timestamp, status)
                } else {
                    format!("{} {} {}", attempt.timestamp, status, detail)
                }
            })
            .collect::<Vec<_>>()
            .join(" | ");
        return format!(
            "尝试时间戳 {} 个，{}{}",
            receipt.timestamp_attempts.len(),
            attempted,
            if receipt.timestamp_attempts.len() > 12 {
                " | 其余已省略"
            } else {
                ""
            }
        );
    }
    let attempted = receipt
        .attempted_timestamps
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let successful = receipt
        .successful_timestamp
        .map(|value| value.to_string())
        .unwrap_or_else(|| "未返回".into());
    format!(
        "尝试时间戳 {} 个 [{}]，命中 {}",
        receipt.attempted_timestamps.len(),
        attempted,
        successful
    )
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
        state.core.timestamp_adjustment(),
    );
    if let Err(error) = app.emit("automation://status-updated", payload) {
        warn!(error = %error, "failed to emit automation status update");
    }
}
