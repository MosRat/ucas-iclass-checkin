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
  product: "UCAS iCLASS"
};
</script>

<template>
  <section class="mx-auto w-full max-w-5xl">
    <div class="grid gap-5 lg:grid-cols-[0.92fr_1.08fr]">
      <div class="rounded-4xl border border-white/80 bg-[linear-gradient(160deg,rgba(20,66,156,0.94),rgba(49,108,216,0.88),rgba(109,161,255,0.75))] p-6 text-white shadow-[0_28px_80px_rgba(17,54,122,0.28)] md:p-8">
        <span class="inline-flex rounded-full border border-white/20 bg-white/10 px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.24em] text-white/88">
          Workspace
        </span>
        <h2 class="mt-5 text-3xl font-semibold tracking-tight md:text-[2rem]">登录后进入课程工作台</h2>
        <p class="mt-3 max-w-md text-sm leading-7 text-white/78">
          查看个人信息与课表，自动判断打卡时间窗口，并在可用时直接发起打卡。
        </p>

        <div class="mt-8 grid gap-3 sm:grid-cols-2">
          <div class="rounded-[1.6rem] border border-white/14 bg-white/10 px-4 py-4 backdrop-blur-md">
            <p class="text-[11px] uppercase tracking-[0.22em] text-white/55">登录体验</p>
            <p class="mt-2 text-base font-semibold">默认记住我</p>
            <p class="mt-1 text-sm leading-6 text-white/72">下次打开时自动恢复最近一次可用登录态。</p>
          </div>
          <div class="rounded-[1.6rem] border border-white/14 bg-white/10 px-4 py-4 backdrop-blur-md">
            <p class="text-[11px] uppercase tracking-[0.22em] text-white/55">数据视图</p>
            <p class="mt-2 text-base font-semibold">课表与打卡一体</p>
            <p class="mt-1 text-sm leading-6 text-white/72">登录成功后直接进入工作区，不需要再次切换页面。</p>
          </div>
        </div>

        <div class="mt-8 flex flex-wrap gap-2 text-xs text-white/76">
          <span class="rounded-full border border-white/14 bg-white/10 px-3 py-1">版本 {{ buildMeta.version }}</span>
          <span class="rounded-full border border-white/14 bg-white/10 px-3 py-1">许可证 {{ buildMeta.license }}</span>
          <span class="rounded-full border border-white/14 bg-white/10 px-3 py-1">{{ buildMeta.product }}</span>
        </div>
      </div>

      <div class="rounded-4xl border border-white/80 bg-white/90 p-6 shadow-pane backdrop-blur-xl md:p-8">
        <span class="inline-flex rounded-full bg-accent-100 px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.2em] text-accent-700">
          Sign In
        </span>
        <h3 class="mt-4 text-2xl font-semibold text-ink-950">登录 iCLASS</h3>
        <p class="mt-2 text-sm leading-6 text-ink-500">
          输入 UCAS 账号与密码后即可进入工作台。默认会勾选“记住我”，便于下次自动恢复登录。
        </p>
      <form class="mt-6 space-y-5" @submit.prevent="submit">
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
        <label class="inline-flex items-center gap-3 text-sm text-ink-600">
          <input
            v-model="form.rememberPassword"
            class="h-4 w-4 rounded border-slate-300 text-accent-600 focus:ring-accent-500"
            type="checkbox"
          />
          记住我，并在下次打开时自动登录
        </label>
        <button class="primary-btn w-full justify-center" :disabled="loading" type="submit">
          {{ loading ? "登录中..." : "进入工作台" }}
        </button>
      </form>

        <div class="mt-6 grid gap-3 sm:grid-cols-3">
          <div class="rounded-[1.5rem] border border-slate-200/80 bg-slate-50/90 px-4 py-4 text-sm leading-6 text-ink-600">
            <p class="text-[11px] uppercase tracking-[0.2em] text-ink-400">用户名</p>
            <p class="mt-2 font-medium text-ink-800">UCAS 学号或账号</p>
          </div>
          <div class="rounded-[1.5rem] border border-slate-200/80 bg-slate-50/90 px-4 py-4 text-sm leading-6 text-ink-600">
            <p class="text-[11px] uppercase tracking-[0.2em] text-ink-400">密码</p>
            <p class="mt-2 font-medium text-ink-800">iCLASS 对应登录密码</p>
          </div>
          <div class="rounded-[1.5rem] border border-slate-200/80 bg-slate-50/90 px-4 py-4 text-sm leading-6 text-ink-600">
            <p class="text-[11px] uppercase tracking-[0.2em] text-ink-400">默认选项</p>
            <p class="mt-2 font-medium text-ink-800">记住我已开启</p>
          </div>
        </div>

        <div class="mt-6 flex items-center justify-between gap-3 border-t border-slate-200/80 pt-4 text-xs text-ink-400">
          <span>Copyright © UCAS iCLASS</span>
          <span>{{ buildMeta.license }}</span>
        </div>
      </div>
    </div>
  </section>
</template>
