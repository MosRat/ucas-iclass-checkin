<script setup lang="ts">
import { computed } from "vue";
import type { DashboardSnapshot } from "../lib/types";

const props = defineProps<{
  dashboard: DashboardSnapshot;
}>();

const currentSemester = computed(
  () => props.dashboard.semesters.find((semester) => semester.current) ?? props.dashboard.semesters[0] ?? null
);

const pendingTotal = computed(() =>
  props.dashboard.courses.reduce((total, course) => total + (course.pending_checkins ?? 0), 0)
);
</script>

<template>
  <section class="grid gap-4 xl:grid-cols-[1.2fr_0.8fr]">
    <div class="glass-panel p-5">
      <div class="flex flex-col gap-5 md:flex-row md:items-center md:justify-between">
        <div>
          <p class="text-sm font-medium text-accent-700">当前登录</p>
          <h2 class="mt-2 text-2xl font-semibold text-ink-950">
            {{ props.dashboard.session.real_name }}
          </h2>
          <p class="mt-1 text-sm text-ink-500">{{ props.dashboard.session.account }}</p>
          <p class="mt-2 text-sm text-ink-500">
            上次同步
            {{ new Date(props.dashboard.generated_at).toLocaleString("zh-CN", { hour12: false }) }}
          </p>
        </div>
        <div class="grid gap-3 sm:grid-cols-3">
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

    <div class="glass-panel p-5">
      <p class="text-sm font-medium text-ink-500">当前学期</p>
      <template v-if="currentSemester">
        <h3 class="mt-2 text-lg font-semibold text-ink-900">
          {{ currentSemester.name }}
        </h3>
        <p class="mt-2 text-sm text-ink-500">
          {{ currentSemester.begin_date }} - {{ currentSemester.end_date }}
        </p>
        <div class="mt-4 grid gap-3 sm:grid-cols-2">
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
