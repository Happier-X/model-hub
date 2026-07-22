# 设计：补齐代理超时与 HalfOpen 单探测

## 边界

- 改动集中在 `src-tauri/src/proxy/{forward,circuit,server}.rs` 与相关测试。
- 不改 domain schema、不改前端协议字段；健康展示继续读 `list_health`。
- 不引入后台探活任务；探测即真实用户请求。

## 超时

| 常量 | 值 | 使用点 |
|------|----|--------|
| `STREAM_FIRST_BYTE_TIMEOUT` | 60s | `attempt_stream_prime` 等待首 chunk |
| `STREAM_IDLE_TIMEOUT` | 120s | 首包后每个后续 chunk 的等待上限 |
| `NON_STREAM_TIMEOUT` | 600s | 非流式 reqwest client 总超时 |
| `CONNECT_TIMEOUT` | 10s | 保持 |

流式静默超时实现：

1. `stream_body_from_prime` 对后续 `chunk()` 包一层 `timeout(STREAM_IDLE_TIMEOUT)`。
2. 超时后：结束 body stream（客户端侧表现为流中断/错误）；通过回调或旁路通知 `record_failure` 与写日志。
3. 因响应已提交，**不得**回到 `forward_with_failover` 队列换源。

推荐实现形态：

- `stream_body_from_prime` 接收 `on_idle_timeout: impl FnOnce()` 或 `Arc` 的失败记录闭包（provider_id + stores 日志信息）。
- 或在 `server.rs` 包装 stream 并在 drop/error 路径记日志；优先在 `forward.rs` 内闭环，避免 server 泄漏细节。

## HalfOpen 单探测锁

`CircuitRegistry` 每个 provider 的 `Entry` 增加：

- `probe_in_flight: bool`

`allow_request(provider_id)` 语义：

| 状态 | 行为 |
|------|------|
| Closed | 允许 |
| Open 且未到恢复时间 | 拒绝 |
| Open 且到恢复时间 | 转 HalfOpen，若 `probe_in_flight==false` 则置 true 并允许；否则拒绝 |
| HalfOpen 且 `probe_in_flight==false` | 置 true 并允许 |
| HalfOpen 且 `probe_in_flight==true` | 拒绝（跳过该 provider） |

`record_success` / `record_failure`：

- 清除 `probe_in_flight`。
- 既有 HalfOpen 成功计数与 Open 回退逻辑不变。

并发安全：继续使用 `Mutex<HashMap<...>>`；`allow_request` 与 `record_*` 在同一把锁内更新标志。

## 故障转移循环（保持）

```text
for candidate in queue (or first only if !auto_failover):
  skip if !enabled
  skip if !allow_request  # 含 half-open 被占用
  attempt
  success -> record_success, return
  non-retryable -> return upstream body (no circuit fail)
  retryable -> record_failure, log attempt, continue if auto_failover
exhausted -> 502
```

## 日志

- 静默超时：`error` / `failover_reason` 写「流式静默超时」类摘要；不写 Key/messages。
- 换源成功时保留 `failover_from` / `failover_to` / `failover_reason`。

## 测试策略

- 单元：`circuit` 半开只允许一个探测；第二个并发 `allow_request` 为 false；成功/失败释放锁。
- 单元：超时常量断言。
- 集成：既有 5xx 换源保留。
- 集成/单元：流式静默超时可测性 —— 若 wiremock 难模拟 chunk 间隔，优先在 `forward` 内对 timeout 分支做单元级注入或缩小测试专用超时 feature；MVP 至少覆盖 circuit 与超时常量，静默路径用可控 mock 或内部 helper。

## 回滚

- 仅代理行为变更；失败时可恢复 `NON_STREAM_TIMEOUT=120` 与去掉 idle/probe 标志。无 schema 迁移。
