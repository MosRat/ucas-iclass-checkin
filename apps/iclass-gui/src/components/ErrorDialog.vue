<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import TimestampAttemptList from "./TimestampAttemptList.vue";
import type { CheckInTimestampAttempt } from "../lib/types";

const props = defineProps<{
  open: boolean;
  title: string;
  message: string;
  tone?: "error" | "success" | "info";
  actionLabel?: string;
  debugDetails?: string;
  timestampAttempts?: CheckInTimestampAttempt[];
}>();

const emit = defineEmits<{
  close: [];
  action: [];
}>();

const copied = ref(false);
const hasDebugDetails = computed(() => Boolean(props.debugDetails?.trim()));
const timestampAttempts = computed(() => props.timestampAttempts ?? []);

function handleKeydown(event: KeyboardEvent) {
  if (props.open && event.key === "Escape") {
    emit("close");
  }
}

onMounted(() => window.addEventListener("keydown", handleKeydown));
onBeforeUnmount(() => window.removeEventListener("keydown", handleKeydown));

async function copyDebugDetails() {
  if (!props.debugDetails) {
    return;
  }

  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(props.debugDetails);
    } else {
      const textarea = document.createElement("textarea");
      textarea.value = props.debugDetails;
      textarea.setAttribute("readonly", "true");
      textarea.style.position = "absolute";
      textarea.style.left = "-9999px";
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand("copy");
      document.body.removeChild(textarea);
    }
    copied.value = true;
    window.setTimeout(() => {
      copied.value = false;
    }, 2000);
  } catch {
    copied.value = false;
  }
}
</script>

<template>
  <transition name="dialog-fade">
    <div
      v-if="open"
      class="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(36,28,20,0.22)] px-3 py-4 sm:px-4 sm:py-8"
      @click.self="emit('close')"
    >
      <div
        aria-modal="true"
        class="max-h-[calc(100vh-2rem)] w-full max-w-xl overflow-y-auto rounded-4xl border border-[rgba(224,214,198,0.88)] bg-[rgba(255,252,247,0.96)] p-5 shadow-fluent backdrop-blur-2xl sm:p-6"
        role="dialog"
        :aria-label="title"
      >
        <div
          class="mb-4 flex h-12 w-12 items-center justify-center rounded-3xl"
          :class="{
            'bg-rose-100 text-rose-600': tone === 'error',
            'bg-emerald-100 text-emerald-600': tone === 'success',
            'bg-[rgba(245,236,224,0.9)] text-[rgb(122,90,54)]': tone !== 'error' && tone !== 'success'
          }"
        >
          {{ tone === "error" ? "!" : tone === "success" ? "✓" : "i" }}
        </div>
        <h2 class="text-lg font-semibold text-ink-900">{{ title }}</h2>
        <p class="mt-2 whitespace-pre-line break-words text-sm leading-6 text-ink-600">{{ message }}</p>
        <TimestampAttemptList
          v-if="timestampAttempts.length"
          class="mt-4"
          :attempts="timestampAttempts"
          :max-visible="8"
        />
        <details v-if="hasDebugDetails" class="mt-4 rounded-3xl border border-[rgba(224,214,198,0.88)] bg-[rgba(250,245,238,0.88)] p-4">
          <summary class="cursor-pointer list-none text-sm font-semibold text-ink-900">
            调试信息
          </summary>
          <p class="mt-2 text-xs leading-5 text-ink-500">
            这里保留给排查问题使用。界面默认不全部展示，但你可以展开后复制给开发者。
          </p>
          <pre class="mt-3 max-h-52 overflow-auto rounded-2xl bg-slate-950 px-3 py-3 text-xs leading-5 text-slate-100">{{ debugDetails }}</pre>
          <div class="mt-3 flex justify-end">
            <button class="secondary-btn" type="button" @click="copyDebugDetails">
              {{ copied ? "已复制" : "复制调试信息" }}
            </button>
          </div>
        </details>
        <div class="mt-6 flex flex-col-reverse gap-3 sm:flex-row sm:justify-end">
          <button v-if="actionLabel" class="secondary-btn justify-center" type="button" @click="emit('action')">
            {{ actionLabel }}
          </button>
          <button class="primary-btn justify-center" type="button" @click="emit('close')">知道了</button>
        </div>
      </div>
    </div>
  </transition>
</template>
