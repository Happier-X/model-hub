# 技术设计：分组编辑对齐 octopus 交互

## 范围

- **层**：frontend（主）；backend **零改**（复用 `create_group` / `update_group` / `delete_group` / `fetch_provider_models`）。
- **入口页**：`src/pages/GroupsPage.vue`；可拆 `src/components/groups/*` 降低单文件体积。
- **不做**：增量 items API、mode/权重、Morphing、供应商页。

## 组件边界

| 单元 | 职责 |
|------|------|
| `GroupsPage.vue` | 列表加载、`groups`/`providers` 真源、对话框 open/create/edit、全局 error/message、榜单加载编排 |
| `GroupCard`（新建或页内区块） | 单卡展示；卡片内拖拽/删成员即时保存；删组二次确认；导出 Pi；打开编辑 |
| `GroupEditorForm`（新建或页内） | 双栏表单：元数据字段 + 左选模 + 右队列；`editingGroupId` 仍由页面持有 |
| 可选 `useProviderModelCache` | 按 `provider_id` 缓存模型 id 列表、fetching/error、ensure/refresh |

页面仍拥有：

- `editingGroupId: number | null`
- `dialogOpen` / `saving`（对话框提交）
- `form`（`@tanstack/vue-form`）或等价表单状态
- 卡片级 `cardSavingIds: Set<number>`（或 `Record<id, boolean>`）

## 数据流

### 卡片即时保存

```
用户拖拽/删成员
  → 本地乐观更新该卡展示 items（可选，失败则 refresh）
  → updateGroup({
       id, name, thinking_effort,
       source_provider_id,
       items: 新顺序的 { provider_id, upstream_model }[]
     })
  → 成功：用返回 Group 或 refresh() 对齐
  → 失败：error 文案 + refresh() 回滚；禁止保留假成功顺序
```

约束：

- 绑定分组（`source_provider_id != null`）不进入拖拽/删成员路径。
- 同一 `group.id` 保存中：禁用该卡拖拽/删成员/删组确认提交，避免交错全量写。
- 全量 `items` 替换：前端必须提交**完整**队列（含未改动项），顺序即 `sort_order`。

### 对话框保存

与现网一致：

```
form.handleSubmit
  → targetId = snapshot(editingGroupId)
  → mode = getGroupSaveMode(targetId)
  → createGroup | updateGroup
  → 成功：关对话框 + reset + refresh
  → 失败：保留 form + editingGroupId
```

### 左侧模型缓存（D4=L1）

```
providerModelsCache: Record<providerId, string[]>
providerModelsStatus: Record<providerId, 'idle'|'loading'|'error'|'ready'>

首次展开 providerId:
  if ready 且有缓存 → 仅展开 UI
  else → fetchProviderModels({ provider_id }) → 写入缓存

刷新:
  强制再 fetch 覆盖缓存

打开/关闭对话框:
  不自动 fetch；缓存可跨打开保留（同页会话），关闭不必清空（降低重复上游请求）
```

「全部加入」：对**当前已缓存**的该供应商模型，按 `provider_id + upstream_model` 去重 append 到 form items；未缓存时先提示展开或触发与展开相同的拉取后再加入（仍属用户动作）。

## UI 结构

### 卡片

```
[ 名称 | 思考标签 | 自动同步 ]
[ n 个模型 · 故障转移 ]
[ 可滚动队列 ]
  #  ⋮⋮  provider / model  [x]   ← 非绑定可拖可删
  #     provider / model         ← 绑定只读
[ 配置到 Pi | 编辑 | 删除 ]
  删除 → 头部覆盖：取消 | 确认删除
```

拖拽：MVP 可用现有 HTML5 DnD（与对话框一致）或 `@hello-pangea/dnd`；**优先复用页面已有 HTML5 拖拽**，避免新增依赖，除非手感不可接受再引入。

### 对话框（wide）

```
分组名 | 思考强度 | 绑定供应商
[ 绑定只读提示 + 立即同步 | 非绑定工具：按能力排序 / 刷新榜单 ]
┌ 左：供应商手风琴 + 搜索     ┐ ┌ 右：已选队列 + 清空          ┐
│ 展开 → 拉模型 / 刷新        │ │ 拖拽 / 删 / OpenRouter 分    │
│ 点选加入 / 全部加入         │ │                              │
└─────────────────────────────┘ └──────────────────────────────┘
[ 保存 | 取消 ]
```

- 去掉逐行「选供应商 + 手填 + 拉取模型 + 上移下移」旧布局。
- 去掉独立「批量添加供应商全部模型」条（D6）。
- 若 wide 宽度不够双栏，可在 `app-dialog-host--wide` 上微调 `max-width`（仅分组编辑使用时注意不影响供应商页，或加 `size="xl"`）。

## 兼容与回滚

- **API**：无 breaking change。
- **数据**：无 schema 迁移。
- **回滚**：还原 `GroupsPage.vue` / 新增 components 即可；无后端回滚。
- **风险**：卡片全量 replace 在快速连续拖拽时可能写穿——用 per-card saving 锁 + 以服务端返回为准。

## 测试策略

- 保留/扩展 `groupSaveMode` 单测。
- 可选：纯函数测「队列重排 payload 构造」「去重 key」「缓存 ensure 不重复请求」若抽出 util。
- 手测：非绑定拖/删、绑定只读、删组确认、双栏展开拉模、全部加入、排序不自动保存、保存失败保留编辑。

## 与 spec 关系

实现后若形成稳定约定，应回写：

- frontend `component-guidelines.md`：卡片内即时保存 + 双栏选模 + 展开拉模
- 引用 backend `upstream-access.md`（不弱化）
