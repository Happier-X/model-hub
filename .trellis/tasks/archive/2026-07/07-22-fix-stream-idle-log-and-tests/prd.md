# 修复流式静默超时日志与集成测试

## Goal

修正流式 Chat 在「首包成功提交后发生静默超时」时的请求日志语义，避免管理端出现误导性的「先 200 成功、再 504 失败」双记录；并补齐可自动验证的静默超时回归测试。

## 背景（代码证据）

- `server.rs` 的 `chat_completions` 在 `forward_with_failover` **返回 Ok** 时立刻 `insert_log`，状态码取自上游响应头（流式 prime 成功时多为 **200**）。
- `forward.rs` 在流式静默超时回调里 **再次** `insert_log`（`status_code: 504`，error=`流式静默超时`），并 `record_failure`。
- 结果：同一次客户端请求在 `request_logs` 中可能出现两条记录，第一条像成功，第二条才是真实失败。
- 当前仅有单元级 idle 语义测试，**没有**驱动 `stream_body_from_prime` / HTTP 路径的静默超时集成测。

## Requirements

- R1：同一次流式请求在静默超时后，管理端请求日志的**最终语义**不得表现为成功完成（不得仅以 200 + 空 error 作为该请求的唯一/最终结论）。
- R2：优先采用**单条最终日志**或**明确关联的修正策略**（二选一写入 design 后实现）；禁止无说明的双成功/失败混淆。
  - 推荐：**流式成功路径延迟写最终日志**——仅在 body 正常结束时写 200；静默超时只写一条失败日志（或回写/替换最终状态），server 侧不再在 prime 返回时立即记成功。
- R3：静默超时仍：记供应商 `record_failure`、**不换源拼接**、不写完整 Key/messages。
- R4：非静默超时的流式成功、非流式成功/失败、5xx 换源行为不回归。
- R5：增加自动化测试：可强制触发 idle 超时（测试用短超时或注入），断言日志条数与状态/error 文案，以及熔断失败被记录。

## Acceptance Criteria

- [x] 流式请求静默超时后，`request_logs` 不会留下「仅 200 且 error 为空」作为该请求的唯一记录。
- [x] 静默超时路径有明确失败摘要（如「流式静默超时」），无完整 Key/messages。
- [x] 流式正常结束仍可记成功（200 或等价成功语义）。
- [x] 既有 `proxy_failover` 集成测（缺 Key、models、5xx 换源）通过。
- [x] 新增静默超时相关测试通过。
- [x] `cargo test --manifest-path src-tauri/Cargo.toml` 与 `cargo check` 通过。

## Out of Scope

- 修改熔断阈值/恢复时间默认值。
- 前端日志页大改版（除非为展示字段所必需）。
- 公网部署与协议扩展。

## Notes

- 轻量任务：PRD 足够；实现前可补简短 design 段落于 PRD Technical Notes 或单独 `design.md`。
- 清理 `target-check-proxy/` / `target-implement-proxy/` 可并行手动完成，不阻塞本任务。
