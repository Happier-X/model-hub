<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  clearLogs,
  extractInvokeError,
  listLogs,
  purgeExpiredLogs,
  type LogStatusClass,
  type RequestLog,
} from "../api/tauri";
import { statusCodeClass } from "../utils/health";

const items = ref<RequestLog[]>([]);
const total = ref(0);
const storedTotal = ref(0);
const retentionDays = ref(30);
const page = ref(1);
const pageSize = ref(50);
const groupName = ref("");
const statusClass = ref<LogStatusClass>("all");
const failoverOnly = ref(false);
const loading = ref(false);
const error = ref("");
const message = ref("");

const totalPages = computed(() => Math.max(1, Math.ceil(total.value / pageSize.value) || 1));

function formatTime(unix: number) {
  if (!unix) return "-";
  return new Date(unix * 1000).toLocaleString();
}

async function refresh() {
  loading.value = true;
  try {
    const result = await listLogs({
      page: page.value,
      page_size: pageSize.value,
      group_name: groupName.value.trim() || undefined,
      status_class: statusClass.value,
      failover_only: failoverOnly.value,
    });
    items.value = result.items;
    total.value = result.total;
    storedTotal.value = result.stored_total ?? result.total;
    retentionDays.value = result.retention_days ?? 30;
    page.value = result.page;
    pageSize.value = result.page_size;
    error.value = "";
  } catch (e) {
    error.value = extractInvokeError(e);
  } finally {
    loading.value = false;
  }
}

async function applyFilters() {
  page.value = 1;
  await refresh();
}

async function goPage(next: number) {
  const p = Math.min(Math.max(1, next), totalPages.value);
  if (p === page.value && items.value.length > 0) {
    // 仍允许强制刷新
  }
  page.value = p;
  await refresh();
}

async function onPageSizeChange() {
  page.value = 1;
  await refresh();
}

