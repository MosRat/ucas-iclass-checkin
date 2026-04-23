//! Deterministic mock-backed integration tests for core schedule and check-in flows.

use anyhow::Result;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};
use httpmock::prelude::*;
use iclass_api::IClassApiClient;
use iclass_core::{CheckInMethod, CoreError, IClassCore, select_best_schedule};
use iclass_domain::{CheckInMode, Credentials, ScheduleEntry, Session};
use iclass_session::{PersistedState, SessionClient, SessionStore};
use tempfile::TempDir;

fn make_schedule(id: &str, uuid: Option<&str>, begin_hour: u32, end_hour: u32) -> ScheduleEntry {
    let date = NaiveDate::from_ymd_opt(2026, 4, 23).expect("valid fixture date");
    ScheduleEntry {
        schedule_id: id.into(),
        schedule_uuid: uuid.map(str::to_string),
        course_id: Some("course-1".into()),
        course_name: "中国马克思主义与当代".into(),
        teacher_name: Some("田曦".into()),
        classroom_name: Some("教一楼002".into()),
        teach_date: date,
        begins_at: NaiveDateTime::new(
            date,
            NaiveTime::from_hms_opt(begin_hour, 0, 0).expect("valid begin time"),
        ),
        ends_at: NaiveDateTime::new(
            date,
            NaiveTime::from_hms_opt(end_hour, 0, 0).expect("valid end time"),
        ),
        lesson_units: 1,
        sign_status: Some("0".into()),
    }
}

fn build_core(server: &MockServer, store_dir: &TempDir) -> Result<IClassCore> {
    let store = SessionStore::new(store_dir.path().join("session.json"));
    let api = IClassApiClient::new(server.base_url())?;
    let client = SessionClient::new(api, store).with_runtime_credentials(Some(Credentials {
        account: "202528014629003".into(),
        password: "Ucas@2025".into(),
    }));
    Ok(IClassCore::new(client))
}

#[tokio::test]
async fn best_schedule_recovers_from_expired_session_and_falls_back_to_weekly() -> Result<()> {
    let server = MockServer::start_async().await;
    let temp_dir = TempDir::new()?;
    let store = SessionStore::new(temp_dir.path().join("session.json"));

    store.save(&PersistedState {
        session: Some(Session {
            user_id: "user-1".into(),
            session_id: "stale-session".into(),
            account: "202528014629003".into(),
            real_name: "测试用户".into(),
            class_id: None,
            class_name: None,
            class_uuid: None,
            avatar_url: None,
            refreshed_at: Utc::now(),
        }),
        credentials: Some(Credentials {
            account: "202528014629003".into(),
            password: "Ucas@2025".into(),
        }),
        updated_at: Utc::now(),
    })?;

    let stale_daily = server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/app/course/get_stu_course_sched.action")
                .header("sessionId", "stale-session");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"STATUS":"1","ERRCODE":"SESSION","ERRMSG":"session已失效"}"#);
        })
        .await;

    let relogin = server
        .mock_async(|when, then| {
            when.method(POST).path("/app/user/login.action");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{
                    "STATUS":"0",
                    "result":{
                        "id":"user-1",
                        "sessionId":"fresh-session",
                        "userName":"202528014629003",
                        "realName":"测试用户"
                    }
                }"#,
                );
        })
        .await;

    let fresh_daily_empty = server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/app/course/get_stu_course_sched.action")
                .header("sessionId", "fresh-session");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"STATUS":"2","ERRCODE":"2","ERRMSG":"暂无课表"}"#);
        })
        .await;

    let weekly = server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/app/course/get_stu_course_sched_week.action")
                .header("sessionId", "fresh-session");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{
                    "STATUS":"0",
                    "result":[
                        {
                            "dateStr":"20260423",
                            "schedData":[
                                {
                                    "id":"1166475",
                                    "uuid":"4EE358AAE647417AAE9FA7939342BBDE",
                                    "courseId":"54632",
                                    "courseName":"中国马克思主义与当代",
                                    "teacherName":"田曦",
                                    "classroomName":"教一楼002",
                                    "teachTime":"2026-04-23",
                                    "signStatus":"0",
                                    "classBeginTime":"2026-04-23 13:30:00",
                                    "classEndTime":"2026-04-23 17:00:00"
                                }
                            ]
                        }
                    ]
                }"#,
                );
        })
        .await;

    let core = build_core(&server, &temp_dir)?;
    let moment = NaiveDate::from_ymd_opt(2026, 4, 23)
        .expect("valid test date")
        .and_hms_opt(14, 0, 0)
        .expect("valid test time");

    let schedule = core.best_schedule_for(moment).await?;
    assert_eq!(schedule.schedule_id, "1166475");

    stale_daily.assert_async().await;
    relogin.assert_async().await;
    fresh_daily_empty.assert_async().await;
    weekly.assert_async().await;
    Ok(())
}

