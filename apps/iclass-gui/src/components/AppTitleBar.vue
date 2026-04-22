<script setup lang="ts">
import { Minus, Settings2, Square, SquareStack, X } from "lucide-vue-next";

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
    class="sticky top-0 z-30 flex h-14 items-center border-b border-white/60 bg-white/70 px-3 backdrop-blur-xl select-none"
    :data-tauri-drag-region="desktopShell ? '' : undefined"
    @mousedown="handlePointerDown"
  >
    <div class="flex min-w-0 flex-1 items-center gap-3">
      <div class="flex h-9 w-9 items-center justify-center rounded-2xl bg-accent-500 text-sm font-semibold text-white shadow-pane">
        U
      </div>
      <div class="min-w-0">
        <p class="truncate text-sm font-semibold text-ink-900">UCAS iCLASS</p>
        <p class="truncate text-xs text-ink-500">课程查看与打卡</p>
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
        <SquareStack v-if="maximized" class="titlebar-icon" aria-hidden="true" />
        <Square v-else class="titlebar-icon" aria-hidden="true" />
      </button>
      <button class="titlebar-btn titlebar-btn-danger" title="关闭" type="button" @click="$emit('close')">
        <X class="titlebar-icon" aria-hidden="true" />
      </button>
    </div>
  </header>
</template>
