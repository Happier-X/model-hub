<script setup lang="ts">
import { onMounted, ref } from "vue";
import { HButton } from "happier-ui";
import {
  extractInvokeError,
  getLastSuccessRequest,
  getRequestStats,
  proxyStart,
  proxyStatus,
  proxyStop,
  type LastSuccessRequest,
  type ProxyStatus,
  type RequestStats,
} from "../api/tauri";

const status = ref<ProxyStatus | null>(null);
const loading = ref(false);
const message = ref("");
const error = ref("");
const stats = ref<RequestStats | null>(null);
const statsError = ref("");
const lastSuccess = ref<LastSuccessRequest | null>(null);
const lastSuccessError = ref("");

function formatSuccessTime(unix: number): string {
  if (!unix) return "-";
  try {
    return new Date(unix * 1000).toLocaleString("zh-CN", { hour12: false });
  } catch {
    return String(unix);
  }
}

async function refreshStats() {
  const statsPromise = getRequestStats()
    .then((value) => {
      stats.value = value;
      statsError.value = "";
    })
    .catch((e) => {
      statsError.value = extractInvokeError(e);
    });
  const lastSuccessPromise = getLastSuccessRequest()
    .then((value) => {
      lastSuccess.value = value;
      lastSuccessError.value = "";
    })
    .catch((e) => {
      lastSuccessError.value = extractInvokeError(e);
    });
  await Promise.all([statsPromise, lastSuccessPromise]);
}

