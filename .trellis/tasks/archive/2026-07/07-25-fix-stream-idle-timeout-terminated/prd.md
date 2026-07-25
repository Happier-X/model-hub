# 修复流式静默超时导致客户端 Error: terminated

## 背景

客户端（Node undici / `fetch`）在走 model-hub 代理做流式（SSE）请求时，偶发 `Error: terminated`。该报错是 undici 在**流式响应连接中途被切断、chunked 传输未正常收尾**时抛出，即首包已提交客户端之后被断流。

代码定位（`src-tauri/src/proxy/forward.rs::stream_body_from_prime`）：首包透传成功后，流的续传只有两处会塞入 `io::Error` 强行中断，客户端即表现为 `Error: terminated`：

1. **静默超时**：后续 chunk 等待超过 `STREAM_IDLE_TIMEOUT = 120s` 无新数据 → `ErrorKind::TimedOut("流式静默超时")`，日志记 504。
2. **上游读错误**：读上游 chunk 报错 → `io::Error(msg)`，日志记 502「流式中断」。

## 根因假设（实现前需用日志确认）

v0.0.6 新增「分组级思考强度 + 自动最佳档位」后，高思考档模型（gpt-5 `high`、claude `budget_tokens` 高档等）**首包返回后在思考阶段会长时间不吐 token**。当该静默间隔超过 120s，命中第 1 类，代理主动掐断，客户端收到 `Error: terminated`。

判定方法：请求日志页定位报错时间点记录的 `status_code` / `error`：
- 504 +「流式静默超时」→ 第 1 类（思考期静默），**本任务主要目标**。
- 502 +「流式中断」→ 第 2 类（上游断连/网络）。
- 499 +「客户端断开」→ 客户端侧提前取消，非代理问题。

## Goal

让长思考模型的流式请求不再因「思考期静默」被代理误判为超时而断流，同时保留对「上游真正卡死」的兜底保护，避免连接无限挂起。

## Requirements

- 首包已提交客户端后，思考期长时间静默不得导致 `Error: terminated`（不得向客户端流塞入中断错误）。
- 仍需保留对上游真实卡死 / 无响应的兜底超时，避免连接永久挂起、句柄泄漏。
- 换源语义不变：首包提交前失败仍可换源；首包提交后一律不换源，仅记日志。
- 日志语义保持可区分：静默超时、上游读错误、客户端断开、正常结束四态仍能在请求日志中区分。
- 不改动非流式路径与错误信封识别逻辑。
- 若采用 SSE 心跳保活，仅在响应确为 SSE（`text/event-stream`）时注入心跳，非 SSE 流不得注入额外字节污染 body。

## Acceptance Criteria

- [ ] 复现场景（高思考档模型思考期静默 > 原 120s）下，客户端不再出现 `Error: terminated`，流式内容完整返回。
- [ ] 上游在超过兜底阈值内完全无数据时，连接仍会被终止并记录可区分的超时日志，不会永久挂起。
- [ ] 首包提交前的失败仍能正常换源（现有 failover 相关测试保持通过）。
- [ ] 新增/调整的单元测试覆盖：静默超时行为、（如实现）心跳注入仅作用于 SSE、兜底超时仍生效。
- [ ] `cargo test`（src-tauri）与前端 `pnpm typecheck` / `pnpm lint` 通过。

## Notes

- 复杂任务：需 `design.md` 给方案对比与选型，`implement.md` 给执行步骤与验证命令。
- 方案候选（design 中细化）：(a) 放宽 / 动态化 `STREAM_IDLE_TIMEOUT`；(b) SSE 心跳保活（等待上游期间周期性发 `: ping` 注释行）＋更大的绝对静默兜底；(c) 组合。
- 心跳方案需注意：SSE 注释行（`:` 开头）客户端会忽略，是规范内保活手段；但必须确认响应 Content-Type 为 `text/event-stream` 再注入。
