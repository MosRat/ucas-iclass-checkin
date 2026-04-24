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
  checkInCustom,
  getAutomationSettings,
  getDesktopSettings,
  loadDashboard,
  loadWeekSchedule,
  login,
  logout,
  updateAutomationSettings,
  updateDesktopSettings
} from "./lib/tauri";
import SettingsPanel from "./components/SettingsPanel.vue";
import type {
  AutomationSettings,
  CheckInRequest,
  CustomCheckInRequest,
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

function localDateInputValue(date = new Date()) {
  const offset = date.getTimezoneOffset();
  const localDate = new Date(date.getTime() - offset * 60_000);
  return localDate.toISOString().slice(0, 10);
}

const dashboard = ref<DashboardSnapshot | null>(null);
const weeklySchedule = ref<WeeklyScheduleSnapshot | null>(null);
const loginLoading = ref(false);
const dashboardLoading = ref(false);
const submittingCheckIn = ref(false);
const bootstrapping = ref(true);
const selectedDate = ref(localDateInputValue());
const rememberedAccount = ref("");
const selectedCard = ref<ScheduleCard | null>(null);
const currentTime = ref(new Date().toISOString());
const statusMessage = ref("正在尝试恢复本地 session…");
const statusTone = ref<"info" | "success" | "error">("info");
const settingsOpen = ref(false);
const desktopSettingsLoading = ref(false);
const automationSettingsLoading = ref(false);
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
const automationSettings = reactive<AutomationSettings>({
  autoCheckInEnabled: false,
  autoCheckIntervalSeconds: 45,
  autoCheckInMode: "auto",
  lastAutoCheckAction: null,
  currentStatus: {
    updatedAt: new Date().toISOString(),
    status: "idle",
    message: "等待下一轮自动打卡检查。",
    schedule: null,
    availability: null,
    checkInOpensAt: null,
    canCheckIn: false,
    isSignedIn: false
  }
});
const automationSettingsSnapshot = reactive<AutomationSettings>({
  autoCheckInEnabled: false,
  autoCheckIntervalSeconds: 45,
  autoCheckInMode: "auto",
  lastAutoCheckAction: null,
  currentStatus: {
    updatedAt: new Date().toISOString(),
    status: "idle",
    message: "等待下一轮自动打卡检查。",
    schedule: null,
    availability: null,
    checkInOpensAt: null,
    canCheckIn: false,
    isSignedIn: false
  }
});
const hasShownTrayHint = ref(false);
let dashboardRequestSerial = 0;
let weekScheduleRequestSerial = 0;
let dashboardRefreshInFlight: Promise<void> | null = null;
let weekScheduleRefreshInFlight: Promise<void> | null = null;
let dashboardRefreshDate: string | null = null;
let weekScheduleRefreshDate: string | null = null;

const dialog = reactive({
  open: false,
  title: "",
  message: "",
  tone: "error" as "error" | "success" | "info",
  actionLabel: "",
  debugDetails: ""
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
      message: "输入账号密码即可同步课表、个人信息和打卡能力。",
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
let unlistenAutomationStatus: UnlistenFn | undefined;

function openDialog(
  title: string,
  message: string,
  tone: "error" | "success" | "info" = "error",
  actionLabel = "",
  debugDetails = ""
) {
  dialog.open = true;
  dialog.title = title;
  dialog.message = message;
  dialog.tone = tone;
  dialog.actionLabel = actionLabel;
  dialog.debugDetails = debugDetails;
}

function closeDialog() {
  dialog.open = false;
  dialog.actionLabel = "";
  dialog.debugDetails = "";
}

function buildDebugDetails(payload: GuiErrorPayload, context: string) {
  const lines = [
    `context=${context}`,
    `code=${payload.code}`,
    `retryable=${payload.retryable}`,
    `message=${payload.message}`
  ];
  if (payload.debug_details?.trim()) {
    lines.push("", payload.debug_details);
  }
  return lines.join("\n");
}

function openErrorDialog(title: string, payload: GuiErrorPayload, actionLabel = "", context = "gui") {
  openDialog(title, payload.message, "error", actionLabel, buildDebugDetails(payload, context));
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

function pickAutoOpenSchedule(cards: ScheduleCard[], now = new Date()) {
  const candidates = cards.filter((card) => card.can_check_in);
  if (candidates.length === 0) {
    return null;
  }

  return [...candidates].sort((left, right) => {
    const leftBeginsAt = new Date(left.schedule.begins_at).getTime();
    const leftEndsAt = new Date(left.schedule.ends_at).getTime();
    const rightBeginsAt = new Date(right.schedule.begins_at).getTime();
    const rightEndsAt = new Date(right.schedule.ends_at).getTime();
    const nowMs = now.getTime();

    const leftIsActive = leftBeginsAt <= nowMs && nowMs <= leftEndsAt;
    const rightIsActive = rightBeginsAt <= nowMs && nowMs <= rightEndsAt;

    if (leftIsActive !== rightIsActive) {
      return leftIsActive ? -1 : 1;
    }

    if (leftIsActive && rightIsActive) {
      if (leftBeginsAt !== rightBeginsAt) {
        return leftBeginsAt - rightBeginsAt;
      }
      return leftEndsAt - rightEndsAt;
    }

    const leftDistanceToStart = Math.abs(leftBeginsAt - nowMs);
    const rightDistanceToStart = Math.abs(rightBeginsAt - nowMs);
    if (leftDistanceToStart !== rightDistanceToStart) {
      return leftDistanceToStart - rightDistanceToStart;
    }

    return leftBeginsAt - rightBeginsAt;
  })[0] ?? null;
}

function syncDashboardState(next: DashboardSnapshot) {
  dashboard.value = next;
  selectedDate.value = next.schedule_date;
  rememberedAccount.value = preferences.rememberLastAccount ? next.session.account : "";
  const preserved =
    selectedCard.value &&
    next.schedules.find((card) => card.schedule.schedule_id === selectedCard.value?.schedule.schedule_id);
  const autoCheckable = pickAutoOpenSchedule(next.schedules);
  selectedCard.value =
    preserved ??
    (preferences.autoOpenCheckableSchedule
      ? autoCheckable
      : null);
}

function syncAutomationSettings(next: AutomationSettings) {
  automationSettings.autoCheckInEnabled = next.autoCheckInEnabled;
  automationSettings.autoCheckIntervalSeconds = next.autoCheckIntervalSeconds;
  automationSettings.autoCheckInMode = next.autoCheckInMode;
  automationSettings.lastAutoCheckAction = next.lastAutoCheckAction ?? null;
  automationSettings.currentStatus = { ...next.currentStatus };
  automationSettingsSnapshot.autoCheckInEnabled = automationSettings.autoCheckInEnabled;
  automationSettingsSnapshot.autoCheckIntervalSeconds = automationSettings.autoCheckIntervalSeconds;
  automationSettingsSnapshot.autoCheckInMode = automationSettings.autoCheckInMode;
  automationSettingsSnapshot.lastAutoCheckAction = automationSettings.lastAutoCheckAction;
  automationSettingsSnapshot.currentStatus = { ...automationSettings.currentStatus };
}

function isTodaySelected() {
  return selectedDate.value === localDateInputValue();
}

async function refreshWeekSchedule(date = selectedDate.value) {
  if (weekScheduleRefreshInFlight && weekScheduleRefreshDate === date) {
    return weekScheduleRefreshInFlight;
  }

  const requestId = ++weekScheduleRequestSerial;
  weekScheduleRefreshDate = date;
  weekScheduleRefreshInFlight = (async () => {
    try {
      const next = await loadWeekSchedule(date);
      if (requestId === weekScheduleRequestSerial) {
        weeklySchedule.value = next;
      }
    } catch {
      if (requestId === weekScheduleRequestSerial) {
        weeklySchedule.value = null;
      }
    } finally {
      if (weekScheduleRefreshDate === date) {
        weekScheduleRefreshDate = null;
        weekScheduleRefreshInFlight = null;
      }
    }
  })();

  return weekScheduleRefreshInFlight;
}

async function refreshDashboard(date = selectedDate.value) {
  if (dashboardRefreshInFlight && dashboardRefreshDate === date) {
    return dashboardRefreshInFlight;
  }

  const requestId = ++dashboardRequestSerial;
  dashboardRefreshDate = date;
  dashboardRefreshInFlight = (async () => {
    dashboardLoading.value = true;
    statusMessage.value = "正在同步课表与个人信息…";
    statusTone.value = "info";
    try {
      const [next, nextAutomationSettings] = await Promise.all([
        loadDashboard(date),
        getAutomationSettings().catch(() => null)
      ]);
      if (requestId !== dashboardRequestSerial) {
        return;
      }
      syncDashboardState(next);
      if (nextAutomationSettings) {
        syncAutomationSettings(nextAutomationSettings);
      }
      if (scheduleViewMode.value === "week") {
        await refreshWeekSchedule(next.schedule_date);
      }
      statusMessage.value = `已同步 ${next.schedules.length} 条课表，用时 ${next.profile.total_ms} ms。`;
      statusTone.value = "success";
    } catch (error) {
      if (requestId !== dashboardRequestSerial) {
        return;
      }
      const payload = error as GuiErrorPayload;
      if (isAuthError(payload)) {
        dashboard.value = null;
        selectedCard.value = null;
        statusMessage.value = "自动恢复失败，请重新登录。";
        statusTone.value = "error";
        if (payload.code === "InvalidCredentials") {
          openErrorDialog("登录失败", payload, "", "dashboard.auto_restore");
        }
      } else {
        statusMessage.value = payload.message;
        statusTone.value = "error";
        openErrorDialog("同步失败", payload, payload.retryable ? "重新同步" : "", "dashboard.refresh");
      }
    } finally {
      if (dashboardRefreshDate === date) {
        dashboardRefreshDate = null;
        dashboardRefreshInFlight = null;
      }
      dashboardLoading.value = false;
    }
  })();

  return dashboardRefreshInFlight;
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
    try {
      const nextAutomationSettings = await getAutomationSettings();
      syncAutomationSettings(nextAutomationSettings);
    } catch {
      statusMessage.value = "读取自动打卡设置失败，已使用当前默认设置。";
      statusTone.value = "error";
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
    const nextAutomationSettings = await getAutomationSettings().catch(() => null);
    if (nextAutomationSettings) {
      syncAutomationSettings(nextAutomationSettings);
    }
    if (scheduleViewMode.value === "week") {
      await refreshWeekSchedule(next.schedule_date);
    }
    rememberedAccount.value = preferences.rememberLastAccount ? request.account : "";
    statusMessage.value = `登录成功，工作台已准备就绪，用时 ${next.profile.total_ms} ms。`;
    statusTone.value = "success";
    openDialog(
      "登录成功",
      `欢迎回来，工作台已经同步到最新课表。\n\n本次登录与同步耗时 ${next.profile.total_ms} ms。`,
      "success"
    );
  } catch (error) {
    const payload = error as GuiErrorPayload;
    statusMessage.value = payload.message;
    statusTone.value = "error";
    openErrorDialog(
      payload.code === "InvalidCredentials" ? "账号或密码错误" : "登录失败",
      payload,
      "",
      "login.submit"
    );
  } finally {
    loginLoading.value = false;
  }
}

async function performCheckIn(card: ScheduleCard) {
  submittingCheckIn.value = true;
  statusMessage.value = `正在为 ${card.schedule.course_name} 提交打卡…`;
  statusTone.value = "info";
  try {
    await refreshDashboard(selectedDate.value);
    const freshCard =
      dashboard.value?.schedules.find((entry) => entry.schedule.schedule_id === card.schedule.schedule_id) ?? null;
    if (!freshCard) {
      throw {
        code: "NoSchedule",
        message: "刷新后未找到目标课程，请重试。",
        retryable: true
      } as GuiErrorPayload;
    }
    if (freshCard.schedule.sign_status === "1") {
      openDialog("无需重复打卡", `${freshCard.schedule.course_name} 当前已显示为已打卡。`, "info");
      statusMessage.value = `${freshCard.schedule.course_name} 已是打卡完成状态。`;
      statusTone.value = "success";
      return;
    }

    const request: CheckInRequest = {
      schedule: freshCard.schedule,
      mode: preferences.defaultCheckInMode
    };
    const result = await checkIn(request);
    await refreshDashboard(selectedDate.value);
    const refreshedCard =
      dashboard.value?.schedules.find((entry) => entry.schedule.schedule_id === result.schedule.schedule_id) ?? null;
    const verifiedSigned = result.receipt.verified_signed_in ?? (refreshedCard?.schedule.sign_status === "1");
    const verificationLine = verifiedSigned
      ? "复核：课表已显示为已打卡"
      : "复核：课表暂未显示为已打卡，请稍后刷新确认";
    openDialog(
      verifiedSigned ? "打卡成功" : "打卡已提交",
      `${result.schedule.course_name}\n方式：${result.receipt.method}\n记录：${result.receipt.record_id ?? "无"}\n${verificationLine}\n耗时：${result.profile.total_ms} ms`,
      verifiedSigned ? "success" : "info"
    );
    statusMessage.value = verifiedSigned
      ? `${result.schedule.course_name} 已完成打卡并通过课表复核。`
      : `${result.schedule.course_name} 打卡请求已提交，等待课表状态刷新。`;
    statusTone.value = verifiedSigned ? "success" : "info";
  } catch (error) {
    const payload = error as GuiErrorPayload;
    statusMessage.value = payload.message;
    statusTone.value = "error";
    openErrorDialog("打卡失败", payload, payload.retryable ? "重新同步" : "", "check_in.schedule");
  } finally {
    submittingCheckIn.value = false;
  }
}

async function performCustomCheckIn(request: CustomCheckInRequest) {
  submittingCheckIn.value = true;
  statusMessage.value = `正在使用自定义 ${request.mode.toUpperCase()} 发起打卡…`;
  statusTone.value = "info";
  let shouldRefreshDashboard = false;
  try {
    const result = await checkInCustom(request);
    openDialog(
      result.receipt.signed_in ? "打卡成功" : "打卡完成",
      `${result.schedule.course_name}\n方式：${result.receipt.method}\n标识：${request.identifier}\n记录：${result.receipt.record_id ?? "无"}\n耗时：${result.profile.total_ms} ms`,
      "success"
    );
    statusMessage.value = `自定义打卡请求已完成，用时 ${result.profile.total_ms} ms。`;
    statusTone.value = "success";
    shouldRefreshDashboard = true;
  } catch (error) {
    const payload = error as GuiErrorPayload;
    const debugDetails = [
      `gui.custom.identifier=${request.identifier}`,
      `gui.custom.mode=${request.mode}`,
      payload.debug_details?.trim() ?? ""
    ]
      .filter(Boolean)
      .join("\n");
    statusMessage.value = payload.message;
    statusTone.value = "error";
    openErrorDialog(
      "自定义打卡失败",
      {
        ...payload,
        debug_details: debugDetails
      },
      "",
      "check_in.custom"
    );
  } finally {
    submittingCheckIn.value = false;
  }

  if (shouldRefreshDashboard) {
    void refreshDashboard(selectedDate.value);
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
  Object.assign(automationSettings, automationSettingsSnapshot);
  settingsOpen.value = true;
}

function closeSettings() {
  Object.assign(desktopSettings, desktopSettingsSnapshot);
  Object.assign(automationSettings, automationSettingsSnapshot);
  settingsOpen.value = false;
}

function restoreDefaultSettings() {
  resetPreferences();
  scheduleViewMode.value = preferences.defaultScheduleView;
  statusMessage.value = "已恢复默认偏好设置。";
  statusTone.value = "success";
}

async function saveAndCloseSettings() {
  const [desktopSaved, automationSaved] = await Promise.all([
    persistDesktopSettings(),
    persistAutomationSettings()
  ]);
  const saved = desktopSaved && automationSaved;
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
    openErrorDialog(
      "更新设置失败",
      {
        ...payload,
        message: `未能保存桌面设置。\n\n${payload.message}`
      },
      "",
      "settings.desktop"
    );
    return false;
  } finally {
    desktopSettingsLoading.value = false;
  }
}

async function persistAutomationSettings(): Promise<boolean> {
  const automationChanged =
    automationSettings.autoCheckInEnabled !== automationSettingsSnapshot.autoCheckInEnabled ||
    automationSettings.autoCheckIntervalSeconds !== automationSettingsSnapshot.autoCheckIntervalSeconds ||
    automationSettings.autoCheckInMode !== automationSettingsSnapshot.autoCheckInMode;

  if (!automationChanged) {
    return true;
  }

  automationSettingsLoading.value = true;
  try {
    const next = await updateAutomationSettings({
      autoCheckInEnabled: automationSettings.autoCheckInEnabled,
      autoCheckIntervalSeconds: automationSettings.autoCheckIntervalSeconds,
      autoCheckInMode: automationSettings.autoCheckInMode
    });
    syncAutomationSettings(next);
    statusMessage.value = next.autoCheckInEnabled
      ? `自动打卡已开启，轮询间隔 ${next.autoCheckIntervalSeconds} 秒。`
      : "自动打卡已关闭。";
    statusTone.value = "success";
    return true;
  } catch (error) {
    const payload = error as GuiErrorPayload;
    statusMessage.value = payload.message;
    statusTone.value = "error";
    openErrorDialog("更新自动打卡设置失败", payload, "", "settings.automation");
    return false;
  } finally {
    automationSettingsLoading.value = false;
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
  void listen<AutomationSettings>("automation://status-updated", ({ payload }) => {
    syncAutomationSettings(payload);
    if (
      dashboard.value &&
      isTodaySelected() &&
      (payload.currentStatus.status === "success" || payload.currentStatus.status === "error")
    ) {
      void refreshDashboard(selectedDate.value);
    }
  }).then((unlisten) => {
    unlistenAutomationStatus = unlisten;
  });
  void bootstrap();
});

onBeforeUnmount(() => {
  if (ticker) {
    clearInterval(ticker);
  }
  if (unlistenTrayHidden) {
    unlistenTrayHidden();
  }
  if (unlistenAutomationStatus) {
    unlistenAutomationStatus();
  }
});
</script>

<template>
  <div class="relative isolate h-screen overflow-hidden bg-[radial-gradient(circle_at_top,_rgba(199,167,128,0.15),_rgba(250,247,241,1)_34%,_rgba(241,236,228,1)_100%)] text-ink-900">
    <div class="pointer-events-none absolute -left-20 top-16 h-56 w-56 rounded-full bg-[rgba(197,170,133,0.22)] blur-3xl"></div>
    <div class="pointer-events-none absolute right-[-4rem] top-[-2rem] h-64 w-64 rounded-full bg-[rgba(232,220,201,0.42)] blur-3xl"></div>
    <div class="pointer-events-none absolute bottom-[-5rem] left-1/3 h-72 w-72 rounded-full bg-white/45 blur-3xl"></div>
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

      <main class="min-h-0 flex-1 overflow-y-auto px-3 pb-5 pt-3 sm:px-4 md:px-6 md:pb-8">
        <div class="mx-auto max-w-7xl space-y-4 sm:space-y-5">
          <section
            class="glass-panel relative overflow-hidden"
            :class="[
              showLogin ? 'px-3 py-3 sm:px-5 sm:py-4 md:px-6' : 'px-3.5 py-3.5 sm:px-5 sm:py-4 md:px-6',
              {
                'border-[rgba(215,201,182,0.9)]': topStatus.tone === 'info',
                'border-emerald-200/70': topStatus.tone === 'success',
                'border-rose-200/80': topStatus.tone === 'error'
              }
            ]"
          >
            <div
              class="pointer-events-none absolute inset-y-0 right-0 w-52 opacity-80"
              :class="{
                'bg-[radial-gradient(circle_at_center,_rgba(196,162,118,0.2),_rgba(255,255,255,0))]': topStatus.tone === 'info',
                'bg-[radial-gradient(circle_at_center,_rgba(16,185,129,0.16),_rgba(255,255,255,0))]': topStatus.tone === 'success',
                'bg-[radial-gradient(circle_at_center,_rgba(244,63,94,0.16),_rgba(255,255,255,0))]': topStatus.tone === 'error'
              }"
            ></div>
            <div
              class="relative flex flex-col"
              :class="showLogin ? 'gap-2 sm:gap-3 md:flex-row md:items-center md:justify-between' : 'gap-3 md:flex-row md:items-center md:justify-between'"
            >
              <div class="flex min-w-0 items-start gap-2.5 sm:gap-3">
                <div
                  class="flex shrink-0 items-center justify-center rounded-3xl text-sm font-semibold"
                  :class="[
                    showLogin ? 'mt-0.5 h-8 w-8 sm:mt-1 sm:h-10 sm:w-10' : 'mt-0.5 h-9 w-9 sm:mt-1 sm:h-11 sm:w-11',
                    {
                      'bg-[rgba(244,234,218,0.92)] text-[rgb(118,85,47)]': topStatus.tone === 'info',
                      'bg-emerald-100 text-emerald-700': topStatus.tone === 'success',
                      'bg-rose-100 text-rose-700': topStatus.tone === 'error'
                    }
                  ]"
                >
                  {{ topStatus.tone === "success" ? "✓" : topStatus.tone === "error" ? "!" : "…" }}
                </div>
                <div class="min-w-0">
                  <h2 class="font-semibold text-ink-950" :class="showLogin ? 'text-[14px] sm:text-lg' : 'text-[15px] sm:text-lg'">
                    {{ topStatus.title }}
                  </h2>
                  <p class="mt-1 text-ink-600" :class="showLogin ? 'text-sm leading-6' : 'text-sm leading-6 sm:text-sm'">
                    {{ showLogin ? statusMessage : topStatus.message }}
                  </p>
                </div>
              </div>
              <div class="flex flex-wrap items-center gap-2 text-sm text-ink-500" :class="showLogin ? 'hidden sm:flex' : ''">
                <span class="hidden rounded-full border border-[rgba(223,213,198,0.9)] bg-[rgba(255,252,247,0.9)] px-3 py-1.5 shadow-[0_8px_20px_rgba(90,70,43,0.05)] sm:inline-flex">
                  {{ scheduleViewMode === "week" ? "周视图" : "日视图" }}
                </span>
                <span class="hidden rounded-full border border-[rgba(223,213,198,0.9)] bg-[rgba(255,252,247,0.9)] px-3 py-1.5 shadow-[0_8px_20px_rgba(90,70,43,0.05)] sm:inline-flex">
                  {{ dashboard ? `已同步 ${dashboard.schedules.length} 个时段` : "等待同步" }}
                </span>
                <span class="max-w-full rounded-full border border-[rgba(223,213,198,0.9)] bg-[rgba(255,252,247,0.9)] px-2.5 py-1.5 text-xs shadow-[0_8px_20px_rgba(90,70,43,0.05)] sm:px-3 sm:text-sm">{{ statusMessage }}</span>
              </div>
            </div>
          </section>

          <section
            v-if="desktopSettings.closeToTray && hasShownTrayHint"
            class="rounded-4xl border border-[rgba(218,205,188,0.9)] bg-[rgba(255,251,246,0.9)] px-5 py-4 text-sm leading-6 text-ink-600 shadow-pane"
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
            <ProfileSummary :dashboard="dashboard" :automation-settings="automationSettings" />
            <ScheduleBoard
              :dashboard="dashboard"
              :weekly-schedule="weeklySchedule"
              :loading="dashboardLoading || submittingCheckIn"
              :automation-settings="automationSettings"
              :compact="preferences.compactScheduleCards"
              :search="scheduleSearch"
              :selected-date="selectedDate"
              :selected-schedule-id="selectedCard?.schedule.schedule_id"
              :view-mode="scheduleViewMode"
              @change-date="refreshDashboard"
              @check-in="performCheckIn"
              @refresh="handleRefresh"
              @select="selectSchedule"
              @custom-check-in="performCustomCheckIn"
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
        :debug-details="dialog.debugDetails"
        :message="dialog.message"
        :open="dialog.open"
        :title="dialog.title"
      :tone="dialog.tone"
      @action="runDialogAction"
      @close="closeDialog"
    />

    <SettingsPanel
      :automation-settings="automationSettings"
      :automation-loading="automationSettingsLoading"
      :desktop-shell="desktopShell"
      :open="settingsOpen"
      :desktop-loading="desktopSettingsLoading"
      :desktop-settings="desktopSettings"
      :preferences="preferences"
      @close="saveAndCloseSettings"
      @reset="restoreDefaultSettings"
    />
  </div>
</template>
