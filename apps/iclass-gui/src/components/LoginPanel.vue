<script setup lang="ts">
import { reactive, watch } from "vue";

const props = defineProps<{
  loading: boolean;
  account: string;
}>();

const emit = defineEmits<{
  submit: [{ account: string; password: string; rememberPassword: boolean }];
}>();

const form = reactive({
  account: props.account,
  password: "",
  rememberPassword: true
});

watch(
  () => props.account,
  (value) => {
    form.account = value;
  }
);

function submit() {
  emit("submit", { ...form });
}

const buildMeta = {
  version: "v0.1.0",
  license: "MIT",
  product: "UCAS iCLASS",
  developer: "MosRat"
};
</script>

<template>
  <section class="mx-auto w-full max-w-5xl">
    <div class="grid gap-4 lg:grid-cols-[0.92fr_1.08fr] lg:gap-5">
      <div class="rounded-[1.8rem] border border-[rgba(224,214,198,0.88)] bg-[linear-gradient(180deg,rgba(255,252,247,0.97),rgba(249,244,236,0.95))] p-4 shadow-pane sm:p-6 md:p-7 lg:p-8">
        <span class="inline-flex rounded-full border border-[rgba(221,205,183,0.9)] bg-[rgba(245,236,224,0.9)] px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.2em] text-[rgb(122,90,54)]">
          Sign In
        </span>
        <h3 class="mt-3 text-[1.65rem] font-semibold leading-tight text-ink-950 sm:text-2xl">登录 iCLASS</h3>
        <p class="mt-2 max-w-md text-sm leading-6 text-ink-500">
          输入 UCAS 账号与密码后即可进入工作台。默认会勾选“记住我”，便于下次自动恢复登录。
        </p>
        <form class="mt-5 space-y-3.5 sm:space-y-4" @submit.prevent="submit">
          <label class="block">
            <span class="field-label">账号</span>
            <input
              v-model.trim="form.account"
              class="field-input"
              autocomplete="username"
              inputmode="numeric"
              placeholder="2025xxxxxxxxxx"
              required
            />
          </label>
          <label class="block">
            <span class="field-label">密码</span>
            <input
              v-model="form.password"
              class="field-input"
              autocomplete="current-password"
              placeholder="请输入密码"
              required
              type="password"
            />
          </label>
          <label class="inline-flex items-center gap-3 text-sm leading-6 text-ink-600">
            <input
              v-model="form.rememberPassword"
              class="h-4 w-4 rounded border-[rgba(197,178,152,0.9)] text-[rgb(123,92,57)] focus:ring-[rgba(194,164,122,0.26)]"
              type="checkbox"
            />
            记住我，并在下次打开时自动登录
          </label>
          <button class="primary-btn w-full justify-center py-3.5 text-base" :disabled="loading" type="submit">
            {{ loading ? "登录中..." : "进入工作台" }}
          </button>
        </form>

        <div class="mt-4 rounded-[1.35rem] border border-[rgba(224,214,198,0.88)] bg-[rgba(250,245,238,0.92)] px-4 py-3 text-sm leading-6 text-ink-600 sm:hidden">
          <p>用户名：UCAS 学号或账号</p>
          <p>密码：iCLASS 登录密码</p>
          <p>默认：记住我已开启</p>
        </div>

        <div class="mt-5 hidden gap-3 sm:grid sm:grid-cols-3">
          <div class="rounded-[1.4rem] border border-[rgba(224,214,198,0.88)] bg-[rgba(250,245,238,0.92)] px-4 py-4 text-sm leading-6 text-ink-600">
            <p class="text-[11px] uppercase tracking-[0.2em] text-ink-400">用户名</p>
            <p class="mt-2 font-medium text-ink-800">UCAS 学号或账号</p>
          </div>
          <div class="rounded-[1.4rem] border border-[rgba(224,214,198,0.88)] bg-[rgba(250,245,238,0.92)] px-4 py-4 text-sm leading-6 text-ink-600">
            <p class="text-[11px] uppercase tracking-[0.2em] text-ink-400">密码</p>
            <p class="mt-2 font-medium text-ink-800">iCLASS 对应登录密码</p>
          </div>
          <div class="rounded-[1.4rem] border border-[rgba(224,214,198,0.88)] bg-[rgba(250,245,238,0.92)] px-4 py-4 text-sm leading-6 text-ink-600">
            <p class="text-[11px] uppercase tracking-[0.2em] text-ink-400">默认选项</p>
            <p class="mt-2 font-medium text-ink-800">记住我已开启</p>
          </div>
        </div>

        <div class="mt-5 flex flex-col gap-2 border-t border-[rgba(224,214,198,0.88)] pt-4 text-xs text-ink-400 sm:flex-row sm:items-center sm:justify-between">
          <span>{{ buildMeta.product }} {{ buildMeta.version }} · {{ buildMeta.license }}</span>
          <span>Developer: {{ buildMeta.developer }}</span>
        </div>
      </div>

      <div class="rounded-[1.8rem] border border-[rgba(220,208,192,0.8)] bg-[linear-gradient(180deg,rgba(245,239,231,0.98),rgba(235,227,215,0.94))] p-4 text-ink-900 shadow-[0_22px_56px_rgba(90,70,43,0.1)] sm:p-6 lg:p-8">
        <div class="flex items-center justify-between gap-3">
          <span class="inline-flex rounded-full border border-[rgba(211,194,171,0.9)] bg-[rgba(255,250,244,0.76)] px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.24em] text-[rgb(118,85,47)]">
            Workspace
          </span>
          <div class="flex flex-wrap gap-2 text-xs text-ink-500">
            <span class="rounded-full border border-[rgba(219,207,191,0.9)] bg-[rgba(255,250,244,0.7)] px-3 py-1">v{{ buildMeta.version.slice(1) }}</span>
            <span class="rounded-full border border-[rgba(219,207,191,0.9)] bg-[rgba(255,250,244,0.7)] px-3 py-1">{{ buildMeta.license }}</span>
          </div>
        </div>
        <h2 class="mt-5 max-w-lg text-[1.55rem] font-semibold leading-tight tracking-[-0.02em] text-ink-950 sm:text-[1.9rem] lg:text-[2.2rem]">
          A calmer way to sign in and work through today’s classes.
        </h2>
        <p class="mt-3 max-w-md text-sm leading-6 text-ink-600">
          登录后直接进入课程工作台，查看当日课表、个人信息和课程状态，在开放时间内完成打卡。
        </p>

        <div class="mt-4 grid gap-3 sm:hidden">
          <div class="rounded-[1.25rem] border border-[rgba(219,207,191,0.9)] bg-[rgba(255,250,244,0.72)] px-4 py-3 text-sm leading-6 text-ink-600">
            <p>课表、个人信息与打卡集中在一个页面。</p>
            <p>课程开始前 30 分钟会自动允许打卡。</p>
          </div>
        </div>

        <div class="mt-5 hidden gap-3 sm:grid sm:grid-cols-2">
          <div class="rounded-[1.4rem] border border-[rgba(219,207,191,0.9)] bg-[rgba(255,250,244,0.72)] px-4 py-4">
            <p class="text-[11px] uppercase tracking-[0.22em] text-ink-400">登录体验</p>
            <p class="mt-2 text-base font-semibold text-ink-900">默认记住我</p>
            <p class="mt-1 text-sm leading-6 text-ink-600">下次打开时自动恢复最近一次可用登录态。</p>
          </div>
          <div class="rounded-[1.4rem] border border-[rgba(219,207,191,0.9)] bg-[rgba(255,250,244,0.72)] px-4 py-4">
            <p class="text-[11px] uppercase tracking-[0.22em] text-ink-400">数据视图</p>
            <p class="mt-2 text-base font-semibold text-ink-900">课表与打卡一体</p>
            <p class="mt-1 text-sm leading-6 text-ink-600">登录后直接进入工作区，不需要反复切换页面。</p>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>
