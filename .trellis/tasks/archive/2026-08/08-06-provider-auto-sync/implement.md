# 执行计划

## 阶段 1：后端 schema 迁移（migrate.rs）
- [ ] `migrate.rs`：`ALTER TABLE providers ADD COLUMN auto_sync INTEGER NOT NULL DEFAULT 1`（幂等）
- [ ] `migrate.rs`：`ALTER TABLE providers ADD COLUMN last_sync_at INTEGER`（幂等）
- [ ] `migrate.rs`：`CREATE TABLE IF NOT EXISTS provider_models (...)`，UNIQUE(provider_id, model_name)
- [ ] 补迁移测试：新列存在、provider_models 建表、旧数据保留

## 阶段 2：Provider 领域层（domain/provider.rs）
- [ ] `Provider` 结构体加 `auto_sync: bool`、`last_sync_at: Option<i64>`
- [ ] `CreateProviderPayload` / `UpdateProviderPayload` 加 `auto_sync: bool`
- [ ] `map_row` / `list_providers` / `get_provider` SQL 增列
- [ ] `create_provider` / `update_provider` 写入 auto_sync，返回完整 Provider
- [ ] 新增 `set_provider_auto_sync(id, enabled)` → 返回 Provider
- [ ] 新增 `replace_provider_models(provider_id, &[String])`（事务 delete+insert）
- [ ] 新增 `list_provider_models(provider_id) -> Vec<String>`
- [ ] 新增 `touch_provider_synced_at(id, unix)`
- [ ] 更新 provider_crud 测试 + 新增 models 替换/读取测试

## 阶段 3：同步任务改造（commands.rs + runtime.rs）
- [ ] `perform_due_bound_groups` → `perform_due_provider_syncs`（遍历 auto_sync 供应商，跳过 disabled/未到期）
- [ ] 新增 `perform_sync_provider(stores, provider_id)`：拉取 → replace → touch
- [ ] 移除 `perform_sync_bound_group`、`sync_group_now` 命令；`runtime.rs` 调度改调用 `perform_due_provider_syncs`
- [ ] `delete_provider` 解绑逻辑保留
- [ ] 新增命令：`sync_provider_now(provider_id)`、`get_provider_models(provider_id)`、`set_provider_auto_sync`
- [ ] 更新 lib.rs invoke_handler 注册
- [ ] 补同步任务测试（无绑定分组逻辑）

## 阶段 4：前端 API 层（api/tauri.ts）
- [ ] `Provider` 类型加 `auto_sync` / `last_sync_at`
- [ ] 新增 `setProviderAutoSync` / `syncProviderNow` / `getProviderModels`
- [ ] 移除 `syncGroupNow`

## 阶段 5：前端供应商页（ProvidersPage.vue）
- [ ] 表格列加「自动同步」「上次同步」，HSwitch 就地切换
- [ ] 操作列加「立即同步」按钮 + loading
- [ ] 移除/调整不再适用的逻辑

## 阶段 6：前端分组页（GroupFormPage.vue + useProviderModelCache.ts）
- [ ] `useProviderModelCache`：优先读本地 `getProviderModels`，空则实时拉取兜底
- [ ] GroupFormPage：移除 isBound 相关 UI 与逻辑（绑定下拉、提示块、立即同步、禁用分支）
- [ ] 提交 payload 不再带 source_provider_id
- [ ] 左侧供应商条目显示同步状态小字

## 阶段 7：质量检查
- [ ] `cargo test` 全绿
- [ ] `pnpm typecheck` / `pnpm lint` / `pnpm test:unit` / `pnpm build` 全绿
- [ ] 手工核对 AC3-AC6（后台同步、开关、读本地模型、无绑定残留）
