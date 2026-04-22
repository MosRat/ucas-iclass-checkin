<script setup lang="ts">
import { computed, ref } from "vue";
import type {
  AutomationSettings,
  CustomCheckInRequest,
  DashboardSnapshot,
  ScheduleCard,
  WeeklyScheduleSnapshot
} from "../lib/types";

const props = defineProps<{
  dashboard: DashboardSnapshot;
  weeklySchedule: WeeklyScheduleSnapshot | null;
  automationSettings: AutomationSettings;
  loading: boolean;
  selectedDate: string;
  selectedScheduleId?: string | null;
  compact?: boolean;
  viewMode: "day" | "week";
  search: string;
}>();

const emit = defineEmits<{
  changeDate: [string];
  checkIn: [ScheduleCard];
  select: [ScheduleCard];
  customCheckIn: [CustomCheckInRequest];
  refresh: [];
  updateViewMode: ["day" | "week"];
  updateSearch: [string];
  logout: [];
}>();

const customIdentifier = ref("");
const customMode = ref<"uuid" | "id">("uuid");

const visibleSchedules = computed(() => {
  const source =
    props.viewMode === "week" ? props.weeklySchedule?.schedules ?? [] : props.dashboard.schedules;
  const query = props.search.trim().toLowerCase();
  if (!query) {
    return source;
  }

  return source.filter((card) => {
    const haystack = [
      card.schedule.course_name,
      card.schedule.teacher_name,
      card.schedule.classroom_name,
      card.schedule.schedule_id
    ]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();
    return haystack.includes(query);
  });
});

const groupedSchedules = computed(() => {
  if (props.viewMode !== "week") {
    return [];
  }

  const groups = new Map<string, ScheduleCard[]>();
  for (const card of visibleSchedules.value) {
    const key = card.schedule.teach_date;
    const existing = groups.get(key);
    if (existing) {
      existing.push(card);
    } else {
      groups.set(key, [card]);
    }
  }

  return Array.from(groups.entries()).map(([date, cards]) => ({
    date,
    cards
  }));
});

const courseDigest = computed(() =>
  props.dashboard.courses.slice(0, 4).map((course) => ({
    id: course.id,
    name: course.name,
    teacher: course.teacher_name ?? "未标注教师",
    pending: course.pending_checkins ?? 0
  }))
);

function availabilityLabel(card: ScheduleCard) {
  if (card.availability === "Open") {
    return "现在可打卡";
  }
  if (card.availability === "Closed") {
    return "课程已结束";
  }
  return `将于 ${new Date(card.check_in_opens_at).toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    minute: "2-digit"
  })} 开放`;
}

function availabilityHint(card: ScheduleCard) {
  if (card.availability === "Open") {
    return "已进入打卡时间窗口";
  }
  if (card.availability === "Closed") {
    return "可切换日期回看课程详情";
  }
  return "可先查看详情，等待时间到达后直接打卡";
}

function renderCard(card: ScheduleCard) {
  return {
    id: card.schedule.schedule_id,
    canCheckIn: card.can_check_in,
    label: availabilityLabel(card),
    hint: availabilityHint(card)
  };
}

function submitCustomCheckIn() {
  const identifier = customIdentifier.value.trim();
  if (!identifier) {
    return;
  }

  emit("customCheckIn", {
    identifier,
    mode: customMode.value
  });
}
</script>

