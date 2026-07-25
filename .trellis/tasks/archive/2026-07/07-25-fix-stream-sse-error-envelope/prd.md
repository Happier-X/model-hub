# PRD: 修复流式 SSE 帧错误信封漏检导致不换源

## 问题描述

上游供应商（如 xAI/Grok 网关）在**流式**请求下可能返回：

- HTTP 状态码 `200`
- Content-Type 为 `text/event-stream`（或等价 SSE）
- 首包（或前几个 chunk）是 **SSE 帧**，形如：

  ```
  data: {"error":"Invalid or expired credentials (auth_kind=bearer, ...)"}

  ```

  或

  ```
  data: {"error":{"message":"No available accounts. Add an account first.","type":"invalid_request_error"}}

  ```

当前 `is_structured_error_body()` 对首包以 `data:` / `event:` / `:` / `id:` 开头的内容**直接返回 `None`**，视为正常 SSE 放行。结果：

1. `attempt_stream_prime` 把该响应当成功流提交给客户端；
2. 故障转移循环**停止**，后续候选（分组队列里还有很多）不再尝试；
3. 请求日志记为 `status_code=200` + 空 `error`（或 defer 后的成功日志），用户侧却在流里读到认证/账号类错误；
4. 从用户视角看就是「报了 401 类错误却没触发故障转移」。

非流式路径与裸 JSON 错误信封（HTTP 200 + 非 SSE 的 `{"error":...}`）在上轮任务 `07-25-fix-failover-2xx-error-envelope` 已覆盖；本任务只补 **流式 SSE 帧内错误信封** 这一缺口。

## 影响范围

- 主要：`src-tauri/src/proxy/forward.rs`
  - `is_structured_error_body`（或新增的 SSE 帧解析辅助函数）
  - `attempt_stream_prime` 的首包判定
- 测试：`src-tauri/tests/failover_any_error.rs`（及必要时 `proxy_failover.rs`）
- 文档：`.trellis/spec/backend/error-handling.md` 的故障转移判定表
- **不改**：非流式路径、响应提交后的流中断语义、客户端断开 499、overlay、前端

## 约束

1. **仅响应提交前可换源**——首包已提交客户端后仍禁止拼接第二家（既有合同不变）。
2. **不能误伤正常 SSE**：正常 chat completion 首包形如 `data: {"id":"...","choices":[{"delta":{"content":"..."}}]}`，必须继续放行、不换源。
3. **判定口径与既有 2xx 错误信封一致**：字符串 `error`、对象 `error.message`、`type: "error"`、无 `choices` 时的顶层 `message` 等；有 `choices` 且非 `type=error` 的不当错误。
4. **保持脱敏**：错误摘要进日志前仍走 `redact_sensitive_summary`。
5. 不引入熔断、不按状态码白名单过滤；「错误即换源」合同不变。
6. Overlay 窗口与生命周期零触碰。

## 验收标准

- [ ] 流式请求下，上游返回 HTTP 200 + SSE 帧内错误信封（`data: {"error":"..."}` 或 `data: {"error":{"message":"..."}}`）时，**首包提交前换源**到下一候选；不再把错误帧提交给客户端。
- [ ] 流式请求下，正常 SSE 首包（`data: {... "choices":[...]}` 或 `data: [DONE]` 等非错误形态）**不换源**，行为与现网一致。
- [ ] 非 SSE 的裸 JSON 错误信封（既有路径）行为不变；非流式路径行为不变。
- [ ] 队列耗尽且最后失败是 SSE 帧错误信封时：不向客户端提交该错误帧流；按既有耗尽语义处理（透传最后 HTTP 或升级为 502——与 2xx 错误信封耗尽规则对齐）。
- [ ] 请求日志：中间失败候选写入尝试失败摘要（含状态码与截断错误文案）；最终成功候选正常记成功；最终失败不误记为 200 成功。
- [ ] 新增/扩展自动化测试覆盖：SSE 帧错误换源、正常 SSE 不换源、非流式回归。
- [ ] `cargo test`（含相关集成测）、`cargo build`、前端 `pnpm typecheck/lint/test:unit` 不受影响（本任务无前端改动）。
- [ ] `.trellis/spec/backend/error-handling.md` 更新故障转移判定表，写明 SSE 帧内错误信封也换源。

## 非目标

- 不处理「流已提交后中途出现错误帧」的换源（合同禁止拼接第二家）。
- 不改变分组候选构建策略（仍整组遍历，不做模型名过滤）。
- 不修上游供应商本身的账号/额度问题。
- 不改前端 UI / happier-ui。

## Notes

- 用户现场证据：日志 `status_code=200`，客户端却收到 `Invalid or expired credentials (auth_kind=bearer, x_xai_token_auth=xai-grok-cli, ...)`；分组队列有 270+ 候选，却停在返回 200 的那一家。
- 相关已归档任务：`07-25-fix-failover-2xx-error-envelope`（非流式/裸 JSON 2xx 信封 + 耗尽升级 502）。
