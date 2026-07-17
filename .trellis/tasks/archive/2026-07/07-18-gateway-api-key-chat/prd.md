# 网关 API Key 与 Chat 可验收

## Goal

在 Model Hub 管理 UI 中管理侧车 **客户端网关 API Key**（`sk-octopus-...`），并更新客户端文档与冒烟脚本，使本机 OpenAI 兼容路径（`/v1/models` 等）在**不依赖真实上游 Key** 的前提下可验证鉴权闭环；具备真实上游时用户可自行完成 Chat 转发。

## Background

- M1 已完成：侧车启停、静默管理 JWT、渠道/分组/日志。
- 真机 **octopus v0.9.28** 上 `/v1/*` 走 `APIKeyAuth`：必须使用侧车签发的网关 Key（前缀 `sk-octopus-`），与管理 JWT 不是同一套。
- 管理 API：`/api/v1/apikey/create|list|update|delete/:id`（需 Bearer 管理 Token）。
- 创建时服务端生成 `api_key`；请求体至少需要 `name`（及可选 `enabled` / `expire_at` / `max_cost` / `supported_models`）。

## Requirements

### R1. 管理台 API Key 能力

- 新增导航页 **「API 密钥」**（或等价命名）：列表、创建、删除；启用状态可读（更新可做最小支持：改 name/enabled）。
- 创建成功后**完整展示一次** Key 明文，并提供复制；列表可对 Key 脱敏展示（与渠道 Key 一致风格）。
- 仅在网关 `running` 且管理鉴权成功时可用（复用 `GatewayGate`）。

### R2. 设置页 / 客户端提示修正

- 设置页与对接提示中删除「本机免鉴权、api_key 可填任意占位」的误导表述。
- 展示：`Base URL`、`/v1`、**须使用网关 API Key**；引导用户到 API 密钥页创建。

### R3. 文档

- 更新 `docs/client-integration.md`、`docs/mvp-acceptance.md`、`gateway/README.md`、`README.md` 相关段落：JWT vs 客户端 Key、创建步骤、示例 curl / OpenAI SDK。

### R4. 冒烟与验收

- 扩展 `scripts/e2e-octopus-smoke.py`（或等价）：登录 → 创建 apikey → 用该 Key 访问 `GET /v1/models`（期望非 401；空模型列表可接受）→ 可选探测 `POST /v1/chat/completions` 在无分组/无上游时的**可预期错误**（非鉴权失败）。
- 不强制真实上游 Chat 成功（无供应商 Key 时无法保证 200）。

### R5. 约束

- 不引入管理登录页；不提交真实 Key / 上游密钥。
- 清理进程策略：仅测试端口/PID，不按进程名杀全机 octopus。
- 兼容钉扎版本 **v0.9.28** 字段行为（若 create body 与 dev 源码有差异，以真机实测为准）。

## Out of Scope

- 完整 Dashboard 统计图、价格、models.dev。
- 强制「本机免客户端 Key」改造侧车（不 patch octopus 二进制）。
- 真实供应商 Key 的 Chat 成功率 SLA。
- macOS/Linux 验收。

## Acceptance Criteria

- [x] AC1：UI 可创建网关 API Key 并展示完整 `sk-octopus-...` 明文 + 复制。
- [x] AC2：列表可看到已创建 Key（脱敏）；可删除。
- [x] AC3：使用该 Key 调用 `GET {base}/v1/models` 返回非 401（200 或业务空列表等均可）。
- [x] AC4：文档与设置页正确区分管理 JWT 与客户端 Key。
- [x] AC5：`pnpm lint` / `pnpm build` / `cargo test` 通过；冒烟脚本通过（独立测试端口）。

## Decisions Log

| # | 决策 | 选择 | 日期 |
|---|------|------|------|
| D1 | 导航入口 | 独立页「API 密钥」 | 2026-07-18 |
| D2 | Chat 验收深度 | 鉴权闭环必须；真实上游 Chat 可选 | 2026-07-18 |
| D3 | 侧车版本 | 继续钉扎 v0.9.28 | 2026-07-18 |

## Notes

- 本任务为独立交付，不挂父任务树。
- 实现前需用户确认本 PRD + design/implement 后 `task.py start`。
