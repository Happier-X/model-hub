# 08-10-cost-tracking

## Goal

开发「消耗费用」功能：自动同步 OpenRouter 模型单价 → 统计时按 token 用量现算费用 → 首页统计卡片费用从「-」变为真实金额（总费用/输入费用/输出费用）。

## Background（已确认事实）

- `request_logs` 现表有 `input_tokens`/`output_tokens`（成功请求才有非零值），无 cost 列（legacy 旧表曾有，MIGRATION_V1 建表无）。
- `OverviewRow.cost: f64` 已存在，后端恒填 0（log.rs:480）；首页 StatsCards 费用三项显示「-」。
- 项目无任何单价表/价格代码。
- 统计口径：仅成功请求（2xx 且 error 空），`request_overview` 返回 total/today。
- OpenRouter `https://openrouter.ai/api/v1/models` 实测可达：400+ 模型，`pricing.prompt/completion` 为每 token 美元。
- 后台定时先例：`proxy/runtime.rs` timer_fut（启动静默 5 分钟 + 周期 tick 检查，24h 到期才拉），provider auto_sync 复用该模式。
- 项目约束「禁止对用户上游自动探测」仅限用户配置的上游；OpenRouter 官方价格 API 为用户明确要求的同步源。

## Requirements

- **R1 单价表**：新表 `model_pricing`（模型名主键、输入/输出每百万 token 价格、更新时间）；OpenRouter 未覆盖模型无行（视为 0 价）。
- **R2 自动同步**：B2 纯自动——启动静默期后 + 每 24h 到期检查，拉取 OpenRouter models 价格 upsert 到单价表；失败静默（warn 日志，不阻断启动/代理）。同步时机与节奏复用 provider auto_sync 模式。
- **R3 手动触发**：设置页「立即同步」按钮（点击触发一次同步，非定时自动；无手动编辑价格表单）。
- **R4 统计时算**：`request_overview` 聚合时 LEFT JOIN 单价表按模型计算费用（输入/输出分别算），`request_logs` 不加 cost 列；改价/补同步后历史统计自动重算。
- **R5 模型匹配**：日志 `upstream_model` 精确匹配单价表；附加别名匹配（单价表 `xxx/deepseek-chat` 可匹配日志 `deepseek-chat`，LIKE 前缀通配）。
- **R6 费用展示**：首页四卡片三项费用（总/输入/输出）显示美元金额；无价模型按 0 计（用户已选 B）；货币符号 `$`。
- **R7 设置页展示**：新增「模型单价」只读表格（模型名/输入价/输出价/更新时间）+ 同步状态（上次同步时间/已同步模型数）+「立即同步」按钮 + 搜索过滤；数据量大（400+）需可搜索。
- **R8 金额格式**：`formatCost` 工具——0 显示 `$0`；>0 显示 `$` + 4 位小数去尾 0（如 `$1.25`、`$0.0012`）。

## Acceptance Criteria

- [ ] 启动代理后（静默期外）单价表自动从 OpenRouter 填充，无网络时静默跳过且不影响代理/首页
- [ ] `get_request_overview` 返回真实费用：总/输入/输出费用 = Σ(tokens × 单价 / 1e6)，无价模型贡献 0
- [ ] 设置页「模型单价」只读表格渲染、搜索可用、「立即同步」按钮触发同步并更新状态
- [ ] 首页四卡片费用显示美元金额（`$0` 或 `$x.xxxx`），不再是「-」
- [ ] 新增单元测试：OpenRouter 价格解析纯函数、overview 费用聚合（含别名匹配）、formatCost 边界
- [ ] `cargo test --lib`、`pnpm typecheck/lint/test:unit/build` 全绿

## Out of Scope

- 手动编辑/新增单价表单（B2 无手动编辑）
- 人民币/多币种换算；OpenRouter 之外的价格源
- 日志页单条费用展示；费用图表/排行
- 供应商级/分组级定价覆盖

## Key Decisions（用户已确认）

- 粒度：按模型全局表
- 计算时机：统计时算（改价可重算，不加 cost 列）
- 来源：B2 纯自动定时同步 OpenRouter + 设置页手动「立即同步」触发（无手动编辑）
- 无价模型：按 0 计（显示 $0）

## Notes

- 金额单位：美元 `$`（与 OpenRouter 数据源一致）；如后续需要 ¥ 再做换算/切换。
