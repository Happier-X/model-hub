<script setup lang="ts">
import { computed, onMounted, ref, type Component } from "vue";
import {
  Activity,
  ArrowDownToLine,
  ArrowUpFromLine,
  Bot,
  ChartColumnBig,
  Clock,
  DollarSign,
  FastForward,
  MessageSquare,
  Rewind,
} from "@lucide/vue";
import { AnimatedNumber } from "@/components/AnimatedNumber";
import { formatCount, formatMoney, formatTime } from "@/utils/formatOctopus";
import type { OverviewRow, RequestOverview } from "@/api/tauri";

const props = defineProps<{
  overview: RequestOverview | null;
  error: string;
}>();

const entered = ref(false);
onMounted(() => {
  entered.value = true;
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

const displayRow = computed<OverviewRow>(() => props.overview?.total ?? EMPTY_ROW);

interface MetricItem {
  label: string;
  icon: Component;
  color: string;
  bgColor: string;
  /** 已格式化的数值部分（octopus 风格，如 "12.35"）。 */
  value: string;
  /** 单位（K/M/B、$、s/m/h/d 等），空则不渲染。 */
  unit: string;
}

interface StatsCard {
  title: string;
  headerIcon: Component;
  items: MetricItem[];
}

const cards = computed<StatsCard[]>(() => {
  const r = displayRow.value;
  const requests = formatCount(r.requests);
  const duration = formatTime(r.use_time_ms);
  const totalToken = formatCount(r.input_tokens + r.output_tokens);
  const inputToken = formatCount(r.input_tokens);
  const outputToken = formatCount(r.output_tokens);
  const totalCost = formatMoney(r.cost);
  const inputCost = formatMoney(r.input_cost);
  const outputCost = formatMoney(r.output_cost);
  return [
    {
      title: "请求统计",
      headerIcon: Activity,
      items: [
        {
          label: "请求次数",
          icon: MessageSquare,
          color: "text-primary",
          bgColor: "bg-primary/10",
          value: requests.value,
          unit: requests.unit,
        },
        {
          label: "消耗时间",
          icon: Clock,
          color: "text-primary",
          bgColor: "bg-primary/10",
          value: duration.value,
          unit: duration.unit,
        },
      ],
    },
    {
      title: "全部统计",
      headerIcon: ChartColumnBig,
      items: [
        {
          label: "总 Token",
          icon: Bot,
          color: "text-primary",
          bgColor: "bg-primary/10",
          value: totalToken.value,
          unit: totalToken.unit,
        },
        {
          label: "总费用",
          icon: DollarSign,
          color: "text-primary",
          bgColor: "bg-primary/10",
          value: totalCost.value,
          unit: totalCost.unit,
        },
      ],
    },
    {
      title: "输入统计",
      headerIcon: ArrowDownToLine,
      items: [
        {
          label: "输入 Tokens",
          icon: Rewind,
          color: "text-primary",
          bgColor: "bg-primary/10",
          value: inputToken.value,
          unit: inputToken.unit,
        },
        {
          label: "输入费用",
          icon: DollarSign,
          color: "text-primary",
          bgColor: "bg-primary/10",
          value: inputCost.value,
          unit: inputCost.unit,
        },
      ],
    },
    {
      title: "输出统计",
      headerIcon: ArrowUpFromLine,
      items: [
        {
          label: "输出 Tokens",
          icon: FastForward,
          color: "text-primary",
          bgColor: "bg-primary/10",
          value: outputToken.value,
          unit: outputToken.unit,
        },
        {
          label: "输出费用",
          icon: DollarSign,
          color: "text-primary",
          bgColor: "bg-primary/10",
          value: outputCost.value,
          unit: outputCost.unit,
        },
      ],
    },
  ];
});
</script>

<template>
  <div>
    <!-- octopus 风格四统计卡片（总计，实时轮询刷新） -->
    <div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-4">
      <section
        v-for="(card, index) in cards"
        :key="card.title"
        class="flex flex-row items-center gap-4 rounded-2xl border border-border bg-card p-5 text-card-foreground transition-all duration-500"
        :style="{
          transitionDelay: `${index * 80}ms`,
          opacity: entered ? 1 : 0,
          transform: entered ? 'none' : 'translateY(16px)',
        }"
      >
        <!-- 左区：竖排标题 + 头部图标 -->
        <div class="flex flex-col items-center justify-center gap-3 self-stretch border-r border-border/50 py-1 pr-4">
          <component :is="card.headerIcon" class="h-4 w-4" />
          <h3 class="text-sm font-medium [writing-mode:vertical-lr]">{{ card.title }}</h3>
        </div>
        <!-- 右区：两个指标 -->
        <div class="flex min-w-0 flex-1 flex-col gap-4">
          <div v-for="item in card.items" :key="item.label" class="flex items-center gap-3">
            <div
              class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl"
              :class="`${item.bgColor} ${item.color}`"
            >
              <component :is="item.icon" class="h-5 w-5" />
            </div>
            <div class="flex min-w-0 flex-col">
              <span class="text-xs text-muted-foreground">{{ item.label }}</span>
              <div class="flex items-baseline gap-1">
                <span class="text-xl font-semibold">
                  <AnimatedNumber :value="item.value" />
                </span>
                <span v-if="item.unit" class="text-sm text-muted-foreground">{{ item.unit }}</span>
              </div>
            </div>
          </div>
        </div>
      </section>
    </div>

    <p v-if="error" class="mt-3 text-sm text-destructive">{{ error }}</p>
  </div>
</template>
