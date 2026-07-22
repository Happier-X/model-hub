<script setup lang="ts">
import { onMounted, ref } from "vue";
import { clearLogs, extractInvokeError, listLogs, type RequestLog } from "../api/tauri";
import { statusCodeClass } from "../utils/health";

const items = ref<RequestLog[]>([]);
const error = ref("");

function formatTime(unix: number) {
  if (!unix) return "-";
  return new Date(unix * 1000).toLocaleString();
}

async function refresh() {
  try {
    items.value = await listLogs(1, 100);
    error.value = "";
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

async function clear() {
  if (!confirm("确认清空全部日志？")) return;
  try {
    await clearLogs();
    await refresh();
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

onMounted(refresh);
</script>

<template>
  <div class="space-y-4">
    <div class="flex gap-2">
      <button type="button" class="rounded-lg border border-slate-300 px-4 py-2 text-sm" @click="refresh">
        刷新
      </button>
      <button type="button" class="rounded-lg bg-rose-600 px-4 py-2 text-sm text-white" @click="clear">
        清空
      </button>
    </div>
    <p v-if="error" class="text-sm text-rose-600">{{ error }}</p>
    <section class="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      <div v-if="items.length === 0" class="text-sm text-slate-500">暂无日志</div>
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
