import { invoke } from "@tauri-apps/api/core";
import type {
  CheckInRequest,
  CheckInViewModel,
  DashboardSnapshot,
  DesktopSettings,
  GuiErrorPayload,
  LoginRequest,
  WeeklyScheduleSnapshot
} from "./types";

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function normalizeError(error: unknown): GuiErrorPayload {
  if (typeof error === "string") {
    try {
      return normalizeError(JSON.parse(error));
    } catch {
      return {
        code: "Business",
        message: error,
        retryable: false
      };
    }
  }

  if (typeof error === "object" && error !== null) {
    const payload = error as Partial<GuiErrorPayload>;
    if (
      typeof payload.code === "string" &&
      typeof payload.message === "string" &&
      typeof payload.retryable === "boolean"
    ) {
      return payload as GuiErrorPayload;
    }

    const withMessage = error as { message?: unknown; error?: unknown; cause?: unknown };
    for (const candidate of [withMessage.message, withMessage.error, withMessage.cause]) {
      if (typeof candidate === "string" && candidate.trim()) {
        return {
          code: "Business",
          message: candidate,
          retryable: false
        };
      }
    }
  }

  return {
    code: "Business",
    message: error instanceof Error ? error.message : "发生了未知错误。",
    retryable: false
  };
}

async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauriRuntime()) {
    throw normalizeError({
      code: "Business",
      message: "当前是浏览器预览模式，请使用 `pnpm tauri dev` 启动桌面应用。",
      retryable: false
    });
  }

  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw normalizeError(error);
  }
}

export function loadDashboard(date?: string): Promise<DashboardSnapshot> {
  return invokeCommand("load_dashboard", { date });
}

export function login(request: LoginRequest): Promise<DashboardSnapshot> {
  return invokeCommand("login", { request });
}

export function loadWeekSchedule(date?: string): Promise<WeeklyScheduleSnapshot> {
  return invokeCommand("load_week_schedule", { date });
}

export function checkIn(request: CheckInRequest): Promise<CheckInViewModel> {
  return invokeCommand("check_in", { request });
}

export function getDesktopSettings(): Promise<DesktopSettings> {
  return invokeCommand("get_desktop_settings");
}

export function updateDesktopSettings(request: DesktopSettings): Promise<DesktopSettings> {
  return invokeCommand("update_desktop_settings", { request });
}

export function logout(): Promise<void> {
  return invokeCommand("logout");
}
