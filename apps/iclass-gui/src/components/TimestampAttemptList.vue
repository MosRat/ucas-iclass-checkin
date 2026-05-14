<script setup lang="ts">
import { computed, ref } from "vue";
import { formatTimestampMs } from "../lib/datetime";
import type { CheckInTimestampAttempt } from "../lib/types";

const props = withDefaults(
  defineProps<{
    attempts: CheckInTimestampAttempt[];
    maxVisible?: number;
    title?: string;
    dense?: boolean;
  }>(),
  {
    maxVisible: 8,
    title: "时间戳尝试",
    dense: false
  }
);

const expanded = ref(false);
const visibleAttempts = computed(() => expanded.value ? props.attempts : props.attempts.slice(0, props.maxVisible));
const hiddenCount = computed(() => Math.max(props.attempts.length - visibleAttempts.value.length, 0));
const successCount = computed(() => props.attempts.filter((attempt) => attempt.signed_in).length);
const failureCount = computed(() => props.attempts.length - successCount.value);
const firstSuccess = computed(() => props.attempts.find((attempt) => attempt.signed_in) ?? null);
</script>

<template>
  <section
    v-if="attempts.length"
    class="rounded-3xl border border-[rgba(224,214,198,0.88)] bg-[rgba(255,252,247,0.84)] p-4"
  >
    <div class="flex items-center justify-between gap-3">
      <p class="text-sm font-semibold text-ink-900">{{ title }}</p>
      <p class="text-xs text-ink-400">{{ attempts.length }} 次</p>
    </div>
    <div class="mt-3 flex flex-wrap gap-2 text-xs">
      <span class="rounded-full border border-emerald-200 bg-emerald-50 px-2.5 py-1 font-medium text-emerald-700">
        成功 {{ successCount }}
      </span>
      <span class="rounded-full border border-rose-200 bg-rose-50 px-2.5 py-1 font-medium text-rose-700">
        失败 {{ failureCount }}
      </span>
      <span v-if="firstSuccess" class="rounded-full border border-[rgba(224,214,198,0.88)] bg-white/70 px-2.5 py-1 text-ink-500">
        命中 {{ formatTimestampMs(firstSuccess.timestamp) }}
      </span>
    </div>
    <ul :class="dense ? 'mt-2 space-y-1' : 'mt-3 space-y-2'" class="text-xs leading-5 text-ink-600">
      <li
        v-for="attempt in visibleAttempts"
        :key="`${attempt.timestamp}-${attempt.status_code ?? 'na'}-${attempt.message ?? ''}`"
        class="rounded-2xl border px-3 py-2"
        :class="
          attempt.signed_in
            ? 'border-emerald-200 bg-emerald-50/70'
            : 'border-transparent bg-white/70'
        "
      >
        <div class="flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between">
          <div class="min-w-0">
            <span class="font-medium text-ink-700">{{ formatTimestampMs(attempt.timestamp) }}</span>
            <span class="ml-2" :class="attempt.signed_in ? 'text-emerald-700' : 'text-rose-700'">
              {{ attempt.signed_in ? "成功" : "失败" }}
            </span>
          </div>
          <span
            v-if="attempt.status_code || attempt.message"
            class="max-w-full break-words text-ink-500 sm:text-right"
          >
            {{ [attempt.status_code, attempt.message].filter(Boolean).join(" ") }}
          </span>
        </div>
      </li>
    </ul>
    <div v-if="attempts.length > maxVisible" class="mt-2 flex items-center justify-between gap-3">
      <p class="text-xs text-ink-400">
        {{ expanded ? "已显示全部时间点。" : `其余 ${hiddenCount} 个时间点已折叠。` }}
      </p>
      <button class="text-xs font-semibold text-ink-600 hover:text-ink-900" type="button" @click="expanded = !expanded">
        {{ expanded ? "收起" : "展开全部" }}
      </button>
    </div>
  </section>
</template>
