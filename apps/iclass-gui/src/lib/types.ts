export type GuiErrorCode =
  | "AuthenticationRequired"
  | "InvalidCredentials"
  | "MissingCredentials"
  | "NoSchedule"
  | "UnsupportedCheckInMode"
  | "QrExpired"
  | "CheckInTooEarly"
  | "CheckInClosed"
  | "Storage"
  | "Network"
  | "Parameter"
  | "Parse"
  | "Business";

export interface GuiErrorPayload {
  code: GuiErrorCode;
  message: string;
  retryable: boolean;
}

export interface SessionSummary {
  account: string;
  real_name: string;
  authenticated: boolean;
}

export interface Semester {
  code: string;
  name: string;
  begin_date: string;
  end_date: string;
  current: boolean;
}

export interface Course {
  id: string;
  name: string;
  course_num?: string | null;
  teacher_name?: string | null;
  classroom_name?: string | null;
  begin_date?: string | null;
  end_date?: string | null;
  pending_checkins?: number | null;
}

export interface ScheduleEntry {
  schedule_id: string;
  schedule_uuid?: string | null;
  course_id?: string | null;
  course_name: string;
  teacher_name?: string | null;
  classroom_name?: string | null;
  teach_date: string;
  begins_at: string;
  ends_at: string;
  lesson_units: number;
  sign_status?: string | null;
}

export type CheckInAvailability = "NotOpenYet" | "Open" | "Closed";

export interface ScheduleCard {
  schedule: ScheduleEntry;
  check_in_opens_at: string;
  availability: CheckInAvailability;
  can_check_in: boolean;
}

export interface DashboardSnapshot {
  generated_at: string;
  session: SessionSummary;
  semesters: Semester[];
  courses: Course[];
  schedule_date: string;
  schedules: ScheduleCard[];
}

export interface WeeklyScheduleSnapshot {
  generated_at: string;
  week_start: string;
  week_end: string;
  schedules: ScheduleCard[];
}

export interface CheckInReceipt {
  method: "Uuid" | "Id";
  record_id?: string | null;
  signed_in: boolean;
  status_code: string;
}

export interface CheckInViewModel {
  schedule: ScheduleEntry;
  receipt: CheckInReceipt;
}

export interface LoginRequest {
  account: string;
  password: string;
  rememberPassword: boolean;
}

export interface CheckInRequest {
  schedule: ScheduleEntry;
  mode?: "auto" | "uuid" | "id";
}

export interface DesktopSettings {
  autostartEnabled: boolean;
  closeToTray: boolean;
  autostartAvailable: boolean;
  closeToTrayAvailable: boolean;
}
