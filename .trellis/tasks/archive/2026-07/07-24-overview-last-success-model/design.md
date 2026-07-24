# 技术设计

## 数据定义

`LastSuccessRequest`（命名可微调）：

| 字段 | 来源 `request_logs` | 说明 |
|------|---------------------|------|
| `time` | `time` | unix 秒 |
| `group_name` | `group_name` | 客户端 model / 分组名 |
| `provider_name` | `provider_name` | 实际命中供应商 |
| `upstream_model` | `upstream_model` | 实际上游模型 id |
| `status_code` | `status_code` | 便于调试展示（可选展示） |

成功谓词（与 `request_stats_between` 一致）：

```sql
status_code BETWEEN 200 AND 299
AND (error IS NULL OR length(error) = 0)
```

查询：

```sql
SELECT time, group_name, provider_name, upstream_model, status_code
FROM request_logs
WHERE status_code BETWEEN 200 AND 299
  AND (error IS NULL OR length(error) = 0)
ORDER BY time DESC, id DESC
LIMIT 1
```

无行 → IPC 返回 `null`（JSON null），前端空态。

## API

- 新增 domain：`Stores::last_success_request() -> Result<Option<LastSuccessRequest>, AppError>`
- 新增 command：`get_last_success_request` → 注册 `lib.rs`
- 前端 `src/api/tauri.ts`：`getLastSuccessRequest()` + 类型

不扩展 `list_logs` 筛选：`status_class=2xx` 仍可能含 error 字段非空，且 page 语义不适合「单条最新」。

## UI

- 位置：概览「今日请求」区块内下方，或紧随其后的小卡片；标题建议 **「最近成功请求」**。
- 展示：分组 · 供应商 · 上游模型 · 本地化时间。
- 刷新：`refreshStats` 与 `onMounted` 的 `refresh` 路径一并拉取；可与 stats 并行 `Promise.all`。
- 错误：独立 `lastSuccessError`，不覆盖 stats 错误。

## 测试

- Rust：插入成功/失败/旧成功，断言返回最新成功；全失败返回 `None`。
- 前端：以 typecheck + 页面接线为主；无强制组件单测。

## 兼容

- 只读查询，无 schema 变更。
- 日志保留/清理后自然可能变为空态。
