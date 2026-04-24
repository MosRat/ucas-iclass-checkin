<script setup lang="ts">
import { computed } from "vue";
import type { AutomationSettings, AutoCheckStatusKind } from "../lib/types";

const props = defineProps<{
  automationSettings: AutomationSettings;
}>();

const currentStatus = computed(() => props.automationSettings.currentStatus);
const currentSchedule = computed(() => currentStatus.value.schedule ?? null);

const statusTone = computed(() => {
  if (!props.automationSettings.autoCheckInEnabled) {
    return "muted";
  }

  const kind = currentStatus.value.status;
  if (kind === "success") {
    return "success";
  }
  if (kind === "error") {
    return "error";
  }
  if (kind === "attempting" || kind === "ready") {
    return "active";
  }
  return "muted";
});

const statusLabel = computed(() => renderStatusLabel(currentStatus.value.status));

const modeLabel = computed(() => {
  if (props.automationSettings.autoCheckInMode === "uuid") {
    return "UUID";
  }
  if (props.automationSettings.autoCheckInMode === "id") {
    return "ID";
  }
  return "Auto";
});

function renderStatusLabel(kind: AutoCheckStatusKind) {
  switch (kind) {
    case "waitingWindow":
      return "等待开放";
    case "ready":
      return "候选已就绪";
    case "attempting":
      return "正在尝试";
    case "success":
      return "最近成功";
    case "error":
      return "最近失败";
    default:
      return "待命中";
  }
}

function renderAvailability() {
  if (!currentSchedule.value) {
    return "当前没有候选课程";
  }
  if (currentStatus.value.is_signed_in) {
    return "课表已显示已打卡";
  }
  if (currentStatus.value.availability === "Open") {
    return "当前在打卡时间窗口内";
  }
  if (currentStatus.value.availability === "Closed") {
    return "本课程的打卡窗口已结束";
  }
  if (currentStatus.value.check_in_opens_at) {
    return `预计 ${new Date(currentStatus.value.check_in_opens_at).toLocaleTimeString("zh-CN", {
      hour: "2-digit",
      minute: "2-digit"
    })} 开放`;
  }
  return "等待下一次状态刷新";
}
</script>

<template>
  <div class="glass-panel p-3.5 sm:p-5">
    <div class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
      <div>
        <h3 class="text-lg font-semibold text-ink-950">自动打卡状态</h3>
        <p class="mt-1 text-sm leading-6 text-ink-600">
          {{
            automationSettings.autoCheckInEnabled
              ? `已开启，按 ${modeLabel} 模式每 ${automationSettings.autoCheckIntervalSeconds} 秒刷新一次候选课程。`
              : "未开启，可在设置中打开自动打卡。"
          }}
        </p>
      </div>
      <span
        class="inline-flex items-center gap-2 self-start rounded-full border px-3 py-1.5 text-xs font-semibold sm:text-sm"
        :class="
          statusTone === 'success'
            ? 'border-emerald-200 bg-emerald-50 text-emerald-700'
            : statusTone === 'error'
              ? 'border-rose-200 bg-rose-50 text-rose-700'
              : statusTone === 'active'
                ? 'border-accent-200 bg-accent-50 text-accent-700'
                : 'border-slate-200 bg-white/80 text-ink-500'
        "
      >
        <span
          class="h-2 w-2 rounded-full"
          :class="
            statusTone === 'success'
              ? 'bg-emerald-500'
              : statusTone === 'error'
                ? 'bg-rose-500'
                : statusTone === 'active'
                  ? 'bg-accent-500'
                  : 'bg-slate-300'
          "
        ></span>
        {{ statusLabel }}
      </span>
    </div>

    <div class="mt-4 grid gap-3 md:grid-cols-[1.2fr_0.8fr]">
      <article class="rounded-3xl border border-white/70 bg-white/80 px-4 py-4">
        <p class="text-xs font-medium uppercase tracking-[0.18em] text-ink-400">当前候选课程</p>
        <template v-if="currentSchedule">
          <h4 class="mt-2 text-base font-semibold text-ink-900">{{ currentSchedule.course_name }}</h4>
          <p class="mt-1 text-sm text-ink-500">
            {{ currentSchedule.teacher_name || "教师未标注" }} · {{ currentSchedule.classroom_name || "地点未标注" }}
          </p>
          <p class="mt-2 text-sm leading-6 text-ink-600">{{ renderAvailability() }}</p>
          <p class="mt-3 text-xs leading-5 text-ink-500">
            {{ new Date(currentSchedule.begins_at).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" }) }}
            -
            {{ new Date(currentSchedule.ends_at).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" }) }}
            · {{ currentSchedule.schedule_id }}
          </p>
        </template>
        <template v-else>
          <p class="mt-2 text-sm leading-6 text-ink-600">{{ currentStatus.message }}</p>
        </template>
      </article>

      <article class="rounded-3xl border border-white/70 bg-white/80 px-4 py-4">
        <p class="text-xs font-medium uppercase tracking-[0.18em] text-ink-400">最近动作</p>
        <template v-if="automationSettings.lastAutoCheckAction">
          <p
            class="mt-2 text-sm font-semibold"
            :class="automationSettings.lastAutoCheckAction.succeeded ? 'text-emerald-700' : 'text-rose-700'"
          >
            {{ automationSettings.lastAutoCheckAction.succeeded ? "自动打卡成功" : "自动打卡未完成" }}
          </p>
          <p class="mt-1 text-sm text-ink-600">
            {{ automationSettings.lastAutoCheckAction.course_name }} ·
            {{ automationSettings.lastAutoCheckAction.schedule_id }}
          </p>
          <p class="mt-2 text-xs leading-5 text-ink-500">
            {{ new Date(automationSettings.lastAutoCheckAction.attempted_at).toLocaleString("zh-CN", { hour12: false }) }}
            · {{ automationSettings.lastAutoCheckAction.message }}
          </p>
        </template>
        <template v-else>
          <p class="mt-2 text-sm leading-6 text-ink-600">当前还没有自动打卡记录。</p>
        </template>
      </article>
    </div>

    <p class="mt-3 text-xs leading-5 text-ink-500">
      最近状态刷新：
      {{ new Date(currentStatus.updated_at).toLocaleString("zh-CN", { hour12: false }) }}
      <span v-if="currentSchedule"> · {{ currentStatus.message }}</span>
    </p>
  </div>
</template>
