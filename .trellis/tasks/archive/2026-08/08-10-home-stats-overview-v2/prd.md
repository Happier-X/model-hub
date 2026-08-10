# 08-10-home-stats-overview-v2

## Goal

首页顶部统计改为 **octopus（bestruirui/octopus）同款四统计卡片**：四张卡片并排，每卡左侧竖排标题+图标，右侧两个指标（图标块 + 标签 + 数值 + 单位）。替换上一轮实现的「总计/今日两行表格」卡片。

参考实现：octopus `web/src/components/modules/home/total.tsx`（已调研源码）。

## Requirements

- **R1 四卡片布局**：`grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4`；每卡圆角卡片（`bg-card border`），左区竖排标题（`[writing-mode:vertical-lr]`）+ 头部图标，右区垂直排列 2 个指标项（`w-10 h-10 rounded-xl` 图标块 + 标签 + 数值 + 单位），左区与右区间有分隔线。
- **R2 卡片内容**（数据来自 `get_request_overview` 后端命令）：
  - 卡片 1「请求统计」：请求次数、耗时（`formatDuration` 自适应单位）
  - 卡片 2「总统计」：总 token（输入+输出）、费用
  - 卡片 3「输入统计」：输入 tokens、输入费用
  - 卡片 4「输出统计」：输出 tokens、输出费用
- **R3 今日/总计切换**：卡片上方保留「总计 / 今日」切换（上一轮「总计+今日」需求仍有效），默认「总计」；切换后四卡片同一批指标显示今日数据。
- **R4 费用**：本期显示「-」（单价配置后续任务，沿用已确认方案 A）。
- **R5 样式**：全部用 shadcn design token（`bg-card`/`border`/`text-muted-foreground`/`text-primary`/`bg-chart-1..4/10`）；图标用 `@lucide/vue`（10 个图标名已在 node_modules 验证存在）。
- **R6 动画**：卡片入场淡入（可选，stagger 0.08s）+ 数值滚动动画（轻量 AnimatedNumber，requestAnimationFrame 500ms），对齐 octopus 观感；动画失败不影响数值显示。
- **R7 刷新**：刷新按钮保留（octopus 无刷新按钮，但 model-hub 需要手动刷新入口），放卡片上方工具条。

## Acceptance Criteria

- [ ] 首页顶部渲染 4 张 octopus 风格统计卡片（竖排标题、图标块、分隔线、数值+单位），布局在宽屏 4 列、窄屏降级 1 列
- [ ] 数据映射正确：请求次数 / 耗时 / 总 token / 输入 token / 输出 token 与 `get_request_overview` 数值一致（仅成功请求口径）
- [ ] 今日/总计切换生效，默认总计；空库时全部显示 0
- [ ] 费用列显示「-」
- [ ] 旧「统计总览两行表格」卡片移除，「最近成功请求」卡片保留
- [ ] `pnpm typecheck` / `lint` / `test:unit` / `build` 全绿；新增前端测试（若有纯函数）
