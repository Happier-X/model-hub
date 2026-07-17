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

---

## Anti-Patterns

- 用 `console.log` 在壳里当唯一诊断手段。
- 每个 token 一条 info 日志。
