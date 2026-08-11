# 技术设计：自动同步模型单价并移除设置页模型单价模块

## 1. 边界

- 后端：`src-tauri/src/commands.rs`（删 `get_model_pricing` / `sync_pricing_now`）、`src-tauri/src/domain/pricing.rs`（删 `PricingInfo` / `PricingSyncInfo` / `pricing_info()`）、`src-tauri/src/lib.rs`（删命令注册）。
- 前端：`src/pages/SettingsPage.vue`（删「模型单价」Card 与相关 script）、`src/api/tauri.ts`（删接口与封装）。
- **保留**：`fetch_openrouter_pricing`、`replace_pricing`、`last_pricing_sync_at`、`parse_openrouter_pricing`、`perform_due_price_syncs`、`ModelPrice`（Rust）、`model_pricing` 表、`request_overview` 费用 SQL、`SYNC_*` 常量与 runtime.rs 定时任务。

## 2. 数据流（不变部分）

```
runtime.rs 定时任务 → perform_due_price_syncs（从未同步或 >24h）
  → fetch_openrouter_pricing → parse_openrouter_pricing
  → replace_pricing（全量替换 model_pricing）
  → request_overview 按 token × 单价 / 1e6 计算首页费用
```

移除「立即同步」IPC 路径不影响上述链路；手动入口消失，自动入口不变。

## 3. 删除清单

### 前端 SettingsPage.vue

- import：`getModelPricing`、`syncPricingNow`、`PricingInfo`（第 23/31/34 行附近）、`Table`/`TableBody`/`TableCell`/`TableHead`/`TableHeader`/`TableRow`（第 10-16 行）。
- script：`pricingInfo`/`pricingError`/`pricingLoading`/`syncLoading`/`pricingSearch` 状态、`refreshPricing`/`syncPricing`/`pricingFiltered`/`formatPricingTime` 函数、`refresh()` 内 `await refreshPricing()`。
- template：整个「模型单价」Card（含按钮、状态行、搜索框、说明、错误、表格）。
- 保留 `computed`/`Input`/`Button`/`Card*` import（均有其他使用处）。

### 前端 tauri.ts

- 删 `ModelPrice`（仅被 `PricingInfo` 引用）、`PricingInfo`、`PricingSyncInfo` 接口与 `getModelPricing` / `syncPricingNow` 封装。

### 后端

- commands.rs：删 `get_model_pricing`（379 行起）、`sync_pricing_now`（387 行起）；import 行改为只保留 `ModelPrice`（`fetch_openrouter_pricing` 返回值用）。
- pricing.rs：删 `PricingInfo` / `PricingSyncInfo` 结构（18-27 行附近）、`pricing_info()` 方法（149 行起）；`ModelPrice`、`replace_pricing`、`last_pricing_sync_at`、`parse_openrouter_pricing` 及测试保留。
- lib.rs：删 `commands::get_model_pricing`、`commands::sync_pricing_now` 两行注册。

## 4. 验证方案

1. `cargo test` / `cargo build`（后端无引用残留）。
2. 前端类型检查（`pnpm vue-tsc` 或等价）确认无残留引用；`pnpm lint` 确认无 unused import。
3. 端到端（AC4）：SQL 将 `model_pricing.updated_at` 相关同步时间置 NULL（`providers` 表或价格同步时间来源——见 `last_pricing_sync_at` 实现），重启应用，等待 `SYNC_STARTUP_DELAY`（5 分钟）后验证 `model_pricing` 非零价格行重新出现；期间检查日志无异常。
   - 若等待成本过高，可临时以环境变量/短延迟构建验证，验证后还原（不提交该改动）。

## 5. 兼容性与回滚

- 纯删除型重构：无 schema、IPC 契约变更对保留功能的依赖；`request_overview` 与前端首页不感知变化。
- 回滚：git revert 即可；无数据迁移。
- 风险：前端遗留未使用 import（lint 报错）或后端误删保留函数（编译/测试失败）——由编译与测试门禁拦截。
