# 执行计划: 流式 SSE 帧错误信封识别与换源

## 前置

- 分支 `master`，工作区干净
- 不动：overlay、前端、候选构建、分组语义、数据库 schema
- 单文件为主：`src-tauri/src/proxy/forward.rs`；测试可能加到 `src-tauri/tests/failover_any_error.rs`

## 步骤

### 1. 抽出 JSON 错误信封判定为独立函数
- [ ] 在 `forward.rs` 新增 `fn classify_json_error_envelope(bytes: &[u8]) -> Option<String>`
- [ ] 把现有 `is_structured_error_body` 里「serde_json 解析 + choices/error/type/message」规则整体搬入，行为**逐字保持**
- [ ] 保证既有单测（`structured_error_*` / `success_completion_not_error` / `empty_choices_*`）不改断言即可通过

### 2. 新增 SSE 帧解析
- [ ] `fn looks_like_sse(trimmed: &[u8]) -> bool`：前缀 `data:` / `event:` / `:` / `id:` / `retry:`
- [ ] `fn extract_sse_data_payload(trimmed: &[u8]) -> Option<Vec<u8>>`：
  - 按 `\n`/`\r\n` 拆行
  - 收集 `data:`（可选单空格）行值，多行以 `\n` 拼接
  - 忽略 `event:`/`id:`/`retry:`/注释(`:`)/空行
  - 无 data 行或 payload 空 → `None`

### 3. 改写 `is_structured_error_body`
- [ ] 保留开头 strip 空白 + empty→None
- [ ] `if looks_like_sse(trimmed)`：
  - `extract_sse_data_payload` → `None` 则返回 `None`（正常 SSE 放行）
  - payload trim 后 == `[DONE]` → `None`
  - 否则 `classify_json_error_envelope(&payload)`
- [ ] 否则走 `classify_json_error_envelope(trimmed)`（裸 JSON）
- [ ] 确认 `attempt_stream_prime` 与耗尽分支调用点无需改签名

### 4. 单元测试（`forward.rs` #[cfg(test)]）
- [ ] `sse_data_error_string_failovers`：`data: {"error":"invalid key"}` → `Some`
- [ ] `sse_data_error_object_message`：`data: {"error":{"message":"..."}}` → `Some`
- [ ] `sse_normal_delta_not_error`：`data: {"choices":[{"delta":{"content":"hi"}}]}` → `None`（回归既有 `sse_first_chunk_not_error`）
- [ ] `sse_done_not_error`：`data: [DONE]` → `None`
- [ ] `sse_comment_only_not_error`：`: ping` / `event: message` → `None`
- [ ] `sse_multiline_data_error`：多 `data:` 行拼成错误 JSON → `Some`
- [ ] 裸 JSON 与成功 completion 回归不变

### 5. 集成测试（`failover_any_error.rs`）
- [ ] 参照现有 `structured_2xx_error_failovers_before_stream_commit` 加：
  - 第一家流式 200 + `data: {"error":"..."}`，第二家正常流式成功 → 客户端得第二家；断言换源发生
  - 单候选 SSE 错误耗尽 → 最终 502
  - 正常 SSE 首包流不换源（防误伤）

### 6. 全量验证
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml`
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`（若项目用 clippy）
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml`（含新单测 + 集成）
- [ ] `cargo build --manifest-path src-tauri/Cargo.toml`
- [ ] 确认前端/overlay 无触碰

## 验证命令
```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
```

## 回滚点
- 第 3 步改写后若集成测试暴露误伤正常流 → 收紧 `extract_sse_data_payload`（仅单 data 行 + 严格 JSON），或回退到早退实现
- 单文件改动，`git checkout src-tauri/src/proxy/forward.rs` 即回滚

## 审查门
- Gate A（第 1 步后）：既有单测全绿 = 重构无行为漂移
- Gate B（第 4 步后）：新单测覆盖错误/正常两侧
- Gate C（第 5 步后）：集成验证换源真实发生且不误伤
- Gate D（第 6 步）：fmt/clippy/test/build 全绿
