<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { X } from "@lucide/vue";
import { useRoute, RouterLink, RouterView } from "vue-router";
import { checkForUpdate, getShellPrefs } from "../api/tauri";

const route = useRoute();
const title = computed(() => (route.meta.title as string) || "Model Hub");
const availableVersion = ref("");

const nav = [
  { to: "/", label: "首页" },
  { to: "/providers", label: "供应商" },
  { to: "/groups", label: "分组" },
  { to: "/logs", label: "日志" },
  { to: "/settings", label: "设置" },
];

async function checkUpdateOnAppStartup() {
  try {
    const prefs = await getShellPrefs();
    if (!prefs.check_update_on_startup) return;
    const update = await checkForUpdate();
    if (!update) return;
    availableVersion.value = update.version;
    try {
      await update.close();
    } catch {
      /* 启动检查只保留版本号，资源关闭失败不阻塞应用 */
    }
  } catch {
    /* 启动检查失败保持静默，不阻塞应用渲染 */
  }
}

onMounted(checkUpdateOnAppStartup);
</script>

<template>
  <div class="flex min-h-screen bg-slate-100 text-slate-900">
    <aside class="flex w-56 shrink-0 flex-col border-r border-slate-200 bg-slate-900 text-slate-100">
      <div class="border-b border-slate-700 px-5 py-4">
        <div class="text-lg font-semibold tracking-wide">Model Hub</div>
        <div class="mt-1 text-xs text-slate-400">Vue3 · 内嵌代理</div>
      </div>
      <nav class="flex flex-1 flex-col gap-1 p-3">
        <RouterLink
          v-for="item in nav"
          :key="item.to"
          :to="item.to"
          class="rounded-lg px-3 py-2 text-sm transition"
          :class="
            route.path === item.to
              ? 'bg-cyan-500/20 text-cyan-200'
              : 'text-slate-300 hover:bg-slate-800 hover:text-white'
          "
        >
          {{ item.label }}
        </RouterLink>
      </nav>
    </aside>
    <main class="flex min-w-0 flex-1 flex-col">
      <div
        v-if="availableVersion"
        class="flex min-h-11 items-center gap-3 border-b border-cyan-200 bg-cyan-50 px-6 py-2 text-sm text-cyan-950"
      >
        <span class="min-w-0 flex-1">发现新版本 {{ availableVersion }}</span>
        <RouterLink class="shrink-0 font-medium text-cyan-800 hover:text-cyan-950" to="/settings">
          前往设置
        </RouterLink>
        <button
          class="inline-flex size-7 shrink-0 items-center justify-center rounded text-cyan-700 hover:bg-cyan-100 hover:text-cyan-950"
          type="button"
          title="关闭更新提示"
          aria-label="关闭更新提示"
          @click="availableVersion = ''"
        >
          <X :size="16" aria-hidden="true" />
        </button>
      </div>
      <header class="border-b border-slate-200 bg-white px-6 py-4">
        <h1 class="text-xl font-semibold">{{ title }}</h1>
      </header>
      <div class="flex-1 overflow-auto p-6">
        <RouterView />
      </div>
    </main>
  </div>
</template>