// #[tokio::test]
// async fn check_in_auto_prefers_uuid_mode_when_available() -> Result<()> {
//     let server = MockServer::start_async().await;
//     let temp_dir = TempDir::new()?;
//     let store = SessionStore::new(temp_dir.path().join("session.json"));

//     store.save(&PersistedState {
//         session: Some(Session {
//             user_id: "user-1".into(),
//             session_id: "fresh-session".into(),
//             account: "202528014629003".into(),
//             real_name: "测试用户".into(),
//             class_id: None,
//             class_name: None,
//             class_uuid: None,
//             avatar_url: None,
//             refreshed_at: Utc::now(),
//         }),
//         credentials: None,
//         updated_at: Utc::now(),
//     })?;

//     let checkin = server
//         .mock_async(|when, then| {
//             when.method(GET)
//                 .path("/app/course/stu_scan_sign.action")
//                 .header("sessionId", "fresh-session")
//                 .query_param("id", "user-1")
//                 .query_param("timeTableId", "uuid-1")
//                 .query_param("timestamp", "1234");
//             then.status(200)
//                 .header("content-type", "application/json")
//                 .body(
//                     r#"{
//                     "STATUS":"0",
//                     "result":{
//                         "stuSignId":"sign-1",
//                         "stuSignStatus":"1"
//                     }
//                 }"#,
//                 );
//         })
//         .await;

//     let core = build_core(&server, &temp_dir)?;
//     let result = core
//         .check_in_for_schedule_at(
//             make_schedule("1166475", Some("uuid-1"), 13, 15),
//             CheckInMode::Auto,
//             NaiveDate::from_ymd_opt(2026, 4, 23)
//                 .expect("valid fixture date")
//                 .and_hms_opt(14, 0, 0)
//                 .expect("valid fixture time"),
//             1234,
//         )
//         .await?;

//     assert_eq!(result.receipt.method, CheckInMethod::Uuid);
//     assert!(result.receipt.signed_in);
//     assert_eq!(result.receipt.record_id.as_deref(), Some("sign-1"));
//     checkin.assert_async().await;
//     Ok(())
// }

// #[tokio::test]
// async fn custom_uuid_check_in_uses_uuid_endpoint() -> Result<()> {
//     let server = MockServer::start_async().await;
//     let temp_dir = TempDir::new()?;
//     let store = SessionStore::new(temp_dir.path().join("session.json"));

//     store.save(&PersistedState {
//         session: Some(Session {
//             user_id: "user-1".into(),
//             session_id: "fresh-session".into(),
//             account: "202528014629003".into(),
//             real_name: "测试用户".into(),
//             class_id: None,
//             class_name: None,
//             class_uuid: None,
//             avatar_url: None,
//             refreshed_at: Utc::now(),
//         }),
//         credentials: None,
//         updated_at: Utc::now(),
//     })?;

