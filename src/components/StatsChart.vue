<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { Card, CardContent } from "@/components/ui/card";
import { AnimatedNumber } from "@/components/AnimatedNumber";
import { extractInvokeError, getTimeseriesStats, type DailyStatRow, type HourlyStatRow } from "@/api/tauri";
import { formatCount, formatMoney } from "@/utils/formatOctopus";
import { buildSmoothPath } from "@/utils/smoothPath";

type Metric = "count" | "cost" | "tokens";
type Period = "today" | "7" | "30";

const METRICS: { key: Metric; label: string }[] = [
  { key: "count", label: "请求数" },
  { key: "cost", label: "费用" },
  { key: "tokens", label: "Token" },
];
const PERIODS: { key: Period; label: string }[] = [
  { key: "today", label: "今日" },
  { key: "7", label: "近7天" },
  { key: "30", label: "近30天" },
];

const daily = ref<DailyStatRow[]>([]);
const hourly = ref<HourlyStatRow[]>([]);
const error = ref("");
const metric = ref<Metric>("count");
const period = ref<Period>("today");
const chartEl = ref<HTMLDivElement | null>(null);
const chartW = ref(0);
const hoverIdx = ref<number | null>(null);
let timer: ReturnType<typeof setInterval> | undefined;
let ro: ResizeObserver | undefined;

const CHART_H = 150;
const PAD = { top: 14, right: 10, bottom: 24, left: 46 };

/** 当前周期选中的序列（label + 值），升序。 */
const series = computed<{ label: string; value: number }[]>(() => {
  const m = metric.value;
  if (period.value === "today") {
    return hourly.value.map((h) => ({
      label: `${h.hour}:00`,
      value: m === "cost" ? h.cost : m === "count" ? h.requests : h.input_tokens + h.output_tokens,
    }));
  }
  const days = Number(period.value);
  return daily.value.slice(-days).map((d) => ({
    label: new Date(d.day_start_unix * 1000).toLocaleDateString("zh-CN", { month: "2-digit", day: "2-digit" }),
    value: m === "cost" ? d.cost : m === "count" ? d.requests : d.input_tokens + d.output_tokens,
  }));
});

/** 汇总行（octopus 同款：总请求 / 总费用 / 总 Token，随周期变化）。 */
const totals = computed(() => {
  if (period.value === "today") {
    const requests = hourly.value.reduce((a, h) => a + h.requests, 0);
    const cost = hourly.value.reduce((a, h) => a + h.cost, 0);
    const tokens = hourly.value.reduce((a, h) => a + h.input_tokens + h.output_tokens, 0);
    return { requests, cost, tokens };
  }
  const rows = daily.value.slice(-Number(period.value));
  const requests = rows.reduce((a, d) => a + d.requests, 0);
  const cost = rows.reduce((a, d) => a + d.cost, 0);
  const tokens = rows.reduce((a, d) => a + d.input_tokens + d.output_tokens, 0);
  return { requests, cost, tokens };
});

const maxValue = computed(() => Math.max(1, ...series.value.map((s) => s.value)));
const valueLabel = (v: number): string => {
  if (metric.value === "cost") {
    const f = formatMoney(v);
    return `${f.value}${f.unit}`;
  }
  const f = formatCount(v);
  return `${f.value}${f.unit}`;
};

/** 面积图几何（随容器宽度自适应）。 */
const geometry = computed(() => {
  const w = Math.max(0, chartW.value - PAD.left - PAD.right);
  const h = CHART_H - PAD.top - PAD.bottom;
  const n = series.value.length;
  const pts = series.value.map((s, i) => {
    const x = n <= 1 ? PAD.left + w / 2 : PAD.left + (i * w) / (n - 1);
    const y = PAD.top + h - (s.value / maxValue.value) * h;
    return { x, y };
  });
  const line = buildSmoothPath(pts);
  const base = pts.length === 0 ? "" : ` L${pts[pts.length - 1].x.toFixed(1)},${(PAD.top + h).toFixed(1)} L${pts[0].x.toFixed(1)},${(PAD.top + h).toFixed(1)} Z`;
  const area = pts.length === 0 ? "" : `${line}${base}`;
  const gridY = [0, 0.25, 0.5, 0.75, 1].map((f) => PAD.top + h - f * h);
  const gridVals = [0, 0.25, 0.5, 0.75, 1].map((f) => maxValue.value * f);
  return { pts, line, area, gridY, gridVals, w, h };
});