async function refresh() {
  try {
    status.value = await proxyStatus();
    if (status.value.port_note) {
      message.value = status.value.port_note;
    }
    await refreshStats();
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
    message.value = status.value.port_note || "代理已启动";
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

async function copyBaseUrl() {
  if (!status.value?.base_url) return;
  await navigator.clipboard.writeText(status.value.base_url);
  message.value = "Base URL 已复制";
}

const exampleCurl = () => {
  const base = status.value?.base_url || "http://127.0.0.1:8888";
  return `curl ${base}/v1/chat/completions \\
  -H "Content-Type: application/json" \\
  -d '{"model":"你的分组名","messages":[{"role":"user","content":"hi"}]}'`;
};

onMounted(refresh);
</script>

<template>
  <div class="space-y-6">
    <section class="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
        <h2 class="text-base font-semibold">今日请求（本地日）</h2>
        <HButton variant="outline" size="sm" type="button" @click="refreshStats">刷新统计</HButton>
      </div>
      <p class="mb-3 text-xs text-slate-500">
        基于请求日志；成功 = 2xx 且无 error；失败 = 状态 ≥400 或有 error；故障转移 = 记录了换源。
      </p>
      <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <div class="rounded-lg bg-slate-50 px-3 py-3">
          <div class="text-xs text-slate-500">总请求</div>
          <div class="mt-1 text-2xl font-semibold tabular-nums">{{ stats?.total ?? 0 }}</div>
        </div>
        <div class="rounded-lg bg-emerald-50 px-3 py-3">
          <div class="text-xs text-emerald-700">成功</div>
          <div class="mt-1 text-2xl font-semibold tabular-nums text-emerald-800">
            {{ stats?.success ?? 0 }}
          </div>
        </div>
        <div class="rounded-lg bg-rose-50 px-3 py-3">
          <div class="text-xs text-rose-700">失败</div>
          <div class="mt-1 text-2xl font-semibold tabular-nums text-rose-800">
            {{ stats?.failure ?? 0 }}
          </div>
        </div>
        <div class="rounded-lg bg-amber-50 px-3 py-3">
          <div class="text-xs text-amber-800">故障转移</div>
          <div class="mt-1 text-2xl font-semibold tabular-nums text-amber-900">
            {{ stats?.failover ?? 0 }}
          </div>
        </div>
      </div>
      <p v-if="statsError" class="mt-3 text-sm text-rose-600">{{ statsError }}</p>

      <div class="mt-4 rounded-lg border border-slate-100 bg-slate-50/80 px-3 py-3">
        <div class="text-sm font-medium text-slate-800">最近成功请求</div>
        <p class="mt-0.5 text-xs text-slate-500">全局最近一次 2xx 且无 error 的请求（日志态，非配置首选）</p>
        <div v-if="lastSuccess" class="mt-3 grid gap-2 text-sm sm:grid-cols-2">
          <div>
            <div class="text-xs text-slate-500">分组</div>
            <div class="mt-0.5 font-medium break-all">{{ lastSuccess.group_name || "-" }}</div>
          </div>
          <div>
            <div class="text-xs text-slate-500">供应商</div>
            <div class="mt-0.5 font-medium break-all">{{ lastSuccess.provider_name || "-" }}</div>
          </div>
          <div>
            <div class="text-xs text-slate-500">上游模型</div>
            <div class="mt-0.5 font-mono text-xs break-all">{{ lastSuccess.upstream_model || "-" }}</div>
          </div>
          <div>
            <div class="text-xs text-slate-500">请求时间</div>
            <div class="mt-0.5 tabular-nums">{{ formatSuccessTime(lastSuccess.time) }}</div>
          </div>
        </div>
        <p v-else-if="!lastSuccessError" class="mt-3 text-sm text-slate-500">暂无成功请求</p>
        <p v-if="lastSuccessError" class="mt-3 text-sm text-rose-600">{{ lastSuccessError }}</p>
      </div>
    </section>

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
            <HButton variant="tertiary" size="sm" type="button" @click="copyBaseUrl">复制</HButton>
          </div>
        </div>
        <div>
          <div class="text-slate-500">监听</div>
          <div class="mt-1 font-mono">{{ status?.host }}:{{ status?.port }}</div>
        </div>
      </div>

      <div class="mt-5 flex flex-wrap items-center gap-3">
        <HButton variant="secondary" type="button" :disabled="loading" @click="start">启动</HButton>
        <HButton variant="danger" type="button" :disabled="loading" @click="stop">停止</HButton>
        <HButton variant="outline" type="button" @click="refresh">刷新</HButton>
      </div>

      <p v-if="message" class="mt-3 whitespace-pre-line text-sm text-emerald-700">{{ message }}</p>
      <p
        v-if="status?.port_note && status.port_note !== message"
        class="mt-2 text-sm text-amber-800"
      >
        {{ status.port_note }}
      </p>
      <p v-if="error || status?.last_error" class="mt-3 text-sm text-rose-600">
        {{ error || status?.last_error }}
      </p>
      <p class="mt-2 text-xs text-slate-500">
        关闭窗口会隐藏到系统托盘，代理继续运行；仅托盘菜单「退出」会停止代理并释放端口。若首选端口被占用，会自动向后寻找可用端口；若意外多开旧实例，请在旧进程托盘选择「退出」。端口配置和数据目录可在「设置」页查看。
      </p>
    </section>

    <section class="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      <h2 class="mb-3 text-base font-semibold">本机接入步骤</h2>
      <ol class="list-decimal space-y-2 pl-5 text-sm text-slate-700">
        <li>
          <span class="font-medium">启动代理</span>
          ：确认状态为 running，记下或复制 Base URL（默认 127.0.0.1）。
        </li>
        <li>
          <span class="font-medium">新建供应商</span>
          ：填写上游 Base URL 与 API Key，并启用。
        </li>
        <li>
          <span class="font-medium">新建分组与队列</span>
          ：分组名即客户端 model；按优先级添加供应商与上游模型，失败时按队列顺序自动故障转移。
        </li>
        <li>
          <span class="font-medium">客户端 / curl 调用</span>
          ：Base URL 用本机地址，Authorization 可省略，body 中 model 填分组名。
        </li>
      </ol>
      <p class="mt-3 text-xs text-slate-500">
        完整可勾选验收步骤见仓库
        <code class="rounded bg-slate-100 px-1">docs/local-acceptance.md</code>。
      </p>
    </section>

    <section class="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      <h2 class="mb-3 text-base font-semibold">调用示例</h2>
      <p class="mb-2 text-sm text-slate-500">
        客户端使用统一 Base URL；请求体中的 model 填分组名，无需配置客户端密钥。
      </p>
      <pre class="overflow-x-auto rounded-lg bg-slate-900 p-4 text-xs text-slate-100">{{ exampleCurl() }}</pre>
    </section>
  </div>
</template>
