# 设计：分组新建/编辑独立页

## 边界

| 层 | 改动 |
|----|------|
| 路由 | 增 `/groups/new`、`/groups/:id/edit` |
| 页面 | 新 `GroupFormPage.vue`；`GroupsPage.vue` 瘦身为列表 |
| 外壳 | `AppShell` 侧栏 active 匹配改为前缀/归一化，使子路径高亮「分组」 |
| 组件 | `GroupCard` 编辑事件由父级改为 `router.push`，卡片本身可仍 emit `edit` |
| API | 不改；编辑用 `listGroups` 按 id 查找 |
| 工具 | 继续用 `getGroupSaveMode` |

## 组件拆分

```text
GroupsPage.vue          # 列表 + 卡片操作（无 dialog）
GroupFormPage.vue       # 新建/编辑共用表单页（从现 GroupsPage dialog 逻辑迁出）
GroupCard.vue           # 不变（父级改跳转）
AppShell.vue            # 侧栏 model-value / 高亮逻辑
router/index.ts         # 新路由
```

可选后续再抽 `components/groups/GroupForm.vue`；本轮优先整页迁移，避免过度拆分阻塞交付。若 `GroupFormPage` 过大，实现时可在同目录内再拆展示子块，但状态真源仍在表单页。

## 路由合同

| name | path | 组件 | meta.title |
|------|------|------|------------|
| `groups` | `/groups` | GroupsPage | 分组 |
| `groups-new` | `/groups/new` | GroupFormPage | 新建分组 |
| `groups-edit` | `/groups/:id/edit` | GroupFormPage | 编辑分组 |

注意：`/groups/new` 必须注册在 `/groups/:id/edit` 之前，或 `id` 用自定义正则排除 `new`，避免 `new` 被当成 id。

推荐：

```ts
{ path: "/groups/new", name: "groups-new", component: GroupFormPage, meta: { title: "新建分组" } },
{ path: "/groups/:id(\\d+)/edit", name: "groups-edit", component: GroupFormPage, meta: { title: "编辑分组" } },
{ path: "/groups", name: "groups", component: GroupsPage, meta: { title: "分组" } },
```

## 页面模式判定

```ts
const route = useRoute();
const isCreate = computed(() => route.name === "groups-new");
const editingGroupId = computed(() => {
  if (isCreate.value) return null;
  const n = Number(route.params.id);
  return Number.isInteger(n) && n > 0 ? n : null;
});
// save: getGroupSaveMode(editingGroupId.value)
```

## 数据流

### 新建

1. 进入 `/groups/new` → `reset` 空表单 + `listProviders`（及按需 leaderboard）
2. 用户编辑 → submit → `createGroup` → `router.push({ name: "groups" })`

### 编辑

1. 进入 `/groups/:id/edit`
2. 并行 `listGroups` + `listProviders`
3. 用 id 找分组；缺失 → 设 error，push 回列表（或页内错误 + 返回按钮，推荐直接回列表并依赖列表 error 难；更清晰是表单页展示错误 +「返回列表」按钮，**同时**自动 `replace` 回列表也可）
4. `form.reset(entity)`，`editingGroupId` 来自路由
5. submit → `updateGroup({ id, ... })` → push 列表
6. 绑定「立即同步」：`syncGroupNow(id)` 后用返回值 `form.reset`

**推荐异常 UX**：找不到分组时页内展示错误文案 + 主按钮「返回列表」，不自动闪回（避免用户看不清原因）。非法路由若被正则拦下则自然 无匹配——若无 catch-all，未匹配路由保持现状；`(\\d+)` 已排除非数字。

### 列表入口

```ts
// GroupsPage
router.push({ name: "groups-new" });
router.push({ name: "groups-edit", params: { id: String(g.id) } });
```

## 侧栏高亮

现状：`HSidebar :model-value="route.path"`，items key 为 `"/groups"`。

方案：计算 `activeNavKey`：

- 若 `route.path === "/groups" || route.path.startsWith("/groups/")` → `"/groups"`
- 其它 nav 仍精确匹配 `route.path`

`@update:model-value` 仍 `router.push(key)`，点「分组」回列表。

## 布局与滚动

- 表单页内容多：沿用 AppShell 右主区整体滚动即可（不必强制「内部双栏锁高」）
- 双栏区域可用 `min-h` + 各栏 `max-h`/`overflow-y-auto` 保持可操作，参考现 dialog 内结构放大到全宽
- 顶部可放简短说明 +「返回列表」次要操作；主操作仍是底部保存/取消

## 状态归属

| 状态 | 位置 |
|------|------|
| groups 列表、卡片保存、导出 | GroupsPage |
| 表单 values、选模缓存、榜单、saving | GroupFormPage |
| 无跨页共享 form store | — |

## 兼容性

- 不改 invoke payload / 后端
- 不改 `groupSaveMode` 语义
- `AppDialog` 仍被供应商页使用，不可删除组件
- 既有测试：`groupSaveMode.test.ts` 仍有效；若有 GroupsPage 相关 e2e/组件测需更新选择器（当前以 unit 为主）

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| `GroupsPage` 迁移时漏搬逻辑 | 以现 dialog 区块整段搬迁；checklist 对照 R3 |
| 编辑页 id 与列表不同步 | 进入时拉 listGroups；保存失败不离页 |
| 侧栏高亮回归 | 专用 activeNavKey + 手动点验 `/groups/new` |
| 规范文档过时 | 完成实现后更新 `component-guidelines` §17、`directory-structure` |

## 回滚

- 恢复路由与 `GroupsPage` dialog 即可；无数据迁移
- 单 commit 或清晰文件边界便于 `git revert`
