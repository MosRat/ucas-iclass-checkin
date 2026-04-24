<script setup lang="ts">
import { Maximize2, Minimize2, Minus, Settings2, X } from "lucide-vue-next";
import appMarkUrl from "../assets/brand/app-mark.svg";

const props = defineProps<{
  desktopShell: boolean;
  maximized: boolean;
}>();

const emit = defineEmits<{
  drag: [];
  settings: [];
  minimize: [];
  maximize: [];
  close: [];
}>();

function handlePointerDown(event: MouseEvent) {
  if (!props.desktopShell || event.button !== 0) {
    return;
  }

  const target = event.target;
  if (!(target instanceof HTMLElement)) {
    return;
  }

  if (target.closest(".no-drag")) {
    return;
  }

  emit("drag");
}
</script>

<template>
  <header
    class="sticky top-0 z-30 flex h-16 items-center border-b border-[rgba(221,211,197,0.8)] bg-[rgba(250,247,241,0.86)] px-3 backdrop-blur-xl select-none md:px-4"
    :data-tauri-drag-region="desktopShell ? '' : undefined"
    @mousedown="handlePointerDown"
  >
    <div class="flex min-w-0 flex-1 items-center gap-3">
      <div class="flex h-10 w-10 items-center justify-center rounded-[1.35rem] border border-[rgba(222,209,191,0.95)] bg-[linear-gradient(180deg,rgba(255,252,247,0.98),rgba(243,236,226,0.98))] shadow-[0_12px_24px_rgba(90,70,43,0.08)]">
        <img :src="appMarkUrl" alt="" class="h-6 w-6" />
      </div>
      <div class="min-w-0">
        <p class="truncate text-sm font-semibold tracking-[0.01em] text-ink-900">UCAS iCLASS</p>
        <p class="truncate text-xs text-ink-500">Course workspace</p>
      </div>
    </div>

    <div v-if="desktopShell" class="ml-4 flex items-center gap-1.5 no-drag">
      <button class="titlebar-btn titlebar-btn-subtle" title="设置" type="button" @click="$emit('settings')">
        <Settings2 class="titlebar-icon" aria-hidden="true" />
      </button>
      <button class="titlebar-btn titlebar-btn-window" title="最小化" type="button" @click="$emit('minimize')">
        <Minus class="titlebar-icon" aria-hidden="true" />
      </button>
      <button class="titlebar-btn titlebar-btn-window" :title="maximized ? '还原' : '最大化'" type="button" @click="$emit('maximize')">
        <Minimize2 v-if="maximized" class="titlebar-icon" aria-hidden="true" />
        <Maximize2 v-else class="titlebar-icon" aria-hidden="true" />
      </button>
      <button class="titlebar-btn titlebar-btn-danger" title="关闭" type="button" @click="$emit('close')">
        <X class="titlebar-icon" aria-hidden="true" />
      </button>
    </div>

    <div v-else class="ml-4 flex items-center gap-2 no-drag">
      <button
        class="inline-flex h-11 items-center gap-2 rounded-2xl border border-[rgba(221,211,197,0.9)] bg-[rgba(255,251,246,0.94)] px-3.5 text-sm font-semibold text-ink-700 shadow-[0_10px_24px_rgba(90,70,43,0.06)] transition active:scale-[0.98]"
        title="设置"
        type="button"
        @click="$emit('settings')"
      >
        <Settings2 class="h-4 w-4" aria-hidden="true" />
        <span>设置</span>
      </button>
    </div>
  </header>
</template>
