# 执行计划：流式心跳保活修复 terminated

前置：改动全部落在 `src-tauri/src/proxy/forward.rs`。不动 server.rs、非流式路径、错误信封识别、prime 换源逻辑。

## 步骤

1. **常量与策略**
   - 将 `STREAM_IDLE_TIMEOUT` 由 `120s` 改为 `300s`（绝对兜底）。
   - 新增 `STREAM_HEARTBEAT_INTERVAL: Duration = 15s`。
   - `ForwardPolicy` 新增 `heartbeat_interval: Duration`，`Default` 填 `STREAM_HEARTBEAT_INTERVAL`；`stream_idle_timeout` 保留。
   - 校验点：`cargo build` 通过；`ForwardPolicy` 所有构造处（含测试）编译不破。

2. **SSE 判定辅助**
   - 加 `fn is_sse_response(headers: &HeaderMap) -> bool`（`content-type` 含 `text/event-stream`）。
   - 校验点：单测 `is_sse_response` 对 `text/event-stream; charset=utf-8` 返回 true，对 `application/json` 返回 false。

3. **StreamState 扩展**
   - 新增字段：`is_sse: bool`、`idle_acc: Duration`、`heartbeat: Duration`。
   - `stream_body_from_prime` 签名加 `is_sse: bool` 与 `heartbeat: Duration`；`forward_with_failover` 调用处传入 `is_sse_response(&ok.headers)` 与 `policy.heartbeat_interval`。
   - 校验点：编译通过；`mark_finalized` / `Drop` 逻辑不变。

4. **续传状态机改为心跳循环**
   - 用 `tokio::select!` 在 `resp.chunk()` 与 `tokio::time::sleep(heartbeat)` 间竞争：
     - chunk `Some` → 透传，`idle_acc = 0`。
     - chunk `None` → `on_success`，finalize，结束。
     - chunk `Err` → `on_error`，塞 `io::Error`，finalize。
     - sleep 命中 → `idle_acc += heartbeat`；若 `idle_acc >= stream_idle_timeout` → `on_idle_timeout` + 塞 `TimedOut` + finalize；否则若 `is_sse` 产出 `Ok(": ping\n\n")` 保活，非 SSE 不产 ping 继续累计。
   - 保留 `first` 首包优先产出、`done` 短路、`response=None` 时 `on_success` 收尾等既有分支。
   - 校验点：无 `await` 期间持有非 Send 值；`Body::from_stream` 类型不变（`Result<Bytes, io::Error>`）。

5. **测试**
   - 更新 `timeout_constants_match_prd`：`STREAM_IDLE_TIMEOUT == 300s`，新增 `STREAM_HEARTBEAT_INTERVAL == 15s` 断言。
   - 新增 `is_sse_response` 判定单测。
   - 新增心跳循环行为测试（用短 heartbeat + 短 idle 的 mock/构造，断言：SSE 路径先出 ping 再到硬兜底 504；非 SSE 直接到超时；读错误仍 502）。若纯函数难以脱离 `reqwest::Response` 构造，退化为对状态机分支的可测拆分或以策略注入的最小 mock 覆盖。
   - 校验点：`cargo test -p model-hub proxy::forward` 全绿。

6. **全量校验**
   - `cargo test`（src-tauri）全绿。
   - `pnpm typecheck` + `pnpm lint`（前端未改，仅确认无连带破坏）。
   - `cargo build` release 剖面无警告新增。

## 验证命令

```bash
cd src-tauri && cargo test
cd .. && pnpm typecheck && pnpm lint
```

## 回滚点

- 每步独立可回滚；核心风险在步骤 4，若心跳循环导致测试不稳，可先只做步骤 1（放宽 idle 到 300s）作为止血，再迭代心跳。
- 完整回滚：还原 `forward.rs` 的常量、`ForwardPolicy`、`stream_body_from_prime`、`StreamState` 即可，无持久化 / 迁移影响。

## 审查门

- 步骤 4 完成后先跑 `cargo test proxy::forward`，确认换源与错误信封相关既有测试未回归，再进入全量校验。
