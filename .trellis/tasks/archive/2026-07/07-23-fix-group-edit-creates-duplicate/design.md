# 技术设计

## 根因假设

同名新分组说明最终至少有一次 `create_group` 插入路径被执行，或后端/前端状态在保存时把编辑目标视为新建。当前 `editing` 是对象引用，保存分支依赖其真值，状态合同不够明确；本次改为稳定 id 并让 UI 明确显示保存模式。

## 前端状态合同

```ts
const editingGroupId = ref<number | null>(null)
const isEditing = computed(() => editingGroupId.value !== null)
```

- `startEdit(g)`：只设置 `editingGroupId.value = g.id`，复制表单数据。
- `resetForm()`：清空 `editingGroupId`，进入新建态。
- 添加条目、批量拉取、排序、刷新健康不得改 `editingGroupId`。
- `save()`：先快照 `editingGroupId.value`；保存期间 `saving=true`，拒绝重复调用。
  - id 非空：只调用 `updateGroup({ id, ...payload })`。
  - id 为空：才调用 `createGroup(payload)`。
- 保存成功后 reset；失败保留编辑态和表单，方便重试。

## 后端防回归

`update_group` 继续只更新指定 group id 和 group_items，不改变 groups 行数。补领域测试：

1. 创建两个供应商与一个分组。
2. 调用 `update_group`，提交原 id + 原条目和另一供应商条目。
3. `list_groups().len()` 仍为 1。
4. 原分组 items 为 2，id/name 不变。

## UI

- 标题：编辑态「编辑分组」，新建态「新建分组」。
- 主按钮：编辑态「保存修改」，新建态「创建分组」。
- 保存期间按钮 disabled，避免重复 IPC。
- 可在编辑表单显示「正在编辑：<name>」，降低同名误解。

## 兼容与边界

- 明确新建仍调用 `createGroup`。
- 改名仍由 `updateGroup` 处理，唯一约束错误仍保留。
- 不自动请求上游；模型拉取仅用户点击。
