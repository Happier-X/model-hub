# 渠道能力补强

## Goal

补强渠道管理体验：支持**编辑**名称 / Base URL / 上游模型 / 上游 Key，列表可**临时显示**脱敏 Key，完善 type=0 说明与删除确认，降低「建错只能删重建」成本。

## Background

- 当前渠道页：创建 + 启用/禁用 + 删除；列表仅脱敏展示，无法改配置。
- 侧车（dev 源码）提供 `POST /api/v1/channel/update`（`ChannelUpdateRequest`：字段指针 + `keys_to_add|update|delete`）与 `POST /api/v1/channel/enable`。
- **v0.9.28** 字段可能与 dev 有差异；实现前须在测试端口实测 update 最小 body。

## Requirements

### R1. 编辑渠道

- 列表每条提供「编辑」：可改 `name`、`base_url`（主 endpoint）、`model`。
- 上游 API Key：支持「轮换/覆盖」——输入新 Key 则更新（空则保持原 Key 不变）。
- 保存走侧车 update 接口；成功后刷新列表。

### R2. Key 可见性

- 列表默认脱敏；提供「显示/隐藏」切换（内存态，不写 localStorage）。
- 显示态仍可复制完整 Key（可选复制按钮）。

### R3. type 说明

- 创建表单旁固定说明：MVP / v0.9.28 使用数字 **type=0（OpenAI Chat）**；不要填字符串 type。
- `CHANNEL_TYPE_LABELS` 对 0 保持「OpenAI Chat」；未知数字仍显示「类型 N」。

### R4. 删除与启用体验

- 删除前二次确认（`window.confirm` 或内联确认即可）。
- 启用/禁用失败时展示错误（已有 setError 路径统一）。

### R5. 约束

- 不引入多端点/多 Key 完整矩阵 UI（单 Base URL + 单 Key 编辑即可）。
- 不提交真实密钥；冒烟脚本可不强制覆盖 update（可选扩展）。
- 仅 Windows 验收；清理策略仍只动测试端口。

## Out of Scope

- fetch-model / auto_sync / auto_group / 多 Base URL 延迟选择 UI。
- 渠道统计图表。
- 修改侧车二进制。

## Acceptance Criteria

- [x] AC1：可编辑已有渠道的 name / base_url / model 并保存成功。
- [x] AC2：可轮换上游 Key（填新 Key 保存后列表脱敏变化或探测成功）。
- [x] AC3：列表 Key 默认证脱敏，可切换显示完整 Key。
- [x] AC4：删除有确认；type=0 说明可见。
- [x] AC5：`pnpm lint` / `pnpm build` 通过；既有 smoke 不回归。

## Decisions Log

| # | 决策 | 选择 | 日期 |
|---|------|------|------|
| D1 | 编辑形态 | 列表内联/展开表单，非独立路由 | 2026-07-18 |
| D2 | 多 Key | 只管首条 Key 的轮换 | 2026-07-18 |
| D3 | 版本 | 以 v0.9.28 真机 update 行为为准 | 2026-07-18 |

## Notes

- 复杂任务：需 design + implement；用户批准后 `task.py start`。