async function clear() {
  if (!confirm("确认清空全部日志？")) return;
  try {
    await clearLogs();
    page.value = 1;
    message.value = "已清空全部日志";
    await refresh();
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

async function purgeExpired() {
  try {
    const result = await purgeExpiredLogs();
    message.value = `已清理 ${result.deleted} 条超过 ${result.retention_days} 天的日志，库内剩余 ${result.retained} 条`;
    page.value = 1;
    await refresh();
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

onMounted(refresh);
</script>

<template>
  <div class="space-y-4">
    <section class="rounded-xl border border-slate-200 bg-white p-4 shadow-sm">
      <div class="flex flex-wrap items-end gap-3">
        <label class="text-sm">
          <span class="mb-1 block text-slate-500">分组名</span>
          <input
            v-model="groupName"
            type="search"
            placeholder="子串匹配"
            class="w-40 rounded-lg border border-slate-300 px-3 py-2 text-sm"
            @keyup.enter="applyFilters"
          />
        </label>
        <label class="text-sm">
          <span class="mb-1 block text-slate-500">状态</span>
          <select v-model="statusClass" class="rounded-lg border border-slate-300 px-3 py-2 text-sm">
            <option value="all">全部</option>
            <option value="2xx">2xx 成功</option>
            <option value="4xx">4xx 客户端</option>
            <option value="5xx">5xx 上游/网关</option>
            <option value="error">错误（≥400 或有 error）</option>
          </select>
        </label>
        <label class="flex items-center gap-2 pb-2 text-sm">
          <input v-model="failoverOnly" type="checkbox" />
          仅故障转移
        </label>
        <label class="text-sm">
          <span class="mb-1 block text-slate-500">每页</span>
          <select
            v-model.number="pageSize"
            class="rounded-lg border border-slate-300 px-3 py-2 text-sm"
            @change="onPageSizeChange"
          >
            <option :value="20">20</option>
            <option :value="50">50</option>
            <option :value="100">100</option>
          </select>
        </label>
        <button
          type="button"
          class="rounded-lg bg-slate-800 px-4 py-2 text-sm text-white hover:bg-slate-700 disabled:opacity-50"
          :disabled="loading"
          @click="applyFilters"
        >
          筛选
        </button>
        <button
          type="button"
          class="rounded-lg border border-slate-300 px-4 py-2 text-sm hover:bg-slate-50 disabled:opacity-50"
          :disabled="loading"
          @click="refresh"
        >
          刷新
        </button>
        <button
          type="button"
          class="rounded-lg border border-amber-300 bg-amber-50 px-4 py-2 text-sm text-amber-900 hover:bg-amber-100"
          :disabled="loading"
          @click="purgeExpired"
        >
          清理过期
        </button>
        <button type="button" class="rounded-lg bg-rose-600 px-4 py-2 text-sm text-white" @click="clear">
          清空全部
        </button>
      </div>
      <p class="mt-3 text-xs text-slate-500">
        默认保留 {{ retentionDays }} 天；打开列表/写入日志时会自动清理更早记录。库内现有
        {{ storedTotal }} 条。
      </p>
    </section>

    <p v-if="message" class="text-sm text-emerald-700">{{ message }}</p>
    <p v-if="error" class="text-sm text-rose-600">{{ error }}</p>

    <section class="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      <div class="mb-3 flex flex-wrap items-center justify-between gap-2 text-sm text-slate-600">
        <span
          >筛选 {{ total }} 条 · 库内 {{ storedTotal }} 条 · 第 {{ page }} / {{ totalPages }} 页</span
        >
        <div class="flex gap-2">
          <button
            type="button"
            class="rounded border border-slate-300 px-3 py-1 text-sm disabled:opacity-40"
            :disabled="loading || page <= 1"
            @click="goPage(page - 1)"
          >
            上一页
          </button>
          <button
            type="button"
            class="rounded border border-slate-300 px-3 py-1 text-sm disabled:opacity-40"
            :disabled="loading || page >= totalPages"
            @click="goPage(page + 1)"
          >
            下一页
          </button>
        </div>
      </div>
      <div v-if="items.length === 0" class="text-sm text-slate-500">
        {{ loading ? "加载中…" : "暂无日志" }}
      </div>
      <div v-else class="overflow-x-auto">
        <table class="min-w-full text-left text-xs">
          <thead class="border-b text-slate-500">
            <tr>
              <th class="px-2 py-2">时间</th>
              <th class="px-2 py-2">分组</th>
              <th class="px-2 py-2">供应商</th>
              <th class="px-2 py-2">上游模型</th>
              <th class="px-2 py-2">状态</th>
              <th class="px-2 py-2">耗时(ms)</th>
              <th class="px-2 py-2">错误</th>
              <th class="px-2 py-2">故障转移</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="log in items" :key="log.id" class="border-b border-slate-100 align-top">
              <td class="px-2 py-2 whitespace-nowrap">{{ formatTime(log.time) }}</td>
              <td class="px-2 py-2">{{ log.group_name }}</td>
              <td class="px-2 py-2">{{ log.provider_name }}</td>
              <td class="px-2 py-2 font-mono">{{ log.upstream_model }}</td>
              <td class="px-2 py-2">
                <span
                  class="inline-flex rounded-full px-2 py-0.5 text-xs font-medium tabular-nums"
                  :class="statusCodeClass(log.status_code)"
                >
                  {{ log.status_code || "-" }}
                </span>
              </td>
              <td class="px-2 py-2">{{ log.use_time_ms }}</td>
              <td class="max-w-[200px] px-2 py-2 break-words text-rose-600">{{ log.error || "-" }}</td>
              <td class="max-w-[220px] px-2 py-2 break-words">
                <template v-if="log.failover_from || log.failover_to">
                  {{ log.failover_from }} → {{ log.failover_to }}
                  <div class="text-slate-500">{{ log.failover_reason }}</div>
                </template>
                <template v-else>-</template>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  </div>
</template>
