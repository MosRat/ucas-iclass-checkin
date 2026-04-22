import { invoke } from "@tauri-apps/api/core";
export function isTauriRuntime() {
    return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
function normalizeError(error) {
    if (typeof error === "string") {
        try {
            return normalizeError(JSON.parse(error));
        }
        catch {
            return {
                code: "Business",
                message: error,
                retryable: false
            };
        }
    }
    if (typeof error === "object" && error !== null) {
        const payload = error;
        if (typeof payload.code === "string" &&
            typeof payload.message === "string" &&
            typeof payload.retryable === "boolean") {
            return payload;
        }
        const withMessage = error;
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
async function invokeCommand(command, args) {
    if (!isTauriRuntime()) {
        throw normalizeError({
            code: "Business",
            message: "当前是浏览器预览模式，请使用 `pnpm tauri dev` 启动桌面应用。",
            retryable: false
        });
    }
    try {
        return await invoke(command, args);
    }
    catch (error) {
        throw normalizeError(error);
    }
}
export function loadDashboard(date) {
    return invokeCommand("load_dashboard", { date });
}
export function login(request) {
    return invokeCommand("login", { request });
}
export function loadWeekSchedule(date) {
    return invokeCommand("load_week_schedule", { date });
}
export function checkIn(request) {
    return invokeCommand("check_in", { request });
}
export function checkInCustom(request) {
    return invokeCommand("check_in_custom", { request });
}
export function getDesktopSettings() {
    return invokeCommand("get_desktop_settings");
}
export function updateDesktopSettings(request) {
    return invokeCommand("update_desktop_settings", { request });
}
export function getAutomationSettings() {
    return invokeCommand("get_automation_settings");
}
export function updateAutomationSettings(request) {
    return invokeCommand("update_automation_settings", { request });
}
export function logout() {
    return invokeCommand("logout");
}
