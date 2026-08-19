<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef } from "vue";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { renderMarkdown } from "../utils/markdown";
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
  setOverlayEnabled,
  setUpstreamProxy,
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
/** 把更新日志 markdown 渲染为安全 HTML（renderMarkdown 已关闭原始 HTML，防 XSS）。 */
const releaseNotesHtml = computed(() => renderMarkdown(pendingUpdate.value?.body));
const downloadLoaded = ref(0);
const downloadTotal = ref<number | null>(null);
const checkUpdateOnStartup = ref(false);
const overlayEnabled = ref(false);
const prefsLoading = ref(false);
// 上游代理配置
const proxyEnabled = ref(false);
const proxyUrl = ref("");
const proxyUser = ref("");
const proxyPass = ref("");
const proxySaving = ref(false);

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
      proxyEnabled.value = prefs.upstream_proxy_enabled;
      proxyUrl.value = prefs.upstream_proxy_url;
      proxyUser.value = prefs.upstream_proxy_user;
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

async function saveProxy() {
  proxySaving.value = true;
  message.value = "";
  try {
    const prefs = await setUpstreamProxy({
      enabled: proxyEnabled.value,
      url: proxyUrl.value.trim(),
      username: proxyUser.value.trim(),
      password: proxyPass.value,
    });
    proxyEnabled.value = prefs.upstream_proxy_enabled;
    proxyUrl.value = prefs.upstream_proxy_url;
    proxyUser.value = prefs.upstream_proxy_user;
    proxyPass.value = "";
    message.value = proxyEnabled.value ? "上游代理已更新" : "上游代理已关闭";
    error.value = "";
  } catch (e) {
    error.value = extractInvokeError(e);
  } finally {
    proxySaving.value = false;
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
  <div class="flex flex-col gap-6">
    <Card>
      <CardHeader class="py-3">
        <h2 class="text-base font-semibold">代理配置</h2>
      </CardHeader>
      <CardContent class="flex flex-col gap-3">
      <div class="grid gap-3 text-sm md:grid-cols-2">
        <div>
          <div class="text-muted-foreground">当前监听</div>
          <div class="mt-1 font-mono">{{ status?.host || "-" }}:{{ status?.port ?? "-" }}</div>
        </div>
        <div>
          <div class="text-muted-foreground">数据目录</div>
          <div class="mt-1 break-all font-mono text-xs">
            {{ paths?.gateway_dir || status?.data_dir || "-" }}
          </div>
        </div>
      </div>

      <div class="mt-5 flex flex-wrap items-end gap-3">
        <div class="w-28">
          <Field>
            <FieldLabel>端口</FieldLabel>
            <Input
              :model-value="String(portInput)"
              type="number"
              inputmode="numeric"
              @update:model-value="portInput = Number($event) || 0"
            />
          </Field>
        </div>
        <Button variant="default" type="button" :disabled="loading" @click="savePort">
          保存端口
        </Button>
        <Button variant="outline" type="button" @click="refresh">刷新</Button>
      </div>

      <p v-if="message" class="mt-3 whitespace-pre-line text-sm text-success">{{ message }}</p>
      <p
        v-if="status?.port_note && status.port_note !== message"
        class="mt-2 text-sm text-warning"
      >
        {{ status.port_note }}
      </p>
      <p v-if="error || status?.last_error" class="mt-3 text-sm text-destructive">
        {{ error || status?.last_error }}
      </p>
      <div class="mt-5 border-t border-border pt-4">
        <h3 class="mb-3 text-sm font-medium">上游代理</h3>
        <div class="mb-3">
          <Field orientation="horizontal">
            <Checkbox
              id="proxy-enabled"
              :model-value="proxyEnabled"
              :disabled="proxySaving"
              @update:model-value="proxyEnabled = $event === true"
            />
            <FieldLabel for="proxy-enabled">通过代理访问上游供应商</FieldLabel>
          </Field>
        </div>
        <div v-if="proxyEnabled" class="flex flex-col gap-3">
          <Field>
            <FieldLabel>代理地址</FieldLabel>
            <Input
              v-model="proxyUrl"
              :disabled="proxySaving"
              placeholder="http://127.0.0.1:7890 或 socks5://127.0.0.1:1080"
            />
          </Field>
          <div class="grid gap-3 md:grid-cols-2">
            <Field>
              <FieldLabel>用户名（可选）</FieldLabel>
              <Input
                v-model="proxyUser"
                :disabled="proxySaving"
                placeholder="留空则不认证"
              />
            </Field>
            <Field>
              <FieldLabel>密码（可选）</FieldLabel>
              <Input
                v-model="proxyPass"
                type="password"
                :disabled="proxySaving"
                placeholder="留空则不认证"
              />
            </Field>
          </div>
        </div>
        <Button
          variant="default"
          type="button"
          class="mt-3"
          :disabled="proxySaving"
          @click="saveProxy"
        >
          {{ proxySaving ? "保存中…" : "保存代理设置" }}
        </Button>
        <p class="mt-2 text-xs text-muted-foreground">
          修改后代理会自动重启以应用新配置。支持 HTTP/HTTPS 和 SOCKS5 协议。
        </p>
      </div>
      <p class="mt-2 text-xs text-muted-foreground">
        若首选端口被占用，会自动向后寻找可用端口并写入配置，不会结束占用进程。改口后若用
        Pi，请到「分组」页重新「配置到 Pi」。
      </p>
    </CardContent>
    </Card>

    <Card>
      <CardHeader class="py-3">
        <h2 class="text-base font-semibold">桌面悬浮条</h2>
      </CardHeader>
      <CardContent class="flex flex-col gap-3">
      <div class="mb-3">
        <Field orientation="horizontal">
          <Checkbox
            id="overlay-enabled"
            :model-value="overlayEnabled"
            :disabled="prefsLoading"
            @update:model-value="(v) => toggleOverlay(v === true)"
          />
          <FieldLabel for="overlay-enabled">显示最近成功模型悬浮条</FieldLabel>
        </Field>
      </div>
      <p class="text-sm text-muted-foreground">
        开启后会在主显示器任务栏上方显示无边框状态条；关闭主窗口时代理仍继续运行，托盘「退出」才会停止代理。
      </p>
    </CardContent>
    </Card>

    <Card>
      <CardHeader class="py-3">
        <h2 class="text-base font-semibold">应用更新</h2>
      </CardHeader>
      <CardContent class="flex flex-col gap-3">
      <p class="mb-3 text-sm text-muted-foreground">
        检查 GitHub Release 上的更新清单；发现新版本后须确认才会下载安装并重启。默认不在启动时自动检查。
      </p>
      <div class="mb-3">
        <Field orientation="horizontal">
          <Checkbox
            id="startup-check"
            :model-value="checkUpdateOnStartup"
            :disabled="prefsLoading"
            @update:model-value="(v) => toggleStartupCheck(v === true)"
          />
          <FieldLabel for="startup-check">应用启动时自动检查更新（仍需确认后才安装）</FieldLabel>
        </Field>
      </div>
      <div class="flex flex-wrap items-center gap-3">
        <Button variant="default" type="button" :disabled="updateBusy" @click="checkUpdate()">
          {{ updatePhase === "checking" ? "检查中…" : "检查更新" }}
        </Button>
        <span v-if="currentVersion" class="text-xs text-muted-foreground">当前版本 {{ currentVersion }}</span>
      </div>

      <div
        v-if="pendingUpdate && (updatePhase === 'available' || updatePhase === 'error')"
        class="mt-4 rounded-lg border border-info/20 bg-info/10 p-4 text-sm"
      >
        <p class="font-medium text-info">
          发现新版本 {{ pendingUpdate.version }}
          <span v-if="pendingUpdate.currentVersion" class="font-normal text-info">
            （当前 {{ pendingUpdate.currentVersion }}）
          </span>
        </p>
        <!-- eslint-disable-next-line vue/no-v-html -- markdown-it html:false 已转义原始 HTML -->
        <div
          v-if="releaseNotesHtml"
          class="markdown-body mt-2 max-h-40 overflow-auto rounded bg-card/80 p-2 text-xs text-foreground"
          v-html="releaseNotesHtml"
        ></div>
        <p class="mt-2 text-xs text-muted-foreground">
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
      <div v-if="updatePhase === 'downloading'" class="mt-3 flex flex-col gap-1">
        :<Progress
          :value="downloadLoaded"
          :max="downloadTotal ?? 0"
          :indeterminate="!downloadTotal"
          size="md"
          variant="default"
          rounded
        />
        <p class="text-sm text-success">{{ updateMessage }}</p>
      </div>
      <!-- 其他状态：纯文本 -->
      <p
        v-else-if="updateMessage"
        class="mt-3 text-sm"
        :class="updatePhase === 'available' ? 'text-info' : 'text-success'"
      >
        {{ updateMessage }}
      </p>
      <p v-if="updateError" class="mt-3 text-sm text-destructive">{{ updateError }}</p>
    </CardContent>
    </Card>
  </div>
</template>
