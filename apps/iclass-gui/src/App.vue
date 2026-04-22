<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import AppTitleBar from "./components/AppTitleBar.vue";
import ErrorDialog from "./components/ErrorDialog.vue";
import LoginPanel from "./components/LoginPanel.vue";
import ProfileSummary from "./components/ProfileSummary.vue";
import ScheduleDetailSheet from "./components/ScheduleDetailSheet.vue";
import ScheduleBoard from "./components/ScheduleBoard.vue";
import { useDesktopWindow } from "./composables/useDesktopWindow";
import { usePreferences } from "./composables/usePreferences";
import {
  checkIn,
  getDesktopSettings,
  loadDashboard,
  loadWeekSchedule,
  login,
  logout,
  updateDesktopSettings
} from "./lib/tauri";
import SettingsPanel from "./components/SettingsPanel.vue";
import type {
  CheckInRequest,
  DashboardSnapshot,
  DesktopSettings,
  GuiErrorPayload,
  LoginRequest,
  ScheduleCard,
  WeeklyScheduleSnapshot
} from "./lib/types";

const desktopWindow = useDesktopWindow();
const { desktopShell, maximized, syncMaximized, minimize, toggleMaximize, startDragging, close } = desktopWindow;
const { preferences, resetPreferences } = usePreferences();
const dashboard = ref<DashboardSnapshot | null>(null);
const weeklySchedule = ref<WeeklyScheduleSnapshot | null>(null);
const loginLoading = ref(false);
const dashboardLoading = ref(false);
const submittingCheckIn = ref(false);
const bootstrapping = ref(true);
const selectedDate = ref(new Date().toISOString().slice(0, 10));
const rememberedAccount = ref("");
const selectedCard = ref<ScheduleCard | null>(null);
const currentTime = ref(new Date().toISOString());
const statusMessage = ref("正在尝试恢复本地 session…");
const statusTone = ref<"info" | "success" | "error">("info");
const settingsOpen = ref(false);
const desktopSettingsLoading = ref(false);
const scheduleViewMode = ref<"day" | "week">(preferences.defaultScheduleView);
const scheduleSearch = ref("");
const desktopSettings = reactive<DesktopSettings>({
  autostartEnabled: false,
  closeToTray: false,
  autostartAvailable: false,
  closeToTrayAvailable: false
});
const desktopSettingsSnapshot = reactive<DesktopSettings>({
  autostartEnabled: false,
  closeToTray: false,
  autostartAvailable: false,
  closeToTrayAvailable: false
});
const hasShownTrayHint = ref(false);

const dialog = reactive({
  open: false,
  title: "",
  message: "",
  tone: "error" as "error" | "success" | "info",
  actionLabel: ""
});

const showLogin = computed(() => dashboard.value === null);
const busy = computed(() => bootstrapping.value || loginLoading.value || dashboardLoading.value || submittingCheckIn.value);
const currentSemesterName = computed(
  () => dashboard.value?.semesters.find((semester) => semester.current)?.name ?? dashboard.value?.semesters[0]?.name ?? "尚未同步"
);
const topStatus = computed(() => {
  if (bootstrapping.value) {
    return {
      title: "正在恢复工作台",
      message: "优先使用本地 session 自动恢复，如果失效会回退到登录界面。",
      tone: "info" as const
    };
  }
  if (!dashboard.value) {
    return {
      title: "等待登录",
      message: "输入账号密码后即可同步个人信息、课表和打卡能力。",
      tone: statusTone.value
    };
  }
  return {
    title: "同步完成",
    message: `当前学期：${currentSemesterName.value}，已同步 ${dashboard.value.schedules.length} 条课表。`,
    tone: "success" as const
  };
});

let ticker: ReturnType<typeof setInterval> | undefined;
let unlistenTrayHidden: UnlistenFn | undefined;

function openDialog(
  title: string,
  message: string,
  tone: "error" | "success" | "info" = "error",
  actionLabel = ""
) {
  dialog.open = true;
  dialog.title = title;
  dialog.message = message;
  dialog.tone = tone;
  dialog.actionLabel = actionLabel;
}

function closeDialog() {
  dialog.open = false;
  dialog.actionLabel = "";
}

function runDialogAction() {
  const actionLabel = dialog.actionLabel;
  closeDialog();
  if (actionLabel === "重新同步") {
    void refreshDashboard(selectedDate.value);
  }
}

function selectSchedule(card: ScheduleCard) {
  selectedCard.value = card;
}

