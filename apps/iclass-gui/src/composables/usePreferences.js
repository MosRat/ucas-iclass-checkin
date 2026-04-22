import { reactive, watch } from "vue";
const STORAGE_KEY = "ucas-iclass.preferences";
const DEFAULT_PREFERENCES = {
    autoSyncOnLaunch: true,
    autoOpenCheckableSchedule: true,
    rememberLastAccount: true,
    compactScheduleCards: false,
    defaultCheckInMode: "auto",
    defaultScheduleView: "day"
};
function loadPreferences() {
    if (typeof window === "undefined") {
        return { ...DEFAULT_PREFERENCES };
    }
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) {
        return { ...DEFAULT_PREFERENCES };
    }
    try {
        const parsed = JSON.parse(raw);
        return {
            autoSyncOnLaunch: parsed.autoSyncOnLaunch ?? DEFAULT_PREFERENCES.autoSyncOnLaunch,
            autoOpenCheckableSchedule: parsed.autoOpenCheckableSchedule ?? DEFAULT_PREFERENCES.autoOpenCheckableSchedule,
            rememberLastAccount: parsed.rememberLastAccount ?? DEFAULT_PREFERENCES.rememberLastAccount,
            compactScheduleCards: parsed.compactScheduleCards ?? DEFAULT_PREFERENCES.compactScheduleCards,
            defaultCheckInMode: parsed.defaultCheckInMode ?? DEFAULT_PREFERENCES.defaultCheckInMode,
            defaultScheduleView: parsed.defaultScheduleView ?? DEFAULT_PREFERENCES.defaultScheduleView
        };
    }
    catch {
        return { ...DEFAULT_PREFERENCES };
    }
}
export function usePreferences() {
    const preferences = reactive(loadPreferences());
    watch(preferences, (value) => {
        if (typeof window !== "undefined") {
            window.localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
        }
    }, { deep: true });
    function resetPreferences() {
        Object.assign(preferences, DEFAULT_PREFERENCES);
    }
    return {
        preferences,
        resetPreferences
    };
}
