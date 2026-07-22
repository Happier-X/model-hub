# 设计：流式静默超时日志

## 策略（R2 推荐方案）

**单条最终日志 + 流式成功路径延迟写入。**

| 路径 | 谁写 `request_logs` |
|------|---------------------|
| 流式 prime 成功后 body 正常结束 | `stream_body_from_prime` 的 `on_success` → 状态取上游成功码，error 空 |
| 流式静默超时 | `on_idle_timeout` → 504 +「流式静默超时」+ `record_failure`，**不换源** |
| 流式中途读错误 | `on_error` → 502 + 摘要 + `record_failure` |
| 非流式成功/失败、不可重试 4xx、整体 Err | 仍由 `server::chat_completions` 统一写（`defer_request_log=false`） |
| 可重试失败（换源过程中） | 保持 `forward` 内对失败 attempt 的中间日志 |

`ForwardOutcome.defer_request_log=true` 时 server **禁止**在返回响应头时再记成功。

## 可测注入

`ForwardPolicy { stream_idle_timeout }` 挂在 `AppState`；生产默认 `STREAM_IDLE_TIMEOUT`（120s）；集成测试可注入毫秒级超时。