function closeScheduleDetail() {
  selectedCard.value = null;
}

function isAuthError(error: GuiErrorPayload) {
  return (
    error.code === "AuthenticationRequired" ||
    error.code === "MissingCredentials" ||
    error.code === "InvalidCredentials"
  );
}

function syncDashboardState(next: DashboardSnapshot) {
  dashboard.value = next;
  selectedDate.value = next.schedule_date;
  rememberedAccount.value = preferences.rememberLastAccount ? next.session.account : "";
  const preserved =
    selectedCard.value &&
    next.schedules.find((card) => card.schedule.schedule_id === selectedCard.value?.schedule.schedule_id);
  selectedCard.value =
    preserved ??
    (preferences.autoOpenCheckableSchedule
      ? next.schedules.find((card) => card.can_check_in) ?? next.schedules[0] ?? null
      : null);
}

async function refreshWeekSchedule(date = selectedDate.value) {
  try {
    weeklySchedule.value = await loadWeekSchedule(date);
  } catch {
    weeklySchedule.value = null;
  }
}

async function refreshDashboard(date = selectedDate.value) {
  dashboardLoading.value = true;
  statusMessage.value = "正在同步课表与个人信息…";
  statusTone.value = "info";
  try {
    const next = await loadDashboard(date);
    syncDashboardState(next);
    if (scheduleViewMode.value === "week") {
      await refreshWeekSchedule(next.schedule_date);
    }
    statusMessage.value = `已同步 ${next.schedules.length} 条课表。`;
    statusTone.value = "success";
  } catch (error) {
    const payload = error as GuiErrorPayload;
    if (isAuthError(payload)) {
      dashboard.value = null;
      selectedCard.value = null;
      statusMessage.value = "自动恢复失败，请重新登录。";
      statusTone.value = "error";
      if (payload.code === "InvalidCredentials") {
        openDialog("登录失败", payload.message);
      }
    } else {
      statusMessage.value = payload.message;
      statusTone.value = "error";
      openDialog("同步失败", payload.message, "error", payload.retryable ? "重新同步" : "");
    }
  } finally {
    dashboardLoading.value = false;
  }
}

async function bootstrap() {
  bootstrapping.value = true;
  try {
    await syncMaximized();
    if (desktopShell.value) {
      try {
        const nextDesktopSettings = await getDesktopSettings();
        desktopSettings.autostartEnabled = nextDesktopSettings.autostartEnabled;
        desktopSettings.closeToTray = nextDesktopSettings.closeToTray;
        desktopSettings.autostartAvailable = nextDesktopSettings.autostartAvailable;
        desktopSettings.closeToTrayAvailable = nextDesktopSettings.closeToTrayAvailable;
        Object.assign(desktopSettingsSnapshot, desktopSettings);
      } catch {
        statusMessage.value = "读取桌面设置失败，已使用当前默认设置继续启动。";
        statusTone.value = "error";
      }
    }
    if (preferences.autoSyncOnLaunch) {
      await refreshDashboard();
    } else {
      statusMessage.value = "已跳过启动同步，可在登录后或工作台内手动刷新。";
      statusTone.value = "info";
    }
  } finally {
    bootstrapping.value = false;
  }
}

async function submitLogin(request: LoginRequest) {
  loginLoading.value = true;
  statusMessage.value = "正在验证账号并同步工作台…";
  statusTone.value = "info";
  try {
    const next = await login(request);
    syncDashboardState(next);
    if (scheduleViewMode.value === "week") {
      await refreshWeekSchedule(next.schedule_date);
    }
    rememberedAccount.value = preferences.rememberLastAccount ? request.account : "";
    statusMessage.value = "登录成功，工作台已准备就绪。";
    statusTone.value = "success";
    openDialog("登录成功", "欢迎回来，工作台已经同步到最新课表。", "success");
  } catch (error) {
    const payload = error as GuiErrorPayload;
    statusMessage.value = payload.message;
    statusTone.value = "error";
    openDialog(payload.code === "InvalidCredentials" ? "账号或密码错误" : "登录失败", payload.message);
  } finally {
    loginLoading.value = false;
  }
}

