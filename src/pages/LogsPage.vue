<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Badge } from "@/components/ui/badge";
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
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { extractInvokeError, listLogs, type RequestLog } from "../api/tauri";

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
const page = ref(1);
const pageSize = ref(50);
const loading = ref(false);
const error = ref("");

const totalPages = computed(() => Math.max(1, Math.ceil(total.value / pageSize.value) || 1));

function formatTime(unix: number) {
  if (!unix) return "-";
  return new Date(unix * 1000).toLocaleString();
}

async function refresh() {
  // 分页操作重叠时跳过，防止 invoke 重叠及旧响应覆盖新响应。
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
  page.value = p;
  await refresh();
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <div class="h-full flex flex-col overflow-hidden">
    <p v-if="error" class="shrink-0 text-sm text-destructive">{{ error }}</p>

    <Card class="min-h-0 flex-1 flex flex-col">
      <CardHeader class="shrink-0 py-3">
        <p class="text-sm text-muted-foreground">
          共 {{ total }} 条 · 库内 {{ storedTotal }} 条 · 第 {{ page }} / {{ totalPages }} 页
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
                    <span class="block max-w-[200px] break-words text-destructive">{{
                      row.error || "-"
                    }}</span>
                  </template>
                  <template v-else-if="col.key === 'failover'">
                    <div class="max-w-[220px] break-words">
                      <template v-if="row.failover_from || row.failover_to">
                        {{ row.failover_from }} → {{ row.failover_to }}
                        <div class="text-muted-foreground">{{ row.failover_reason }}</div>
                      </template>
                      <template v-else>-</template>
                    </div>
                  </template>
                  <template v-else>{{ (row as Record<string, unknown>)[col.key] }}</template>
                </TableCell>
              </TableRow>
              <TableRow v-if="items.length === 0">
                <TableCell :colspan="logColumns.length" class="py-8 text-center text-muted-foreground">
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
