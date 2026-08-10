<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import {
  Pagination,
  PaginationContent,
  PaginationEllipsis,
  PaginationFirst,
  PaginationItem,
  PaginationLast,
  PaginationNext,
  PaginationPrevious,
} from "@/components/ui/pagination";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  clearLogs,
  extractInvokeError,
  listLogs,
  purgeExpiredLogs,
  type RequestLog,
} from "../api/tauri";

const pageSizeOptions: { value: number; label: string }[] = [
  { value: 20, label: "20" },
  { value: 50, label: "50" },
  { value: 100, label: "100" },
];

function statusCodeVariant(
  code: number | null | undefined,
): "default" | "success" | "warning" | "danger" {
  if (!code) return "default";
  if (code >= 200 && code < 300) return "success";
  if (code >= 400 && code < 500) return "warning";
  if (code >= 500) return "danger";
  return "default";
}

function statusCodeBadgeVariant(
  code: number | null | undefined,
): "default" | "secondary" | "outline" | "destructive" {
  const v = statusCodeVariant(code);
  if (v === "success") return "secondary";
  if (v === "danger") return "destructive";
  if (v === "warning") return "outline";
  return "default";
}

const logColumns: { key: string; title: string }[] = [
  { key: "time", title: "时间" },
  { key: "group_name", title: "分组" },
  { key: "provider_name", title: "供应商" },
  { key: "upstream_model", title: "上游模型" },
  { key: "status_code", title: "状态" },
  { key: "use_time_ms", title: "耗时(ms)" },
  { key: "error", title: "错误" },
  { key: "failover", title: "故障转移" },
];

const items = ref<RequestLog[]>([]);
const total = ref(0);
const storedTotal = ref(0);
const retentionDays = ref(7);
const maxRows = ref(10000);
const page = ref(1);
const pageSize = ref(50);
const loading = ref(false);
const error = ref("");
const message = ref("");
const autoRefresh = ref(true);
const AUTO_REFRESH_MS = 3_000;
let refreshTimer: ReturnType<typeof setInterval> | undefined;

const totalPages = computed(() => Math.max(1, Math.ceil(total.value / pageSize.value) || 1));

function formatTime(unix: number) {
  if (!unix) return "-";
  return new Date(unix * 1000).toLocaleString();
}

