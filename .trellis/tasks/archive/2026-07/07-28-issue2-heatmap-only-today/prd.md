# Issue #2: 每日请求量热力图只展示当天色块，未展示一整年

## Goal

修复 GitHub Issue [#2](https://github.com/Happier-X/model-hub/issues/2) "每日请求量热力图
展示有问题：没有展示一整年的热力图，然后每次只展示了当天的色块"。

## 根因（研究已确认）

1. **数据源（主要、已由 #1 大幅缓解）**：`LOG_MAX_ROWS=1000` purge 策略将历史日日志删除，
   导致 `request_daily_counts` 在 365 天窗口内只查到今天的记录。`#1` 已将默认上限提至 10000，
   使历史日志能尽量保留。该根因不需再动。

2. **热力图渲染窗口（本次修复目标）**：前端 `heatmapData` 直接把后端返回的 `days` 映射为
   `HHeatmapItem[]`。`HHeatmap` 内部以 `minTs`/`maxTs`（输入数据的实际范围）作为渲染首尾日，
   而非硬编码的"一整年"。当 `days` 只有当天 1 项时，热力图就只渲染 1 个格子，不显示全年
   上下文。用户期望始终看到 365 天的完整网格。

## 范围内

- 在前端 `HomePage.vue` 将 sparse `daily.value?.days` 补全为 365 天完整 `HHeatmapItem[]`：
  - 取 `daily.end_unix`（后端返回的窗口上限，今日次日 00:00 的 unix 秒）作为"今天"；
  - 向前覆 365 天（`end_unix - 365 × 86400` 到 `end_unix`），每一天生成一个 `HHeatmapItem`；
  - value = 从 `days` 按 `day_start_unix` 查找 → 有数据取 count，无数据取 0/null；
  - 不再依赖 `HHeatmap` 的自动 `minTs`/`maxTs` 推断。
- 保持现有接口调用（`getRequestDailyCounts()` 不改）与后端逻辑（`request_daily_counts` 不改）。

## Out of Scope

- 不改后端 `request_daily_counts` 逻辑。
- 不改 `HHeatmap` 组件（在 happier-ui 库中，非本仓库）。
- 不改 LOG_MAX_ROWS 以外的持久化策略（#1 已修）。
- 不引入设置页"热力图天数"可配置项（留给后续）。

## Requirements

- 热力图网格始终显示 365 天的完整日历格子（从窗口上界 `end_unix` 往前 365 天到最新一天）。
- 后端返回的 `days` 只含当日有记录的日（`count>0`）；前端补全中间"无记录"日填 0。
- `daily.value?.days` 为空数组时，365 个格子都填 0，网格仍完整渲染。
- `end_unix` 不存在（初始化时 `daily` 为 null）时，`heatmapData` 为空数组（<365 天）。

## Acceptance Criteria

- [ ] 修改 `src/pages/HomePage.vue` 中 `heatmapData` 计算属性
- [ ] 热力图始终显示全 365 天网格，而非只有实际数据日
- [ ] 后端 `days` 数组为空时 365 天网格不退化（全部 value=0）
- [ ] `npm run build` / `npm run lint` / `npm run test:unit` 通过
- [ ] 已提交 `fix(frontend): 热力图补全 365 天全网格显示 (#2)`

## Notes

- 后端 `days` 已经是"仅 count>0 的日期"语义（实际实现中只在 buckets 有数据时插入 map），
  无需在后端产出"365 天 0-padded"—那样 365×条太冗余且数据库查询本已覆盖 365 天窗口。
- 前端补全是干净的方案，只需对 ~365 天的循环做一次 HashMap lookup，20ms 级开销不感知。
- `end_unix` 语义：今日次日 00:00 的 unix 秒，即今天午夜。窗口是 `[end_unix-365*86400, end_unix)`。