async function performCheckIn(card: ScheduleCard) {
  submittingCheckIn.value = true;
  statusMessage.value = `正在为 ${card.schedule.course_name} 提交打卡…`;
  statusTone.value = "info";
  try {
    const request: CheckInRequest = {
      schedule: card.schedule,
      mode: preferences.defaultCheckInMode
    };
    const result = await checkIn(request);
    openDialog(
      result.receipt.signed_in ? "打卡成功" : "打卡完成",
      `${result.schedule.course_name}\n方式：${result.receipt.method}\n记录：${result.receipt.record_id ?? "无"}`,
      "success"
    );
    statusMessage.value = `${result.schedule.course_name} 打卡请求已完成。`;
    statusTone.value = "success";
    await refreshDashboard(selectedDate.value);
  } catch (error) {
    const payload = error as GuiErrorPayload;
    statusMessage.value = payload.message;
    statusTone.value = "error";
    openDialog("打卡失败", payload.message, "error", payload.retryable ? "重新同步" : "");
  } finally {
    submittingCheckIn.value = false;
  }
}

async function handleLogout() {
  await logout();
  dashboard.value = null;
  weeklySchedule.value = null;
  selectedCard.value = null;
  statusMessage.value = "本地 session 已清除。";
  statusTone.value = "info";
  openDialog("已退出登录", "本地 session 已清除，下次需要重新登录。", "info");
}

function openSettings() {
  Object.assign(desktopSettings, desktopSettingsSnapshot);
  settingsOpen.value = true;
}

function closeSettings() {
  Object.assign(desktopSettings, desktopSettingsSnapshot);
  settingsOpen.value = false;
}

function restoreDefaultSettings() {
  resetPreferences();
  scheduleViewMode.value = preferences.defaultScheduleView;
  statusMessage.value = "已恢复默认偏好设置。";
  statusTone.value = "success";
}

async function saveAndCloseSettings() {
  const saved = await persistDesktopSettings();
  if (saved) {
    settingsOpen.value = false;
  }
}

async function persistDesktopSettings(): Promise<boolean> {
  if (!desktopShell.value) {
    return true;
  }

  const desktopSettingsChanged =
    desktopSettings.autostartEnabled !== desktopSettingsSnapshot.autostartEnabled ||
    desktopSettings.closeToTray !== desktopSettingsSnapshot.closeToTray;

  if (!desktopSettingsChanged) {
    statusMessage.value = "桌面设置未变更。";
    statusTone.value = "info";
    return true;
  }

  desktopSettingsLoading.value = true;
  try {
    const next = await updateDesktopSettings({
      autostartEnabled: desktopSettings.autostartEnabled,
      closeToTray: desktopSettings.closeToTray
    });
    desktopSettings.autostartEnabled = next.autostartEnabled;
    desktopSettings.closeToTray = next.closeToTray;
    desktopSettings.autostartAvailable = next.autostartAvailable;
    desktopSettings.closeToTrayAvailable = next.closeToTrayAvailable;
    Object.assign(desktopSettingsSnapshot, desktopSettings);
    statusMessage.value = "桌面设置已更新。";
    statusTone.value = "success";
    return true;
  } catch (error) {
    const payload = error as GuiErrorPayload;
    statusMessage.value = payload.message;
    statusTone.value = "error";
    openDialog("更新设置失败", `未能保存桌面设置。\n\n${payload.message}`, "error");
    return false;
  } finally {
    desktopSettingsLoading.value = false;
  }
}

function updateViewMode(mode: "day" | "week") {
  scheduleViewMode.value = mode;
  preferences.defaultScheduleView = mode;
  if (mode === "week" && !weeklySchedule.value) {
    void refreshWeekSchedule(selectedDate.value);
  }
}

function updateSearch(value: string) {
  scheduleSearch.value = value;
}

async function handleRefresh() {
  await refreshDashboard(selectedDate.value);
  if (scheduleViewMode.value === "week") {
    await refreshWeekSchedule(selectedDate.value);
  }
}

onMounted(() => {
  ticker = setInterval(() => {
    currentTime.value = new Date().toISOString();
  }, 1000);
  if (desktopShell.value) {
    void listen("desktop://tray-hidden", () => {
      if (desktopSettings.closeToTray && !hasShownTrayHint.value) {
        hasShownTrayHint.value = true;
      }
    }).then((unlisten) => {
      unlistenTrayHidden = unlisten;
    });
  }
  void bootstrap();
});

onBeforeUnmount(() => {
  if (ticker) {
    clearInterval(ticker);
  }
  if (unlistenTrayHidden) {
    unlistenTrayHidden();
  }
});
</script>

