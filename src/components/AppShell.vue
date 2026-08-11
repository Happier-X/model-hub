<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { X } from "@lucide/vue";
import { useRoute, RouterLink, RouterView } from "vue-router";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
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
  <div class="flex h-screen flex-col overflow-hidden bg-muted text-foreground">
    <AppTitleBar />
    <SidebarProvider class="flex min-h-0 flex-1">
      <div class="flex min-h-0 flex-1 overflow-hidden">
        <Sidebar collapsible="none" class="border-r border-border bg-card">
          <SidebarHeader class="flex flex-col items-start gap-1 px-4 py-3">
            <div class="text-lg font-semibold tracking-wide">Model Hub</div>
            <div class="text-xs text-muted-foreground">Vue3 · 内嵌代理</div>
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
          class="flex min-h-11 items-center gap-3 border-b border-info/20 bg-info/10 px-6 py-2 text-sm text-info"
        >
          <span class="min-w-0 flex-1">发现新版本 {{ availableVersion }}</span>
          <RouterLink class="shrink-0 font-medium text-info hover:text-info" to="/settings">
            前往设置
          </RouterLink>
          <Button
            variant="ghost"
            size="icon"
            aria-label="关闭更新提示"
            title="关闭更新提示"
            @click="availableVersion = ''"
          >
            <X aria-hidden="true" />
          </Button>
        </div>
        <header class="border-b border-border bg-card px-6 py-4">
          <h1 class="text-xl font-semibold">{{ title }}</h1>
        </header>
        <ScrollArea class="min-h-0 flex-1">
          <div class="p-6">
            <RouterView />
          </div>
        </ScrollArea>
      </main>
      </div>
    </SidebarProvider>
  </div>
</template>
