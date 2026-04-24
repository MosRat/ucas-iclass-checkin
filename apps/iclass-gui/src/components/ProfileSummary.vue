<script setup lang="ts">
import { computed } from "vue";
import { formatDateTime } from "../lib/datetime";
import type { AutomationSettings, DashboardSnapshot } from "../lib/types";

const props = defineProps<{
  dashboard: DashboardSnapshot;
  automationSettings: AutomationSettings;
}>();

const currentSemester = computed(
  () => props.dashboard.semesters.find((semester) => semester.current) ?? props.dashboard.semesters[0] ?? null
);

const pendingTotal = computed(() =>
  props.dashboard.courses.reduce((total, course) => total + (course.pending_checkins ?? 0), 0)
);

const signedToday = computed(() =>
  props.dashboard.schedules.filter((item) => item.schedule.sign_status === "1").length
);

const automationStatus = computed(() => {
  if (props.automationSettings.autoCheckInEnabled) {
    const modeLabel =
      props.automationSettings.autoCheckInMode === "auto"
        ? "Auto"
        : props.automationSettings.autoCheckInMode === "uuid"
          ? "UUID"
          : "ID";
    const currentCourse = props.automationSettings.currentStatus.schedule?.course_name;
    return {
      active: true,
      label: "自动打卡运行中",
      detail: props.automationSettings.currentStatus.message,
      course: currentCourse ?? "当前没有候选课程",
      cadence: `每 ${props.automationSettings.autoCheckIntervalSeconds} 秒检查一次，模式 ${modeLabel}`,
      updatedAt: props.automationSettings.currentStatus.updatedAt
    };
  }

  return {
    active: false,
    label: "自动打卡已关闭",
    detail: "需要时可在设置中开启后台轮询。",
    course: "后台不会主动选择课程",
    cadence: "仅支持应用运行期间自动打卡",
    updatedAt: props.automationSettings.currentStatus.updatedAt
  };
});
</script>

<template>
  <section class="grid gap-4 xl:grid-cols-[1.2fr_0.8fr]">
    <div class="glass-panel p-3.5 sm:p-5">
      <div class="grid gap-4 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-start">
        <div class="min-w-0">
          <p class="text-xs font-medium uppercase tracking-[0.18em] text-[rgb(122,90,54)]">Current session</p>
          <h2 class="mt-2 text-lg font-semibold text-ink-950 sm:text-2xl">
            {{ props.dashboard.session.real_name }}
          </h2>
          <p class="mt-1 text-sm text-ink-500">{{ props.dashboard.session.account }}</p>
          <p class="mt-2 text-xs text-ink-500 sm:text-sm">
            上次同步
            {{ formatDateTime(props.dashboard.generated_at) }}
          </p>
        </div>

        <div class="grid grid-cols-2 gap-2 sm:grid-cols-4 sm:gap-3 lg:w-[22rem]">
          <div class="metric-card">
            <p class="metric-label">今日课表</p>
            <p class="metric-value">{{ props.dashboard.schedules.length }}</p>
          </div>
          <div class="metric-card">
            <p class="metric-label">可打卡</p>
            <p class="metric-value">
              {{ props.dashboard.schedules.filter((item) => item.can_check_in).length }}
            </p>
          </div>
          <div class="metric-card">
            <p class="metric-label">已打卡</p>
            <p class="metric-value">{{ signedToday }}</p>
          </div>
          <div class="metric-card">
            <p class="metric-label">待处理</p>
            <p class="metric-value">{{ pendingTotal }}</p>
          </div>
        </div>
      </div>

      <div
        class="mt-4 rounded-[1.5rem] border px-4 py-3 shadow-[0_10px_24px_rgba(90,70,43,0.05)]"
        :class="
          automationStatus.active
            ? 'border-[rgba(211,194,171,0.88)] bg-[linear-gradient(135deg,rgba(248,242,233,0.98),rgba(243,236,226,0.94))] text-[rgb(92,72,50)]'
            : 'border-[rgba(224,214,198,0.88)] bg-[rgba(255,252,247,0.88)] text-ink-500'
        "
      >
        <div class="grid gap-3 md:grid-cols-[auto_minmax(0,1fr)] md:items-start">
          <div class="flex items-start">
            <span
              class="inline-flex items-center gap-2 rounded-full border px-3 py-1 text-xs font-semibold sm:text-sm"
              :class="
                automationStatus.active
                  ? 'border-[rgba(211,194,171,0.88)] bg-[rgba(255,252,247,0.84)] text-[rgb(118,85,47)]'
                  : 'border-[rgba(224,214,198,0.88)] bg-[rgba(255,252,247,0.84)] text-ink-500'
              "
            >
              <span
                class="h-2 w-2 rounded-full"
                :class="automationStatus.active ? 'bg-[rgb(151,118,79)]' : 'bg-slate-300'"
              ></span>
              {{ automationStatus.label }}
            </span>
          </div>

          <div class="min-w-0">
            <p class="text-sm font-medium leading-6 text-current/95">{{ automationStatus.detail }}</p>
            <div class="mt-1 flex flex-col gap-1 text-sm leading-6 text-current/80 md:flex-row md:flex-wrap md:items-center md:gap-x-3">
              <span>{{ automationStatus.course }}</span>
              <span class="hidden text-current/40 md:inline">•</span>
              <span>{{ automationStatus.cadence }}</span>
            </div>
            <p class="mt-2 text-xs leading-5 text-current/65">
              最后一次调度
              {{ formatDateTime(automationStatus.updatedAt) }}
            </p>
          </div>
        </div>
      </div>
    </div>

    <div class="glass-panel p-3.5 sm:p-5">
      <p class="text-sm font-medium text-ink-500">当前学期</p>
      <template v-if="currentSemester">
        <h3 class="mt-2 text-base font-semibold text-ink-900 sm:text-lg">
          {{ currentSemester.name }}
        </h3>
        <p class="mt-2 text-sm text-ink-500">
          {{ currentSemester.begin_date }} - {{ currentSemester.end_date }}
        </p>
        <div class="mt-3 grid gap-2 sm:mt-4 sm:gap-3 sm:grid-cols-2">
          <div class="metric-card">
            <p class="metric-label">学期总数</p>
            <p class="text-lg font-semibold text-ink-900">{{ props.dashboard.semesters.length }}</p>
          </div>
          <div class="metric-card">
            <p class="metric-label">课程总数</p>
            <p class="text-lg font-semibold text-ink-900">{{ props.dashboard.courses.length }}</p>
          </div>
        </div>
      </template>
      <template v-else>
        <p class="mt-3 text-sm text-ink-500">暂未获取到学期信息。</p>
      </template>
    </div>
  </section>
</template>
