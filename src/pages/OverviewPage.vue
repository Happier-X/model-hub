<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  checkForUpdate,
  downloadAndInstallUpdate,
  extractInvokeError,
  getAppVersion,
  getPaths,
  getShellPrefs,
  proxySetPort,
  proxyStart,
  proxyStatus,
  proxyStop,
  relaunchApp,
  setCheckUpdateOnStartup,
  type AppPaths,
  type DownloadEvent,
  type ProxyStatus,
  type Update,
} from "../api/tauri";

const status = ref<ProxyStatus | null>(null);
const paths = ref<AppPaths | null>(null);
const portInput = ref(8080);
const loading = ref(false);
const message = ref("");
const error = ref("");

/** idle | checking | available | downloading | installing | error */
type UpdatePhase = "idle" | "checking" | "available" | "downloading" | "installing" | "error";
const updatePhase = ref<UpdatePhase>("idle");
const updateMessage = ref("");
const updateError = ref("");
const currentVersion = ref("");
const pendingUpdate = ref<Update | null>(null);
const downloadLoaded = ref(0);
const downloadTotal = ref<number | null>(null);
const checkUpdateOnStartup = ref(false);
const prefsLoading = ref(false);

const updateBusy = computed(
  () =>
    updatePhase.value === "checking" ||
    updatePhase.value === "downloading" ||
    updatePhase.value === "installing",
);

const downloadProgressText = computed(() => {
  if (downloadTotal.value != null && downloadTotal.value > 0) {
    const pct = Math.min(100, Math.round((downloadLoaded.value / downloadTotal.value) * 100));
    return `已下载 ${formatBytes(downloadLoaded.value)} / ${formatBytes(downloadTotal.value)}（${pct}%）`;
  }
  if (downloadLoaded.value > 0) {
    return `已下载 ${formatBytes(downloadLoaded.value)}`;
  }
  return "准备下载…";
});

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

function onDownloadEvent(event: DownloadEvent) {
  if (event.event === "Started") {
    updatePhase.value = "downloading";
    downloadLoaded.value = 0;
    downloadTotal.value = event.data.contentLength ?? null;
    updateMessage.value = "开始下载更新…";
  } else if (event.event === "Progress") {
    downloadLoaded.value += event.data.chunkLength;
    updateMessage.value = downloadProgressText.value;
  } else if (event.event === "Finished") {
    updatePhase.value = "installing";
    updateMessage.value = "下载完成，正在安装…";
  }
}

async function checkUpdate(options?: { quietIfLatest?: boolean; softError?: boolean }) {
  if (updateBusy.value) return;
  const quietIfLatest = options?.quietIfLatest ?? false;
  const softError = options?.softError ?? false;
  updatePhase.value = "checking";
  updateMessage.value = quietIfLatest ? "启动时检查更新…" : "正在检查更新…";
  updateError.value = "";
  pendingUpdate.value = null;
  downloadLoaded.value = 0;
  downloadTotal.value = null;
  try {
    try {
      currentVersion.value = await getAppVersion();
    } catch {
      /* 版本仅展示用，检查本身仍继续 */
    }
    const update = await checkForUpdate();
    if (!update) {
      updatePhase.value = "idle";
      if (quietIfLatest) {
        updateMessage.value = "";
      } else {
        const ver = currentVersion.value ? `（当前版本 ${currentVersion.value}）` : "";
        updateMessage.value = `当前已是最新版本${ver}`;
      }
      return;
    }
    pendingUpdate.value = update;
    currentVersion.value = update.currentVersion || currentVersion.value;
    updatePhase.value = "available";
    updateMessage.value = `发现新版本 ${update.version}`;
  } catch (e) {
    const msg = extractInvokeError(e);
    if (softError) {
      updatePhase.value = "idle";
      updateError.value = "";
      updateMessage.value = `启动时检查更新失败：${msg}`;
      return;
    }
    updatePhase.value = "error";
    updateError.value = msg;
    updateMessage.value = "";
  }
}

async function toggleStartupCheck(enabled: boolean) {
  const previous = checkUpdateOnStartup.value;
  checkUpdateOnStartup.value = enabled;
  prefsLoading.value = true;
  try {
    const prefs = await setCheckUpdateOnStartup(enabled);
    checkUpdateOnStartup.value = prefs.check_update_on_startup;
    message.value = prefs.check_update_on_startup
      ? "已开启：下次进入概览将自动检查更新"
      : "已关闭启动时自动检查更新";
    error.value = "";
  } catch (e) {
    error.value = extractInvokeError(e);
    checkUpdateOnStartup.value = previous;
  } finally {
    prefsLoading.value = false;
  }
}

