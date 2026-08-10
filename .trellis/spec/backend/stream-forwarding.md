# 流式转发与超时 / 心跳保活

> 内嵌代理 `/v1/chat/completions` 流式（SSE）路径的续传超时、心跳保活与日志四态契约。
> 代码位置：`src-tauri/src/proxy/forward.rs`。

---

## Scenario: 流式响应续传

### 1. Scope / Trigger

- Trigger：客户端 `stream: true` 请求，且某候选项 **prime 成功**（首包已到、非错误信封）后进入续传阶段。
- 边界：本契约只约束「首包提交客户端之后」的 body 续传；首包提交前的换源仍归 `forward_with_failover`。

### 2. 超时常量

| 常量 | 值 | 语义 |
|------|----|----|
| `STREAM_FIRST_BYTE_TIMEOUT` | 60s | prime 阶段等首包上限 |
| `STREAM_IDLE_TIMEOUT` | **300s** | 续传阶段「上游连续无任何数据」的绝对兜底；到点判定卡死并断流 |
| `STREAM_HEARTBEAT_INTERVAL` | **15s** | 等待上游 chunk 期间，向 SSE 客户端发心跳注释行的间隔 |
| `NON_STREAM_TIMEOUT` | 600s | 非流式总超时 |
| `CONNECT_TIMEOUT` | 10s | 连接超时 |

`ForwardPolicy` 暴露 `stream_idle_timeout` 与 `heartbeat_interval` 供测试注入短值。

### 3. 核心契约

**心跳保活（根因修复）**

- 续传等待上游 chunk 时，每 `heartbeat_interval` 未收到数据，就向 SSE 客户端产出一个注释行 `: ping\n\n` 保活。
- 目的：长思考模型首包后思考期可能长时间不吐 token；心跳保活让 chunked 传输保持活跃，客户端（undici/`fetch`）不再抛 `Error: terminated`。
- 心跳 **不触发任何回调、不改状态码、不计入错误**，仅保活字节。
- `: ` 开头是 SSE 规范内注释行，OpenAI SDK / undici 全部忽略。

**仅 SSE 注入**

- 只有响应 `Content-Type` 含 `text/event-stream`（`is_sse_response`）才注入心跳。
- 非 SSE 流：不注入任何字节，静默累计到 `STREAM_IDLE_TIMEOUT` 直接判超时（等价旧行为），绝不污染 body。

**绝对兜底**

- 上游连续无数据累计 `idle_acc >= stream_idle_timeout` → 判定卡死，向流塞 `ErrorKind::TimedOut("流式静默超时")` 结束，记 504。
- `idle_acc` 每收到真实 chunk 归零；心跳 tick 累加 `heartbeat_interval`。

### 4. 日志四态（续传阶段，`defer_request_log=true`）

| 终态 | 回调 | status_code | error |
|------|------|-------------|-------|
| 正常结束（上游 EOF） | `on_success` | 上游成功码（如 200） | 空 |
| 绝对兜底超时 | `on_idle_timeout` | 504 | `流式静默超时` |
| 上游读错误 | `on_error` | 502 | `流式中断: …` |
| 客户端提前断开（`StreamState` Drop 未 finalize） | `on_abort` | 499 | `流式响应未完整结束（客户端断开或中止）` |

**token 用量**：`on_success` 携带旁路观察到的 usage（输入/输出 token，写 `request_logs.input_tokens/output_tokens`）；非流式成功由 `ForwardOutcome.input_tokens/output_tokens` 携带，`server.rs` 写入。失败终态（504/502/499）token 恒 0。

**流式 usage 获取**：转发前对 OpenAI 兼容模型家族（`supports_include_usage`：gpt/o 系、deepseek、moonshot/kimi、glm、qwen、minimax、doubao）注入 `stream_options.include_usage=true`（客户端已带 `stream_options` 不覆盖，未知模型不注入）；透传 chunk 旁路解析顶层 `usage`（后到覆盖，失败静默，不改透传字节与时序）。

`server.rs` 见 `defer_request_log=true` 时**不得**再记成功日志；最终日志一律由 body 终态回调写入，避免 prime 成功即误记 200。

### 5. 换源边界（不变）

- 首包提交前：错误信封 / 传输错误在 `forward_with_failover` 内换下一候选项。
- 首包提交后（进入续传）：一律不换源，仅记日志。心跳只存在于续传阶段。

### 6. Tests Required

- 单测：`timeout_constants_match_prd`（含 300s / 15s 断言）、`is_sse_response_*`。
- 集成（`tests/proxy_failover.rs`）：
  - `stream_sse_heartbeat_keeps_alive_before_hard_timeout`：短 heartbeat + 短 idle 的挂起上游，兜底前 body 含首包与至少一个 `: ping`，兜底后记 504。
  - `stream_idle_timeout_single_failure_log`：静默超时单条 504，不留误导性空 error 200。
  - `stream_abort_on_drop_writes_log`：drop body 记 499。
  - `stream_success_single_ok_log`：正常结束单条 200。

### 7. Wrong vs Correct

#### Wrong

```rust
// 用单一 idle 超时既判卡死又误伤思考期静默：思考 > 120s 直接断流 → Error: terminated
match tokio::time::timeout(idle, resp.chunk()).await {
    Err(_) => yield Err(TimedOut), // 上游其实还活着，只是没吐 token
    ...
}
```

```rust
// 不判 Content-Type 就注入 ": ping"：非 SSE 流被污染
return Some((Ok(Bytes::from_static(b": ping\n\n")), state));
```

#### Correct

```rust
// 心跳窗口内轮询：未到兜底则发保活（仅 SSE），到兜底才真正超时
match tokio::time::timeout(heartbeat, resp.chunk()).await {
    Ok(Ok(Some(bytes))) => { idle_acc = ZERO; yield Ok(bytes) }
    Ok(Ok(None))        => { on_success(); return None }
    Ok(Err(e))          => { on_error(e); yield Err(io::Error::other(e)) }
    Err(_) => {
        idle_acc += heartbeat;
        if idle_acc >= idle { on_idle_timeout(); yield Err(TimedOut) }
        else if is_sse { yield Ok(": ping\n\n") } // 非 SSE 不产字节，继续累计
    }
}
```

---

## Anti-Patterns

- 把 `STREAM_IDLE_TIMEOUT` 无限调大来「治」思考期静默：真卡死的连接迟迟不释放、句柄泄漏。
- 心跳触发日志回调或改状态码：污染四态语义。
- 对非 SSE 流注入心跳字节。
- 续传阶段回到换源循环：首包已提交，换源会导致响应错乱。
