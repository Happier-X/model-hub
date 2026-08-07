# 按模型能力排序后自动保存（编辑态落库，留在页面）

## Goal

点击「按模型能力排序」成功后，编辑态（已有分组）自动将新队列顺序保存到后端（`updateGroup`），并**留在当前页**供继续拖拽微调；新建态保持现状（仅改内存顺序，提示保存后生效）。

## Background

- 当前 `sortQueueByCapability` 只改前端 `formValues.items` 顺序，提示「点击保存后生效」——用户重开页面恢复数据库原顺序（本任务不解决「打开不加载榜单导致全未匹配」的展示问题，用户已明确只做 2）。
- `onSubmit`（表单提交）保存成功后会 `router.push({ name: "groups" })` 跳走——自动保存**不能**复用 onSubmit，需独立保存函数留在页面。

## Decisions

| # | 决策 | 结论 |
|---|------|------|
| 1 | 编辑态自动保存 | 排序成功且 `isEditing` → 调 `updateGroup` 落库（payload 与 onSubmit 一致：name/thinking_effort/过滤后 items） |
| 2 | 不跳转 | 自动保存成功后留在页面，`formMessage` 提示「已保存，可继续拖拽微调」；不 `router.push` |
| 3 | 新建态 | 排序仍只改内存，提示「保存后生效」（避免误创建空分组） |
| 4 | 失败处理 | 自动保存失败 → `error.value` 展示错误，队列顺序保持（不回滚内存排序，用户可再点保存） |
| 5 | 并发 | 复用 `saving` 防重入：排序/保存进行中禁用按钮 |

## Requirements

### R1 `sortQueueByCapability` 改造
- 排序成功且顺序变化后：
  - 编辑态 → `await autoSaveAfterSort()`（updateGroup + formMessage「已保存…」）
  - 新建态 → 现状提示
- 顺序未变化分支提示不变

### R2 独立保存函数
- `async function autoSaveAfterSort(): Promise<boolean>`：构建 payload（复用 onSubmit 逻辑），`updateGroup({ id: editingGroupId.value, ...payload })`；成功返回 true + formMessage；失败 `error.value` + 返回 false
- 不跳转、不 `saving` 全流程（或仅短暂置 saving 防重入）

### R3 验证
- typecheck / lint / test:unit / build 全绿；后端零改动

## Out of Scope

- 不解决「打开页面全未匹配」展示问题（用户选 2 不做 1）
- 不改 onMounted / 榜单加载时机
- 新建态不做自动创建

## Acceptance Criteria

- [ ] AC1：编辑态排序成功 → 队列顺序落库；留在页面；提示「已保存，可继续拖拽微调」
- [ ] AC2：重开页面（编辑态）顺序为上次排序结果（含未匹配沉底顺序）
- [ ] AC3：新建态排序后仍提示「保存后生效」，不自动创建分组
- [ ] AC4：自动保存失败显示错误，不跳转不崩
- [ ] AC5：typecheck/lint/unit/build 全绿
