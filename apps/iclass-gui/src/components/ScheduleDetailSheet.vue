<script setup lang="ts">
import { computed } from "vue";
import { X } from "lucide-vue-next";
import type { ScheduleCard } from "../lib/types";

const props = defineProps<{
  card: ScheduleCard | null;
  currentTime: string;
  loading: boolean;
}>();

const emit = defineEmits<{
  close: [];
  checkIn: [ScheduleCard];
}>();

const currentDate = computed(() => new Date(props.currentTime));

const timeline = computed(() => {
  if (!props.card) {
    return "";
  }

  const now = currentDate.value.getTime();
  const open = new Date(props.card.check_in_opens_at).getTime();
  const begin = new Date(props.card.schedule.begins_at).getTime();
  const end = new Date(props.card.schedule.ends_at).getTime();

  if (props.card.availability === "NotOpenYet") {
    return `距离开放还有 ${formatDuration(open - now)}`;
  }
  if (props.card.availability === "Open") {
    return `距离下课还有 ${formatDuration(end - now)}`;
  }
  return `课程已结束 ${formatDuration(now - end)} 前`;
});

function formatDuration(milliseconds: number) {
  const totalSeconds = Math.max(Math.floor(Math.abs(milliseconds) / 1000), 0);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);

  if (hours > 0) {
    return `${hours}小时${minutes}分钟`;
  }
  return `${Math.max(minutes, 1)}分钟`;
}

function formatDateTime(value: string) {
  return new Date(value).toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit"
  });
}
</script>

<template>
  <transition name="dialog-fade">
    <div v-if="card" class="fixed inset-0 z-40 bg-slate-950/28 md:bg-transparent">
      <div class="pointer-events-none absolute inset-0 flex items-end justify-end p-0 md:p-6">
        <aside class="pointer-events-auto flex h-[82vh] w-full flex-col rounded-t-[2rem] border border-white/70 bg-white/96 shadow-fluent backdrop-blur-2xl md:h-[calc(100vh-8rem)] md:max-h-[880px] md:w-[28rem] md:rounded-[2rem]">
          <div class="flex items-center justify-between border-b border-slate-200/70 px-5 py-4">
            <div>
              <p class="text-xs uppercase tracking-[0.24em] text-ink-400">Schedule Detail</p>
              <h3 class="mt-1 text-lg font-semibold text-ink-950">课程详情</h3>
            </div>
            <button class="titlebar-btn titlebar-btn-subtle" title="关闭详情" type="button" @click="emit('close')">
              <X class="titlebar-icon" aria-hidden="true" />
            </button>
          </div>

          <div class="flex-1 space-y-5 overflow-y-auto px-5 py-5">
            <section class="rounded-[1.75rem] bg-[linear-gradient(135deg,_rgba(33,101,210,0.95),_rgba(72,145,255,0.84))] p-5 text-white shadow-pane">
              <p class="text-xs uppercase tracking-[0.24em] text-white/70">Selected Class</p>
              <h4 class="mt-3 text-2xl font-semibold leading-tight">{{ card.schedule.course_name }}</h4>
              <p class="mt-3 text-sm text-white/78">
                {{ card.schedule.teacher_name || "教师未标注" }} ·
                {{ card.schedule.classroom_name || "地点未标注" }}
              </p>
              <div class="mt-5 flex flex-wrap gap-2 text-xs font-medium">
                <span class="rounded-full bg-white/18 px-3 py-1 text-white/88">
                  {{ timeline }}
                </span>
                <span class="rounded-full bg-white/18 px-3 py-1 text-white/88">
                  {{ card.schedule.schedule_uuid ? "UUID 可用" : "仅 ID 模式" }}
                </span>
              </div>
            </section>

            <section class="grid gap-3 sm:grid-cols-2">
              <div class="metric-card">
                <p class="metric-label">开放打卡</p>
                <p class="mt-2 text-base font-semibold text-ink-900">{{ formatDateTime(card.check_in_opens_at) }}</p>
              </div>
              <div class="metric-card">
                <p class="metric-label">课程状态</p>
                <p class="mt-2 text-base font-semibold text-ink-900">
                  {{
                    card.schedule.sign_status === "1"
                      ? "已打卡"
                      : card.availability === "Open"
                      ? "可立即打卡"
                      : card.availability === "NotOpenYet"
                        ? "等待开放"
                        : "已结束"
                  }}
                </p>
              </div>
              <div class="metric-card">
                <p class="metric-label">上课时间</p>
                <p class="mt-2 text-base font-semibold text-ink-900">{{ formatDateTime(card.schedule.begins_at) }}</p>
              </div>
              <div class="metric-card">
                <p class="metric-label">下课时间</p>
                <p class="mt-2 text-base font-semibold text-ink-900">{{ formatDateTime(card.schedule.ends_at) }}</p>
              </div>
            </section>

            <section class="glass-panel p-5">
              <h4 class="text-base font-semibold text-ink-950">课程信息</h4>
              <dl class="mt-4 space-y-3 text-sm text-ink-600">
                <div class="flex items-start justify-between gap-4">
                  <dt class="text-ink-400">课程编号</dt>
                  <dd class="text-right font-medium text-ink-900">{{ card.schedule.schedule_id }}</dd>
                </div>
                <div class="flex items-start justify-between gap-4">
                  <dt class="text-ink-400">教师</dt>
                  <dd class="text-right font-medium text-ink-900">{{ card.schedule.teacher_name || "未标注" }}</dd>
                </div>
                <div class="flex items-start justify-between gap-4">
                  <dt class="text-ink-400">地点</dt>
                  <dd class="text-right font-medium text-ink-900">{{ card.schedule.classroom_name || "未标注" }}</dd>
                </div>
                <div class="flex items-start justify-between gap-4">
                  <dt class="text-ink-400">签到状态</dt>
                  <dd class="text-right font-medium text-ink-900">
                    {{
                      card.schedule.sign_status === "1"
                        ? "已打卡"
                        : card.schedule.sign_status === "0"
                          ? "未打卡"
                          : "未知"
                    }}
                  </dd>
                </div>
              </dl>
            </section>

            <section class="rounded-[1.75rem] border border-dashed border-slate-300 bg-slate-50/90 p-5 text-sm leading-6 text-ink-600">
              <p>课程开始前 30 分钟会自动开放打卡。</p>
              <p>如果上游接口返回二维码过期、参数错误或 session 失效，弹窗会给出可区分的错误信息。</p>
            </section>
          </div>

          <div class="flex items-center justify-between gap-3 border-t border-slate-200/70 px-5 py-4">
            <button class="secondary-btn" type="button" @click="emit('close')">收起详情</button>
            <button
              class="primary-btn justify-center"
              :disabled="loading || !card.can_check_in"
              type="button"
              @click="emit('checkIn', card)"
            >
              {{
                loading
                  ? "处理中..."
                  : card.schedule.sign_status === "1"
                    ? "已完成"
                    : card.can_check_in
                      ? "立即打卡"
                      : "等待开放"
              }}
            </button>
          </div>
        </aside>
      </div>
    </div>
  </transition>
</template>