<template>
  <section class="grid gap-4 sm:gap-5 xl:grid-cols-[1.2fr_0.8fr]">
    <div class="glass-panel overflow-hidden p-0">
      <div class="flex flex-col gap-3 border-b border-slate-200/70 px-3.5 py-3.5 sm:px-5 sm:py-5">
        <div class="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
          <div>
            <h3 class="text-lg font-semibold text-ink-950">课表工作台</h3>
            <p class="mt-1 text-sm leading-6 text-ink-500">按日期查看、切换周视图，并在开放时间内直接打卡。</p>
          </div>
          <div class="grid grid-cols-1 gap-2 sm:flex sm:flex-wrap sm:items-center sm:gap-3">
            <input
              :value="selectedDate"
              class="field-input min-w-[10rem] bg-white/90 py-2.5"
              type="date"
              @change="emit('changeDate', ($event.target as HTMLInputElement).value)"
            />
            <button class="secondary-btn justify-center py-2.5" :disabled="loading" type="button" @click="emit('refresh')">
              {{ loading ? "同步中..." : "刷新" }}
            </button>
            <button class="secondary-btn justify-center py-2.5" type="button" @click="emit('logout')">退出登录</button>
          </div>
        </div>

        <div class="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
          <div class="inline-flex self-start rounded-3xl border border-white/70 bg-white/90 p-1 shadow-[0_8px_20px_rgba(27,46,89,0.08)]">
            <button
              class="rounded-[1.1rem] px-3.5 py-2 text-sm font-semibold transition sm:px-4"
              :class="props.viewMode === 'day' ? 'bg-accent-600 text-white shadow-pane' : 'text-ink-600'"
              type="button"
              @click="emit('updateViewMode', 'day')"
            >
              日视图
            </button>
            <button
              class="rounded-[1.1rem] px-3.5 py-2 text-sm font-semibold transition sm:px-4"
              :class="props.viewMode === 'week' ? 'bg-accent-600 text-white shadow-pane' : 'text-ink-600'"
              type="button"
              @click="emit('updateViewMode', 'week')"
            >
              周视图
            </button>
          </div>

          <div class="flex flex-1 items-center justify-end">
            <input
              :value="props.search"
              class="field-input w-full max-w-md py-2.5"
              placeholder="搜索课程、教师、地点或课程编号"
              type="search"
              @input="emit('updateSearch', ($event.target as HTMLInputElement).value)"
            />
          </div>
        </div>
      </div>

      <div v-if="visibleSchedules.length > 0" class="space-y-3 p-3.5 sm:space-y-4 sm:p-5">
        <template v-if="props.viewMode === 'week'">
          <section v-for="group in groupedSchedules" :key="group.date" class="space-y-3">
            <div class="flex items-center justify-between px-1">
              <h4 class="text-sm font-semibold uppercase tracking-[0.18em] text-ink-500">{{ group.date }}</h4>
              <p class="text-xs text-ink-400">{{ group.cards.length }} 门课程</p>
            </div>

            <article
              v-for="card in group.cards"
              :key="card.schedule.schedule_id"
              class="relative overflow-hidden rounded-[1.4rem] border border-white/80 bg-white/88 p-3.5 shadow-[0_10px_24px_rgba(15,23,42,0.08)] transition hover:-translate-y-0.5 hover:shadow-[0_18px_34px_rgba(15,23,42,0.12)] sm:rounded-[1.75rem] sm:p-5"
              :class="{
                'ring-2 ring-accent-300 shadow-[0_18px_34px_rgba(30,91,214,0.14)]': props.selectedScheduleId === card.schedule.schedule_id,
                'p-4': props.compact
              }"
            >
              <div class="absolute inset-y-4 left-0 w-1 rounded-r-full bg-accent-500"></div>
              <div class="ml-3 flex flex-col" :class="props.compact ? 'gap-3' : 'gap-4'">
                <div class="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                  <div>
                    <h4 class="text-base font-semibold leading-6 text-ink-950 sm:text-lg">{{ card.schedule.course_name }}</h4>
                    <p class="mt-1 text-sm text-ink-500">
                      {{ card.schedule.teacher_name || "教师未标注" }} ·
                      {{ card.schedule.classroom_name || "地点未标注" }}
                    </p>
                    <p v-if="card.schedule.lesson_units > 1" class="mt-2 text-xs font-medium text-accent-700">
                      本堂课共 {{ card.schedule.lesson_units }} 课时
                    </p>
                  </div>
                  <span
                    class="inline-flex self-start rounded-full px-3 py-1 text-xs font-semibold"
                    :class="{
                      'bg-emerald-100 text-emerald-700': card.availability === 'Open',
                      'bg-amber-100 text-amber-700': card.availability === 'NotOpenYet',
                      'bg-slate-200 text-slate-600': card.availability === 'Closed'
                    }"
                  >
                    {{ renderCard(card).label }}
                  </span>
                </div>

                <div class="grid grid-cols-2 text-sm text-ink-600 sm:grid-cols-3" :class="props.compact ? 'gap-2' : 'gap-2.5 sm:gap-3'">
                  <div class="rounded-2xl bg-slate-50 px-3.5 py-3 sm:px-4">
                    <p class="text-xs uppercase tracking-[0.2em] text-ink-400">开始</p>
                    <p class="mt-2 font-medium text-ink-800">
                      {{ new Date(card.schedule.begins_at).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" }) }}
                    </p>
                  </div>
                  <div class="rounded-2xl bg-slate-50 px-3.5 py-3 sm:px-4">
                    <p class="text-xs uppercase tracking-[0.2em] text-ink-400">结束</p>
                    <p class="mt-2 font-medium text-ink-800">
                      {{ new Date(card.schedule.ends_at).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" }) }}
                    </p>
                  </div>
                  <div class="col-span-2 rounded-2xl bg-slate-50 px-3.5 py-3 sm:col-span-1 sm:px-4">
                    <p class="text-xs uppercase tracking-[0.2em] text-ink-400">开放打卡</p>
                    <p class="mt-2 font-medium text-ink-800">
                      {{ new Date(card.check_in_opens_at).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" }) }}
                    </p>
                  </div>
                </div>

                <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                  <p class="text-sm leading-6 text-ink-500">
                    课程编号 {{ card.schedule.schedule_id }}
                    <span v-if="card.schedule.schedule_uuid"> · UUID 模式可用</span>
                    <span class="block text-xs text-ink-400">{{ renderCard(card).hint }}</span>
                  </p>
                  <div class="grid grid-cols-2 gap-2 sm:flex sm:flex-wrap">
                    <button class="secondary-btn justify-center py-2.5" type="button" @click="emit('select', card)">详情</button>
                    <button
                      class="primary-btn justify-center py-2.5"
                      :disabled="loading || !renderCard(card).canCheckIn"
                      type="button"
                      @click="emit('checkIn', card)"
                    >
                      {{ loading ? "处理中..." : renderCard(card).canCheckIn ? "立即打卡" : "等待开放" }}
                    </button>
                  </div>
                </div>
              </div>
            </article>
          </section>
        </template>

        <template v-else>
          <article
            v-for="card in visibleSchedules"
            :key="card.schedule.schedule_id"
            class="relative overflow-hidden rounded-[1.4rem] border border-white/80 bg-white/88 p-3.5 shadow-[0_10px_24px_rgba(15,23,42,0.08)] transition hover:-translate-y-0.5 hover:shadow-[0_18px_34px_rgba(15,23,42,0.12)] sm:rounded-[1.75rem] sm:p-5"
            :class="{
              'ring-2 ring-accent-300 shadow-[0_18px_34px_rgba(30,91,214,0.14)]': props.selectedScheduleId === card.schedule.schedule_id,
              'p-4': props.compact
            }"
          >
            <div class="absolute inset-y-4 left-0 w-1 rounded-r-full bg-accent-500"></div>
            <div class="ml-3 flex flex-col" :class="props.compact ? 'gap-3' : 'gap-4'">
              <div class="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                <div>
                  <h4 class="text-base font-semibold leading-6 text-ink-950 sm:text-lg">{{ card.schedule.course_name }}</h4>
                  <p class="mt-1 text-sm text-ink-500">
                    {{ card.schedule.teacher_name || "教师未标注" }} ·
                    {{ card.schedule.classroom_name || "地点未标注" }}
                  </p>
                  <p v-if="card.schedule.lesson_units > 1" class="mt-2 text-xs font-medium text-accent-700">
                    本堂课共 {{ card.schedule.lesson_units }} 课时
                  </p>
                </div>
                <span
                  class="inline-flex self-start rounded-full px-3 py-1 text-xs font-semibold"
                  :class="{
                    'bg-emerald-100 text-emerald-700': card.availability === 'Open',
                    'bg-amber-100 text-amber-700': card.availability === 'NotOpenYet',
                    'bg-slate-200 text-slate-600': card.availability === 'Closed'
                  }"
                >
                  {{ renderCard(card).label }}
                </span>
              </div>

              <div class="grid grid-cols-2 text-sm text-ink-600 sm:grid-cols-3" :class="props.compact ? 'gap-2' : 'gap-2.5 sm:gap-3'">
                <div class="rounded-2xl bg-slate-50 px-3.5 py-3 sm:px-4">
                  <p class="text-xs uppercase tracking-[0.2em] text-ink-400">开始</p>
                  <p class="mt-2 font-medium text-ink-800">
                    {{ new Date(card.schedule.begins_at).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" }) }}
                  </p>
                </div>
                <div class="rounded-2xl bg-slate-50 px-3.5 py-3 sm:px-4">
                  <p class="text-xs uppercase tracking-[0.2em] text-ink-400">结束</p>
                  <p class="mt-2 font-medium text-ink-800">
                    {{ new Date(card.schedule.ends_at).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" }) }}
                  </p>
                </div>
                <div class="col-span-2 rounded-2xl bg-slate-50 px-3.5 py-3 sm:col-span-1 sm:px-4">
                  <p class="text-xs uppercase tracking-[0.2em] text-ink-400">开放打卡</p>
                  <p class="mt-2 font-medium text-ink-800">
                    {{ new Date(card.check_in_opens_at).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" }) }}
                  </p>
                </div>
              </div>

              <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                <p class="text-sm leading-6 text-ink-500">
                  课程编号 {{ card.schedule.schedule_id }}
                  <span v-if="card.schedule.schedule_uuid"> · UUID 模式可用</span>
                  <span class="block text-xs text-ink-400">{{ renderCard(card).hint }}</span>
                </p>
                <div class="grid grid-cols-2 gap-2 sm:flex sm:flex-wrap">
                  <button class="secondary-btn justify-center py-2.5" type="button" @click="emit('select', card)">详情</button>
                  <button
                    class="primary-btn justify-center py-2.5"
                    :disabled="loading || !renderCard(card).canCheckIn"
                    type="button"
                    @click="emit('checkIn', card)"
                  >
                    {{ loading ? "处理中..." : renderCard(card).canCheckIn ? "立即打卡" : "等待开放" }}
                  </button>
                </div>
              </div>
            </div>
          </article>
        </template>
      </div>

      <div v-else class="p-4 sm:p-6">
        <div class="rounded-[1.75rem] border border-dashed border-slate-300 bg-slate-50/80 px-6 py-10 text-center">
          <p class="text-lg font-semibold text-ink-800">
            {{ props.search ? "没有匹配的课程" : props.viewMode === "week" ? "本周没有课表" : "当前日期没有课表" }}
          </p>
          <p class="mt-2 text-sm text-ink-500">
            {{ props.search ? "可以尝试更换关键字、切换视图或调整日期。" : "可以切换日期继续查看，也可能是上游接口当前无课程数据。" }}
          </p>
        </div>
      </div>
    </div>

    <aside class="space-y-4 xl:space-y-5">
      <div class="glass-panel p-3.5 sm:p-5">
        <h3 class="text-lg font-semibold text-ink-950">课程摘要</h3>
        <div class="mt-3 grid gap-2.5 sm:mt-4 sm:gap-3 sm:grid-cols-2 xl:grid-cols-1">
          <article
            v-for="course in courseDigest"
            :key="course.id"
            class="rounded-3xl border border-white/70 bg-white/80 px-4 py-4"
          >
            <p class="text-sm font-semibold text-ink-900">{{ course.name }}</p>
            <p class="mt-1 text-xs text-ink-500">{{ course.teacher }}</p>
            <p class="mt-2 text-xs text-accent-700">待打卡 {{ course.pending }}</p>
          </article>
        </div>
      </div>

      <div class="glass-panel p-3.5 sm:p-5">
        <h3 class="text-lg font-semibold text-ink-950">使用提示</h3>
        <ul class="mt-3 space-y-2.5 text-sm leading-6 text-ink-600 sm:mt-4 sm:space-y-3">
          <li>托盘驻留开启后，关闭窗口不会退出进程，可从系统托盘再次唤起。</li>
          <li>周视图支持按课程名、教师、地点和课程编号快速筛选。</li>
          <li>打卡按钮会在课程开始前 30 分钟自动变为可用。</li>
        </ul>
      </div>

      <div class="glass-panel p-3.5 sm:p-5">
        <h3 class="text-lg font-semibold text-ink-950">自动打卡状态</h3>
        <p class="mt-3 text-sm leading-6 text-ink-600">
          {{
            props.automationSettings.autoCheckInEnabled
              ? `已开启，按 ${props.automationSettings.autoCheckInMode.toUpperCase()} 模式每 ${props.automationSettings.autoCheckIntervalSeconds} 秒轮询一次。`
              : "未开启，可在设置里打开后台自动打卡。"
          }}
        </p>
      </div>

      <div class="glass-panel p-3.5 sm:p-5">
        <h3 class="text-lg font-semibold text-ink-950">自定义打卡</h3>
        <p class="mt-3 text-sm leading-6 text-ink-600">适合你已经明确知道目标课程的排课 ID 或 UUID 时手动补打卡。</p>
        <div class="mt-4 space-y-3">
          <div class="inline-flex rounded-3xl border border-white/70 bg-white/90 p-1 shadow-[0_8px_20px_rgba(27,46,89,0.08)]">
            <button
              class="rounded-[1.1rem] px-3.5 py-2 text-sm font-semibold transition sm:px-4"
              :class="customMode === 'uuid' ? 'bg-accent-600 text-white shadow-pane' : 'text-ink-600'"
              type="button"
              @click="customMode = 'uuid'"
            >
              UUID
            </button>
            <button
              class="rounded-[1.1rem] px-3.5 py-2 text-sm font-semibold transition sm:px-4"
              :class="customMode === 'id' ? 'bg-accent-600 text-white shadow-pane' : 'text-ink-600'"
              type="button"
              @click="customMode = 'id'"
            >
              ID
            </button>
          </div>

          <input
            v-model.trim="customIdentifier"
            class="field-input"
            :placeholder="customMode === 'uuid' ? '输入 timeTableId / UUID' : '输入 courseSchedId / ID'"
            type="text"
          />

          <button
            class="primary-btn w-full justify-center"
            :disabled="loading || !customIdentifier.trim()"
            type="button"
            @click="submitCustomCheckIn"
          >
            {{ loading ? "处理中..." : `按 ${customMode.toUpperCase()} 打卡` }}
          </button>
        </div>
      </div>
    </aside>
  </section>
</template>
