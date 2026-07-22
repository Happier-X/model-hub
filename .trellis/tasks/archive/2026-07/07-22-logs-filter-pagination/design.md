# 设计：日志筛选与分页

## 契约

```text
list_logs(query) -> LogPage

LogQuery {
  page?: number,          // default 1
  page_size?: number,     // default 50, clamp 1..=100
  group_name?: string,    // trim；空则忽略；LIKE %name%
  status_class?: string,  // all|2xx|4xx|5xx|error；默认 all
  failover_only?: bool,   // default false
}

LogPage {
  items: RequestLog[],
  total: number,
  page: number,
  page_size: number,
}
```

### status_class SQL 语义

| 值 | 条件 |
|----|------|
| all | 无额外条件 |
| 2xx | status_code BETWEEN 200 AND 299 |
| 4xx | status_code BETWEEN 400 AND 499 |
| 5xx | status_code BETWEEN 500 AND 599 |
| error | status_code >= 400 OR (error IS NOT NULL AND error != '') |

### failover_only

`(failover_from != '' OR failover_to != '')`

## 兼容

- 前端改为只调新形状；无旧客户端兼容包袱。
- command 参数可用单个 payload 结构体，避免可选参数爆炸。

## 测试

- 插入 3+ 条不同 group/status/failover，断言 total、page 切片、各筛选。
