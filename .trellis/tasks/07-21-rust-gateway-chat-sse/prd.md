# Rust 网关 Chat SSE 流式

## Goal

在 `gateway-rust` 为 `POST /v1/chat/completions` 增加 **`stream=true` SSE 代理**：鉴权与分组路由与非流式一致，将上游 `text/event-stream` 字节流透明转发给客户端。

## Background

- 非流式转发已完成；当前 `stream=true` 返回 `400 STREAM_NOT_SUPPORTED`。
- 客户端（OpenAI SDK / 各类桌面客户端）依赖 SSE 帧格式（`data: {...}` + 最终 `data: [DONE]`）。
- 产品约定仍为：请求 `model` = **分组名**；上游 `model` = item.`model_name`。

## Requirements

### R1. 流式请求

- `stream=true`（或缺省 false 走非流式）时：
  - 仍需客户端 API Key；管理 JWT 拒绝。
  - 同样按分组路由、改写上游 model。
  - 请求体保留 `stream: true` 发给上游（不再 strip stream）。
- 上游成功时响应：
  - 状态码透传（通常 200）
  - `Content-Type: text/event-stream`（优先上游，缺省则设标准值）
  - body 为字节流代理，不整包缓冲完整 SSE 后再发。
- 上游非 2xx：可将错误 body 透传（可整包读错误 JSON）；不得伪装 401。

### R2. 客户端断开

- 客户端取消/断开时停止读取上游（best-effort），不泄漏 panic。
- 日志不打印完整 Key 与 messages 全文。

### R3. 非流式兼容

- `stream` 缺省或 `false` 行为与现网非流式完全一致。
- 删除/替换 `STREAM_NOT_SUPPORTED` 路径。

### R4. 测试

- wiremock（或等价）模拟 SSE 上游：多帧 + `[DONE]`。
- 集成测：stream=true 收到 event-stream 内容；鉴权失败 401；未知分组非 401。
- 单测：rewrite 在 stream 路径保留 `stream:true`。

### R5. 边界

- 不实现流式请求日志落库、计费。
- 不改 Tauri/前端/发布。
- 不强制解析/改写每一帧 JSON（透明代理即可）。

## Acceptance Criteria

- [x] AC1：`stream=true` + 有效 Key + mock 上游 → 200 且 body 含 SSE 数据与 `[DONE]`（或 mock 等价帧）。
- [x] AC2：`stream=false`/缺省仍走非流式整包 JSON。
- [x] AC3：无 Key / 坏 Key / 管理 JWT → 401。
- [x] AC4：未知分组 → 非 401 业务错误。
- [x] AC5：流式路径不再返回 `STREAM_NOT_SUPPORTED`。
- [x] AC6：fmt/check/test/clippy 通过；不改壳发布。

## Out of Scope

- 逐 token 改造/计量
- WebSocket
- 壳切换网关
