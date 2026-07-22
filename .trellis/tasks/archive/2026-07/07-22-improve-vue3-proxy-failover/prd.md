# 继续完善 Vue3 内嵌代理与故障转移

## Goal

审视并补齐当前 Vue3 + Tauri 2 + Rust 内嵌代理中与 CC Switch 路由/故障转移相关的可靠性缺口，确保本机统一 URL + 客户端 Key 的 Chat 代理在非流式和 SSE 流式场景下按有序队列、默认熔断与健康状态稳定工作。

## 已确认实现现状

- 前端：Vue 3 + Tailwind；后端：Tauri 进程内 Axum 代理。
- 对外接口：`/health`、`/v1/models`、`/v1/chat/completions`。
- 路由：客户端 `model` = 分组名；分组内按 `sort_order` 遍历供应商 + 上游模型。
- 非流式：完整读 body 后返回；流式：首包前可换源，提交首 chunk 后不拼接第二家。
- 熔断：Closed / Open / HalfOpen；默认失败阈值 4、恢复 60 秒、半开成功 2 次。
- 管理端：供应商、分组、日志、健康状态已存在。

## 已确认产品决策

| 决策 | 结论 |
|------|------|
| 默认超时 | 首包 60s、流式静默 120s、非流式 600s |
| 流式静默超时后 | 终止当前流，记录可重试失败/熔断，不换源拼接 |
| HalfOpen 并发 | 每个 provider 同时最多 1 个探测；其余跳过并尝试下一项 |
| 熔断粒度 | 按 provider id 共享熔断（不是 provider+model） |
| 协议 | 仅 Chat + `/v1/models`；不扩 embeddings/images |

## Requirements

- R1：保留分组 = 对外 model、有序队列、`auto_failover` 开关。
- R2：保留非流式完整 body 成功语义；流式首 chunk 前可换源、提交后不拼接。
- R3：新增流式静默超时 120s；超时终止当前流，记供应商失败，不换源。
- R4：非流式总超时改为 600s；首包超时保持 60s。
- R5：HalfOpen 单探测锁；探测成功/失败正确转换并释放占用。
- R6：熔断继续按 provider id；健康页与 list_health 展示状态与连续失败次数。
- R7：故障转移日志保留 from/to/reason；禁止完整 Key 与 messages。
- R8：补充单元/集成回归测试覆盖静默超时、半开单探测、超时常量与既有 5xx 换源。

## Acceptance Criteria

- [x] 非流式超时常量为 600s；流式首包 60s；流式静默 120s。
- [x] 流式首 chunk 后静默超时：记录失败并计入熔断，不向客户端追加第二家响应。
- [x] 流式首包超时仍可按队列换源成功。
- [x] 非流式 5xx/网络错误可换源；典型不可重试 4xx 不换源且不记熔断失败。
- [x] Open 后恢复窗口到达时仅放行一个 HalfOpen 探测；并发请求跳过该 provider。
- [x] `auto_failover=false` 时不尝试队列第二项。
- [x] 请求日志含 failover 字段且无完整 Key/messages。
- [x] `cargo test --manifest-path src-tauri/Cargo.toml`、`cargo check`、`pnpm typecheck`、`pnpm build` 通过。

## Out of Scope

- 公网部署、多租户、计费。
- 旧 React / gateway-rust 兼容。
- 高级熔断参数面板、自定义 Header/协议转换。
- embeddings/images/旧式 completions。
