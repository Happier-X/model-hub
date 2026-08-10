<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { X } from "@lucide/vue";
import { useRoute, useRouter, RouterLink, RouterView } from "vue-router";
import { Button } from "@/components/ui/button";
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
} from "@/components/ui/sidebar";
import { checkForUpdate, getShellPrefs } from "../api/tauri";
import AppTitleBar from "./AppTitleBar.vue";

const route = useRoute();
const router = useRouter();
const title = computed(() => (route.meta.title as string) || "Model Hub");
const availableVersion = ref("");

/** 分组子路径（新建/编辑）仍高亮「分组」侧栏项。 */
const activeNavKey = computed(() => {
  const path = route.path;
  if (path === "/groups" || path.startsWith("/groups/")) return "/groups";
  return path;
});

const navItems: { key: string; label: string }[] = [
  { key: "/", label: "首页" },
  { key: "/providers", label: "供应商" },
  { key: "/groups", label: "分组" },
  { key: "/logs", label: "日志" },
  { key: "/settings", label: "设置" },
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
  <div class="flex h-screen flex-col overflow-hidden bg-slate-100 text-slate-900">
    <AppTitleBar />
    <SidebarProvider class="flex min-h-0 flex-1">
      <div class="flex min-h-0 flex-1 overflow-hidden">
        <Sidebar collapsible="none" class="border-r border-slate-200 bg-white">
          <SidebarHeader class="flex flex-col items-start gap-1 px-4 py-3">
            <div class="text-lg font-semibold tracking-wide">Model Hub</div>
            <div class="text-xs text-slate-400">Vue3 · 内嵌代理</div>
          </SidebarHeader>
          <SidebarContent>
            <SidebarGroup>
              <SidebarMenu>
                <SidebarMenuItem v-for="item in navItems" :key="item.key">
                  <SidebarMenuButton
                    as-child
                    :is-active="activeNavKey === item.key"
                  >
                    <RouterLink :to="item.key">{{ item.label }}</RouterLink>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </SidebarMenu>
            </SidebarGroup>
          </SidebarContent>
        </Sidebar>
        <main class="flex min-w-0 flex-1 flex-col">
        <div
          v-if="availableVersion"
          class="flex min-h-11 items-center gap-3 border-b border-cyan-200 bg-cyan-50 px-6 py-2 text-sm text-cyan-950"
        >
          <span class="min-w-0 flex-1">发现新版本 {{ availableVersion }}</span>
          <RouterLink class="shrink-0 font-medium text-cyan-800 hover:text-cyan-950" to="/settings">
            前往设置
          </RouterLink>
          <Button
            variant="ghost"
            size="icon"
            aria-label="关闭更新提示"
            title="关闭更新提示"
            @click="availableVersion = ''"
          >
            <X :size="16" aria-hidden="true" />
          </Button>
        </div>
        <header class="border-b border-slate-200 bg-white px-6 py-4">
          <h1 class="text-xl font-semibold">{{ title }}</h1>
        </header>
        <div class="min-h-0 flex-1 overflow-auto p-6">
          <RouterView />
        </div>
      </main>
      </div>
    </SidebarProvider>
  </div>
</template>
