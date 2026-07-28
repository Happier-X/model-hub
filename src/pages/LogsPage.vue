<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import {
  HBadge,
  HButton,
  HCard,
  HCheckbox,
  HInput,
  HPagination,
  HSelect,
  HTable,
  type HSelectOption,
  type HTableColumn,
} from "happier-ui";
import {
  clearLogs,
  extractInvokeError,
  listLogs,
  purgeExpiredLogs,
  type LogStatusClass,
  type RequestLog,
} from "../api/tauri";

const statusOptions: HSelectOption[] = [
  { value: "all", label: "全部" },
  { value: "2xx", label: "2xx 成功" },
  { value: "4xx", label: "4xx 客户端" },
  { value: "5xx", label: "5xx 上游/网关" },
  { value: "error", label: "错误（≥400 或有 error）" },
];

const pageSizeOptions: HSelectOption[] = [
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

const logColumns: HTableColumn[] = [
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
const maxRows = ref(1000);
const page = ref(1);
const pageSize = ref(50);
const groupName = ref("");
const statusClass = ref<LogStatusClass>("all");
const failoverOnly = ref(false);
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
      group_name: groupName.value.trim() || undefined,
      status_class: statusClass.value,
      failover_only: failoverOnly.value,
    });
    items.value = result.items;
    total.value = result.total;
    storedTotal.value = result.stored_total ?? result.total;
    retentionDays.value = result.retention_days ?? 7;
    maxRows.value = result.max_rows ?? 1000;
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
    <!-- 筛选卡片：顶部不滚 -->
    <HCard variant="outlined" padding="md">
      <div class="flex flex-wrap items-end gap-3">
        <div class="w-40" @keydown.enter="applyFilters">
          <HInput
            v-model="groupName"
            type="search"
            label="分组名"
            placeholder="子串匹配"
          />
        </div>
        <div class="w-44">
          <HSelect
            label="状态"
            :options="statusOptions"
            :model-value="statusClass"
            @update:model-value="(v) => (statusClass = v as LogStatusClass)"
          />
        </div>
        <HCheckbox v-model="failoverOnly" label="仅故障转移" />
        <div class="w-24">
          <HSelect
            label="每页"
            :options="pageSizeOptions"
            :model-value="pageSize"
            @update:model-value="
              (v) => {
                pageSize = Number(v);
                void onPageSizeChange();
              }
            "
          />
        </div>
        <HButton variant="primary" type="button" :disabled="loading" @click="applyFilters">
          筛选
        </HButton>
        <HButton variant="outline" type="button" :disabled="loading" @click="refresh">
          刷新
        </HButton>
        <HButton
          :variant="autoRefresh ? 'secondary' : 'outline'"
          type="button"
          @click="toggleAutoRefresh"
        >
          {{ autoRefresh ? "自动刷新：3 秒" : "自动刷新：已暂停" }}
        </HButton>
        <HButton variant="tertiary" type="button" :disabled="loading" @click="purgeExpired">
          清理过期
        </HButton>
        <HButton variant="danger" type="button" @click="clear">清空全部</HButton>
      </div>
      <p class="mt-3 text-xs text-slate-500">
        默认仅保留最近 {{ retentionDays }} 天内的最新 {{ maxRows }} 条；打开列表/写入日志时会自动清理。库内现有
        {{ storedTotal }} 条。
      </p>
    </HCard>

    <p v-if="message" class="shrink-0 text-sm text-emerald-700">{{ message }}</p>
    <p v-if="error" class="shrink-0 text-sm text-rose-600">{{ error }}</p>

    <HCard variant="outlined" padding="md" class="min-h-0 flex-1 flex flex-col">
      <p class="mb-3 shrink-0 text-sm text-slate-600">
        筛选 {{ total }} 条 · 库内 {{ storedTotal }} 条 · 第 {{ page }} / {{ totalPages }} 页
      </p>
      <!-- 表格区：flex-1 min-h-0 overflow-y-auto 仅表格 body 滚动 -->
      <div class="min-h-0 flex-1 overflow-y-auto">
        <!-- HTable data 只接受 Record<string, unknown>[]，interface 无索引签名需双重断言；等 happier-ui#9 泛型化后简化 -->
        <HTable
          :columns="logColumns"
          :data="items as unknown as Record<string, unknown>[]"
          row-key="id"
          :loading="loading"
          :sticky-header="true"
          empty-text="暂无日志"
          class="text-xs"
        >
        <template #cell="{ column, row }">
          <template v-if="column.key === 'time'">
            <span class="whitespace-nowrap">{{ formatTime((row as RequestLog).time) }}</span>
          </template>
          <template v-else-if="column.key === 'upstream_model'">
            <span class="font-mono">{{ (row as RequestLog).upstream_model }}</span>
          </template>
          <template v-else-if="column.key === 'status_code'">
            <HBadge :variant="statusCodeVariant((row as RequestLog).status_code)">
              {{ (row as RequestLog).status_code || "-" }}
            </HBadge>
          </template>
          <template v-else-if="column.key === 'error'">
            <span class="block max-w-[200px] break-words text-rose-600">{{
              (row as RequestLog).error || "-"
            }}</span>
          </template>
          <template v-else-if="column.key === 'failover'">
            <div class="max-w-[220px] break-words">
              <template v-if="(row as RequestLog).failover_from || (row as RequestLog).failover_to">
                {{ (row as RequestLog).failover_from }} → {{ (row as RequestLog).failover_to }}
                <div class="text-slate-500">{{ (row as RequestLog).failover_reason }}</div>
              </template>
              <template v-else>-</template>
            </div>
          </template>
          <template v-else>{{ (row as RequestLog)[column.key as keyof RequestLog] }}</template>
        </template>
        </HTable>
      </div>
      <!-- 分页器：表格滚动区之外，不随表格滚动 -->
      <div class="mt-3 flex justify-end shrink-0">
        <HPagination
          :current="page"
          :total="total"
          :page-size="pageSize"
          :disabled="loading"
          @change="({ current }) => goPage(current)"
        />
      </div>
    </HCard>
  </div>
</template>

<style scoped>
:deep(.h-card) {
  display: flex;
  flex-direction: column;
}

:deep(.h-card__body) {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
</style>
