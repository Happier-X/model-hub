# Design：首页统计总览（token / 耗时记录 + 聚合）

## 架构与边界

后端三层改动 + 前端展示：
1. **schema**（`db/migrate.rs`）：`request_logs` 增加 `input_tokens` / `output_tokens` 列。
2. **转发链路**（`proxy/forward.rs` + `domain/log.rs` insert）：提取响应 usage 写入日志。
3. **统计命令**（`commands.rs` + `domain/log.rs`）：`get_request_overview` 返回总计/今日两组指标。
4. **前端**（`src/api/tauri.ts` + `HomePage.vue`）：顶部统计总览卡片，两行（总计 / 今日）。

## 数据流

```
上游响应 (非流式: body bytes; 流式: 透传 chunk 旁路解析)
  → 提取 usage {prompt_tokens, completion_tokens}
  → NewRequestLog { input_tokens, output_tokens, ... } → INSERT
  → get_request_overview: SELECT COUNT/SUM(成功口径) GROUP BY 无（两段独立 SQL）
  → { total: {...}, today: {...} } → 前端两行渲染
```

## 关键决策

### D1 DB 迁移（幂等 ensure 模式）
- 新库建表 SQL 直接带 `input_tokens INTEGER NOT NULL DEFAULT 0` / `output_tokens INTEGER NOT NULL DEFAULT 0`。
- 旧库：复用现有 `request_logs_has_column` + `ALTER TABLE ADD COLUMN` 逻辑（同 `ensure_*` 既有模式，参考 log.rs insert 的旧列兼容分支）。
- 旧 gateway-rust 库已有同名列 → 检测跳过，不冲突。
- `NewRequestLog` 增加 `input_tokens: i64` / `output_tokens: i64`（默认 0），INSERT 双写补列；所有调用点（forward.rs 5 处、测试 2 处）补字段。

### D2 非流式 token 提取
- `attempt_non_stream` 已返回完整 `bytes`：在成功分支解析 JSON 顶层 `usage.prompt_tokens` / `usage.completion_tokens`，缺失或非法 → 0。
- 在 `forward_with_failover` 非流式成功分支传给 `NewRequestLog`。

### D3 流式 token 提取（旁路，不透传拦截）
- **请求侧注入**：`rewrite_model`（forward.rs 已有按模型家族改写 body 的先例：`apply_thinking_effort`）在 OpenAI 兼容家族且请求未带 `stream_options` 时，注入 `{"stream_options": {"include_usage": true}}`——上游在流末尾返回含 `usage` 的 chunk。
- **响应侧旁路**：`stream_body_from_prime` / `StreamState` 增加 usage 观察：每个透传 chunk 尝试解析顶层 `usage`，非空则累积（后到覆盖，OpenAI 只在最后 chunk 带 usage）。不改动透传字节与时序；解析失败静默忽略。
- `on_success` 回调签名增加 `(input_tokens, output_tokens)`，成功日志写入真实 token。
- **白名单注入**：仅对已知 OpenAI 兼容模型家族注入（复用 `thinking_family` 思路），未知模型不注入（token 记 0），避免不兼容供应商因未知字段报错把请求搞挂。

### D4 统计聚合（成功口径 D2 from PRD）
- 新增 `get_request_overview`，SQL（两段，总/今日）：
  ```sql
  SELECT COUNT(*), COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0), COALESCE(SUM(use_time_ms),0)
  FROM request_logs
  WHERE status_code BETWEEN 200 AND 299 AND error = ''
  [AND time >= ? AND time < ?]   -- 今日段：本地自然日边界（复用现有 day_start/day_end 计算）
  ```
- 返回 `{ total: { requests, input_tokens, output_tokens, use_time_ms, cost }, today: {...} }`，`cost` 本期恒 0（UI 显示「-」，D1-PRD）。

### D5 前端
- `tauri.ts`：`RequestOverview` 类型 + `getRequestOverview()`。
- HomePage：顶部「统计总览」卡片**替换现有「今日请求」卡片**（与总计/今日重复），其余卡片保留：
  - 两行（总计 / 今日）× 6 指标：请求次数、输入 tokens、输出 tokens、总 tokens、耗时、费用（「-」）
  - 耗时格式化（D3-PRD）：<1000ms 显示 `N ms`；<60s 显示 `x.x s`；≥60s 显示 `x 分 y 秒`
  - 数字用 tabular-nums；随现有「刷新统计」一起加载
- 失败请求不计入（SQL 已过滤），空数据显示 0。

## 兼容性与迁移

- 旧库（无新列）：首次 insert 前 ensure 加列，幂等。
- 旧 gateway-rust 库（已有新列）：跳过 ADD，insert 双写补列不冲突。
- 历史日志无 token（默认 0）：统计口径「总计」从 0 起步，新请求后增长；不回填（PRD Out of Scope）。
- 流式注入对未知模型不生效：token 记 0，不影响转发。

## 风险与回滚

| 风险 | 缓解 |
|---|---|
| 流式注入 `stream_options` 被部分上游拒绝 | 白名单模型家族注入；注入失败走现有 failover 换源 |
| 旁路解析影响透传性能 | 仅对 chunk 做轻量 JSON 解析（serde_json from_slice），失败即跳过 |
| INSERT 双写漏列导致旧库炸 | ensure 加列 + 双写模式已在前次迁移验证（旧列兼容分支） |
| 统计 SQL 慢 | request_logs 有 time 索引；COUNT/SUM 全表扫在万级行可接受 |

回滚：加列幂等（重复执行无害）；注入逻辑集中在 `rewrite_model` 一处，可一键关闭。
