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
    class="sticky top-0 z-30 flex h-16 items-center border-b border-white/60 bg-white/72 px-3 backdrop-blur-xl select-none md:px-4"
    :data-tauri-drag-region="desktopShell ? '' : undefined"
    @mousedown="handlePointerDown"
  >
    <div class="flex min-w-0 flex-1 items-center gap-3">
      <div class="flex h-10 w-10 items-center justify-center rounded-[1.35rem] bg-[linear-gradient(160deg,rgba(24,80,186,1),rgba(55,124,242,0.96))] text-sm font-semibold text-white shadow-[0_14px_30px_rgba(28,86,190,0.28)]">
        I
      </div>
      <div class="min-w-0">
        <p class="truncate text-sm font-semibold tracking-[0.01em] text-ink-900">UCAS iCLASS</p>
        <p class="truncate text-xs text-ink-500">课程工作台</p>
      </div>
    </div>

    <div v-if="desktopShell" class="ml-4 flex items-center gap-2 no-drag">
      <button class="titlebar-btn titlebar-btn-subtle" title="设置" type="button" @click="$emit('settings')">
        <Settings2 class="titlebar-icon" aria-hidden="true" />
      </button>
      <div class="titlebar-window-group">
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
    </div>

    <div v-else class="ml-4 flex items-center gap-2 no-drag">
      <button
        class="inline-flex h-11 items-center gap-2 rounded-2xl border border-white/80 bg-white/88 px-3.5 text-sm font-semibold text-ink-700 shadow-[0_10px_24px_rgba(15,23,42,0.08)] transition active:scale-[0.98]"
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
