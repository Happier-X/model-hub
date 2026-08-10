<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef } from "vue";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { renderMarkdown } from "../utils/markdown";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  checkForUpdate,
  downloadAndInstallUpdate,
  extractInvokeError,
  getAppVersion,
  getModelPricing,
  getPaths,
  getShellPrefs,
  proxySetPort,
  proxyStatus,
  relaunchApp,
  setCheckUpdateOnStartup,
  setOverlayEnabled,
  syncPricingNow,
  type AppPaths,
  type DownloadEvent,
  type PricingInfo,
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
/** 把更新日志 markdown 渲染为安全 HTML（renderMarkdown 已关闭原始 HTML，防 XSS）。 */
const releaseNotesHtml = computed(() => renderMarkdown(pendingUpdate.value?.body));
const downloadLoaded = ref(0);
const downloadTotal = ref<number | null>(null);
const checkUpdateOnStartup = ref(false);
const overlayEnabled = ref(false);
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

async function toggleOverlay(enabled: boolean) {
  const previous = overlayEnabled.value;
  overlayEnabled.value = enabled;
  prefsLoading.value = true;
  try {
    const prefs = await setOverlayEnabled(enabled);
    overlayEnabled.value = prefs.overlay_enabled;
    message.value = prefs.overlay_enabled ? "桌面悬浮条已显示" : "桌面悬浮条已隐藏";
    error.value = "";
  } catch (e) {
    error.value = extractInvokeError(e);
    overlayEnabled.value = previous;
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
      overlayEnabled.value = prefs.overlay_enabled;
    } catch {
      /* 偏好读取失败不阻塞配置页 */
    }
    try {
      currentVersion.value = await getAppVersion();
    } catch {
      /* 浏览器开发态无版本 */
    }
    await refreshPricing();
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

// ---- 模型单价（OpenRouter 同步，只读） ----

const pricingInfo = ref<PricingInfo | null>(null);
const pricingError = ref("");
const pricingLoading = ref(false);
const syncLoading = ref(false);
const pricingSearch = ref("");

async function refreshPricing() {
  pricingLoading.value = true;
  try {
    pricingInfo.value = await getModelPricing();
    pricingError.value = "";
  } catch (e) {
    pricingError.value = extractInvokeError(e);
  } finally {
    pricingLoading.value = false;
  }
}

async function syncPricing() {
  syncLoading.value = true;
  pricingError.value = "";
  try {
    await syncPricingNow();
    await refreshPricing();
  } catch (e) {
    pricingError.value = extractInvokeError(e);
  } finally {
    syncLoading.value = false;
  }
}

const pricingFiltered = computed(() => {
  const items = pricingInfo.value?.items ?? [];
  const kw = pricingSearch.value.trim().toLowerCase();
  if (!kw) return items;
  return items.filter((item) => item.model_name.toLowerCase().includes(kw));
});

function formatPricingTime(unix: number | null): string {
  if (unix === null) return "从未同步";
  return new Date(unix * 1000).toLocaleString("zh-CN");
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
    <Card class="border border-slate-200 bg-white">
      <CardHeader class="py-3">
        <h2 class="text-base font-semibold">代理配置</h2>
      </CardHeader>
      <CardContent class="flex flex-col gap-3">
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
          <label class="block text-sm">
            <span class="mb-1 block text-slate-600">端口</span>
            <Input
              :model-value="String(portInput)"
              type="number"
              inputmode="numeric"
              @update:model-value="portInput = Number($event) || 0"
            />
          </label>
        </div>
        <Button variant="default" type="button" :disabled="loading" @click="savePort">
          保存端口
        </Button>
        <Button variant="outline" type="button" @click="refresh">刷新</Button>
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
    </CardContent>
    </Card>

    <Card class="border border-slate-200 bg-white">
      <CardHeader class="py-3">
        <h2 class="text-base font-semibold">桌面悬浮条</h2>
      </CardHeader>
      <CardContent class="flex flex-col gap-3">
      <div class="mb-3">
        <label class="flex items-center gap-2 text-sm">
          <Checkbox
            :model-value="overlayEnabled"
            :disabled="prefsLoading"
            @update:model-value="toggleOverlay"
          />
          <span>显示最近成功模型悬浮条</span>
        </label>
      </div>
      <p class="text-sm text-slate-500">
        开启后会在主显示器任务栏上方显示无边框状态条；关闭主窗口时代理仍继续运行，托盘「退出」才会停止代理。
      </p>
    </CardContent>
    </Card>

    <Card class="border border-slate-200 bg-white">
      <CardHeader class="py-3">
        <h2 class="text-base font-semibold">应用更新</h2>
      </CardHeader>
      <CardContent class="flex flex-col gap-3">
      <p class="mb-3 text-sm text-slate-500">
        检查 GitHub Release 上的更新清单；发现新版本后须确认才会下载安装并重启。默认不在启动时自动检查。
      </p>
      <div class="mb-3">
        <label class="flex items-center gap-2 text-sm">
          <Checkbox
            :model-value="checkUpdateOnStartup"
            :disabled="prefsLoading"
            @update:model-value="toggleStartupCheck"
          />
          <span>应用启动时自动检查更新（仍需确认后才安装）</span>
        </label>
      </div>
      <div class="flex flex-wrap items-center gap-3">
        <Button variant="default" type="button" :disabled="updateBusy" @click="checkUpdate()">
          {{ updatePhase === "checking" ? "检查中…" : "检查更新" }}
        </Button>
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
        <!-- eslint-disable-next-line vue/no-v-html -- markdown-it html:false 已转义原始 HTML -->
        <div
          v-if="releaseNotesHtml"
          class="markdown-body mt-2 max-h-40 overflow-auto rounded bg-white/80 p-2 text-xs text-slate-700"
          v-html="releaseNotesHtml"
        ></div>
        <p class="mt-2 text-xs text-slate-600">
          确认后将下载安装包、完成安装并自动重启应用。数据目录中的配置与数据库不会被删除。
        </p>
        <div class="mt-3 flex flex-wrap gap-2">
          <Button
            variant="default"
            size="sm"
            type="button"
            :disabled="updateBusy"
            @click="confirmInstall"
          >
            {{ updatePhase === "error" ? "重试下载安装" : "下载并安装" }}
          </Button>
          <Button
            variant="outline"
            size="sm"
            type="button"
            :disabled="updateBusy"
            @click="cancelPendingUpdate"
          >
            稍后
          </Button>
        </div>
      </div>

      <!-- 下载进行中：Progress 进度条 + 辅助文本 -->
      <div v-if="updatePhase === 'downloading'" class="mt-3 space-y-1">
        :<Progress
          :value="downloadLoaded"
          :max="downloadTotal ?? 0"
          :indeterminate="!downloadTotal"
          size="md"
          variant="default"
          rounded
        />
        <p class="text-sm text-emerald-700">{{ updateMessage }}</p>
      </div>
      <!-- 其他状态：纯文本 -->
      <p
        v-else-if="updateMessage"
        class="mt-3 text-sm"
        :class="updatePhase === 'available' ? 'text-cyan-800' : 'text-emerald-700'"
      >
        {{ updateMessage }}
      </p>
      <p v-if="updateError" class="mt-3 text-sm text-rose-600">{{ updateError }}</p>
    </CardContent>
    </Card>

    <Card class="border border-slate-200 bg-white">
      <CardHeader class="py-3">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <h2 class="text-base font-semibold">模型单价</h2>
          <Button
            variant="outline"
            size="sm"
            type="button"
            :disabled="syncLoading"
            @click="syncPricing"
          >
            {{ syncLoading ? "同步中…" : "立即同步" }}
          </Button>
        </div>
      </CardHeader>
      <CardContent class="flex flex-col gap-3">
        <div class="flex flex-wrap items-center justify-between gap-2 text-xs text-slate-500">
          <span>
            已同步 {{ pricingInfo?.count ?? 0 }} 个模型 · 最后同步：
            {{ formatPricingTime(pricingInfo?.updated_at ?? null) }}
          </span>
          <Input
            v-model="pricingSearch"
            placeholder="搜索模型名…"
            class="h-8 w-56"
          />
        </div>
        <p class="text-xs text-slate-500">
          价格来源 OpenRouter，每百万 token 美元；未覆盖的模型按 $0 计入费用。启动代理后会自动同步（约 24h 检查一次）。
        </p>
        <p v-if="pricingError" class="text-sm text-rose-600">{{ pricingError }}</p>
        <div v-if="pricingLoading" class="py-6 text-center text-sm text-slate-500">加载中…</div>
        <div v-else-if="(pricingFiltered.length === 0 && pricingSearch) || (pricingFiltered.length === 0 && (pricingInfo?.count ?? 0) > 0)" class="py-6 text-center text-sm text-slate-500">
          无匹配模型
        </div>
        <div v-else-if="(pricingInfo?.count ?? 0) === 0" class="py-6 text-center text-sm text-slate-500">
          尚未同步，点击「立即同步」或等待后台自动同步。
        </div>
        <div v-else class="max-h-96 overflow-auto rounded-lg border border-slate-200">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>模型</TableHead>
                <TableHead class="text-right">输入价 /百万 token</TableHead>
                <TableHead class="text-right">输出价 /百万 token</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              <TableRow v-for="item in pricingFiltered" :key="item.model_name">
                <TableCell class="font-mono text-xs break-all">{{ item.model_name }}</TableCell>
                <TableCell class="text-right tabular-nums">${{ item.prompt_price_per_mtok }}</TableCell>
                <TableCell class="text-right tabular-nums">${{ item.completion_price_per_mtok }}</TableCell>
              </TableRow>
            </TableBody>
          </Table>
        </div>
      </CardContent>
    </Card>
  </div>
</template>