<template>
  <div class="h-screen overflow-hidden bg-[radial-gradient(circle_at_top,_rgba(63,131,248,0.18),_rgba(244,247,251,1)_34%,_rgba(232,238,247,1)_100%)] text-ink-900">
    <div class="mx-auto flex h-screen max-w-[1600px] flex-col overflow-hidden">
      <AppTitleBar
        :desktop-shell="desktopShell"
        :maximized="maximized"
        @close="close"
        @drag="startDragging"
        @maximize="toggleMaximize"
        @minimize="minimize"
        @settings="openSettings"
      />

      <main class="min-h-0 flex-1 overflow-y-auto px-4 pb-6 pt-4 md:px-6 md:pb-8">
        <div class="mx-auto max-w-7xl space-y-5">
          <section
            class="glass-panel flex flex-col gap-4 px-5 py-4 md:flex-row md:items-center md:justify-between"
            :class="{
              'border-accent-200/70': topStatus.tone === 'info',
              'border-emerald-200/70': topStatus.tone === 'success',
              'border-rose-200/80': topStatus.tone === 'error'
            }"
          >
            <div class="flex min-w-0 items-start gap-4">
              <div
                class="mt-1 flex h-11 w-11 shrink-0 items-center justify-center rounded-3xl text-sm font-semibold"
                :class="{
                  'bg-accent-100 text-accent-700': topStatus.tone === 'info',
                  'bg-emerald-100 text-emerald-700': topStatus.tone === 'success',
                  'bg-rose-100 text-rose-700': topStatus.tone === 'error'
                }"
              >
                {{ topStatus.tone === "success" ? "✓" : topStatus.tone === "error" ? "!" : "…" }}
              </div>
              <div class="min-w-0">
                <h2 class="text-base font-semibold text-ink-950">{{ topStatus.title }}</h2>
                <p class="mt-1 text-sm leading-6 text-ink-600">{{ topStatus.message }}</p>
              </div>
            </div>
            <div class="flex flex-wrap items-center gap-2 text-sm text-ink-500">
              <span class="rounded-full bg-white/80 px-3 py-1">
                {{ scheduleViewMode === "week" ? "周视图" : "日视图" }}
              </span>
              <span class="rounded-full bg-white/80 px-3 py-1">
                {{ dashboard ? `已同步 ${dashboard.schedules.length} 个时段` : "等待同步" }}
              </span>
              <span class="rounded-full bg-white/80 px-3 py-1">{{ statusMessage }}</span>
            </div>
          </section>

          <section
            v-if="desktopSettings.closeToTray && hasShownTrayHint"
            class="rounded-4xl border border-accent-200/80 bg-white/88 px-5 py-4 text-sm leading-6 text-ink-600 shadow-pane"
          >
            已隐藏到系统托盘。可通过托盘图标重新打开主窗口；如果不希望这样工作，可以在设置里关闭“关闭时隐藏到托盘”。
          </section>

          <template v-if="showLogin">
            <LoginPanel
              :account="rememberedAccount"
              :loading="busy"
              @submit="submitLogin"
            />
          </template>

          <template v-else-if="dashboard">
            <ProfileSummary :dashboard="dashboard" />
            <ScheduleBoard
              :dashboard="dashboard"
              :weekly-schedule="weeklySchedule"
              :loading="dashboardLoading || submittingCheckIn"
              :compact="preferences.compactScheduleCards"
              :search="scheduleSearch"
              :selected-date="selectedDate"
              :selected-schedule-id="selectedCard?.schedule.schedule_id"
              :view-mode="scheduleViewMode"
              @change-date="refreshDashboard"
              @check-in="performCheckIn"
              @refresh="handleRefresh"
              @select="selectSchedule"
              @update-search="updateSearch"
              @update-view-mode="updateViewMode"
              @logout="handleLogout"
            />
          </template>
        </div>
      </main>
    </div>

    <ScheduleDetailSheet
      :card="selectedCard"
      :current-time="currentTime"
      :loading="submittingCheckIn"
      @check-in="performCheckIn"
      @close="closeScheduleDetail"
    />

    <ErrorDialog
      :action-label="dialog.actionLabel"
      :message="dialog.message"
      :open="dialog.open"
      :title="dialog.title"
      :tone="dialog.tone"
      @action="runDialogAction"
      @close="closeDialog"
    />

    <SettingsPanel
      :open="settingsOpen"
      :desktop-loading="desktopSettingsLoading"
      :desktop-settings="desktopSettings"
      :preferences="preferences"
      @close="saveAndCloseSettings"
      @reset="restoreDefaultSettings"
    />
  </div>
</template>
