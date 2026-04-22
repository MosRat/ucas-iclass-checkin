<script setup lang="ts">
defineProps<{
  open: boolean;
  title: string;
  message: string;
  tone?: "error" | "success" | "info";
  actionLabel?: string;
}>();

defineEmits<{
  close: [];
  action: [];
}>();
</script>

<template>
  <transition name="dialog-fade">
    <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/35 px-4 py-8">
      <div class="w-full max-w-md rounded-4xl border border-white/80 bg-white/95 p-6 shadow-fluent backdrop-blur-2xl">
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
