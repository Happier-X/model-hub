# 分组体验补强

## Goal

提升分组页可用性：列表清晰展示**绑定渠道与 model_name**；删除需确认；支持**编辑**分组名与成员绑定（换渠道 / 改上游 model_name），与渠道页编辑体验对齐。

## Background

- 创建时已能选渠道并写入 `items`；列表仅显示模式与成员数量，看不清绑了谁。
- 删除无确认；无法改名或换绑渠道，只能删重建。
- 侧车提供 `POST /api/v1/group/update`（`GroupUpdateRequest`：name/mode 指针 + `items_to_add|update|delete`）。
- **v0.9.28** 行为以实现前真机探测为准。

## Requirements

### R1. 列表可读

- 展示：分组名、模式（轮询等）、**每个 item：渠道名（或 #id）+ model_name**。
- 渠道名通过当前 `listChannels` 映射 `channel_id`；找不到时显示 `#id`。

### R2. 删除确认

- 删除前 `window.confirm`（或等价），文案含分组名。

### R3. 编辑

- 可改：分组 `name`、绑定的主渠道（MVP 按**首条 item** 管理）、该 item 的 `model_name`。
- 换渠道策略（MVP）：
  - 若已有 item：优先 `items_to_delete` 旧 id + `items_to_add` 新绑定；若探测证明可用再固化。
  - 或探测成功的整包策略。
- 仅改名时可用最小 `{id, name}`。
- 保存成功后刷新列表。

### R4. 文案

- 强调：客户端 `model` = **分组名**；`model_name` = 上游模型。
- 无渠道时继续引导去「渠道」页。

### R5. 约束

- 不做多 item 权重/优先级高级编辑矩阵（单成员绑定即可）。
- 不改侧车二进制；不强制扩展 smoke 覆盖 update。
- 仅测试端口探测；不按进程名杀全机 octopus。

## Out of Scope

- 多渠道加权/故障转移模式切换 UI（mode 保持轮询 1，可读展示即可）。
- 正则 match_regex 编辑。
- Dashboard 统计。

## Acceptance Criteria

- [x] AC1：列表展示绑定渠道标识与 model_name。
- [x] AC2：删除有确认。
- [x] AC3：可编辑分组名并保存成功。
- [x] AC4：可更换绑定渠道或 model_name 并保存成功（以真机 update 为准）。
- [x] AC5：`pnpm lint` / `pnpm build` 通过；既有 smoke 不回归。

## Decisions Log

| # | 决策 | 选择 | 日期 |
|---|------|------|------|
| D1 | 成员模型 | MVP 单 item（首条）编辑 | 2026-07-18 |
| D2 | 换绑策略 | 真机探测后固化 delete+add 或等价 | 2026-07-18 |
| D3 | 负载模式 | 保持创建为轮询；列表只读展示 | 2026-07-18 |

## Notes

- 需 design + implement；用户批准后 `task.py start`。
