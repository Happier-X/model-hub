<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef } from "vue";
import { HButton, HCheckbox, HInput } from "happier-ui";
import {
  checkForUpdate,
  downloadAndInstallUpdate,
  extractInvokeError,
  getAppVersion,
  getPaths,
  getShellPrefs,
  proxySetPort,
  proxyStatus,
  relaunchApp,
  setCheckUpdateOnStartup,
  type AppPaths,
  type DownloadEvent,
  type ProxyStatus,
  type Update,
} from "../api/tauri";

const status = ref<ProxyStatus | null>(null);
const paths = ref<AppPaths | null>(null);
const portInput = ref(8888);
const loading = ref(false);
const message = ref("");
const error = ref("");

/** idle | checking | available | downloading | installing | error */
type UpdatePhase = "idle" | "checking" | "available" | "downloading" | "installing" | "error";
const updatePhase = ref<UpdatePhase>("idle");
const updateMessage = ref("");
const updateError = ref("");
const currentVersion = ref("");
/**
 * 必须用 shallowRef：Update 继承 Tauri Resource，内部依赖 JS 私有成员。
 * 深层 ref 会把实例变成 Proxy，调用 downloadAndInstall 时触发 private member 错误。
 */
const pendingUpdate = shallowRef<Update | null>(null);
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

async function releasePendingUpdate(options?: { closeResource?: boolean }) {
  const current = pendingUpdate.value;
  pendingUpdate.value = null;
  if (!current || options?.closeResource === false) return;
  try {
    await current.close();
  } catch {
    /* 资源可能已由安装路径释放；忽略 close 失败以免挡住 UI */
  }
}

async function checkUpdate() {
  if (updateBusy.value) return;
  updatePhase.value = "checking";
  updateMessage.value = "正在检查更新…";
  updateError.value = "";
  await releasePendingUpdate();
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
      const ver = currentVersion.value ? `（当前版本 ${currentVersion.value}）` : "";
      updateMessage.value = `当前已是最新版本${ver}`;
      return;
    }
    pendingUpdate.value = update;
    currentVersion.value = update.currentVersion || currentVersion.value;
    updatePhase.value = "available";
    updateMessage.value = `发现新版本 ${update.version}`;
  } catch (e) {
    updatePhase.value = "error";
    updateError.value = extractInvokeError(e);
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
      ? "已开启：下次应用启动时将自动检查更新"
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
    // downloadAndInstall 成功后 Rust 侧已释放资源，勿再 close
    await releasePendingUpdate({ closeResource: false });
    await relaunchApp();
  } catch (e) {
    updatePhase.value = "error";
    updateError.value = extractInvokeError(e);
    updateMessage.value = "";
    // 保留 pendingUpdate，便于重试下载安装
  }
}

async function cancelPendingUpdate() {
  if (updateBusy.value) return;
  await releasePendingUpdate();
  updatePhase.value = "idle";
  updateMessage.value = "";
  updateError.value = "";
}

async function refresh() {
  try {
    status.value = await proxyStatus();
    portInput.value = status.value.port;
    if (status.value.port_note) {
      message.value = status.value.port_note;
    }
    paths.value = await getPaths();
    try {
      const prefs = await getShellPrefs();
      checkUpdateOnStartup.value = prefs.check_update_on_startup;
    } catch {
      /* 偏好读取失败不阻塞配置页 */
    }
    try {
      currentVersion.value = await getAppVersion();
    } catch {
      /* 浏览器开发态无版本 */
    }
    error.value = "";
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

async function savePort() {
  loading.value = true;
  message.value = "";
  try {
    status.value = await proxySetPort(portInput.value);
    portInput.value = status.value.port;
    message.value = status.value.port_note || `端口已更新为 ${status.value.port}`;
  } catch (e) {
    error.value = extractInvokeError(e);
  } finally {
    loading.value = false;
  }
}

onMounted(async () => {
  await refresh();
});

onUnmounted(() => {
  if (!updateBusy.value) {
    void releasePendingUpdate();
  }
});
</script>

<template>
  <div class="space-y-6">
    <section class="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      <h2 class="mb-4 text-base font-semibold">代理配置</h2>
      <div class="grid gap-3 text-sm md:grid-cols-2">
        <div>
          <div class="text-slate-500">当前监听</div>
          <div class="mt-1 font-mono">{{ status?.host || "-" }}:{{ status?.port ?? "-" }}</div>
        </div>
        <div>
          <div class="text-slate-500">数据目录</div>
          <div class="mt-1 break-all font-mono text-xs">
            {{ paths?.gateway_dir || status?.data_dir || "-" }}
          </div>
        </div>
      </div>

      <div class="mt-5 flex flex-wrap items-end gap-3">
        <div class="w-28">
          <HInput
            :model-value="String(portInput)"
            type="number"
            label="端口"
            inputmode="numeric"
            @update:model-value="portInput = Number($event) || 0"
          />
        </div>
        <HButton variant="primary" type="button" :disabled="loading" @click="savePort">
          保存端口
        </HButton>
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
        若首选端口被占用，会自动向后寻找可用端口并写入配置，不会结束占用进程。改口后若用
        Pi，请到「分组」页重新「配置到 Pi」。
      </p>
    </section>

    <section class="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      <h2 class="mb-3 text-base font-semibold">应用更新</h2>
      <p class="mb-3 text-sm text-slate-500">
        检查 GitHub Release 上的更新清单；发现新版本后须确认才会下载安装并重启。默认不在启动时自动检查。
      </p>
      <div class="mb-3">
        <HCheckbox
          :model-value="checkUpdateOnStartup"
          label="应用启动时自动检查更新（仍需确认后才安装）"
          :disabled="prefsLoading"
          @update:model-value="toggleStartupCheck"
        />
      </div>
      <div class="flex flex-wrap items-center gap-3">
        <HButton variant="primary" type="button" :disabled="updateBusy" @click="checkUpdate()">
          {{ updatePhase === "checking" ? "检查中…" : "检查更新" }}
        </HButton>
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
        <p class="mt-2 text-xs text-slate-600">
          确认后将下载安装包、完成安装并自动重启应用。数据目录中的配置与数据库不会被删除。
        </p>
        <div class="mt-3 flex flex-wrap gap-2">
          <HButton
            variant="primary"
            size="sm"
            type="button"
            :disabled="updateBusy"
            @click="confirmInstall"
          >
            {{ updatePhase === "error" ? "重试下载安装" : "下载并安装" }}
          </HButton>
          <HButton
            variant="outline"
            size="sm"
            type="button"
            :disabled="updateBusy"
            @click="cancelPendingUpdate"
          >
            稍后
          </HButton>
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
  </div>
</template>
