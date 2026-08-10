<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import Heatmap from "@/components/Heatmap.vue";
import StatsCards from "@/components/StatsCards.vue";
import StatsChart from "@/components/StatsChart.vue";
import { type HeatmapValue } from "@/utils/heatmap";
import { Card, CardContent } from "@/components/ui/card";
import {
  extractInvokeError,
  getRequestDailyCounts,
  getRequestOverview,
  type RequestDailyCounts,
  type RequestOverview,
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
      <p v-if="dailyError" class="mt-3 text-sm text-rose-600">{{ dailyError }}</p>
    </CardContent>
    </Card>

    <StatsChart />
  </div>
</template>
