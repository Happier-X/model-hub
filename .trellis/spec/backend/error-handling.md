# Error Handling

> 错误如何产生、传播与展示。

---

## Layers

| 层 | 策略 |
|----|------|
| Rust 壳 / IPC | `Result` + `InvokeError`；前端 toast 展示 message |
| 内嵌代理 HTTP | JSON / 透传上游状态；**不**因客户端 Key 返回 401 |
| 端口占用 / 绑定失败 | 启动时从首选端口起最多扫描 50 个可用端口并自动改口写入 `shell.json`；仍失败则 `NoAvailablePort` / `last_error` 可行动文案；**不**结束占用进程 |

---

## Rules

1. 用户可见错误须可行动（改端口、检查供应商 URL/Key 等）。
2. **不要**把完整上游 Key、完整 messages 放进错误或日志。
3. 代理未运行时，UI 明确「未运行」，禁止假装空数据成功。
4. 故障转移：可重试错误换源；明确不可重试 4xx **不**因业务 400 记熔断失败（实现细节见 `proxy/forward.rs`）。
5. 流式：首包前超时/失败可换源；**首 chunk 已提交后**静默超时只终止当前流并记供应商失败，**禁止**拼接第二家响应。
6. 熔断 HalfOpen：同一 provider 同时最多 **一个**恢复探测位，且只能挂在**真实业务请求**上；其余请求跳过该 provider（见 `proxy/circuit.rs` `probe_in_flight`）。**禁止**为测活/恢复而单独请求上游（见 [upstream-access.md](./upstream-access.md)）。
7. 默认超时：首包 60s、流式静默 120s、非流式 600s（`STREAM_*` / `NON_STREAM_TIMEOUT`）。
8. OpenRouter 公共榜单：请求超时 15s、固定 URL、无 Key；网络失败有缓存返回 stale，无缓存才报可行动错误（见 `model-leaderboard.md`）。
9. **禁止**对用户供应商做自动测活、定时 health、预热、空 chat；AI 联调默认不打用户上游。

---

## Tauri invoke 约定

- 命令：`snake_case`。
- 成功：结构化 JSON。
- 失败：可序列化 message。

---

## Anti-Patterns

- 生产路径 `unwrap` 进程管理。
- UI 只显示「未知错误」。
