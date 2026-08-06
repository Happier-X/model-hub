# 分组编辑从 Modal 改为独立页面

## Goal

把分组新建/编辑从 `AppDialog` 宽弹窗迁到独立路由页面，让双栏选模、队列排序、绑定同步等重表单有足够空间；列表页只保留浏览与轻量操作。

## Background

- 当前入口在 `src/pages/GroupsPage.vue`：`openCreate` / `startEdit` 打开 `AppDialog`（`size="wide"`）。
- 弹窗内容已包含：分组名、思考强度、绑定供应商、立即同步、能力榜单排序、左供应商手风琴选模、右故障转移队列拖拽/删除。
- 列表卡片仍保留即时操作：`persistGroupItems`、删除、导出到 Pi；不依赖 modal。
- 路由目前只有 `/groups`；侧栏高亮用 `route.path` 精确匹配（`AppShell.vue`），子路径需可高亮「分组」。
- 无 `getGroup(id)` API，仅有 `listGroups`；编辑页需按 id 从列表定位。
- 仓库无路由级未保存守卫先例；现 modal 关闭即丢弃。
- 供应商页仍用 modal；本任务不同步改造。

## Decisions

| 决策 | 结论 |
|------|------|
| 覆盖范围 | 新建 + 编辑都进独立页 |
| 保存成功 | 回 `/groups` 列表 |
| 未保存离开 | 直接丢弃（取消/返回/切侧栏均不二次确认） |
| 路由 | `/groups/new` + `/groups/:id/edit` |

## Requirements

1. **R1 路由与导航**
   - 新增 `groups-new`：`/groups/new`（新建）
   - 新增 `groups-edit`：`/groups/:id/edit`（编辑，`id` 为数字）
   - 列表「新建」→ `/groups/new`；卡片「编辑」→ `/groups/:id/edit`
   - 侧栏在 `/groups` 及上述子路径时均高亮「分组」
   - 页面标题（`meta.title` / AppShell h1）区分：分组 / 新建分组 / 编辑分组

2. **R2 列表页瘦身**
   - `GroupsPage` 移除 `AppDialog` 及全部对话框表单状态与逻辑
   - 保留列表、卡片即时队列编辑、删除、导出 Pi、错误/成功提示

3. **R3 独立表单页**
   - 新建与编辑复用同一页面组件（按路由区分 create/update）
   - 表单能力与现 modal 对齐：名称、思考强度、绑定供应商、立即同步（仅编辑+绑定）、能力排序、左栏选模、右栏队列拖拽/删除/清空、保存/取消
   - 保存模式仍用稳定 `editingGroupId` + `getGroupSaveMode`（null→create，有 id→update）
   - 左栏模型加载合同不变：仅展开/刷新/全部加入时拉取，禁止挂载/打开预拉
   - 绑定分组：队列只读、左栏禁用；编辑态可「立即同步」
   - 排序只改表单、不自动保存（见 model-queue-sort）

4. **R4 完成与离开**
   - 保存成功 → `router.push('/groups')`
   - 取消 → `router.push('/groups')`
   - 未保存直接离开；不做 dirty 检查 / `beforeRouteLeave`
   - 保存中禁用关闭/重复提交（与现逻辑一致）

5. **R5 编辑加载与异常**
   - 进入编辑页：加载分组列表，按路由 id 定位；找不到则提示并回列表
   - 非法 id（非正整数）→ 提示并回列表
   - 保存失败保留表单与编辑态，便于重试

## Out of Scope

- 后端分组 API / 保存语义变更
- 供应商新建/编辑改独立页
- 未保存脏检查、二次确认
- 新增 `get_group` 后端命令（本轮用 `listGroups` 定位）
- GroupCard 即时编辑/删除/导出 Pi 产品行为变更（仅改编辑入口为路由跳转）

## Acceptance Criteria

- [ ] AC1：点击「新建分组」进入 `/groups/new` 全页表单，无 modal
- [ ] AC2：卡片「编辑」进入 `/groups/:id/edit`，表单预填该分组数据
- [ ] AC3：新建/编辑保存成功后回到 `/groups`，列表可见最新数据
- [ ] AC4：取消或未保存切走直接离开，无二次确认
- [ ] AC5：表单能力与迁移前 modal 对齐（选模、队列、绑定、同步、排序、保存模式）
- [ ] AC6：在新建/编辑子路径时侧栏「分组」仍高亮
- [ ] AC7：编辑非法/不存在 id 时有明确错误并回到列表
- [ ] AC8：`GroupsPage` 不再依赖 `AppDialog`；供应商页 modal 行为未改
- [ ] AC9：绑定分组队列只读；非绑定可拖拽/选模；左栏不预拉模型

## Notes

- 实现与检查前需更新 frontend spec 中「双栏选模对话框」相关表述（改为独立页）。