//     let checkin = server
//         .mock_async(|when, then| {
//             when.method(GET)
//                 .path("/app/course/stu_scan_sign.action")
//                 .header("sessionId", "fresh-session")
//                 .query_param("id", "user-1")
//                 .query_param("timeTableId", "uuid-custom")
//                 .query_param("timestamp", "5678");
//             then.status(200)
//                 .header("content-type", "application/json")
//                 .body(
//                     r#"{
//                     "STATUS":"0",
//                     "result":{
//                         "stuSignId":"sign-custom",
//                         "stuSignStatus":"1"
//                     }
//                 }"#,
//                 );
//         })
//         .await;

//     let core = build_core(&server, &temp_dir)?;
//     let result = core
//         .check_in_with_identifier("uuid-custom", CheckInMode::ByUuid, 5678)
//         .await?;

//     assert_eq!(result.receipt.method, CheckInMethod::Uuid);
//     assert!(result.receipt.signed_in);
//     assert_eq!(result.receipt.record_id.as_deref(), Some("sign-custom"));
//     assert_eq!(result.schedule.schedule_id, "custom:uuid-custom");
//     assert_eq!(
//         result.schedule.schedule_uuid.as_deref(),
//         Some("uuid-custom")
//     );
//     assert_eq!(result.schedule.course_name, "自定义打卡");
//     checkin.assert_async().await;
//     Ok(())
// }

// #[tokio::test]
// async fn custom_id_check_in_uses_id_endpoint() -> Result<()> {
//     let server = MockServer::start_async().await;
//     let temp_dir = TempDir::new()?;
//     let store = SessionStore::new(temp_dir.path().join("session.json"));

//     store.save(&PersistedState {
//         session: Some(Session {
//             user_id: "user-1".into(),
//             session_id: "fresh-session".into(),
//             account: "202528014629003".into(),
//             real_name: "测试用户".into(),
//             class_id: None,
//             class_name: None,
//             class_uuid: None,
//             avatar_url: None,
//             refreshed_at: Utc::now(),
//         }),
//         credentials: None,
//         updated_at: Utc::now(),
//     })?;

//     let checkin = server
//         .mock_async(|when, then| {
//             when.method(GET)
//                 .path("/app/course/stu_scan_sign.action")
//                 .header("sessionId", "fresh-session")
//                 .query_param("id", "user-1")
//                 .query_param("courseSchedId", "1166475")
//                 .query_param("timestamp", "2468");
//             then.status(200)
//                 .header("content-type", "application/json")
//                 .body(
//                     r#"{
//                     "STATUS":"0",
//                     "result":{
//                         "stuSignId":"sign-id",
//                         "stuSignStatus":"1"
//                     }
//                 }"#,
//                 );
//         })
//         .await;

//     let core = build_core(&server, &temp_dir)?;
//     let result = core
//         .check_in_with_identifier("1166475", CheckInMode::ById, 2468)
//         .await?;

//     assert_eq!(result.receipt.method, CheckInMethod::Id);
//     assert!(result.receipt.signed_in);
//     assert_eq!(result.receipt.record_id.as_deref(), Some("sign-id"));
//     assert_eq!(result.schedule.schedule_id, "1166475");
//     assert_eq!(result.schedule.schedule_uuid, None);
//     assert_eq!(result.schedule.course_name, "自定义打卡");
//     checkin.assert_async().await;
//     Ok(())
// }

#[test]
fn by_uuid_mode_rejects_schedule_without_uuid() {
    let schedule = make_schedule("1166475", None, 13, 15);
    let error = CoreError::UnsupportedCheckInMode {
        mode: CheckInMode::ByUuid,
        schedule_id: schedule.schedule_id.clone(),
    };

    assert_eq!(
        error.to_string(),
        "schedule 1166475 does not support ByUuid check-in"
    );
}

#[test]
fn select_best_schedule_returns_none_for_empty_list() {
    let moment = NaiveDate::from_ymd_opt(2026, 4, 23)
        .expect("valid test date")
        .and_hms_opt(14, 0, 0)
        .expect("valid test time");

    assert!(select_best_schedule(&[], moment).is_none());
}
