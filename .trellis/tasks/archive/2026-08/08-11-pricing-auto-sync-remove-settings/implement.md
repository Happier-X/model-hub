# 执行计划：自动同步模型单价并移除设置页模型单价模块

## 实施清单

### 1. 前端移除
- [ ] `src/pages/SettingsPage.vue`：删除 script 中定价状态/函数（`pricingInfo` 等 + `refreshPricing`/`syncPricing`/`pricingFiltered`/`formatPricingTime`）。
- [ ] `src/pages/SettingsPage.vue`：删除 `refresh()` 内 `await refreshPricing()`。
- [ ] `src/pages/SettingsPage.vue`：删除 template 中「模型单价」Card 整块。
- [ ] `src/pages/SettingsPage.vue`：清理 import（`getModelPricing`/`syncPricingNow`/`PricingInfo`、Table 系列组件）。
- [ ] `src/api/tauri.ts`：删除 `ModelPrice`/`PricingInfo`/`PricingSyncInfo` 接口与 `getModelPricing`/`syncPricingNow`。

### 2. 后端清理
- [ ] `src-tauri/src/domain/pricing.rs`：删除 `PricingInfo`/`PricingSyncInfo` 结构与 `pricing_info()`；保留 `ModelPrice`/`replace_pricing`/`last_pricing_sync_at`/`parse_openrouter_pricing` 及测试。
- [ ] `src-tauri/src/commands.rs`：删除 `get_model_pricing`/`sync_pricing_now`；修正 import（只留 `ModelPrice`）。
- [ ] `src-tauri/src/lib.rs`：删除两行命令注册。

### 3. 验证
- [ ] `cargo test` 全量通过、`cargo build` 通过。
- [ ] 前端类型检查通过（`pnpm vue-tsc` 或 `pnpm lint`，无未使用 import/残留引用）。
- [ ] 确认 `perform_due_price_syncs`/`fetch_openrouter_pricing`/`replace_pricing` 仍在代码中且被 runtime.rs 调用。
- [ ] 端到端：重置价格同步时间（`last_pricing_sync_at` 来源置 NULL）→ 重启应用 → 等 `SYNC_STARTUP_DELAY` → `model_pricing` 非零价格行出现。
- [ ] 首页费用统计不受影响（`request_overview` 未改；可选抽查首页费用仍非零）。

## 验证命令

```powershell
cd src-tauri
cargo test
cargo build
```

```powershell
pnpm vue-tsc --noEmit   # 或项目现有类型检查命令
pnpm lint
```

## 风险与回滚点

- 风险文件：`src/pages/SettingsPage.vue`、`src/api/tauri.ts`、`src-tauri/src/commands.rs`、`src-tauri/src/domain/pricing.rs`、`src-tauri/src/lib.rs`。
- 误删保留函数 → 编译/测试门禁拦截；误留前端引用 → lint/类型检查拦截。
- 端到端验证依赖 OpenRouter 网络与 5 分钟启动延迟；单元/编译验证不依赖网络。
- 回滚：git revert 提交即可；无 schema 或数据迁移。

## 开始实施前检查

- [ ] prd.md、design.md、implement.md 已完成并经用户批准。
- [ ] implement.jsonl、check.jsonl 已填充真实规范条目。
