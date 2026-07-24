<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "@lucide/vue";

const win = getCurrentWindow();
const isMaximized = ref(false);
let unlistenResized: UnlistenFn | null = null;

async function syncMaximized() {
  try {
    isMaximized.value = await win.isMaximized();
  } catch {
    /* 查询最大化状态失败不影响标题栏展示 */
  }
}

async function minimize() {
  try {
    await win.minimize();
  } catch {
    /* 最小化失败静默 */
  }
}

async function toggleMaximize() {
  try {
    await win.toggleMaximize();
  } catch {
    /* 最大化/还原失败静默 */
  }
}

async function close() {
  try {
    // 触发 CloseRequested，后端拦截为隐藏到托盘、代理继续
    await win.close();
  } catch {
    /* 关闭请求失败静默 */
  }
}

onMounted(async () => {
  void syncMaximized();
  try {
    unlistenResized = await win.onResized(() => {
      void syncMaximized();
    });
  } catch {
    /* 尺寸变化监听失败不影响标题栏展示 */
  }
});

onUnmounted(() => {
  unlistenResized?.();
  unlistenResized = null;
});
</script>

<template>
  <div class="flex h-9 shrink-0 items-stretch bg-slate-900 text-slate-100 select-none">
    <div
      class="flex flex-1 items-center px-4 text-xs font-medium tracking-wide text-slate-400"
      data-tauri-drag-region
    >
      Model Hub
    </div>
    <div class="flex items-stretch">
      <button
        class="inline-flex w-11 items-center justify-center text-slate-300 transition hover:bg-slate-700 hover:text-white"
        type="button"
        title="最小化"
        aria-label="最小化"
        @click="minimize"
      >
        <Minus :size="16" aria-hidden="true" />
      </button>
      <button
        class="inline-flex w-11 items-center justify-center text-slate-300 transition hover:bg-slate-700 hover:text-white"
        type="button"
        :title="isMaximized ? '还原' : '最大化'"
        :aria-label="isMaximized ? '还原' : '最大化'"
        @click="toggleMaximize"
      >
        <Copy v-if="isMaximized" :size="14" aria-hidden="true" />
        <Square v-else :size="14" aria-hidden="true" />
      </button>
      <button
        class="inline-flex w-11 items-center justify-center text-slate-300 transition hover:bg-red-500 hover:text-white"
        type="button"
        title="关闭"
        aria-label="关闭"
        @click="close"
      >
        <X :size="16" aria-hidden="true" />
      </button>
    </div>
  </div>
</template>
