# 08-10-home-stats-overview-v2 — Design

## 背景

octopus（bestruirui/octopus）首页顶部 `total.tsx`：4 张卡片，每卡左侧竖排标题 + 头部图标，右侧 2 行指标。数据 `useStatsTotal()` 为全局累计。本任务在 model-hub 首页复刻该样式，数据源复用已上线的 `get_request_overview`（total/today 两组，成功口径，cost 恒 0）。

## 组件结构

新建 `src/components/StatsCards.vue`（纯展示 + 切换状态，不自拉数据）：

```
props:
  overview: RequestOverview | null
  error: string
  onRefresh: () => void          // 刷新按钮（HomePage 传入 refreshStats）
state:
  mode: 'total' | 'today'        // 顶部切换，默认 total
模板:
  <div>
    <!-- 工具条：今日/总计切换 + 刷新按钮 -->
    <div class="mb-3 flex items-center justify-between gap-2">
      <div class="flex rounded-lg bg-muted p-0.5">  <!-- 两个 tab 按钮 -->
        总计 | 今日
      </div>
      <Button variant="outline" size="sm" @click="onRefresh">刷新统计</Button>
    </div>
    <!-- 4 卡片网格 -->
    <div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-4">
      4 × <section class="rounded-2xl border border-slate-200 bg-card p-5 ...">
        左区: 竖排标题（[writing-mode:vertical-lr]）+ 头部图标
        右区: 2 个指标项
    </div>
    <p v-if="error">错误提示</p>
  </div>
```

选 `row: OverviewRow`（`mode==='total' ? overview.total : overview.today`）为当前数据。

## 数据映射（OverviewRow 字段 → 8 指标）

| 卡片 | 竖排标题 | 头部图标 | 指标 1 | 指标 2 | 图标块配色 |
|------|---------|---------|--------|--------|-----------|
| 1 | 请求统计 | Activity | 请求次数 `requests` | 耗时 `formatDuration(use_time_ms)` | 指标1 `bg-primary/10 text-primary`；指标2 `bg-accent/10 text-primary` |
| 2 | 总统计 | ChartColumnBig | 总 token `input+output` | 费用 `-` | `bg-chart-1/10 text-primary` |
| 3 | 输入统计 | ArrowDownToLine | 输入 tokens `input_tokens` | 输入费用 `-` | `bg-chart-3/10 text-primary` |
| 4 | 输出统计 | ArrowUpFromLine | 输出 tokens `output_tokens` | 输出费用 `-` | `bg-chart-4/10 text-primary` |

- 指标图标：MessageSquare（请求次数）、Clock（耗时）、Bot（总 token）、DollarSign（费用×3）、Rewind（输入）、FastForward（输出）
- 数值格式：请求次数/token 用 `toLocaleString('zh-CN')` 千分位；耗时用 `formatDuration`；费用「-」
- token 大数缩写：超过 1000 显示 `x.xk`（octopus 有单位缩写，简单实现 `formatTokenCount` 纯函数 + 单测）

## 动画

- **AnimatedNumber**：组件内小函数，`requestAnimationFrame` 从上一值滚动到新值（600ms，easeOutCubic）；更新数值 prop 变化时触发；组件卸载取消 rAF。
- 卡片入场：CSS `transition` + 依索引 `transition-delay: i*80ms`，`opacity/translate-y` 初始态 → 挂载后动画态（Vue `onMounted` 后置 flag）。
- 动画不影响 SSR/测试（测试只测纯函数）。

## 今日/总计切换

octopus 只有总计；上一轮用户需求「总计+今日」保留。切换做成顶部小 tab（总计/今日），默认总计，显示 octopus 同款卡片。切换不重新拉数据（overview 一次拉回两组）。

## HomePage 集成

- 删除旧「统计总览」两行表格卡片模板（含表头/总计行/今日行 grid-cols-7）
- 删除 `formatDuration` 在该模板中的内联用法（保留 utils 供 StatsCards 用）
- 保留 `overview` / `overviewError` / `refreshStats` 状态，模板改为 `<StatsCards :overview="overview" :error="overviewError" :on-refresh="refreshStats" />`
- 「最近成功请求」卡片保留不动

## 边界

- 空库 / overview 为 null：卡片显示 0 与「-」，无崩溃
- 费用恒「-」：后端 cost 恒 0，本期不展示 0 而展示「-」更符合方案 A 语义
- 无新增后端改动（复用 get_request_overview）
