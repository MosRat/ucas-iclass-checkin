<script setup lang="ts">
import { computed } from "vue";
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
      detail: currentCourse
        ? `${props.automationSettings.currentStatus.message} · ${currentCourse} · 每 ${props.automationSettings.autoCheckIntervalSeconds} 秒检查一次，模式 ${modeLabel}`
        : `${props.automationSettings.currentStatus.message} · 每 ${props.automationSettings.autoCheckIntervalSeconds} 秒检查一次，模式 ${modeLabel}`
    };
  }

  return {
    active: false,
    label: "自动打卡已关闭",
    detail: "需要时可在设置中开启后台轮询。"
  };
});
</script>

<template>
  <section class="grid gap-4 xl:grid-cols-[1.2fr_0.8fr]">
    <div class="glass-panel p-3.5 sm:p-5">
      <div class="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
        <div>
          <p class="text-xs font-medium uppercase tracking-[0.18em] text-accent-700">当前登录</p>
          <h2 class="mt-2 text-lg font-semibold text-ink-950 sm:text-2xl">
            {{ props.dashboard.session.real_name }}
          </h2>
          <p class="mt-1 text-sm text-ink-500">{{ props.dashboard.session.account }}</p>
          <p class="mt-2 text-xs text-ink-500 sm:text-sm">
            上次同步
            {{ new Date(props.dashboard.generated_at).toLocaleString("zh-CN", { hour12: false }) }}
          </p>
          <div
            class="mt-3 inline-flex items-center gap-2 rounded-full border px-3 py-1.5 text-xs font-medium sm:text-sm"
            :class="
              automationStatus.active
                ? 'border-emerald-200 bg-emerald-50 text-emerald-700'
                : 'border-slate-200 bg-white/80 text-ink-500'
            "
          >
            <span
              class="h-2 w-2 rounded-full"
              :class="automationStatus.active ? 'bg-emerald-500' : 'bg-slate-300'"
            ></span>
            <span>{{ automationStatus.label }}</span>
            <span class="text-current/70">· {{ automationStatus.detail }}</span>
          </div>
        </div>
        <div class="grid grid-cols-3 gap-2 sm:gap-3">
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
            <p class="metric-label">待处理</p>
            <p class="metric-value">{{ pendingTotal }}</p>
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
