<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { VisArea, VisAxis, VisLine, VisXYContainer } from "@unovis/vue";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  ChartContainer,
  ChartCrosshair,
  ChartTooltip,
  type ChartConfig,
} from "@/components/ui/chart";
import { Separator } from "@/components/ui/separator";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { AnimatedNumber } from "@/components/AnimatedNumber";
import { extractInvokeError, getTimeseriesStats, type DailyStatRow, type HourlyStatRow } from "@/api/tauri";
import { formatCount, formatMoney } from "@/utils/formatOctopus";

type Metric = "count" | "cost" | "tokens";
type Period = "today" | "7" | "30";
/** 图表数据点：x 用序号，y 用当前指标值，label 供轴与 tooltip 显示。 */
type Point = { index: number; label: string; value: number };

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
let timer: ReturnType<typeof setInterval> | undefined;

/** 图表边距：Unovis 不自动为轴标签预留空间，需手动留白（左=Y 轴数值，下=X 轴时间）。 */
const MARGIN = { top: 14, right: 12, bottom: 20, left: 40 };

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

const valueLabel = (v: number): string => {
  const f = metric.value === "cost" ? formatMoney(v) : formatCount(v);
  return `${f.value}${f.unit}`;
};

const points = computed<Point[]>(() =>
  series.value.map((s, i) => ({ index: i, label: s.label, value: s.value })),
);

/** ChartConfig 驱动 --color-value CSS 变量与图例文案。 */
const chartConfig = computed<ChartConfig>(() => ({
  value: {
    label: METRICS.find((m) => m.key === metric.value)?.label ?? "数值",
    color: metric.value === "cost" ? "var(--chart-1)" : metric.value === "count" ? "var(--chart-2)" : "var(--chart-3)",
  },
}));

const xAccessor = (d: Point) => d.index;
const yAccessor = (d: Point) => d.value;
const xTickFormat = (tick: number) => series.value[Math.round(tick)]?.label ?? "";
const yTickFormat = (tick: number) => valueLabel(tick);
/** Crosshair 提示内容（Unovis template 返回 HTML 字符串）。 */
const tooltipTemplate = (d: Point) =>
  `<div class="rounded-lg border bg-background px-2.5 py-1.5 text-xs shadow-sm">` +
  `<div class="font-medium text-foreground">${d.label}</div>` +
  `<div class="text-muted-foreground">${valueLabel(d.value)}</div></div>`;

function onMetricChange(v: unknown) {
  if (typeof v === "string" && METRICS.some((m) => m.key === v)) metric.value = v as Metric;
}
function onPeriodChange(v: unknown) {
  if (typeof v === "string" && PERIODS.some((p) => p.key === v)) period.value = v as Period;
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
});
onUnmounted(() => {
  if (timer !== undefined) clearInterval(timer);
});
</script>

<template>
  <Card>
    <CardHeader class="flex flex-wrap items-center justify-between gap-3">
      <CardTitle class="text-base">使用统计</CardTitle>
      <ToggleGroup
        type="single"
        variant="outline"
        size="sm"
        :model-value="metric"
        aria-label="切换统计指标"
        @update:model-value="onMetricChange"
      >
        <ToggleGroupItem v-for="m in METRICS" :key="m.key" :value="m.key">
          {{ m.label }}
        </ToggleGroupItem>
      </ToggleGroup>
    </CardHeader>

    <CardContent class="flex flex-col gap-4">
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
          <Separator orientation="vertical" class="h-7" />
          <div>
            <div class="text-xs text-muted-foreground">总费用</div>
            <div class="flex items-baseline gap-0.5 text-lg font-semibold">
              <AnimatedNumber :value="formatMoney(totals.cost).value" />
              <span class="text-xs font-normal text-muted-foreground">{{ formatMoney(totals.cost).unit }}</span>
            </div>
          </div>
          <Separator orientation="vertical" class="h-7" />
          <div>
            <div class="text-xs text-muted-foreground">总 Token</div>
            <div class="flex items-baseline gap-0.5 text-lg font-semibold">
              <AnimatedNumber :value="formatCount(totals.tokens).value" />
              <span class="text-xs font-normal text-muted-foreground">{{ formatCount(totals.tokens).unit }}</span>
            </div>
          </div>
        </div>
        <ToggleGroup
          type="single"
          variant="outline"
          size="sm"
          :model-value="period"
          aria-label="切换统计周期"
          @update:model-value="onPeriodChange"
        >
          <ToggleGroupItem v-for="p in PERIODS" :key="p.key" :value="p.key">
            {{ p.label }}
          </ToggleGroupItem>
        </ToggleGroup>
      </div>

      <p v-if="error" class="text-sm text-destructive">{{ error }}</p>

      <!-- 面积 + 平滑折线（Unovis 默认 curveType=monotoneX 即平滑曲线） -->
      <ChartContainer v-if="points.length > 0" :config="chartConfig" cursor class="h-[150px] w-full">
        <VisXYContainer :data="points" :margin="MARGIN">
          <VisArea :x="xAccessor" :y="yAccessor" color="var(--color-value)" :opacity="0.2" />
          <VisLine :x="xAccessor" :y="yAccessor" color="var(--color-value)" :line-width="2" />
          <VisAxis
            type="x"
            :num-ticks="3"
            :tick-format="xTickFormat"
            :grid-line="false"
            :tick-line="false"
            :domain-line="false"
          />
          <VisAxis
            type="y"
            :num-ticks="5"
            :tick-format="yTickFormat"
            :tick-line="false"
            :domain-line="false"
          />
          <ChartCrosshair :x="xAccessor" :y="yAccessor" :template="tooltipTemplate" color="var(--color-value)" />
          <ChartTooltip />
        </VisXYContainer>
      </ChartContainer>
      <p v-else class="py-10 text-center text-sm text-muted-foreground">暂无数据</p>
    </CardContent>
  </Card>
</template>
