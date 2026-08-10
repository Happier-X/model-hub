<script setup lang="ts">
import { computed } from "vue";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { colorTier, type HeatmapTier, type HeatmapValue } from "@/utils/heatmap";

const props = withDefaults(
  defineProps<{
    /** 每日请求数；date 格式 YYYY-MM-DD（本地自然日） */
    values: HeatmapValue[];
    /** 网格结束日期（含），默认今天；向前展示 cols*7 天 */
    endDate?: Date;
    /** 周列数，默认 53（约一年） */
    cols?: number;
  }>(),
  {
    cols: 53,
  },
);
const tierClass: Record<HeatmapTier, string> = {
  0: "bg-muted",
  1: "bg-primary/30",
  2: "bg-primary/60",
  3: "bg-primary",
};

const legendTiers: HeatmapTier[] = [0, 1, 2, 3];

function startOfWeek(d: Date): Date {
  // 周一 = 0
  const day = (d.getDay() + 6) % 7;
  const out = new Date(d);
  out.setDate(out.getDate() - day);
  out.setHours(0, 0, 0, 0);
  return out;
}

interface Cell {
  date: string;
  count: number;
  tier: HeatmapTier;
}

const grid = computed(() => {
  const end = props.endDate ? new Date(props.endDate) : new Date();
  const lastMonday = startOfWeek(end);
  const byDate = new Map<string, number>();
  let maxCount = 0;
  for (const v of props.values) {
    byDate.set(v.date, v.count);
    if (v.count > maxCount) maxCount = v.count;
  }

  const months: (string | null)[] = [];
  const cells: Cell[][] = [];
  let prevMonth = -1;

  for (let col = 0; col < props.cols; col++) {
    const monday = new Date(lastMonday);
    monday.setDate(monday.getDate() - (props.cols - 1 - col) * 7);
    const colMonth = monday.getMonth();
    // 每列取周一所在月份；跨月才显示标签（GitHub 风格月份提示）。
    months.push(colMonth === prevMonth ? null : `${monday.getMonth() + 1}月`);
    prevMonth = colMonth;

    const colCells: Cell[] = [];
    for (let row = 0; row < 7; row++) {
      const d = new Date(monday);
      d.setDate(d.getDate() + row);
      const key = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
      const count = byDate.get(key) ?? 0;
      colCells.push({ date: key, count, tier: colorTier(count, maxCount) });
    }
    cells.push(colCells);
  }
  return { months, cells };
});

function cellTip(cell: Cell): string {
  return cell.count > 0 ? `${cell.date} · ${cell.count} 次请求` : `${cell.date} · 无请求`;
}

const gridStyle = computed(() => ({
  gridTemplateColumns: `repeat(${props.cols}, minmax(8px, 1fr))`,
}));
</script>

<template>
  <TooltipProvider :delay-duration="0">
    <div class="w-full">
      <!-- 月份标签：与列同宽对齐 -->
      <div class="grid gap-[3px]" :style="gridStyle">
        <span
          v-for="(label, i) in grid.months"
          :key="i"
          class="whitespace-nowrap text-[10px] leading-3 text-muted-foreground"
        >
          {{ label || "" }}
        </span>
      </div>

      <!-- 周行（周一~周日），每格一个可悬浮的方块 -->
      <div
        v-for="row in 7"
        :key="row"
        class="mt-[3px] grid gap-[3px]"
        :style="gridStyle"
      >
        <Tooltip v-for="cell in grid.cells" :key="cell[row - 1].date">
          <TooltipTrigger as-child>
            <button
              type="button"
              :aria-label="cellTip(cell[row - 1])"
              class="aspect-square w-full rounded-[3px] transition-colors hover:opacity-75"
              :class="tierClass[cell[row - 1].tier]"
            />
          </TooltipTrigger>
          <TooltipContent side="top">
            <p class="text-xs">{{ cellTip(cell[row - 1]) }}</p>
          </TooltipContent>
        </Tooltip>
      </div>

      <!-- 图例 -->
      <div class="mt-3 flex items-center gap-1.5 text-xs text-muted-foreground">
        <span>少</span>
        <span
          v-for="t in legendTiers"
          :key="t"
          class="h-3 w-3 rounded-[3px]"
          :class="tierClass[t]"
        />
        <span>多</span>
      </div>
    </div>
  </TooltipProvider>
</template>
