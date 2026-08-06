# 供应商级自动同步模型

## Goal

把「自动同步模型」从**分组维度**迁移到**供应商维度**：每个供应商可独立开关自动同步，同步结果持久化到本地 `provider_models`，分组页左侧直接读本地同步结果（离线可用），并保留手动刷新能力。

## Background（已确认事实）

- 现状：`providers` 表无模型持久化；模型列表由分组页左侧展开时实时调用 `fetch_provider_models` 拉取（内存缓存 `useProviderModelCache`）。
- 现状：分组绑定同步 `groups.source_provider_id` + `last_sync_at`，后台 `perform_due_bound_groups` 每 24h 拉取绑定供应商模型 → `replace_group_items` 替换分组队列。
- 后台调度：`runtime.rs` `SYNC_CHECK_INTERVAL=3600s` 每 1h 检查，`SYNC_STALE_AFTER_SECS=24h` 判定到期，`SYNC_STAGGER=5s` 错峰。

## Decisions（用户已确认，1-7 全数采纳）

| # | 决策 | 结论 |
|---|------|------|
| 1 | 分组绑定同步去留 | **彻底移除** `groups.source_provider_id` + `last_sync_at` 机制；分组队列改回纯手动维护 |
| 2 | 开关 UI | 供应商页表格加「自动同步」开关（就地切换）；分组页左侧供应商条目显示同步状态徽标（只读展示） |
| 3 | 同步频率 | 写死 24h（沿用 `SYNC_STALE_AFTER_SECS`）；供应商页展示「上次同步」时间 |
| 4 | 模型读取 | 分组页左侧优先读本地 `provider_models`；保留「重试/刷新」触发重新拉取；失败回退实时拉取 |
| 5 | 持久化表 | 新增 `provider_models(provider_id, model_name)`，同步时全量替换该供应商行 |
| 6 | 后台任务 | `perform_due_bound_groups` → `perform_due_provider_syncs`：遍历开开关的供应商，每 24h 拉取 → 写 `provider_models` + 更新 `providers.last_sync_at` |
| 7 | 历史数据 | `groups.source_provider_id` 字段保留但不再写入/使用，不迁移不删除 |

## Requirements

### R1 后端：schema 迁移
- `providers` 表新增列：`auto_sync INTEGER NOT NULL DEFAULT 1`、`last_sync_at INTEGER`
- 新表 `provider_models(id, provider_id, model_name, sort_order)`，`provider_id` 外键 ON DELETE CASCADE，`UNIQUE(provider_id, model_name)`
- 迁移幂等：仅当列/表缺失时执行（沿用现有 migrate 模式）

### R2 后端：Provider 领域扩展
- `Provider` 结构体新增 `auto_sync: bool`、`last_sync_at: Option<i64>`
- `CreateProviderPayload` / `UpdateProviderPayload` 新增 `auto_sync: bool`
- CRUD SQL 同步增列；`create_provider` / `update_provider` 返回含新字段的完整 Provider
- 新增：
  - `set_provider_auto_sync(id, enabled)` —— 供应商页就地切换
  - `replace_provider_models(provider_id, &[String])` —— 同步时全量替换（事务内 delete + batch insert）
  - `list_provider_models(provider_id) -> Vec<String>` —— 读本地持久化模型
  - `touch_provider_synced_at(id, unix)` —— 更新上次同步时间

### R3 后端：同步任务改造
- `perform_due_bound_groups` → `perform_due_provider_syncs`：遍历 `auto_sync=true` 的供应商，`last_sync_at` 为空或超过 24h 则同步
- 同步逻辑 `perform_sync_provider`：拉取上游模型 → `replace_provider_models` → `touch_provider_synced_at`
- 供应商 `enabled=false` 时跳过（沿用现有约束）
- 保留 `SYNC_STALE_AFTER_SECS` / `SYNC_STAGGER` 常量与 1h 检查间隔
- 移除 `replace_group_items` 在同步路径的调用；`sync_group_now` 命令与分组「立即同步」入口一并移除（前端同步移除）

### R4 后端：commands 调整
- 新增 `sync_provider_now(provider_id)` 命令：立即同步单个供应商（供「立即同步」按钮）
- 新增 `get_provider_models(provider_id)` 命令：读本地持久化模型
- `fetch_provider_models` 保留（表单草稿预览 + 手动刷新兜底）
- 移除 `sync_group_now` 命令与 `list_groups` 中的 `source_provider_id` / `last_sync_at` 读取逻辑
- `delete_provider` 中「解绑分组 source_provider_id」逻辑保留（字段仍在，防孤儿引用）

### R5 前端：供应商页
- 表格新增「自动同步」列：`HSwitch` 就地切换（调 `setProviderAutoSync`），并显示「上次同步」时间（小字，无则「未同步」）
- 每行加「立即同步」操作按钮（调 `syncProviderNow`），同步中显示 loading
- 表格列定义调整：名称 / Base URL / 启用 / 自动同步 / 上次同步 / 操作

### R6 前端：分组页左侧
- `useProviderModelCache` 改为优先读本地 `get_provider_models`；展开时读本地 → 已存在则直接展示（状态 ready），否则实时拉取兜底
- 供应商条目上显示同步状态徽标（如「已同步 HH:mm」/「未同步」小字），沿用 HCell #suffix slot
- 移除 `isBound` 相关 UI：绑定供应商下拉、绑定态提示块、立即同步按钮、清空/拖拽禁用逻辑（分组队列恢复纯手动）

### R7 前端：类型与 API 层
- `src/api/tauri.ts`：`Provider` 类型加 `auto_sync` / `last_sync_at`；新增 `setProviderAutoSync` / `syncProviderNow` / `getProviderModels`；移除 `syncGroupNow`
- 清理 GroupFormPage 中 `isBound` 相关逻辑（可保留 `source_provider_id` 字段定义但不再用于 UI）

## Out of Scope

- 不改请求转发 / 故障转移逻辑
- 不做同步历史记录 / 增量同步（全量替换）
- 不迁移旧 `groups.source_provider_id` 数据
- 不引入新依赖

## Acceptance Criteria

- [ ] AC1：`cargo test`（后端全部测试，含新迁移/领域测试）全绿
- [ ] AC2：`pnpm typecheck` / `pnpm lint` / `pnpm test:unit` / `pnpm build` 全绿
- [ ] AC3：后台同步任务按供应商维度运行——开开关的供应商每 24h 拉取模型并持久化，关开关的不动
- [ ] AC4：供应商页可就地切换自动同步、可立即同步、显示上次同步时间
- [ ] AC5：分组页左侧读本地持久化模型（离线可用），仍可手动刷新；无「绑定供应商」残留 UI
- [ ] AC6：分组队列纯手动维护——无绑定态只读限制，排序/增删恢复正常交互
- [ ] AC7：迁移幂等——重复启动不报错，旧库（含已绑定 source_provider_id 数据）升级不丢数据

## Notes

- 复杂任务（后端 schema + 后台任务 + 双前端页）：需补 `design.md` + `implement.md` 再 `task.py start`。
- 分组绑定移除后 `groups.source_provider_id` 字段保留但不再写入——历史绑定数据静默失效，需在 changelog 说明。
