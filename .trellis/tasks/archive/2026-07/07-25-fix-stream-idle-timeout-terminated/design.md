# 技术设计：流式静默超时 / 心跳保活

## 问题边界

改动集中在 `src-tauri/src/proxy/forward.rs` 的流式续传路径（`stream_body_from_prime` 及其 `StreamState`）。非流式路径、错误信封识别（`is_structured_error_body`）、首包提交前的换源逻辑（`forward_with_failover`）均不改。

当前续传逻辑（简化）：

```
loop:
  timeout(idle=120s, resp.chunk()):
    Err(_)        -> 塞 io::Error("流式静默超时") 到客户端流，记 504   # ← Error: terminated 来源
    Ok(Ok(Some)) -> 透传 chunk
    Ok(Ok(None)) -> 正常结束，记 success
    Ok(Err(e))   -> 塞 io::Error(e) 到客户端流，记 502
```

核心矛盾：`idle=120s` 既用来「判定上游卡死」，又会误伤「上游还活着、只是思考期不吐 token」。二者需要拆开。

## 方案对比

### 方案 A：单纯放宽 / 动态化 idle 超时
把 `STREAM_IDLE_TIMEOUT` 调大（如 300s），或按思考档位动态给不同值。

- 优点：改动最小，只调常量 / 加一个入参。
- 缺点：治标。思考时长不可预测，档位与静默时长非线性；调太大又让真正卡死的连接迟迟不释放。无法同时兼顾「思考期容忍」与「卡死快速释放」。

### 方案 B：SSE 心跳保活 + 绝对兜底（推荐）
等待上游 chunk 期间，周期性向客户端发 SSE 注释行 `: ping\n\n` 保活；只要上游连接本身没断，客户端就不会 terminated。同时保留一个**绝对静默兜底**（上游连续无任何数据超过硬阈值才真正断开）。

- 优点：从根因解决——思考期无论多久，只要上游 TCP 还在，客户端连接就保活；卡死场景仍有硬兜底快速释放。符合 SSE 规范（`:` 注释行客户端忽略）。
- 缺点：改动 unfold 状态机，需区分 SSE 与非 SSE 响应，复杂度中等。
- 风险控制：仅当响应 `Content-Type: text/event-stream` 时注入心跳；非 SSE 流退回原 idle 超时逻辑，绝不污染 body。

### 方案 C：A + B 组合
心跳保活为主，同时把硬兜底设为可配置。等价于 B 的实现加一个可调常量，采用 B 时顺带做。

## 选型：方案 B

理由：只有心跳能真正区分「上游活着但静默」与「上游卡死」——前者 TCP 有底层活性、chunk future 仍 pending 但连接健康；后者硬兜底到点即断。放宽超时（A）无法两全。

## 详细设计

### 常量

```rust
/// 等待上游 chunk 期间，向 SSE 客户端发送心跳注释行的间隔。
pub const STREAM_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
/// 上游连续无任何数据的绝对兜底（含思考期）；到点判定卡死并断开。
pub const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(300); // 由 120 放宽
```

`STREAM_HEARTBEAT_INTERVAL` 远小于 idle 兜底，保证客户端在两次真实 chunk 间隔内持续收到保活字节。`STREAM_IDLE_TIMEOUT` 仍是「彻底无响应」的上限。`ForwardPolicy.stream_idle_timeout` 保留（测试注入短超时），另加 `heartbeat_interval` 便于测试。

### 是否为 SSE 的判定

prime 成功后由响应头判定一次并存入 `StreamState`：

```rust
fn is_sse_response(headers: &HeaderMap) -> bool {
    headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_ascii_lowercase().contains("text/event-stream"))
        .unwrap_or(false)
}
```

chat completions 流式上游基本都是 `text/event-stream`；非 SSE（少见）走原 idle 逻辑，不注入心跳。

### 续传状态机（SSE 路径）

把「单次 `timeout(idle, chunk)`」改为「心跳循环」：

```
累计静默 acc = 0
loop:
  select:
    chunk = resp.chunk():
        Some -> 透传，acc = 0
        None -> 正常结束，on_success
        Err  -> on_error，塞 io::Error   # 上游真错，仍需让客户端感知
    _ = sleep(HEARTBEAT_INTERVAL):
        acc += HEARTBEAT_INTERVAL
        if acc >= idle_hard:            # 绝对兜底
            on_idle_timeout，塞 io::Error("流式静默超时")
        else:
            向客户端发 b": ping\n\n"      # 保活，不计入错误
```

实现上 `unfold` 每次 poll 只能产出一个 item，用 `StreamState` 承载 `acc` 与「是否 SSE」。心跳 tick 未达硬兜底时，产出 `Ok(Bytes::from_static(b": ping\n\n"))` 作为一个正常 chunk；达兜底则产出 `Err` 并 finalize。非 SSE 路径 `acc` 逻辑保留但不发 ping，到 idle 直接超时（等价旧行为）。

### 回调 / 日志语义（不变）

- 正常结束 → `on_success`，记 success status。
- 硬兜底超时 → `on_idle_timeout`，记 504「流式静默超时」。
- 上游读错误 → `on_error`，记 502「流式中断」。
- 客户端断开（Drop 未 finalize）→ `on_abort`，记 499。

心跳 ping 不触发任何回调、不改状态码，仅保活。

### 换源语义（不变）

首包（prime）阶段的错误信封 / 传输错误仍在 `forward_with_failover` 内换源；心跳只存在于 prime 成功之后的续传阶段，此时已 `defer_request_log=true`，一律不换源。

## 兼容性 / 回滚

- 对客户端：多收到若干 `: ping` SSE 注释行，符合规范，OpenAI SDK / undici 均忽略，无语义影响。
- 对非 SSE 流：行为等同旧逻辑。
- 回滚：还原 `forward.rs` 续传函数与常量即可，无迁移、无持久化改动。

## 测试计划

- `heartbeat_injected_only_for_sse`：SSE 响应等待期产出 `: ping`；非 SSE 不产出。
- `hard_idle_timeout_still_fires`：注入短 idle_hard + 短 heartbeat，验证累计到兜底仍产出超时 `Err` 并记 504。
- `upstream_read_error_still_propagates`：读错误仍 `on_error` + 502。
- `timeout_constants` 断言更新（120→300，新增 heartbeat 常量）。
- 现有 SSE 错误信封识别与 failover 测试保持通过。
