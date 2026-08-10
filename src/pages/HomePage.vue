<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import Heatmap from "@/components/Heatmap.vue";
import StatsCards from "@/components/StatsCards.vue";
import { AnimatedNumber } from "@/components/AnimatedNumber";
import { type HeatmapValue } from "@/utils/heatmap";
import { formatCount, formatMoney, formatTime } from "@/utils/formatOctopus";
import { Card, CardContent } from "@/components/ui/card";
import {
  extractInvokeError,
  getRequestDailyCounts,
  getRequestOverview,
  type RequestDailyCounts,
  type RequestOverview,
  type OverviewRow,
} from "../api/tauri";

const overview = ref<RequestOverview | null>(null);
const overviewError = ref("");
const daily = ref<RequestDailyCounts | null>(null);
const dailyError = ref("");

const heatmapData = computed<HeatmapValue[]>(() => {
  const counts = daily.value;
  if (!counts) return [];
  // 后端只返回有记录的日（count>0）；补全 365 天全网格，让热力图不依赖数据的实际时间范围。
  const dayMs = 86_400_000;
  // end_unix 是“今日次日 00:00”的 unix 秒；向前取 365 天（含今日）。
  const endMs = counts.end_unix * 1000;
  const startMs = endMs - 365 * dayMs;
  // 后端 days 由 day_start_unix 升序，借成 Map 供 O(1) 查。
  const byDay = new Map<number, number>();
  for (const d of counts.days) byDay.set(d.day_start_unix, d.count);
  const out: HeatmapValue[] = [];
  for (let t = startMs; t < endMs; t += dayMs) {
    // counts 的 day_start_unix 是 unix 秒，除以 1000 换回去。
    const dayStartUnix = Math.round(t / 1000);
    // 自研 Heatmap 要求 { date, count }；date 传 'YYYY-MM-DD'（本地自然日）。
    const d = new Date(t);
    const yyyy = d.getFullYear();
    const mm = String(d.getMonth() + 1).padStart(2, "0");
    const dd = String(d.getDate()).padStart(2, "0");
    out.push({ date: `${yyyy}-${mm}-${dd}`, count: byDay.get(dayStartUnix) ?? 0 });
  }
  return out;
});

const EMPTY_ROW: OverviewRow = {
  requests: 0,
  input_tokens: 0,
  output_tokens: 0,
  use_time_ms: 0,
  input_cost: 0,
  output_cost: 0,
  cost: 0,
};

/** 今日使用情况（octopus StatsChart 汇总行同款：标签+数值+单位+竖分隔线）。 */
const todayStats = computed(() => {
  const r = overview.value?.today ?? EMPTY_ROW;
  const requests = formatCount(r.requests);
  const duration = formatTime(r.use_time_ms);
  const tokens = formatCount(r.input_tokens + r.output_tokens);
  const cost = formatMoney(r.cost);
  return [
    { label: "请求次数", value: requests.value, unit: requests.unit },
    { label: "消耗时间", value: duration.value, unit: duration.unit },
    { label: "总 Token", value: tokens.value, unit: tokens.unit },
    { label: "总费用", value: cost.value, unit: cost.unit },
  ];
});

async function refreshStats() {
  const overviewPromise = getRequestOverview()
    .then((value) => {
      overview.value = value;
      overviewError.value = "";
    })
    .catch((e) => {
      overviewError.value = extractInvokeError(e);
    });
  const dailyPromise = getRequestDailyCounts()
    .then((value) => {
      daily.value = value;
      dailyError.value = "";
    })
    .catch((e) => {
      dailyError.value = extractInvokeError(e);
    });
  await Promise.all([overviewPromise, dailyPromise]);
}

/** 仅轮询刷新统计总览（5s），避免频繁刷新每日热力图。 */
async function refreshOverviewOnly() {
  try {
    overview.value = await getRequestOverview();
    overviewError.value = "";
  } catch (e) {
    overviewError.value = extractInvokeError(e);
  }
}

let overviewTimer: ReturnType<typeof setInterval> | undefined;
onMounted(() => {
  refreshStats();
  overviewTimer = setInterval(refreshOverviewOnly, 5000);
});
onUnmounted(() => {
  if (overviewTimer !== undefined) clearInterval(overviewTimer);
});

</script>

<template>
  <div class="space-y-6">
    <StatsCards :overview="overview" :error="overviewError" />

    <Card class="border border-slate-200 bg-white">
      <CardContent class="flex flex-col gap-3">
      <Heatmap :values="heatmapData" />

      <!-- 今日使用情况（octopus StatsChart 汇总行同款） -->
      <div class="mt-2 flex flex-wrap items-center gap-x-6 gap-y-3 border-t border-slate-100 pt-4">
        <div class="text-sm font-semibold text-slate-700">今日使用情况</div>
        <template v-for="(item, idx) in todayStats" :key="item.label">
          <div v-if="idx > 0" class="h-8 w-px bg-slate-100" />
          <div>
            <div class="text-xs text-muted-foreground">{{ item.label }}</div>
            <div class="flex items-baseline gap-0.5 text-xl font-semibold">
              <AnimatedNumber :value="item.value" />
              <span class="text-sm font-normal text-muted-foreground">{{ item.unit }}</span>
            </div>
          </div>
        </template>
      </div>

      <p v-if="dailyError" class="mt-3 text-sm text-rose-600">{{ dailyError }}</p>
    </CardContent>
    </Card>
  </div>
</template>
