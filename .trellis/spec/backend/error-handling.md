# Error Handling

> 错误如何产生、传播与展示。

---

## Layers

| 层 | 策略 |
|----|------|
| Rust 壳 / IPC | `Result` + `InvokeError`；前端 toast 展示 message |
| 内嵌代理 HTTP | JSON / 透传上游状态；401 无有效客户端 Key |
| 端口占用 / 绑定失败 | `last_error` 可行动文案 |

---

## Rules

1. 用户可见错误须可行动（改端口、检查供应商 URL/Key 等）。
2. **不要**把完整上游 Key、客户端 Key、完整 messages 放进错误或日志。
3. 代理未运行时，UI 明确「未运行」，禁止假装空数据成功。
4. 故障转移：可重试错误换源；明确不可重试 4xx **不**因业务 400 记熔断失败（实现细节见 `proxy/forward.rs`）。

---

## Tauri invoke 约定

- 命令：`snake_case`。
- 成功：结构化 JSON。
- 失败：可序列化 message。

---

## Anti-Patterns

- 生产路径 `unwrap` 进程管理。
- UI 只显示「未知错误」。
