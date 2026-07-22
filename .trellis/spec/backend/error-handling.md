# Error Handling

> 错误如何产生、传播与展示。

---

## Layers

| 层 | 策略 |
|----|------|
| Rust 壳 / IPC | `Result` + `InvokeError`；前端 toast 展示 message |
| 内嵌代理 HTTP | JSON / 透传上游状态；携带无效客户端 Key 时 401（无 Key 本机默认可放行） |
| 端口占用 / 绑定失败 | `last_error` 可行动文案 |

---

## Rules

1. 用户可见错误须可行动（改端口、检查供应商 URL/Key 等）。
2. **不要**把完整上游 Key、客户端 Key、完整 messages 放进错误或日志。
3. 代理未运行时，UI 明确「未运行」，禁止假装空数据成功。
4. 故障转移：可重试错误换源；明确不可重试 4xx **不**因业务 400 记熔断失败（实现细节见 `proxy/forward.rs`）。
5. 流式：首包前超时/失败可换源；**首 chunk 已提交后**静默超时只终止当前流并记供应商失败，**禁止**拼接第二家响应。
6. 熔断 HalfOpen：同一 provider 同时最多 **一个**恢复探测；其余请求跳过该 provider（见 `proxy/circuit.rs` `probe_in_flight`）。
7. 默认超时：首包 60s、流式静默 120s、非流式 600s（`STREAM_*` / `NON_STREAM_TIMEOUT`）。

---

## Tauri invoke 约定

- 命令：`snake_case`。
- 成功：结构化 JSON。
- 失败：可序列化 message。

---

## Anti-Patterns

- 生产路径 `unwrap` 进程管理。
- UI 只显示「未知错误」。
