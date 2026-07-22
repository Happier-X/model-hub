# 分组批量添加供应商全部模型

## Goal

在分组编辑表单中选择一个供应商，一键拉取其全部模型并追加为队列条目，减少逐条添加。

## Requirements

- R1：故障转移队列标题区域新增批量供应商选择与「拉取并全部添加」。
- R2：调用既有 `fetchProviderModels({ provider_id })`。
- R3：按上游返回顺序追加 `{provider_id, upstream_model}`。
- R4：按 `(provider_id, upstream_model.trim())` 去重；已有条目和顺序不变。
- R5：空列表不改队列；显示中文结果（新增 N 条、跳过 M 条）。
- R6：失败不改队列；加载期间禁用重复点击。
- R7：仅改表单，用户仍须点「保存」才持久化。

## Acceptance Criteria

- [x] 可选择供应商并一键追加全部模型。
- [x] 重复执行不会产生重复条目。
- [x] 已有不同供应商同名模型不误去重。
- [x] 成功/空列表/错误有中文反馈。
- [x] `pnpm typecheck` / `lint` / `build` 通过。
