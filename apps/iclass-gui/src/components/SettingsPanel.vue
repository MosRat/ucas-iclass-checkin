<script setup lang="ts">
import { X } from "lucide-vue-next";
import type { AppPreferences, CheckInModePreference } from "../composables/usePreferences";
import type { AutomationSettings, DesktopSettings } from "../lib/types";

defineProps<{
  open: boolean;
  desktopShell: boolean;
  preferences: AppPreferences;
  desktopSettings: DesktopSettings;
  desktopLoading: boolean;
  automationSettings: AutomationSettings;
  automationLoading: boolean;
}>();

const emit = defineEmits<{
  close: [];
  reset: [];
}>();

const modeOptions: Array<{ value: CheckInModePreference; label: string; description: string }> = [
  {
    value: "auto",
    label: "自动",
    description: "优先使用 UUID，必要时回退到 ID 模式。"
  },
  {
    value: "uuid",
    label: "仅 UUID",
    description: "更严格，适合你明确知道课程支持 UUID 打卡时。"
  },
  {
    value: "id",
    label: "仅 ID",
    description: "只使用课程排课 ID 打卡。"
  }
];
</script>

<template>
  <transition name="dialog-fade">
    <div v-if="open" class="fixed inset-0 z-40 bg-[rgba(36,28,20,0.18)]">
      <div class="absolute inset-0 flex items-start justify-end p-0 md:p-6">
        <aside class="flex h-full w-full flex-col border-l border-[rgba(224,214,198,0.88)] bg-[rgba(250,247,241,0.97)] shadow-fluent backdrop-blur-2xl md:h-[calc(100vh-3rem)] md:max-h-[920px] md:w-[31rem] md:rounded-[2rem]">
          <div class="flex items-center justify-between border-b border-[rgba(224,214,198,0.8)] px-5 py-4">
            <div>
              <p class="text-xs uppercase tracking-[0.24em] text-ink-400">Application Settings</p>
              <h3 class="mt-1 text-lg font-semibold text-ink-950">偏好设置</h3>
            </div>
            <button class="titlebar-btn titlebar-btn-subtle" title="关闭设置" type="button" @click="emit('close')">
              <X class="titlebar-icon" aria-hidden="true" />
            </button>
          </div>

          <div class="flex-1 space-y-6 overflow-y-auto px-5 py-5">
            <section class="glass-panel p-5">
              <h4 class="text-base font-semibold text-ink-950">自动打卡</h4>
              <div class="mt-4 space-y-4">
                <label class="flex items-start gap-3 rounded-3xl border border-[rgba(224,214,198,0.88)] bg-[rgba(255,252,247,0.84)] px-4 py-4">
                  <input
                    v-model="automationSettings.autoCheckInEnabled"
                    :disabled="automationLoading"
                    class="mt-1 h-4 w-4 rounded border-[rgba(197,178,152,0.9)] text-[rgb(123,92,57)] focus:ring-[rgba(194,164,122,0.26)] disabled:opacity-60"
                    type="checkbox"
                  />
                  <span>
                    <span class="block text-sm font-semibold text-ink-900">后台自动打卡</span>
                    <span class="mt-1 block text-sm leading-6 text-ink-500">
                      应用启动后会在后台定时检查最近课程；若已进入打卡窗口，则按下面的模式自动尝试打卡。
                    </span>
                  </span>
                </label>

                <label class="block rounded-3xl border border-[rgba(224,214,198,0.88)] bg-[rgba(255,252,247,0.84)] px-4 py-4">
                  <span class="block text-sm font-semibold text-ink-900">轮询间隔</span>
                  <span class="mt-1 block text-sm leading-6 text-ink-500">建议 30-90 秒。自动打卡只在应用运行期间生效，过短只会增加无效请求。</span>
                  <input
                    v-model.number="automationSettings.autoCheckIntervalSeconds"
                    :disabled="automationLoading"
                    class="field-input mt-3"
                    max="300"
                    min="30"
                    step="15"
                    type="number"
                  />
                </label>

                <div class="space-y-3 rounded-3xl border border-[rgba(224,214,198,0.88)] bg-[rgba(255,252,247,0.84)] px-4 py-4">
                  <span class="block text-sm font-semibold text-ink-900">自动打卡模式</span>
                  <label
                    v-for="option in modeOptions"
                    :key="`auto-${option.value}`"
                    class="flex cursor-pointer items-start gap-3 rounded-3xl border px-4 py-4 transition"
                    :class="
                      automationSettings.autoCheckInMode === option.value
                        ? 'border-[rgba(201,179,149,0.9)] bg-[rgba(245,236,224,0.82)]'
                        : 'border-[rgba(224,214,198,0.88)] bg-[rgba(255,252,247,0.76)]'
                    "
                  >
                    <input
                      v-model="automationSettings.autoCheckInMode"
                      class="mt-1 h-4 w-4 border-[rgba(197,178,152,0.9)] text-[rgb(123,92,57)] focus:ring-[rgba(194,164,122,0.26)]"
                      :value="option.value"
                      type="radio"
                    />
                    <span>
                      <span class="block text-sm font-semibold text-ink-900">{{ option.label }}</span>
                      <span class="mt-1 block text-sm leading-6 text-ink-500">{{ option.description }}</span>
                    </span>
                  </label>
                </div>
              </div>
            </section>

            <section class="glass-panel p-5">
              <h4 class="text-base font-semibold text-ink-950">启动与恢复</h4>
              <div class="mt-4 space-y-4">
                <label class="flex items-start gap-3 rounded-3xl border border-[rgba(224,214,198,0.88)] bg-[rgba(255,252,247,0.84)] px-4 py-4">
                  <input
                    v-model="preferences.autoSyncOnLaunch"
                    class="mt-1 h-4 w-4 rounded border-[rgba(197,178,152,0.9)] text-[rgb(123,92,57)] focus:ring-[rgba(194,164,122,0.26)]"
                    type="checkbox"
                  />
                  <span>
                    <span class="block text-sm font-semibold text-ink-900">启动时自动同步</span>
                    <span class="mt-1 block text-sm leading-6 text-ink-500">
                      打开应用后立即尝试恢复 session 并同步工作台。
                    </span>
                  </span>
                </label>

                <label class="flex items-start gap-3 rounded-3xl border border-[rgba(224,214,198,0.88)] bg-[rgba(255,252,247,0.84)] px-4 py-4">
                  <input
                    v-model="preferences.rememberLastAccount"
                    class="mt-1 h-4 w-4 rounded border-[rgba(197,178,152,0.9)] text-[rgb(123,92,57)] focus:ring-[rgba(194,164,122,0.26)]"
                    type="checkbox"
                  />
                  <span>
                    <span class="block text-sm font-semibold text-ink-900">记住最近账号</span>
                    <span class="mt-1 block text-sm leading-6 text-ink-500">
                      下次回到登录页时，自动带出最近一次成功登录的账号。
                    </span>
                  </span>
                </label>
              </div>
            </section>

            <section v-if="desktopShell" class="glass-panel p-5">
              <h4 class="text-base font-semibold text-ink-950">桌面集成</h4>
              <div class="mt-4 space-y-4">
                <label class="flex items-start gap-3 rounded-3xl border border-[rgba(224,214,198,0.88)] bg-[rgba(255,252,247,0.84)] px-4 py-4">
                  <input
                    v-model="desktopSettings.autostartEnabled"
                    :disabled="desktopLoading || !desktopSettings.autostartAvailable"
                    class="mt-1 h-4 w-4 rounded border-[rgba(197,178,152,0.9)] text-[rgb(123,92,57)] focus:ring-[rgba(194,164,122,0.26)] disabled:opacity-60"
                    type="checkbox"
                  />
                  <span>
                    <span class="block text-sm font-semibold text-ink-900">开机自启</span>
                    <span class="mt-1 block text-sm leading-6 text-ink-500">
                      {{
                        desktopSettings.autostartAvailable
                          ? "在系统登录后自动启动应用，适合长期驻留打卡场景。"
                          : "当前版本暂不支持开机自启。"
                      }}
                    </span>
                  </span>
                </label>

                <label class="flex items-start gap-3 rounded-3xl border border-[rgba(224,214,198,0.88)] bg-[rgba(255,252,247,0.84)] px-4 py-4">
                  <input
                    v-model="desktopSettings.closeToTray"
                    :disabled="desktopLoading || !desktopSettings.closeToTrayAvailable"
                    class="mt-1 h-4 w-4 rounded border-[rgba(197,178,152,0.9)] text-[rgb(123,92,57)] focus:ring-[rgba(194,164,122,0.26)] disabled:opacity-60"
                    type="checkbox"
                  />
                  <span>
                    <span class="block text-sm font-semibold text-ink-900">关闭时隐藏到托盘</span>
                    <span class="mt-1 block text-sm leading-6 text-ink-500">
                      {{
                        desktopSettings.closeToTrayAvailable
                          ? "点击关闭按钮时不退出进程，而是隐藏窗口并继续在托盘待命。"
                          : "当前版本暂不支持托盘驻留。"
                      }}
                    </span>
                  </span>
                </label>
              </div>
            </section>

            <section class="glass-panel p-5">
              <h4 class="text-base font-semibold text-ink-950">课表与交互</h4>
              <div class="mt-4 space-y-4">
                <label class="flex items-start gap-3 rounded-3xl border border-[rgba(224,214,198,0.88)] bg-[rgba(255,252,247,0.84)] px-4 py-4">
                  <input
                    v-model="preferences.autoOpenCheckableSchedule"
                    class="mt-1 h-4 w-4 rounded border-[rgba(197,178,152,0.9)] text-[rgb(123,92,57)] focus:ring-[rgba(194,164,122,0.26)]"
                    type="checkbox"
                  />
                  <span>
                    <span class="block text-sm font-semibold text-ink-900">优先聚焦可打卡课程</span>
                    <span class="mt-1 block text-sm leading-6 text-ink-500">
                      同步完成后自动选中当前最值得关注的课程。
                    </span>
                  </span>
                </label>

                <label class="flex items-start gap-3 rounded-3xl border border-[rgba(224,214,198,0.88)] bg-[rgba(255,252,247,0.84)] px-4 py-4">
                  <input
                    v-model="preferences.compactScheduleCards"
                    class="mt-1 h-4 w-4 rounded border-[rgba(197,178,152,0.9)] text-[rgb(123,92,57)] focus:ring-[rgba(194,164,122,0.26)]"
                    type="checkbox"
                  />
                  <span>
                    <span class="block text-sm font-semibold text-ink-900">紧凑课表卡片</span>
                    <span class="mt-1 block text-sm leading-6 text-ink-500">
                      减少课表卡片留白，适合小屏或课程较多的情况。
                    </span>
                  </span>
                </label>
              </div>
            </section>

            <section class="glass-panel p-5">
              <h4 class="text-base font-semibold text-ink-950">默认打卡模式</h4>
              <div class="mt-4 space-y-3">
                <label
                  v-for="option in modeOptions"
                  :key="option.value"
                  class="flex cursor-pointer items-start gap-3 rounded-3xl border px-4 py-4 transition"
                  :class="
                    preferences.defaultCheckInMode === option.value
                      ? 'border-[rgba(201,179,149,0.9)] bg-[rgba(245,236,224,0.82)]'
                      : 'border-[rgba(224,214,198,0.88)] bg-[rgba(255,252,247,0.84)]'
                  "
                >
                  <input
                    v-model="preferences.defaultCheckInMode"
                    class="mt-1 h-4 w-4 border-[rgba(197,178,152,0.9)] text-[rgb(123,92,57)] focus:ring-[rgba(194,164,122,0.26)]"
                    :value="option.value"
                    type="radio"
                  />
                  <span>
                    <span class="block text-sm font-semibold text-ink-900">{{ option.label }}</span>
                    <span class="mt-1 block text-sm leading-6 text-ink-500">{{ option.description }}</span>
                  </span>
                </label>
              </div>
            </section>

            <section class="glass-panel p-5">
              <h4 class="text-base font-semibold text-ink-950">默认课表视图</h4>
              <div class="mt-4 grid gap-3 sm:grid-cols-2">
                <button
                  class="rounded-3xl border px-4 py-4 text-left transition"
                  :class="
                    preferences.defaultScheduleView === 'day'
                      ? 'border-[rgba(201,179,149,0.9)] bg-[rgba(245,236,224,0.82)]'
                      : 'border-[rgba(224,214,198,0.88)] bg-[rgba(255,252,247,0.84)]'
                  "
                  type="button"
                  @click="preferences.defaultScheduleView = 'day'"
                >
                  <span class="block text-sm font-semibold text-ink-900">日视图</span>
                  <span class="mt-1 block text-sm leading-6 text-ink-500">聚焦当天课程与即时打卡。</span>
                </button>
                <button
                  class="rounded-3xl border px-4 py-4 text-left transition"
                  :class="
                    preferences.defaultScheduleView === 'week'
                      ? 'border-[rgba(201,179,149,0.9)] bg-[rgba(245,236,224,0.82)]'
                      : 'border-[rgba(224,214,198,0.88)] bg-[rgba(255,252,247,0.84)]'
                  "
                  type="button"
                  @click="preferences.defaultScheduleView = 'week'"
                >
                  <span class="block text-sm font-semibold text-ink-900">周视图</span>
                  <span class="mt-1 block text-sm leading-6 text-ink-500">适合先看整周课程安排，再筛选目标课程。</span>
                </button>
              </div>
            </section>
          </div>

          <div class="flex items-center justify-between gap-3 border-t border-[rgba(224,214,198,0.8)] px-5 py-4">
            <button class="secondary-btn" type="button" @click="emit('reset')">恢复默认</button>
            <button class="primary-btn" type="button" @click="emit('close')">完成</button>
          </div>
        </aside>
      </div>
    </div>
  </transition>
</template>
