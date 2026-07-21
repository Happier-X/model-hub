# 设计：Chat SSE 流式代理

## 变更点

1. `rewrite_upstream_body`：增加参数或新函数，**stream 路径保留 `stream: true`**；非流式仍可 strip 或保留 false。
2. `UpstreamClient::forward_chat_stream`：
   - `send()` 后若 status 非 success，读完 body 按非流式错误透传。
   - success：`response.bytes_stream()` → `StreamBody` / `Body::from_stream` 返回 axum Response。
3. `v1_chat`：去掉 `stream_not_supported`；按 stream 标志分支。
4. 更新单测与 `chat_forward` 集成测；新增 `chat_sse` 集成测。

## 依赖

- `reqwest` 已具备 stream（默认）；确认 features 含 stream 能力（`reqwest` 默认 body stream）。
- 可能需要 `futures-util` 精确版本做 `StreamExt` / 错误映射。
- axum 0.8：`Body::from_stream` 或 `StreamBody`。

## 响应头

- 透传 `content-type`；若上游缺失则 `text/event-stream; charset=utf-8`。
- 可选：`Cache-Control: no-cache`、`Connection: keep-alive`（若上游无则补充，利于部分客户端）。

## 超时

- 流式场景默认 timeout 对整响应可能过严：为 stream 使用更长 timeout 或 `timeout(None)` + connect timeout。
- 建议：`connect_timeout=10s`，`timeout` 对流式 builder 使用 `None` 或 300s；非流式保持 60s。可用两个 Client 或 per-request timeout。

## 测试策略

- wiremock 返回 `Content-Type: text/event-stream` body：
  ```
  data: {"id":"1","choices":[{"delta":{"content":"hi"}}]}

  data: [DONE]

  ```
- 客户端用 reqwest/bytes 收齐或分段 assert contains。

## 回滚

恢复 stream 拒绝分支即可。