async function refresh() {
  // 定时器与手动操作撞车时跳过，防止 invoke 重叠及旧响应覆盖新响应。
  if (loading.value) return;
  loading.value = true;
  try {
    const result = await listLogs({
      page: page.value,
      page_size: pageSize.value,
    });
    items.value = result.items;
    total.value = result.total;
    storedTotal.value = result.stored_total ?? result.total;
    retentionDays.value = result.retention_days ?? 7;
    maxRows.value = result.max_rows ?? 10000;
    page.value = result.page;
    pageSize.value = result.page_size;
    error.value = "";
  } catch (e) {
    error.value = extractInvokeError(e);
  } finally {
    loading.value = false;
  }
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
    message.value = `已按最近 ${result.retention_days} 天、最新 ${result.max_rows} 条策略清理 ${result.deleted} 条日志，库内剩余 ${result.retained} 条`;
    page.value = 1;
    await refresh();
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

function startAutoRefresh() {
  if (refreshTimer) clearInterval(refreshTimer);
  refreshTimer = setInterval(() => {
    if (autoRefresh.value && !document.hidden) void refresh();
  }, AUTO_REFRESH_MS);
}

function toggleAutoRefresh() {
  autoRefresh.value = !autoRefresh.value;
  message.value = autoRefresh.value ? "已恢复每 3 秒自动刷新" : "已暂停自动刷新";
  if (autoRefresh.value) void refresh();
}

onMounted(async () => {
  await refresh();
  startAutoRefresh();
});

onUnmounted(() => {
  if (refreshTimer) clearInterval(refreshTimer);
  refreshTimer = undefined;
});
</script>

<template>
  <div class="h-full flex flex-col overflow-hidden">
    <!-- 操作卡片：顶部不滚 -->
    <Card class="border border-slate-200 bg-white">
      <CardContent class="flex flex-col gap-3 py-4">
        <div class="flex flex-wrap items-end gap-3">
          <div class="w-24">
            <label class="block text-sm">
              <span class="mb-1 block text-slate-600">每页</span>
              <Select
                :model-value="String(pageSize)"
                @update:model-value="
                  (v) => {
                    pageSize = Number(v);
                    void onPageSizeChange();
                  }
                "
              >
                <SelectTrigger class="w-24">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem v-for="opt in pageSizeOptions" :key="opt.value" :value="String(opt.value)">
                    {{ opt.label }}
                  </SelectItem>
                </SelectContent>
              </Select>
            </label>
          </div>
          <Button variant="outline" type="button" :disabled="loading" @click="refresh">
            刷新
          </Button>
          <Button
            :variant="autoRefresh ? 'secondary' : 'outline'"
            type="button"
            @click="toggleAutoRefresh"
          >
            {{ autoRefresh ? "自动刷新：3 秒" : "自动刷新：已暂停" }}
          </Button>
          <Button variant="secondary" type="button" :disabled="loading" @click="purgeExpired">
            清理过期
          </Button>
          <Button variant="destructive" type="button" @click="clear">清空全部</Button>
        </div>
        <p class="text-xs text-slate-500">
          默认仅保留最近 {{ retentionDays }} 天内的最新 {{ maxRows }} 条；打开列表/写入日志时会自动清理。库内现有
          {{ storedTotal }} 条。
        </p>
      </CardContent>
    </Card>

    <p v-if="message" class="shrink-0 text-sm text-emerald-700">{{ message }}</p>
    <p v-if="error" class="shrink-0 text-sm text-rose-600">{{ error }}</p>

    <Card class="min-h-0 flex-1 flex flex-col border border-slate-200 bg-white">
      <CardHeader class="shrink-0 py-3">
        <p class="text-sm text-slate-600">
          筛选 {{ total }} 条 · 库内 {{ storedTotal }} 条 · 第 {{ page }} / {{ totalPages }} 页
        </p>
      </CardHeader>
      <CardContent class="flex min-h-0 flex-1 flex-col gap-3">
        <!-- 表格区：flex-1 min-h-0 overflow-y-auto 仅表格 body 滚动 -->
        <div class="min-h-0 flex-1 overflow-y-auto">
          <Table class="text-xs">
            <TableHeader>
              <TableRow>
                <TableHead v-for="col in logColumns" :key="col.key">{{ col.title }}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              <TableRow v-for="row in items" :key="row.id">
                <TableCell v-for="col in logColumns" :key="col.key">
                  <template v-if="col.key === 'time'">
                    <span class="whitespace-nowrap">{{ formatTime(row.time) }}</span>
                  </template>
                  <template v-else-if="col.key === 'upstream_model'">
                    <span class="font-mono">{{ row.upstream_model }}</span>
                  </template>
                  <template v-else-if="col.key === 'status_code'">
                    <Badge :variant="statusCodeBadgeVariant(row.status_code)">
                      {{ row.status_code || "-" }}
                    </Badge>
                  </template>
                  <template v-else-if="col.key === 'error'">
                    <span class="block max-w-[200px] break-words text-rose-600">{{
                      row.error || "-"
                    }}</span>
                  </template>
                  <template v-else-if="col.key === 'failover'">
                    <div class="max-w-[220px] break-words">
                      <template v-if="row.failover_from || row.failover_to">
                        {{ row.failover_from }} → {{ row.failover_to }}
                        <div class="text-slate-500">{{ row.failover_reason }}</div>
                      </template>
                      <template v-else>-</template>
                    </div>
                  </template>
                  <template v-else>{{ (row as Record<string, unknown>)[col.key] }}</template>
                </TableCell>
              </TableRow>
              <TableRow v-if="items.length === 0">
                <TableCell :colspan="logColumns.length" class="py-8 text-center text-slate-400">
                  {{ loading ? "加载中…" : "暂无日志" }}
                </TableCell>
              </TableRow>
            </TableBody>
          </Table>
        </div>
        <!-- 分页器：表格滚动区之外，不随表格滚动 -->
        <div class="flex shrink-0 justify-end">
          <Pagination
            :page="page"
            :total="total"
            :page-size="pageSize"
            :disabled="loading"
            @update:page="goPage"
          >
            <PaginationContent v-slot="{ items: pageItems }" class="gap-0.5">
              <PaginationFirst class="hidden sm:inline-flex" @click="goPage(1)" />
              <PaginationPrevious @click="goPage(page - 1)" />
              <template v-for="item in pageItems" :key="item.type + item.value">
                <PaginationItem
                  v-if="item.type === 'page'"
                  :value="item.value"
                  :is-active="item.value === page"
                  @click="goPage(item.value)"
                >
                  {{ item.value }}
                </PaginationItem>
                <PaginationEllipsis v-else-if="item.type === 'ellipsis'" />
              </template>
              <PaginationNext @click="goPage(page + 1)" />
              <PaginationLast class="hidden sm:inline-flex" @click="goPage(totalPages)" />
            </PaginationContent>
          </Pagination>
        </div>
      </CardContent>
    </Card>
  </div>
</template>

