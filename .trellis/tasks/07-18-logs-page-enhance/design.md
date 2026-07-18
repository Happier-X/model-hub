# 设计：日志页补强

## 边界

| 层 | 职责 |
|----|------|
| `src/api/log.ts` | page/pageSize 透传；可选时间窗后续扩展 |
| `LogsPage` | 分页 UI、过滤、展开、轮询开关、清空确认 |
| 侧车 | list/clear 不变 |

## API

```text
GET /api/v1/log/list?page=1&page_size=20
DELETE /api/v1/log/clear
```

`listLogs(page, pageSize)` 保持；pageSize clamp 到 1–100。

## UI 状态

```text
page, pageSize
keyword, onlyErrors
expandedId
autoRefresh (bool)
logs, loading, error
```

派生：`visibleLogs = filter(logs)`。

## 分页逻辑

- `hasPrev = page > 1`
- `hasNext = logs.length >= pageSize`（近似）
- 改 pageSize 时重置 page=1

## 回滚

- 仅前端 LogsPage / log.ts。