const metricColor = computed(() => {
  if (metric.value === "cost") return "var(--chart-1)";
  if (metric.value === "count") return "var(--chart-2)";
  return "var(--chart-3)";
});

function onChartMove(e: MouseEvent) {
  const rect = (e.currentTarget as SVGElement).getBoundingClientRect();
  const relX = e.clientX - rect.left - PAD.left;
  const n = series.value.length;
  if (n === 0) {
    hoverIdx.value = null;
    return;
  }
  const w = Math.max(1, geometry.value.w);
  const idx = Math.round((relX / w) * (n - 1));
  hoverIdx.value = Math.max(0, Math.min(n - 1, idx));
}

async function refresh() {
  try {
    const stats = await getTimeseriesStats();
    daily.value = stats.daily;
    hourly.value = stats.hourly;
    error.value = "";
  } catch (e) {
    error.value = extractInvokeError(e);
  }
}

onMounted(() => {
  refresh();
  timer = setInterval(refresh, 30000);
  if (chartEl.value) {
    chartW.value = chartEl.value.clientWidth;
    ro = new ResizeObserver(() => {
      if (chartEl.value) chartW.value = chartEl.value.clientWidth;
    });
    ro.observe(chartEl.value);
  }
});
onUnmounted(() => {
  if (timer !== undefined) clearInterval(timer);
  ro?.disconnect();
});
</script>

