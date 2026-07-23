<script setup lang="ts">
import { computed } from "vue";
import { useRoute, RouterLink, RouterView } from "vue-router";

const route = useRoute();
const title = computed(() => (route.meta.title as string) || "Model Hub");

const nav = [
  { to: "/", label: "概览" },
  { to: "/providers", label: "供应商" },
  { to: "/groups", label: "分组" },
  { to: "/logs", label: "日志" },
];
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
      <header class="border-b border-slate-200 bg-white px-6 py-4">
        <h1 class="text-xl font-semibold">{{ title }}</h1>
      </header>
      <div class="flex-1 overflow-auto p-6">
        <RouterView />
      </div>
    </main>
  </div>
</template>
