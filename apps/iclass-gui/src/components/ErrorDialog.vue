<script setup lang="ts">
import { computed, ref } from "vue";

const props = defineProps<{
  open: boolean;
  title: string;
  message: string;
  tone?: "error" | "success" | "info";
  actionLabel?: string;
  debugDetails?: string;
}>();

defineEmits<{
  close: [];
  action: [];
}>();

const copied = ref(false);
const hasDebugDetails = computed(() => Boolean(props.debugDetails?.trim()));

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
    <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/35 px-4 py-8">
      <div class="w-full max-w-xl rounded-4xl border border-white/80 bg-white/95 p-6 shadow-fluent backdrop-blur-2xl">
        <div
          class="mb-4 flex h-12 w-12 items-center justify-center rounded-3xl"
          :class="{
            'bg-rose-100 text-rose-600': tone === 'error',
            'bg-emerald-100 text-emerald-600': tone === 'success',
            'bg-accent-100 text-accent-600': tone !== 'error' && tone !== 'success'
          }"
        >
          {{ tone === "error" ? "!" : tone === "success" ? "✓" : "i" }}
        </div>
        <h2 class="text-lg font-semibold text-ink-900">{{ title }}</h2>
        <p class="mt-2 whitespace-pre-line text-sm leading-6 text-ink-600">{{ message }}</p>
        <details v-if="hasDebugDetails" class="mt-4 rounded-3xl border border-slate-200/80 bg-slate-50/85 p-4">
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
        <div class="mt-6 flex justify-end gap-3">
          <button v-if="actionLabel" class="secondary-btn" type="button" @click="$emit('action')">
            {{ actionLabel }}
          </button>
          <button class="primary-btn" type="button" @click="$emit('close')">知道了</button>
        </div>
      </div>
    </div>
  </transition>
</template>
