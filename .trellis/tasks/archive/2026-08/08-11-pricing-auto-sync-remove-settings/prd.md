# 自动同步模型单价并移除设置页模型单价模块

## Goal

让模型单价完全自动同步，用户无需手动操作；同时移除设置页「模型单价」模块（按钮 + 状态 + 列表 + 搜索），简化设置页。

## Confirmed Facts

- 自动同步机制已存在：代理启动 5 分钟后开始（`SYNC_STARTUP_DELAY`），之后每小时检查一次（`SYNC_CHECK_INTERVAL`），从未同步或超过 24h（`SYNC_STALE_AFTER_SECS`）才从 OpenRouter 拉取；失败仅 tracing warning，不阻塞主流程（`perform_due_price_syncs`，`src-tauri/src/commands.rs:351`，由 `src-tauri/src/proxy/runtime.rs:249` 定时调用）。
- 设置页「模型单价」模块（`src/pages/SettingsPage.vue` 约 255-520 行）是前端唯一使用方：`getModelPricing` / `syncPricingNow`（`src/api/tauri.ts:295-308`）。
- 后端命令：`get_model_pricing`（commands.rs:379）、`sync_pricing_now`（commands.rs:387）、领域 `pricing_info()`（pricing.rs:149）、结构 `PricingInfo` / `PricingSyncInfo`（pricing.rs:18-27）；注册于 lib.rs:139-140。
- 自动同步依赖的 `fetch_openrouter_pricing` / `replace_pricing` / `last_pricing_sync_at` 必须保留（首页费用统计依赖 model_pricing 表）。
- 价格解析修复（字符串兼容）已完成并验证：数据库 402 行中 380 行非零价格，首页费用 $39.17 正常显示。

## Requirements

- R1：自动同步策略维持现状：启动 5 分钟后 + 每小时检查 + 超过 24h 未同步才拉取；不新增网络请求频率，不改变失败处理（warning 日志）。
- R2：移除设置页「模型单价」模块：标题、「立即同步」按钮、同步状态行、搜索框、单价列表。
- R3：后端不再使用的命令/结构/领域方法一并清理：`get_model_pricing`、`sync_pricing_now`、`PricingInfo`、`PricingSyncInfo`、`pricing_info()`，以及 `tauri.ts` 中对应封装与 TS 接口。
- R4：保留 `fetch_openrouter_pricing`、`replace_pricing`、`last_pricing_sync_at`、`parse_openrouter_pricing` 及 `model_pricing` 表结构与统计查询逻辑。
- R5：移除后自动同步仍正常工作：新库/从未同步场景下，启动后自动拉取并写入价格表。
- R6：移除后不提供任何同步状态展示（设置页不留状态提示，完全移除）。

## Acceptance Criteria

- [ ] AC1：设置页不再显示「模型单价」模块；页面无残留引用、TS 类型检查通过。
- [ ] AC2：后端 `get_model_pricing` / `sync_pricing_now` 命令与 `PricingInfo` / `PricingSyncInfo` 结构删除，`lib.rs` 注册移除；`cargo build` 通过、`cargo test` 全量通过。
- [ ] AC3：`perform_due_price_syncs` 及 `fetch_openrouter_pricing` / `replace_pricing` 保留且单测通过（pricing 领域测试仍在）。
- [ ] AC4：端到端：从未同步状态（删除库或重置同步时间）启动应用，自动同步触发后 `model_pricing` 出现非零价格行（可通过调整 SYNC_STARTUP_DELAY 缩短等待或直接验证 perform_due_price_syncs 逻辑）。
- [ ] AC5：首页费用统计与既有请求日志不受影响（`request_overview` 不变）。

## Out of Scope

- 不改变自动同步周期、网络地址、超时或失败重试策略。
- 不修改 `model_pricing` 表结构、全量 replace 策略、费用统计 SQL 或前端费用展示。
- 不引入新的设置项、状态展示或其他价格查看入口。
- 不触碰 `request_logs`、代理转发、首页统计实时刷新等无关模块。
