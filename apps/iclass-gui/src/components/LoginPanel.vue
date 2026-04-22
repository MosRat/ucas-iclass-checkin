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
  developer: "未检测到 Git 仓库，暂无法读取提交作者"
};
</script>

<template>
  <section class="mx-auto w-full max-w-2xl">
    <div class="rounded-4xl border border-white/80 bg-white/88 p-6 shadow-pane backdrop-blur-xl md:p-7">
      <h2 class="text-xl font-semibold text-ink-950">登录 iCLASS</h2>
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

      <div class="mt-6 rounded-[1.75rem] border border-slate-200/80 bg-slate-50/90 px-4 py-4 text-sm leading-6 text-ink-600">
        <p>用户名：UCAS 学号或账号。</p>
        <p>密码：iCLASS 对应登录密码。</p>
        <p>默认选项：记住我已开启。</p>
      </div>

      <div class="mt-5 flex flex-wrap gap-2 text-xs text-ink-500">
        <span class="rounded-full bg-slate-100 px-3 py-1">版本 {{ buildMeta.version }}</span>
        <span class="rounded-full bg-slate-100 px-3 py-1">许可证 {{ buildMeta.license }}</span>
      </div>

      <div class="mt-4 text-xs leading-6 text-ink-400">
        <p>开发者信息：{{ buildMeta.developer }}</p>
        <p>Copyright © UCAS iCLASS Tools</p>
      </div>
    </div>
  </section>
</template>