async function confirmInstall() {
  const update = pendingUpdate.value;
  if (!update || updateBusy.value) return;
  updateError.value = "";
  updatePhase.value = "downloading";
  updateMessage.value = "开始下载更新…";
  downloadLoaded.value = 0;
  downloadTotal.value = null;
  try {
    await downloadAndInstallUpdate(update, onDownloadEvent);
    updatePhase.value = "installing";
    updateMessage.value = "安装完成，正在重启应用…";
    pendingUpdate.value = null;
    await relaunchApp();
  } catch (e) {
    updatePhase.value = "error";
    updateError.value = extractInvokeError(e);
    updateMessage.value = "";
    // 保留 pendingUpdate，便于重试下载安装
  }
}

function cancelPendingUpdate() {
  if (updateBusy.value) return;
  pendingUpdate.value = null;
  updatePhase.value = "idle";
  updateMessage.value = "";
  updateError.value = "";
}

async function refresh() {
  try {
    status.value = await proxyStatus();
    portInput.value = status.value.port;
    paths.value = await getPaths();
    try {
      const prefs = await getShellPrefs();
      checkUpdateOnStartup.value = prefs.check_update_on_startup;
    } catch {
      /* 偏好读取失败不阻塞代理状态 */
    }
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

onMounted(async () => {
  await refresh();
  if (checkUpdateOnStartup.value) {
    await checkUpdate({ quietIfLatest: true, softError: true });
  }
});
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
      <h2 class="mb-3 text-base font-semibold">应用更新</h2>
      <p class="mb-3 text-sm text-slate-500">
        检查 GitHub Release 上的更新清单；发现新版本后须确认才会下载安装并重启。默认不在启动时自动检查。
      </p>
      <label class="mb-3 flex items-center gap-2 text-sm text-slate-700">
        <input
          type="checkbox"
          :checked="checkUpdateOnStartup"
          :disabled="prefsLoading"
          @change="toggleStartupCheck(($event.target as HTMLInputElement).checked)"
        />
        进入概览时自动检查更新（仍需确认后才安装）
      </label>
      <div class="flex flex-wrap items-center gap-3">
        <button
          type="button"
          class="rounded-lg bg-cyan-700 px-4 py-2 text-sm text-white hover:bg-cyan-600 disabled:opacity-50"
          :disabled="updateBusy"
          @click="checkUpdate()"
        >
          {{ updatePhase === "checking" ? "检查中…" : "检查更新" }}
        </button>
        <span v-if="currentVersion" class="text-xs text-slate-500">当前版本 {{ currentVersion }}</span>
      </div>

      <div
        v-if="pendingUpdate && (updatePhase === 'available' || updatePhase === 'error')"
        class="mt-4 rounded-lg border border-cyan-200 bg-cyan-50 p-4 text-sm"
      >
        <p class="font-medium text-cyan-900">
          发现新版本 {{ pendingUpdate.version }}
          <span v-if="pendingUpdate.currentVersion" class="font-normal text-cyan-700">
            （当前 {{ pendingUpdate.currentVersion }}）
          </span>
        </p>
        <pre
          v-if="pendingUpdate.body"
          class="mt-2 max-h-40 overflow-auto whitespace-pre-wrap rounded bg-white/80 p-2 text-xs text-slate-700"
        >{{ pendingUpdate.body }}</pre>
        <p class="mt-2 text-xs text-slate-600">确认后将下载安装包、完成安装并自动重启应用。数据目录中的配置与数据库不会被删除。</p>
        <div class="mt-3 flex flex-wrap gap-2">
          <button
            type="button"
            class="rounded-lg bg-cyan-700 px-3 py-1.5 text-sm text-white hover:bg-cyan-600 disabled:opacity-50"
            :disabled="updateBusy"
            @click="confirmInstall"
          >
            {{ updatePhase === "error" ? "重试下载安装" : "下载并安装" }}
          </button>
          <button
            type="button"
            class="rounded-lg border border-slate-300 bg-white px-3 py-1.5 text-sm hover:bg-slate-50 disabled:opacity-50"
            :disabled="updateBusy"
            @click="cancelPendingUpdate"
          >
            稍后
          </button>
        </div>
      </div>

      <p
        v-if="updateMessage"
        class="mt-3 text-sm"
        :class="updatePhase === 'available' ? 'text-cyan-800' : 'text-emerald-700'"
      >
        {{ updateMessage }}
      </p>
      <p v-if="updateError" class="mt-3 text-sm text-rose-600">{{ updateError }}</p>
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
          ：分组名即客户端 model；按优先级添加供应商与上游模型，按需开启自动故障转移。
        </li>
        <li>
          <span class="font-medium">创建 API Key</span>
          ：明文仅创建成功时展示一次，请立即保存。
        </li>
        <li>
          <span class="font-medium">客户端 / curl 调用</span>
          ：Base URL 用本机地址，Authorization 用客户端 Key，body 中 model 填分组名。
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
        客户端使用统一 Base URL + 客户端 API Key；请求体中的 model 填分组名。
      </p>
      <pre class="overflow-x-auto rounded-lg bg-slate-900 p-4 text-xs text-slate-100">{{ exampleCurl() }}</pre>
    </section>
  </div>
</template>
