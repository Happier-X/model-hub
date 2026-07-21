<script setup lang="ts">
import { onMounted, ref } from "vue";
import {
  extractInvokeError,
  getPaths,
  proxySetPort,
  proxyStart,
  proxyStatus,
  proxyStop,
  type AppPaths,
  type ProxyStatus,
} from "../api/tauri";

const status = ref<ProxyStatus | null>(null);
const paths = ref<AppPaths | null>(null);
const portInput = ref(8080);
const loading = ref(false);
const message = ref("");
const error = ref("");

async function refresh() {
  try {
    status.value = await proxyStatus();
    portInput.value = status.value.port;
    paths.value = await getPaths();
    error.value = "";
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

async function start() {
  loading.value = true;
  message.value = "";
  try {
    status.value = await proxyStart();
    message.value = "代理已启动";
  } catch (e) {
    error.value = extractInvokeError(e);
  } finally {
    loading.value = false;
  }
}

async function stop() {
  loading.value = true;
  message.value = "";
  try {
    status.value = await proxyStop();
    message.value = "代理已停止";
  } catch (e) {
    error.value = extractInvokeError(e);
  } finally {
    loading.value = false;
  }
}

async function savePort() {
  loading.value = true;
  message.value = "";
  try {
    status.value = await proxySetPort(portInput.value);
    message.value = `端口已更新为 ${portInput.value}`;
  } catch (e) {
    error.value = extractInvokeError(e);
  } finally {
    loading.value = false;
  }
}

async function copyBaseUrl() {
  if (!status.value?.base_url) return;
  await navigator.clipboard.writeText(status.value.base_url);
  message.value = "Base URL 已复制";
}

const exampleCurl = () => {
  const base = status.value?.base_url || "http://127.0.0.1:8080";
  return `curl ${base}/v1/chat/completions \\
  -H "Authorization: Bearer sk-modelhub-..." \\
  -H "Content-Type: application/json" \\
  -d '{"model":"你的分组名","messages":[{"role":"user","content":"hi"}]}'`;
};

onMounted(refresh);
</script>

<template>
  <div class="space-y-6">
    <section class="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      <h2 class="mb-4 text-base font-semibold">本地代理</h2>
      <div class="grid gap-3 text-sm md:grid-cols-2">
        <div>
          <div class="text-slate-500">状态</div>
          <div class="mt-1 font-medium">
            <span
              class="inline-flex rounded-full px-2 py-0.5 text-xs"
              :class="
                status?.state === 'running'
                  ? 'bg-emerald-100 text-emerald-700'
                  : status?.state === 'error'
                    ? 'bg-rose-100 text-rose-700'
                    : 'bg-slate-100 text-slate-600'
              "
            >
              {{ status?.state || "未知" }}
            </span>
          </div>
        </div>
        <div>
          <div class="text-slate-500">Base URL</div>
          <div class="mt-1 flex items-center gap-2 font-mono text-sm">
            <span>{{ status?.base_url || "-" }}</span>
            <button
              type="button"
              class="rounded bg-slate-100 px-2 py-1 text-xs hover:bg-slate-200"
              @click="copyBaseUrl"
            >
              复制
            </button>
          </div>
        </div>
        <div>
          <div class="text-slate-500">监听</div>
          <div class="mt-1 font-mono">{{ status?.host }}:{{ status?.port }}</div>
        </div>
        <div>
          <div class="text-slate-500">数据目录</div>
          <div class="mt-1 break-all font-mono text-xs">{{ paths?.gateway_dir || status?.data_dir || "-" }}</div>
        </div>
      </div>

      <div class="mt-5 flex flex-wrap items-end gap-3">
        <label class="text-sm">
          <span class="mb-1 block text-slate-500">端口</span>
          <input
            v-model.number="portInput"
            type="number"
            min="1"
            max="65535"
            class="w-28 rounded-lg border border-slate-300 px-3 py-2"
          />
        </label>
        <button
          type="button"
          class="rounded-lg bg-slate-800 px-4 py-2 text-sm text-white hover:bg-slate-700 disabled:opacity-50"
          :disabled="loading"
          @click="savePort"
        >
          保存端口
        </button>
        <button
          type="button"
          class="rounded-lg bg-emerald-600 px-4 py-2 text-sm text-white hover:bg-emerald-500 disabled:opacity-50"
          :disabled="loading"
          @click="start"
        >
          启动
        </button>
        <button
          type="button"
          class="rounded-lg bg-rose-600 px-4 py-2 text-sm text-white hover:bg-rose-500 disabled:opacity-50"
          :disabled="loading"
          @click="stop"
        >
          停止
        </button>
        <button
          type="button"
          class="rounded-lg border border-slate-300 px-4 py-2 text-sm hover:bg-slate-50"
          @click="refresh"
        >
          刷新
        </button>
      </div>

      <p v-if="message" class="mt-3 text-sm text-emerald-700">{{ message }}</p>
      <p v-if="error || status?.last_error" class="mt-3 text-sm text-rose-600">
        {{ error || status?.last_error }}
      </p>
    </section>

    <section class="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      <h2 class="mb-3 text-base font-semibold">调用示例</h2>
      <p class="mb-2 text-sm text-slate-500">
        客户端使用统一 Base URL + 客户端 API Key；请求体中的 model 填分组名。
      </p>
      <pre class="overflow-x-auto rounded-lg bg-slate-900 p-4 text-xs text-slate-100">{{ exampleCurl() }}</pre>
    </section>
  </div>
</template>
