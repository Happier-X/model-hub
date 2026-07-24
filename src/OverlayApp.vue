<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ExternalLink } from "@lucide/vue";
import {
  getLastSuccessRequest,
  proxyStatus,
  saveOverlayPosition,
  showMainWindow,
  type LastSuccessRequest,
  type ProxyStatus,
} from "./api/tauri";

const POLL_INTERVAL_MS = 2500;

const status = ref<ProxyStatus | null>(null);
const lastSuccess = ref<LastSuccessRequest | null>(null);
/** 上一次拉取是否失败：失败时保留旧数据，只做非阻断提示 */
const fetchFailed = ref(false);

let timer: number | null = null;
let positionTimer: number | null = null;
let unlistenMoved: UnlistenFn | null = null;

/** 四类派生展示状态 */
type OverlayView =
  | { kind: "model"; model: string; group: string; provider: string; time: string }
  | { kind: "running-empty" }
  | { kind: "stopped" }
  | { kind: "error"; detail: string };

const view = computed<OverlayView>(() => {
  const s = status.value;
  if (!s) return { kind: "stopped" };
  if (s.state === "error") {
    return { kind: "error", detail: s.last_error || "代理异常" };
  }
  if (s.state === "running" || s.state === "starting") {
    const ok = lastSuccess.value;
    if (ok) {
      return {
        kind: "model",
        model: ok.upstream_model,
        group: ok.group_name,
        provider: ok.provider_name,
        time: formatTime(ok.time),
      };
    }
    return { kind: "running-empty" };
  }
  // idle / stopping
  return { kind: "stopped" };
});

const dotClass = computed(() => {
  switch (view.value.kind) {
    case "model":
      return "dot dot--ok";
    case "running-empty":
      return "dot dot--idle";
    case "stopped":
      return "dot dot--stopped";
    case "error":
      return "dot dot--error";
    default:
      return "dot";
  }
});

const tooltip = computed(() => {
  const v = view.value;
  switch (v.kind) {
    case "model":
      return `分组：${v.group}\n供应商：${v.provider}\n上游模型：${v.model}\n最近成功：${v.time}`;
    case "running-empty":
      return "代理运行中，尚无成功请求";
    case "stopped":
      return "代理已停止";
    case "error":
      return v.detail;
    default:
      return "";
  }
});

function formatTime(unix: number): string {
  const d = new Date(unix * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

async function poll() {
  try {
    const [s, ok] = await Promise.all([proxyStatus(), getLastSuccessRequest()]);
    status.value = s;
    lastSuccess.value = ok;
    fetchFailed.value = false;
  } catch {
    // 保留上一次有效数据，只标记失败，避免闪烁
    fetchFailed.value = true;
  }
}

function schedulePositionSave(x: number, y: number) {
  if (positionTimer !== null) window.clearTimeout(positionTimer);
  positionTimer = window.setTimeout(() => {
    positionTimer = null;
    void saveOverlayPosition(x, y).catch(() => {
      /* 位置保存失败不影响展示 */
    });
  }, 300);
}

async function openMain() {
  try {
    await showMainWindow();
  } catch {
    /* 打开主窗口失败静默 */
  }
}

onMounted(async () => {
  void poll();
  timer = window.setInterval(poll, POLL_INTERVAL_MS);
  try {
    unlistenMoved = await getCurrentWindow().onMoved(({ payload }) => {
      schedulePositionSave(payload.x, payload.y);
    });
  } catch {
    /* 移动事件监听失败不影响展示 */
  }
});

onUnmounted(() => {
  if (timer !== null) {
    window.clearInterval(timer);
    timer = null;
  }
  if (positionTimer !== null) {
    window.clearTimeout(positionTimer);
    positionTimer = null;
  }
  unlistenMoved?.();
  unlistenMoved = null;
});
</script>

<template>
  <div class="overlay" :title="tooltip">
    <span class="overlay__drag" data-tauri-drag-region @dblclick="openMain">
      <span :class="dotClass" />
      <span class="overlay__text">
        <template v-if="view.kind === 'model'">
          <span class="overlay__model">{{ view.model }}</span>
          <span class="overlay__meta">{{ view.group }} · {{ view.provider }} · {{ view.time }}</span>
        </template>
        <template v-else-if="view.kind === 'running-empty'">
          <span class="overlay__model">暂无模型</span>
          <span class="overlay__meta">代理运行中，尚无成功请求</span>
        </template>
        <template v-else-if="view.kind === 'stopped'">
          <span class="overlay__model">代理已停止</span>
          <span class="overlay__meta">托盘退出可释放端口</span>
        </template>
        <template v-else>
          <span class="overlay__model">代理异常</span>
          <span class="overlay__meta">{{ view.detail }}</span>
        </template>
      </span>
    </span>
    <button class="overlay__open" type="button" title="打开 Model Hub 主窗口" @click="openMain">
      <ExternalLink :size="14" stroke-width="2" aria-hidden="true" />
    </button>
    <span v-if="fetchFailed" class="overlay__stale" title="暂时无法刷新，显示上次数据">·</span>
  </div>
</template>

<style scoped>
.overlay {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100vw;
  height: 100vh;
  padding: 0 12px;
  box-sizing: border-box;
  background: #111827;
  color: #e2e8f0;
  font-family:
    Inter, ui-sans-serif, system-ui, -apple-system, "Segoe UI", "Microsoft YaHei", sans-serif;
  user-select: none;
  overflow: hidden;
}

.overlay__drag {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1 1 auto;
  min-width: 0;
  cursor: move;
}

.dot {
  flex: 0 0 auto;
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: #64748b;
}

.dot--ok {
  background: #22c55e;
}

.dot--idle {
  background: #eab308;
}

.dot--stopped {
  background: #64748b;
}

.dot--error {
  background: #ef4444;
}

.overlay__text {
  display: flex;
  flex-direction: column;
  min-width: 0;
  line-height: 1.2;
}

.overlay__model {
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.overlay__meta {
  font-size: 11px;
  color: #94a3b8;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.overlay__open {
  flex: 0 0 auto;
  border: none;
  background: transparent;
  color: #94a3b8;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  line-height: 1;
  padding: 0;
  border-radius: 6px;
}

.overlay__open:hover {
  background: rgba(148, 163, 184, 0.2);
  color: #e2e8f0;
}

.overlay__stale {
  flex: 0 0 auto;
  color: #eab308;
  font-weight: 700;
}
</style>
