# UCAS iCLASS API Reference

This file records the subset of upstream HTTP behavior currently used by the workspace. It is a maintenance note, not a public contract.

## Base URL

```text
https://iclass.ucas.edu.cn:8181
```

Authenticated requests send the session token in the `sessionId` header.

## Response Envelope

Successful responses usually look like:

```json
{
  "STATUS": "0",
  "result": {}
}
```

Error responses usually look like:

```json
{
  "STATUS": "1",
  "ERRCODE": "101",
  "ERRMSG": "..."
}
```

Observed business codes currently normalized by the client:

| Code | Handling |
| --- | --- |
| `0` | Success |
| `2` | Empty schedule collection |
| `101` | Expired or invalid attendance token / request |
| `107` | Invalid password |

## Authentication

### `POST /app/user/login.action`

Form fields:

| Field | Notes |
| --- | --- |
| `phone` | Account |
| `password` | Password |

Response fields retained by the client:

| Field | Notes |
| --- | --- |
| `result.id` | User ID |
| `result.sessionId` | Session token |
| `result.userName` | Account |
| `result.realName` / `result.nickName` | Display name |
| `result.classId`, `result.classInfoName`, `result.classUUID` | Optional class metadata |
| `result.picUrl` | Optional avatar URL |

## Semesters

### `POST /app/course/get_base_school_year.action`

Form fields:

| Field | Notes |
| --- | --- |
| `userId` | User ID from login |

Useful response fields:

| Field | Notes |
| --- | --- |
| `code` | Semester code |
| `name` | Semester name |
| `beginDate`, `endDate` | `YYYY-MM-DD` |
| `yearStatus` | `1` means current semester |

## Courses

### `POST /app/my/get_my_course.action`

Headers:

| Header | Notes |
| --- | --- |
| `sessionId` | Session token |

Form fields:

| Field | Notes |
| --- | --- |
| `id` | User ID |

Useful response fields:

| Field | Notes |
| --- | --- |
| `id` | Course ID |
| `courseName` | Name |
| `courseNum` | Optional catalog number |
| `teacherName` | Optional teacher |
| `classroomName` | Optional classroom |
| `beginDate`, `endDate` | Optional date range |
| `myNoSignNum` | Optional pending count |

## Schedules

### `POST /app/course/get_stu_course_sched.action`

Returns schedule rows for one day.

### `POST /app/course/get_stu_course_sched_week.action`

Returns a weekly view anchored at a date.

Headers:

| Header | Notes |
| --- | --- |
| `sessionId` | Session token |

Form fields:

| Field | Notes |
| --- | --- |
| `id` | User ID |
| `dateStr` | `YYYYMMDD` |

Useful response fields:

| Field | Notes |
| --- | --- |
| `id` | Schedule ID |
| `uuid` | Schedule UUID |
| `courseId` | Optional course ID |
| `courseName` | Course name |
| `teacherName` | Optional teacher |
| `classroomName` | Optional classroom |
| `teachTime` | `YYYY-MM-DD` |
| `classBeginTime`, `classEndTime` | `YYYY-MM-DD HH:MM:SS` |
| `signStatus` | `1` observed as signed, `0` as unsigned |

Implementation note:

- The daily endpoint is treated as the source of truth for `signStatus` when weekly data is inconsistent.
- Repeated fixed-period rows with the same course name and time range are merged into one logical lesson in the Rust layer.

## Timestamp Sync

### `POST /app/common/get_timestamp.do`

Form fields:

| Field | Notes |
| --- | --- |
| `id` | User ID |

Useful response fields:

| Field | Notes |
| --- | --- |
| `timestamp` | Server-side Unix time in milliseconds |

The client samples this endpoint after login / session refresh and may apply a conservative local offset for later request timestamps.

## Attendance Request

### `GET /app/course/stu_scan_sign.action`

Headers:

| Header | Notes |
| --- | --- |
| `sessionId` | Session token |

Common query parameters:

| Parameter | Notes |
| --- | --- |
| `id` | User ID |
| `timestamp` | Millisecond timestamp generated at request time |

Mode-specific query parameters:

| Mode | Parameter |
| --- | --- |
| UUID mode | `timeTableId=<schedule.uuid>` |
| ID mode | `courseSchedId=<schedule.id>` |

Useful response fields:

| Field | Notes |
| --- | --- |
| `result.stuSignId` | Optional upstream record ID |
| `result.stuSignStatus` | `1` observed as signed |

## Data Scope

Only fields required by the current CLI, GUI, session, and tests should be modeled. Do not mirror full upstream payloads unless a concrete caller needs them.
