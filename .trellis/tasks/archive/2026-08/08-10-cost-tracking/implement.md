# 08-10-cost-tracking — Implement

## 步骤（按依赖排序）

1. **DB**：`migrate.rs` 建 `model_pricing` 表（MIGRATION_V1 追加 + `ensure_model_pricing_table` 幂等 ensure，旧库补建）。
2. **domain/pricing.rs**（新模块）：
   - `ModelPrice` 结构（Serialize/Deserialize）
   - `parse_openrouter_pricing(&[u8]) -> Vec<ModelPrice>` 纯函数（每 token → 每百万，非法行跳过）
   - Stores：`replace_pricing`（事务 upsert+清理）、`list_pricing`、`pricing_sync_info -> (count, updated_at)`
3. **commands.rs**：
   - `fetch_openrouter_pricing()`（reqwest GET，15s 超时）
   - `perform_due_price_syncs(stores)`（24h 到期才拉，失败 warn）
   - `#[tauri::command] get_model_pricing` / `sync_pricing_now`
   - lib.rs invoke_handler 注册两个命令
4. **runtime.rs**：timer_fut interval tick 内追加 `perform_due_price_syncs`
5. **log.rs**：`overview_row` SQL LEFT JOIN model_pricing（含别名匹配）+ OverviewRow 加 `input_cost/output_cost`；cost = 两者和
6. **测试（后端）**：
   - `parse_openrouter_pricing`：正常/空/非法行/每 token→每百万换算
   - overview 费用聚合：有价模型按价算、无价 0、别名匹配（`deepseek/deepseek-chat` ↔ 日志 `deepseek-chat`）、range 分支
   - 迁移：model_pricing 表 ensure 幂等（旧库补建）
7. **前端**：
   - `tauri.ts`：OverviewRow 加字段 + ModelPrice/PricingInfo 类型 + getModelPricing/syncPricingNow
   - `src/utils/formatCost.ts` + 单测（0、1.25、0.00123、去尾 0）
   - `StatsCards.vue`：费用三项改 `formatCost`（总= cost，输入= input_cost，输出= output_cost）
   - `SettingsPage.vue`：新增「模型单价」卡片（状态行 + 立即同步 + 搜索 + Table）
8. **验证**：`cargo test --lib` → `pnpm typecheck/lint/test:unit/build`
9. **spec 同步**：database-guidelines（model_pricing 表）+ component-guidelines（设置页表格/费用展示）
10. **commit + journal**

## 验证命令

```bash
cd src-tauri && cargo test --lib
cd .. && pnpm typecheck && pnpm lint && pnpm test:unit && pnpm build
```

## 风险文件 / 回滚点

- `migrate.rs`（建表幂等）、`log.rs`（overview SQL，改动影响现统计）、`runtime.rs`（启动链路）
- 回滚：删 model_pricing 建表段 + 恢复 log.rs overview SQL + 前端费用常量即可整体回退；新文件删除即回滚
- dev 进程占用 model-hub.exe 时 cargo 全量测试报 os error 5 → 用 `cargo test --lib`
