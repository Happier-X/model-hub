# Design: 流式 SSE 帧错误信封识别与换源

## 背景与现状

`attempt_stream_prime` 在收到 HTTP 2xx 后读首 chunk，再调 `is_structured_error_body(&first_chunk)`：

```rust
// 现状（简化）
if trimmed.starts_with(b"data:") || trimmed.starts_with(b"event:")
    || trimmed.starts_with(b":") || trimmed.starts_with(b"id:")
{
    return None; // 一律当正常 SSE
}
// 否则按裸 JSON 错误信封判定
```

因此 `data: {"error":"..."}` 永远不会被识别为错误，流被提交、转移停止。

## 目标行为

| 首包形态 | 行为 |
|---|---|
| HTTP 非 2xx | 既有：读 body → `http_failure` → 换源 |
| HTTP 2xx + 裸 JSON 错误信封 | 既有：换源 |
| HTTP 2xx + SSE 帧内错误信封 | **新增**：换源，不提交流 |
| HTTP 2xx + 正常 SSE（含 `choices`/`delta`/`[DONE]`/注释/`event:` 心跳） | 放行，提交流 |
| 首包已提交后的后续错误/中断 | 既有：不换源，终态回调记日志 |

## 方案

### 1. 扩展 `is_structured_error_body`（推荐，最小面）

在「SSE 前缀快速返回 None」之前，**先尝试解析 SSE 帧 payload**，再对 payload 复用既有 JSON 错误信封判定。

伪代码：

```text
fn is_structured_error_body(bytes) -> Option<String>:
    trimmed = strip_leading_ws(bytes)
    if trimmed empty: return None

    // 新增：若整体像 SSE，抽出 data 字段 payload 再判定
    if looks_like_sse(trimmed):
        payload = extract_sse_data_payload(trimmed)
        if payload is None:          // 纯注释 / 仅 event/id / 空 data
            return None              // 正常 SSE 放行
        if payload == "[DONE]":
            return None
        // 对 payload 走 JSON 错误信封判定（与裸 JSON 同规则）
        return classify_json_error_envelope(payload)

    // 既有：裸 JSON 路径
    return classify_json_error_envelope(trimmed)
```

#### `looks_like_sse`

首字节（跳过空白后）以 `data:` / `event:` / `:` / `id:` / `retry:` 开头（大小写敏感按 SSE 规范小写；兼容可选单空格 `data: `）。

#### `extract_sse_data_payload`

- 按行拆分（支持 `\n` 与 `\r\n`）。
- 收集所有 `data:` / `data: ` 行的值；多 `data:` 行按 SSE 规范用 `\n` 拼接。
- 忽略 `event:` / `id:` / `retry:` / 注释行（`:` 开头）/ 空行。
- 若无任何 `data:` 行 → `None`（无 payload，当正常放行）。
- 若拼接后 payload 去空白后为空 → `None`。
- **只解析首 chunk 内可见的事件**；不跨 chunk 重组（prime 只看首包，保持简单）。若错误帧被拆到第二 chunk，本轮不处理（属「已提交后」合同外）。

#### `classify_json_error_envelope`

把现有 `is_structured_error_body` 中「`serde_json` 解析 + choices/error/type/message 规则」抽成独立函数，供裸 JSON 与 SSE payload 共用，避免两套规则漂移。

规则（与现状一致，禁止放宽/收紧）：

- 有 `choices` 且 `type != "error"` → 非错误
- 有 `choices` 且 `type == "error"` → 错误
- 无 `choices` 且（字符串 `error` 非空 / 对象 `error.message` 非空 / `type=="error"` / 顶层 `message` 非空且 `object != "chat.completion"`）→ 错误
- 非 JSON → 非错误（SSE 里的非 JSON 文本不当错误信封）

### 2. `attempt_stream_prime` 调用点

保持现有调用：

```rust
if let Some(msg) = is_structured_error_body(&first_chunk) {
    return Err(AttemptError::Http { status, body: first_chunk, headers, message: format!(...) });
}
```

函数语义扩展后，调用点**无需改签名**；错误摘要仍走 `redact_sensitive_summary`。

### 3. 队列耗尽

SSE 帧错误被分类为 `AttemptError::Http { status: 200, body: first_chunk, ... }`。

- 中间失败：写尝试日志，继续下一候选（既有循环）。
- 耗尽且最后是该 2xx 错误：走既有「2xx 错误信封 → 升级 502」分支（`is_structured_error_body` 对 SSE 帧也能返回 `Some`，因此 `last_http` 的 2xx 升级逻辑自动生效）。**需确认**：耗尽分支调用的是 `is_structured_error_body(&body)`——body 仍是原始 SSE 帧字节；扩展后返回 `Some`，会升级 502。这是期望行为。

### 4. 不改动的边界

- 流提交后的 `stream_body_from_prime`：仍不回环换源。
- 非流式 `attempt_non_stream`：间接受益（同一函数），行为对裸 JSON 不变；若上游非流式却返回 SSE 帧，也会被识别——可接受（更严一点）。
- 候选构建、分组语义、overlay、前端：不动。

## 风险与权衡

| 风险 | 缓解 |
|---|---|
| 误伤正常 SSE 首包 | 严格复用 `choices` 规则；`[DONE]`/注释/无 data 一律放行；补正向用例 |
| 多 data 行/拆包 | 只解析首 chunk；错误帧通常小且单包；拆包属已提交后合同外 |
| `data:` 后无空格 / 多空格 | 按 SSE：`data:` 后可选单空格；实现时 strip 前缀后 trim 左空白 |
| 性能 | 仅首包、小 buffer 解析；无额外网络 |

## 测试策略

1. **单元**（`forward.rs` `#[cfg(test)]`）  
   - SSE 帧字符串 error / 对象 error.message → `Some`  
   - 正常 SSE choices delta → `None`  
   - `data: [DONE]` / 纯注释 / 仅 event → `None`  
   - 裸 JSON 错误 / 成功 completion 回归  
2. **集成**（`failover_any_error.rs` 风格 wiremock/本地 mock）  
   - 两候选：第一家流式返回 200 + `data: {"error":"..."}`，第二家成功 → 客户端拿到第二家；日志有第一家失败摘要  
   - 单候选 SSE 错误耗尽 → 最终 502（与 2xx 信封耗尽对齐）  
   - 正常 SSE 流不换源  

## 回滚

单文件逻辑 + 测试；回滚即还原 `is_structured_error_body` 的 SSE 早退。无 schema/API 变更。

## 兼容性

- 日志字段不变。  
- 对外 HTTP 合同：错误场景下客户端不再收到「200 + 错误帧流」，改为换源成功响应或耗尽后的 502——这是 bugfix 期望。
