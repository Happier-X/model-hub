# 设计：请求日志

## Schema v2

```sql
CREATE TABLE IF NOT EXISTS request_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  time INTEGER NOT NULL,
  request_model_name TEXT NOT NULL,
  channel_name TEXT NOT NULL DEFAULT '',
  actual_model_name TEXT NOT NULL DEFAULT '',
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  use_time INTEGER NOT NULL DEFAULT 0,
  cost REAL NOT NULL DEFAULT 0,
  error TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_request_logs_time ON request_logs(time DESC);
```

`schema_migrations` version=2。

## 模块

```text
log/
  mod.rs model.rs store.rs service.rs
routes/log.rs
```

`AppState.logs: Arc<LogService>`

## 写入点

在 `v1_chat` handler 统一：

```rust
let started = Instant::now();
// resolve route → 失败则 insert error log + return
// forward → 根据 response status/body 解析 usage 与 error 摘要
// insert log（流式：tokens=0，error 在非 2xx 时从缓冲错误路径取）
```

非流式成功：尝试解析 body `usage.prompt_tokens` / `completion_tokens`。  
非流式/流式上游错误：`error` 截断 message 字段（≤512 chars）。

channel_name：从 ChannelService.get 取 name（router 可扩展 RouteTarget 带 channel_name，或二次查询）。

## list/clear

管理 JWT + query 参数。

## 测试

- migrate v2 幂等
- chat mock 后 list 有记录
- clear 清空
- 无 JWT 401

## 回滚

删除 log 模块与 v2 表逻辑。
