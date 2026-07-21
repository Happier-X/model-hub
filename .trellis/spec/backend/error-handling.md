# Error Handling

> 错误如何产生、传播与展示。

---

## Layers

| 层 | 策略 |
|----|------|
| Rust 壳 | `Result` + 本项目错误枚举；`invoke` 返回可序列化错误消息给前端 |
| 侧车进程 | 非 0 退出码 + stderr；壳捕获并展示「启动失败」原因 |
| HTTP 管理/转发 API | 稳定 JSON 错误体（对齐侧车）；前端统一解析 |
| 端口占用 / 权限 | 明确用户可读文案，不吞掉底层错误 |

---

## Rules

1. **用户可见错误**必须可行动（例如：端口被占用 → 提示改端口或结束占用进程）。
2. **不要**把完整上游 API Key、Cookie、管理 JWT、客户端 `sk-octopus-...` 放进错误消息或日志。
3. 侧车未就绪时，管理 UI 显示「网关未运行」，禁止假装空数据成功。
4. 健康检查失败与「进程已死」区分：僵尸进程要能 stop 再 start。
5. **gateway-rust 鉴权错误**：401 响应 JSON 必须包含前端可提取的顶层 `message`（`gatewayHttp.extractErrorMessage` 优先读该字段），并可同时提供 `error.code` / `error.message`。管理成功响应使用 `{ "data": T }` 信封。

---

## Tauri invoke 约定

- 命令命名：`snake_case`（如 `gateway_start`、`gateway_status`）。
- 成功：返回结构化状态（running、port、pid、data_dir）。
- 失败：返回字符串或 `{ code, message }`；前端 toast/横幅展示 `message`。

---

## Anti-Patterns

- `unwrap()` 用在生产路径的进程管理上。
- 仅 `println!` 失败原因、UI 只显示「未知错误」。
- 监听 `0.0.0.0` 失败时静默回退且不提示安全含义。
