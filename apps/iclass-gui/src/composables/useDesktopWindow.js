import { computed, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isTauriRuntime } from "../lib/tauri";
export function useDesktopWindow() {
    const maximized = ref(false);
    const mobileShell = computed(() => {
        if (typeof navigator === "undefined") {
            return false;
        }
        return /Android|iPhone|iPad|iPod/i.test(navigator.userAgent);
    });
    const desktopShell = computed(() => isTauriRuntime() && !mobileShell.value);
    async function runWindowCommand(task) {
        try {
            await task();
        }
        catch (error) {
            console.error("desktop window command failed", error);
        }
    }
    async function syncMaximized() {
        if (!desktopShell.value) {
            maximized.value = false;
            return;
        }
        try {
            maximized.value = await getCurrentWindow().isMaximized();
        }
        catch (error) {
            maximized.value = false;
            console.error("desktop window sync failed", error);
        }
    }
    async function minimize() {
        if (desktopShell.value) {
            await runWindowCommand(() => getCurrentWindow().minimize());
        }
    }
    async function toggleMaximize() {
        if (desktopShell.value) {
            await runWindowCommand(() => getCurrentWindow().toggleMaximize());
            await syncMaximized();
        }
    }
    async function close() {
        if (desktopShell.value) {
            await runWindowCommand(() => getCurrentWindow().close());
        }
    }
    async function startDragging() {
        if (desktopShell.value) {
            await runWindowCommand(() => getCurrentWindow().startDragging());
        }
    }
    return {
        desktopShell,
        maximized,
        syncMaximized,
        minimize,
        toggleMaximize,
        startDragging,
        close
    };
}
