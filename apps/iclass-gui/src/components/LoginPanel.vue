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
    <div class="grid gap-4 lg:grid-cols-[0.96fr_1.04fr] lg:gap-5">
      <div class="rounded-[1.8rem] border border-white/82 bg-white/94 p-4 shadow-pane backdrop-blur-xl sm:p-6 md:p-7 lg:p-8">
        <span class="inline-flex rounded-full bg-accent-100 px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.2em] text-accent-700">
          Sign In
        </span>
        <h3 class="mt-3 text-[1.65rem] font-semibold leading-tight text-ink-950 sm:text-2xl">登录 iCLASS</h3>
        <p class="mt-2 text-sm leading-6 text-ink-500">
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
              class="h-4 w-4 rounded border-slate-300 text-accent-600 focus:ring-accent-500"
              type="checkbox"
            />
            记住我，并在下次打开时自动登录
          </label>
          <button class="primary-btn w-full justify-center py-3.5 text-base" :disabled="loading" type="submit">
            {{ loading ? "登录中..." : "进入工作台" }}
          </button>
        </form>

        <div class="mt-4 rounded-[1.35rem] border border-slate-200/80 bg-slate-50/85 px-4 py-3 text-sm leading-6 text-ink-600 sm:hidden">
          <p>用户名：UCAS 学号或账号</p>
          <p>密码：iCLASS 登录密码</p>
          <p>默认：记住我已开启</p>
        </div>

        <div class="mt-5 hidden gap-3 sm:grid sm:grid-cols-3">
          <div class="rounded-[1.4rem] border border-slate-200/80 bg-slate-50/90 px-4 py-4 text-sm leading-6 text-ink-600">
            <p class="text-[11px] uppercase tracking-[0.2em] text-ink-400">用户名</p>
            <p class="mt-2 font-medium text-ink-800">UCAS 学号或账号</p>
          </div>
          <div class="rounded-[1.4rem] border border-slate-200/80 bg-slate-50/90 px-4 py-4 text-sm leading-6 text-ink-600">
            <p class="text-[11px] uppercase tracking-[0.2em] text-ink-400">密码</p>
            <p class="mt-2 font-medium text-ink-800">iCLASS 对应登录密码</p>
          </div>
          <div class="rounded-[1.4rem] border border-slate-200/80 bg-slate-50/90 px-4 py-4 text-sm leading-6 text-ink-600">
            <p class="text-[11px] uppercase tracking-[0.2em] text-ink-400">默认选项</p>
            <p class="mt-2 font-medium text-ink-800">记住我已开启</p>
          </div>
        </div>

        <div class="mt-5 flex flex-col gap-2 border-t border-slate-200/80 pt-4 text-xs text-ink-400 sm:flex-row sm:items-center sm:justify-between">
          <span>{{ buildMeta.product }} {{ buildMeta.version }} · {{ buildMeta.license }}</span>
          <span>Developer: {{ buildMeta.developer }}</span>
        </div>
      </div>

      <div class="rounded-[1.8rem] border border-white/72 bg-[linear-gradient(165deg,rgba(38,92,194,0.95),rgba(84,142,241,0.82))] p-4 text-white shadow-[0_22px_56px_rgba(17,54,122,0.2)] sm:p-6 lg:p-8">
        <div class="flex items-center justify-between gap-3">
          <span class="inline-flex rounded-full border border-white/18 bg-white/10 px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.24em] text-white/88">
            Workspace
          </span>
          <div class="flex flex-wrap gap-2 text-xs text-white/76">
            <span class="rounded-full border border-white/14 bg-white/10 px-3 py-1">v{{ buildMeta.version.slice(1) }}</span>
            <span class="rounded-full border border-white/14 bg-white/10 px-3 py-1">{{ buildMeta.license }}</span>
          </div>
        </div>
        <h2 class="mt-4 text-[1.45rem] font-semibold leading-tight tracking-tight sm:text-[1.75rem] lg:text-[2rem]">
          登录后进入课程工作台
        </h2>
        <p class="mt-3 max-w-md text-sm leading-6 text-white/78">
          查看课表、个人信息和打卡状态，在开放时间内直接发起打卡。
        </p>

        <div class="mt-4 grid gap-3 sm:hidden">
          <div class="rounded-[1.25rem] border border-white/14 bg-white/10 px-4 py-3 text-sm leading-6 text-white/74">
            <p>课表、个人信息与打卡集中在一个页面。</p>
            <p>课程开始前 30 分钟会自动允许打卡。</p>
          </div>
        </div>

        <div class="mt-5 hidden gap-3 sm:grid sm:grid-cols-2">
          <div class="rounded-[1.4rem] border border-white/14 bg-white/10 px-4 py-4 backdrop-blur-md">
            <p class="text-[11px] uppercase tracking-[0.22em] text-white/55">登录体验</p>
            <p class="mt-2 text-base font-semibold">默认记住我</p>
            <p class="mt-1 text-sm leading-6 text-white/72">下次打开时自动恢复最近一次可用登录态。</p>
          </div>
          <div class="rounded-[1.4rem] border border-white/14 bg-white/10 px-4 py-4 backdrop-blur-md">
            <p class="text-[11px] uppercase tracking-[0.22em] text-white/55">数据视图</p>
            <p class="mt-2 text-base font-semibold">课表与打卡一体</p>
            <p class="mt-1 text-sm leading-6 text-white/72">登录后直接进入工作区，不需要反复切换页面。</p>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>