<template>
  <Card class="border border-slate-200 bg-white">
    <CardContent class="flex flex-col gap-4 pt-4">
      <!-- 标题 + 指标切换 -->
      <div class="flex flex-wrap items-center justify-between gap-3">
        <h3 class="text-base font-semibold">使用统计</h3>
        <div class="flex gap-1 rounded-full bg-slate-100 p-1">
          <button
            v-for="m in METRICS"
            :key="m.key"
            type="button"
            class="rounded-full px-3 py-1 text-xs transition-colors"
            :class="metric === m.key ? 'bg-white font-medium shadow-sm' : 'text-muted-foreground hover:text-foreground'"
            @click="metric = m.key"
          >
            {{ m.label }}
          </button>
        </div>
      </div>

      <!-- 汇总行 + 周期切换 -->
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div class="flex items-center gap-4 text-sm">
          <div>
            <div class="text-xs text-muted-foreground">总请求</div>
            <div class="flex items-baseline gap-0.5 text-lg font-semibold">
              <AnimatedNumber :value="formatCount(totals.requests).value" />
              <span class="text-xs font-normal text-muted-foreground">{{ formatCount(totals.requests).unit }}</span>
            </div>
          </div>
          <div class="h-7 w-px bg-slate-100" />
          <div>
            <div class="text-xs text-muted-foreground">总费用</div>
            <div class="flex items-baseline gap-0.5 text-lg font-semibold">
              <AnimatedNumber :value="formatMoney(totals.cost).value" />
              <span class="text-xs font-normal text-muted-foreground">{{ formatMoney(totals.cost).unit }}</span>
            </div>
          </div>
          <div class="h-7 w-px bg-slate-100" />
          <div>
            <div class="text-xs text-muted-foreground">总 Token</div>
            <div class="flex items-baseline gap-0.5 text-lg font-semibold">
              <AnimatedNumber :value="formatCount(totals.tokens).value" />
              <span class="text-xs font-normal text-muted-foreground">{{ formatCount(totals.tokens).unit }}</span>
            </div>
          </div>
        </div>
        <div class="flex gap-1 rounded-full bg-slate-100 p-1">
          <button
            v-for="p in PERIODS"
            :key="p.key"
            type="button"
            class="rounded-full px-3 py-1 text-xs transition-colors"
            :class="period === p.key ? 'bg-white font-medium shadow-sm' : 'text-muted-foreground hover:text-foreground'"
            @click="period = p.key"
          >
            {{ p.label }}
          </button>
        </div>
      </div>

      <p v-if="error" class="text-sm text-rose-600">{{ error }}</p>

      <!-- 面积图 -->
      <div ref="chartEl" class="relative w-full">
        <svg
          v-if="series.length > 0"
          :width="chartW"
          :height="CHART_H"
          class="block"
          @mousemove="onChartMove"
          @mouseleave="hoverIdx = null"
        >
          <defs>
            <linearGradient :id="`grad-${metric}`" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" :stop-color="metricColor" stop-opacity="0.35" />
              <stop offset="95%" :stop-color="metricColor" stop-opacity="0.02" />
            </linearGradient>
          </defs>
          <!-- 网格 + Y 轴标签 -->
          <template v-for="(gy, i) in geometry.gridY" :key="i">
            <line :x1="PAD.left" :x2="chartW - PAD.right" :y1="gy" :y2="gy" stroke="var(--border)" stroke-dasharray="3 3" />
            <text :x="PAD.left - 6" :y="gy + 3" text-anchor="end" class="fill-muted-foreground" font-size="10">
              {{ valueLabel(geometry.gridVals[i]) }}
            </text>
          </template>
          <!-- 面积 + 折线 -->
          <path v-if="geometry.area" :d="geometry.area" :fill="`url(#grad-${metric})`" />
          <path :d="geometry.line" fill="none" :stroke="metricColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round" />
          <!-- hover 十字线 + 圆点 -->
          <template v-if="hoverIdx !== null && geometry.pts[hoverIdx]">
            <line :x1="geometry.pts[hoverIdx].x" :x2="geometry.pts[hoverIdx].x" :y1="PAD.top" :y2="CHART_H - PAD.bottom" stroke="var(--border)" stroke-dasharray="2 2" />
            <circle :cx="geometry.pts[hoverIdx].x" :cy="geometry.pts[hoverIdx].y" r="4" :fill="metricColor" stroke="white" stroke-width="1.5" />
          </template>
          <!-- X 轴标签（首/中/末） -->
          <template v-if="geometry.pts.length > 1">
            <text :x="geometry.pts[0].x" :y="CHART_H - 6" text-anchor="start" class="fill-muted-foreground" font-size="10">{{ series[0].label }}</text>
            <text :x="geometry.pts[Math.floor((geometry.pts.length - 1) / 2)].x" :y="CHART_H - 6" text-anchor="middle" class="fill-muted-foreground" font-size="10">{{ series[Math.floor((series.length - 1) / 2)].label }}</text>
            <text :x="geometry.pts[geometry.pts.length - 1].x" :y="CHART_H - 6" text-anchor="end" class="fill-muted-foreground" font-size="10">{{ series[series.length - 1].label }}</text>
          </template>
        </svg>
        <p v-else class="py-10 text-center text-sm text-muted-foreground">暂无数据</p>

        <!-- tooltip -->
        <div
          v-if="hoverIdx !== null && series[hoverIdx]"
          class="pointer-events-none absolute z-10 -translate-x-1/2 rounded-lg border border-slate-200 bg-white px-2.5 py-1.5 text-xs shadow-sm"
          :style="{ left: `${geometry.pts[hoverIdx]?.x ?? 0}px`, top: `${Math.max(0, (geometry.pts[hoverIdx]?.y ?? 0) - 46)}px` }"
        >
          <div class="font-medium text-slate-700">{{ series[hoverIdx].label }}</div>
          <div class="text-muted-foreground">{{ valueLabel(series[hoverIdx].value) }}</div>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
