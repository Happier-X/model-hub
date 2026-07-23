# Error Handling

> 错误如何产生、传播与展示。

---

## Layers

| 层 | 策略 |
|----|------|
| Rust 壳 / IPC | `Result` + `InvokeError`；前端 toast 展示 message |
| 内嵌代理 HTTP | JSON / 透传上游状态；**不**因客户端 Key 返回 401 |
| 端口占用 / 绑定失败 | 启动时从首选端口起最多扫描 50 个可用端口并自动改口写入 `shell.json`；仍失败则 `NoAvailablePort` / `last_error` 可行动文案；**不**结束占用进程；文案可提示意外多开时托盘「退出」旧实例 |
| 代理停止 / 退出 | `stop` 先发 shutdown，在 `PROXY_STOP_GRACE`（默认 3s）内 await JoinHandle；**超时必须 `abort`**，禁止仅丢弃 handle 导致仍占端口；`ProxyHandle::Drop` best-effort `stop`；托盘「退出」与 `RunEvent::Exit` 均 stop |

---

## Rules

1. 用户可见错误须可行动（改端口、检查供应商 URL/Key 等）。
2. **不要**把完整上游 Key、完整 messages 放进错误或日志。
3. 代理未运行时，UI 明确「未运行」，禁止假装空数据成功。
4. **故障转移（顺序、无熔断）**：每个请求从分组队列**第一个启用候选项**开始；响应尚未提交客户端前，当前候选项**任意失败**都继续下一启用候选项（网络/超时、任意非 `2xx`、明确 `2xx` 结构化错误信封）。**禁止**供应商级熔断、失败计数跳过、HalfOpen 探测。实现见 `proxy/forward.rs`。
5. **队列耗尽**：最后一次失败若有上游 HTTP 响应，透传该候选项原始状态、响应头与错误体；若无上游响应（网络/超时/读失败），返回明确网关错误（如 `502`/`504` + 摘要）。
6. 流式：首包前超时/失败可换源；**首 chunk 已提交后**静默超时或读错误只终止当前流并写**单条**最终日志，**禁止**拼接第二家响应。
7. 默认超时：首包 60s、流式静默 120s、非流式 600s（`STREAM_*` / `NON_STREAM_TIMEOUT`）。
8. OpenRouter 公共榜单：请求超时 15s、固定 URL、无 Key；网络失败有缓存返回 stale，无缓存才报可行动错误（见 `model-leaderboard.md`）。
9. **禁止**对用户供应商做自动测活、定时 health、预热、空 chat；AI 联调默认不打用户上游。

---

## 场景：响应提交前顺序故障转移

### 1. Scope / Trigger

- Trigger：本机 `POST /v1/chat/completions` 进入 `forward_with_failover`，按分组 `group_items.sort_order` 选择上游。
- 目标：无熔断、无 `auto_failover` 开关；错误即换源，直到成功或队列耗尽。

### 2. Signatures

- Rust：`forward_with_failover(stores, clients, group_name, candidates, body, stream, policy) -> Result<ForwardOutcome, (StatusCode, String)>`
- Rust：`is_structured_error_body(bytes: &[u8]) -> Option<String>`
- 领域：`Group` **不含** `auto_failover`；候选项仅 `enabled` 过滤。

### 3. Contracts

- 每次请求从队列第一个启用候选项开始，历史失败不影响起点。
- 响应提交前失败均换源：网络错误、连接/读失败、首包超时、任意非 `2xx`、明确 `2xx` JSON 错误信封。
- `2xx` 错误信封仅认结构化字段：字符串 `error`、对象 `error.message`、`type: "error"`、顶层 `message` 等；正常 chat completion（含 `choices`）与 SSE `data:` 首包不换源。
- 成功返回当前候选项响应；`failover_from` / `failover_to` / `failover_reason` 记录本次路径。
- 流式首包提交后不再回到候选循环。

### 4. Validation & Error Matrix

| 条件 | 行为 |
|------|------|
| 候选项 `enabled=false` | 跳过 |
| 非 `2xx` HTTP（含 400/404/模型不支持） | 换源；写尝试失败摘要 |
| 明确 `2xx` 错误信封 | 换源 |
| 正常 `2xx` completion / SSE | 提交响应，不换源 |
| 网络/超时/读失败 | 换源；无原始 body |
| 队列耗尽且最后有 HTTP 响应 | 透传最后状态/头/体 |
| 队列耗尽且最后无 HTTP 响应 | 网关错误 |
| 流式首包已提交后失败 | 终止当前流 + 单条最终日志，不换源 |
| 错误摘要 | 截断；脱敏 Key/Bearer；优先错误信封字段，禁止整段 messages |

### 5. Good / Base / Bad Cases

- **Good**：第一候选模型不支持 `400`，第二候选成功。
- **Base**：单候选成功，无 failover 字段。
- **Bad**：普通参数 `400` 直接结束且不尝试下一候选；因历史失败跳过队列首项。

### 6. Tests Required

- 集成：模型不支持 / 普通 400 / 404 / 5xx / 网络错误 → 第二候选成功。
- 集成：每次请求都从第一候选开始。
- 集成：全部 HTTP 失败透传最后响应；全部 transport 失败返回网关错误。
- 单元：`is_structured_error_body` 正反例（错误信封 vs completion/SSE）。
- 集成：流式首包后静默超时单日志、不换源。

### 7. Wrong vs Correct

#### Wrong

```rust
// 仅部分状态可换源；400 直接返回
if !is_retryable_status(status) { return non_retryable; }
// 熔断 Open 跳过供应商
if !circuits.allow_request(id) { continue; }
```

#### Correct

```rust
// 响应提交前任意失败继续下一启用候选
// 无 CircuitRegistry；队列耗尽时透传最后 HTTP 或返回网关错误
```

---

## Tauri invoke 约定

- 命令：`snake_case`。
- 成功：结构化 JSON。
- 失败：可序列化 message。

---

## Anti-Patterns

- 生产路径 `unwrap` 进程管理。
- UI 只显示「未知错误」。
- `stop` 超时后丢弃 JoinHandle 而不 `abort`（端口孤儿占用）。
- 为释放端口而 kill 无关第三方进程。
- 重新引入供应商熔断或 `auto_failover` 开关控制路由。
- 把非结构化 body 整段写入 request_logs。
