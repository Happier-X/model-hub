# 修复编辑分组时误新增分组

## Goal

先点已有分组「编辑」（路径 A），再向其故障转移队列添加另一个供应商模型并保存时，只更新原分组，不得额外创建新分组。

## Confirmed Reproduction

1. 在分组列表点击原分组「编辑」。
2. 在编辑表单添加另一个供应商的模型（单条或批量）。
3. 点击保存。
4. 实际出现额外新分组。

## Code Facts

- `save()` 目前以 `editing.value` 对象是否存在决定 update/create。
- `startEdit(g)` 设置 `editing.value = g`；后端 `update_group` 本身只 UPDATE + replace items，不 INSERT groups。
- 因此要么编辑态在交互中丢失导致 `createGroup`，要么用户看到的「新增分组」并非数据库 groups 插入；需要测试与更明确的状态合同定位。

## Requirements

- R1：编辑目标用稳定的 `editingGroupId: number | null` 表达，不依赖列表对象引用。
- R2：进入编辑后，添加条目/拉模型/批量添加/排序/健康刷新不得清空编辑 id。
- R3：编辑态保存只允许 `updateGroup(editingGroupId)`；明确新建态才允许 `createGroup`。
- R4：保存期间禁用重复提交；按钮文案为「保存修改」/「创建分组」。
- R5：后端回归测试：update 已有分组加入第二供应商模型后 groups 数量不变、items 增加。
- R6：前端增加可测试的保存模式判定（提取纯函数或组件测试）；至少覆盖编辑 id 不受异步操作影响。
- R7：不自动对上游测活；拉模型仍仅点击。

## Open Question

- 误新增的分组名称是否与原分组**相同**，还是变成了新增的**模型名/其他名称**？

## Acceptance Criteria

- [ ] 编辑已有分组 + 添加第二供应商模型 + 保存：分组总数不变
- [ ] 原分组 items 包含新增模型
- [ ] 编辑态永不走 create IPC
- [ ] 明确新建态仍可创建
- [ ] 防重复提交
- [ ] typecheck/lint/cargo test/相关单测通过
