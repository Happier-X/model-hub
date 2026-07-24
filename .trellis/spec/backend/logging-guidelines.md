# Logging Guidelines

> 日志级别与业务请求日志。

---

## Levels

| 级别 | 用途 |
|------|------|
| error | 启动失败、崩溃、无法写库 |
| warn | 自动启动代理失败、超时后强杀等 |
| info | 代理启动/停止、监听地址 |
| debug | 请求级细节（开发） |

---

## Rules

1. tracing 字段可含 `port`、路径，**不含**密钥全文。
2. 业务表 `request_logs` 与 tracing 分离；管理 UI 读 SQLite。
3. 请求日志字段：时间、分组、供应商、上游 model、状态码、耗时、error、failover_from/to/reason。
4. **禁止**完整 messages / 上游 Key。
5. **流式最终日志**：`/v1/chat/completions` 在流式 prime 成功后不得立刻记 200 成功；应在 body 正常结束时记成功，或在静默超时/读错误时记**单条**失败（如 504「流式静默超时」）。同一次流式请求禁止「先 200 空 error、再 504」双结论。
6. **成功请求判定**（今日统计 `RequestStats.success` 与「最近成功」共用）：`status_code BETWEEN 200 AND 299` 且 `(error IS NULL OR length(error) = 0)`。
7. **最近成功请求（首页）**：专用 IPC `get_last_success_request` → `Option<LastSuccessRequest>`；查询全库满足成功谓词、`ORDER BY time DESC, id DESC LIMIT 1`；字段 `time` / `group_name` / `provider_name` / `upstream_model` / `status_code`；无行返回 JSON `null`。禁止用 `list_logs` 整页再筛代替此查询。前端在首页与「刷新统计」一并拉取；空态文案，不新增定时轮询。

---

## Anti-Patterns

- 每个 token 一条 info。
- 把完整 Authorization 打进日志。
- 前端拉整页日志再筛「最近成功」。
- 「最近成功」判定与 `RequestStats.success` 分叉。
