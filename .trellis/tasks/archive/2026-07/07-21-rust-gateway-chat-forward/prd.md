# Rust 网关 Chat 非流式转发

## Goal

在 `gateway-rust` 实现 OpenAI 兼容的 **非流式** `POST /v1/chat/completions`：按请求 `model`（**分组名**）解析分组 → 选择渠道与上游模型 → 转发到上游 `/chat/completions`，并让 `GET /v1/models` 返回已配置分组列表。

## Background

- 鉴权、SQLite、渠道/分组 CRUD 已完成。
- 产品约定：客户端 `model` 字段填**分组名**，不是上游模型 id。
- 无真实上游时允许业务错误，但**不得**鉴权 401（与 smoke 一致）。
- 本任务**不做** SSE 流式；`stream=true` 明确返回 400/501 JSON。

## Requirements

### R1. `/v1/models`

- 仍需客户端 API Key。
- `data` 列出当前**分组**（每项 OpenAI model 对象风格：`id`=分组名，`object`=`model`）。
- 无分组时可为 `[]`。

### R2. `/v1/chat/completions`（非流式）

- 客户端 Key 鉴权（Bearer / x-api-key）；管理 JWT 拒绝。
- 请求体至少含 `model` + `messages`（JSON 透传大部分字段到上游）。
- 解析 `model` 为分组名：
  - 找不到分组 → 404/400 + 可读 message（建议 404 或 400，稳定 JSON）。
  - 分组无 items → 业务错误。
- 路由：mode=1 轮询（按请求序号或简单原子计数选 item）；其它 mode 本任务可先按首个可用 item 或同样轮询，至少不 panic。
- 加载 item 绑定的渠道：需 enabled；取首个 enabled base_url 与首个 enabled channel_key。
- 上游 URL：`{base_url 去尾斜杠}/chat/completions`（若 base 已含 `/v1` 则拼 `/chat/completions`）。
- 上游 Header：`Authorization: Bearer {channel_key}`；`Content-Type: application/json`；可合并渠道 `custom_header`（若为对象数组则尽力应用，失败忽略单条）。
- 改写发往上游的 JSON：`model` 换为 item.`model_name`；若存在 `stream: true` 则**不转发**，直接返回网关错误「本版本不支持流式」。
- 将上游状态码与 body **透传**（非流式 JSON）；上游网络失败 → 502 + message。
- 日志不打印完整客户端 Key / 上游 Key / 用户 messages 全文（可记 group/channel id）。

### R3. 超时与依赖

- HTTP 客户端超时可配置，默认 60s。
- 使用 `reqwest`（rustls 或默认 tls）。

### R4. 边界

- 不实现计费、日志落库、故障转移高级策略。
- 不改 Tauri/前端/发布。
- SSE 流式留给后续子任务。

## Acceptance Criteria

- [x] AC1：`/v1/models` 在有效 Key 下返回分组名列表。
- [x] AC2：无 Key / 坏 Key / 管理 JWT 访问 chat → 401。
- [x] AC3：mock 上游时，按分组名 chat 可转发并得到上游 JSON 200。
- [x] AC4：未知分组名 / `stream=true` 返回稳定非 401 业务错误。
- [x] AC5：轮询 mode=1 在多 item 时切换（可用单测验证选择逻辑）。
- [x] AC6：fmt/check/test/clippy 通过；不改壳发布链路。

## Out of Scope

- SSE / stream 代理
- 请求日志表
- octopus 数据迁移
- 壳切换网关实现
