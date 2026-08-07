# 自动同步开关迁移：供应商页 → 分组表单页「可选模型」

## Goal

自动同步开关（`auto_sync`）从供应商管理页（ProvidersPage）迁移到分组表单页（GroupFormPage）左栏「可选模型」的每个供应商行上；供应商页移除自动同步相关 UI（表格列、行内开关逻辑、相关状态）。

## Background

- 自动同步是**供应商维度**字段（`providers.auto_sync`），当前 UI 在供应商页：表格「自动同步」列 + 行内 HSwitch（乐观更新 → `setProviderAutoSync` IPC）。
- 「可选模型」= 分组表单页左栏 HCard：按供应商手风琴（HCell），展开时 `modelCache.ensure` 拉取模型供勾选；行右侧当前显示模型数或 `last_sync_at` 同步状态。
- 后端 `set_provider_auto_sync` / `list_providers`（返回 `auto_sync`/`last_sync_at`）**无需改动**——仅前端 UI 位置迁移。
- 供应商页新建/编辑表单**不含** auto_sync 字段（仅默认值 `auto_sync: true`，提交时透传）——保留默认值语义，不改后端。

## Decisions

| # | 决策 | 结论 |
|---|------|------|
| 1 | 开关位置 | 分组表单页「可选模型」每个供应商行（HCell suffix 区，模型数/同步状态旁）加 HSwitch 自动同步开关 |
| 2 | 交互 | 复用供应商页行内开关逻辑：乐观更新 → `setProviderAutoSync` → 以返回值为准同步 → 失败回滚 + 报错；进行中 id 防重复点击 |
| 3 | 供应商页清理 | 移除「自动同步」表格列、`toggleProviderAutoSync`、`autoSyncTogglingIds`、`setProviderAutoSync` 导入；`ProviderFormValues.auto_sync` 保留（提交透传默认值） |
| 4 | 新建表单默认值 | 保持 `auto_sync: true`（新建默认开），不展示字段 |
| 5 | 触发范围 | 开关点击**不**展开手风琴、不触发拉取模型；仅切换 auto_sync 状态 |

## Requirements

### R1 分组表单页（GroupFormPage.vue）
- 供应商行 suffix 区：HSwitch 绑定 `p.auto_sync`，disabled 条件 = 切换进行中
- 新增 `autoSyncTogglingIds` + `toggleProviderAutoSync(p, next)`（逻辑复制自供应商页）
- 开关与展开点击互不干扰（开关 stopPropagation）

### R2 供应商页（ProvidersPage.vue）
- 移除表格列 `auto_sync`、行内开关模板、`toggleProviderAutoSync`、`autoSyncTogglingIds`、`setProviderAutoSync` 导入
- `defaultFormValues.auto_sync: true` 与提交透传保留

### R3 验证
- typecheck / lint / test:unit / build 全绿；后端不动（无 Rust 改动）

## Out of Scope

- 不改后端 IPC / DB / 自动同步后台任务
- 不改新建供应商表单字段
- 不做「可选模型」页自动同步开关之外的 UI 重排

## Acceptance Criteria

- [ ] AC1：分组表单页「可选模型」每个供应商行显示自动同步开关；切换成功（服务端返回值同步）、失败回滚并提示
- [ ] AC2：供应商页不再出现「自动同步」列与开关；新建供应商仍默认 auto_sync=true
- [ ] AC3：开关点击不触发展开/拉取模型
- [ ] AC4：typecheck/lint/unit/build 全绿
