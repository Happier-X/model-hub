# Logging Guidelines

> 日志级别与内容约定。

---

## Levels

| 级别 | 用途 |
|------|------|
| error | 启动失败、崩溃、数据无法写入 |
| warn | 端口回退、优雅退出超时后强杀、配置缺省 |
| info | 侧车启动/停止、监听地址与端口、数据目录 |
| debug | 请求级细节（开发默认；发行版可关） |

---

## Rules

1. **结构化优先**：Rust 侧推荐 `tracing`（或项目统一框架）；字段含 `port`、`pid`、`data_dir`，不含密钥。
2. **敏感信息**：API Key、Authorization 只打前后缀或 hash，不打全文。
3. **侧车日志**：尽量落到数据目录下的文件，便于用户反馈；壳可暴露「打开日志目录」。
4. **默认发行级别**：`info`。

---

## What to Log (MVP)

- 应用 start / exit
- gateway start 命令摘要（可执行文件路径可记，参数中的密钥不行）
- health check 连续失败
- 绑定地址与端口

## What Not to Log

- 完整聊天内容（除非用户显式开启 debug 且文档说明）
- 上游响应体全文（默认）

## gateway-rust 请求日志（业务表 `request_logs`）

与 tracing 诊断日志分离：管理 UI 的「日志」页读的是 SQLite `request_logs`（migrate v2）。

| 项 | 约定 |
|----|------|
| 字段 | 对齐 UI `RelayLog`：`id, time(Unix 秒), request_model_name, channel_name, actual_model_name, input_tokens, output_tokens, use_time, cost, error` |
| 写入 | 非流式 chat 结束后；流式尽力（tokens 可 0）；路由失败 error 非空；**401 不写** |
| 禁止 | 完整 messages、客户端 Key、上游 channel_key |
| API | `GET /api/v1/log/list`（page_size≤100）、`DELETE /api/v1/log/clear`；管理 JWT；`{ data }` |
| cost | MVP 可固定 0；tokens 从非流式 JSON `usage` 尽力解析 |

---

## Anti-Patterns

- 用 `console.log` 在壳里当唯一诊断手段。
- 每个 token 一条 info 日志。
