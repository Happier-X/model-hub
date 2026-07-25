# 思考强度注入约定

> 分组级 `thinking_effort` 配置，在转发前按 upstream 模型家族翻译成各厂商思考/推理字段。

---

## Scenario: 分组思考强度 → 上游厂商字段

### 1. Scope / Trigger

- Trigger：任何改动 `groups.thinking_effort` 存储、`apply_thinking_effort` 注入逻辑、家族识别 `thinking_family`、或档位映射的代码路径。
- 目标：客户端零改动，仅发 `model=<分组名>`；代理按每个候选项的 `upstream_model` 家族翻译档位为对应厂商字段。
- 关键约束：档位是**分组级**，但字段翻译依赖**每个候选项的 upstream_model**（同分组内 GPT-5 与 Claude 翻译成不同字段），因此注入必须在 `rewrite_model`（已知具体 upstream_model）而非入口。

### 2. Signatures / DB

**DB（`db/migrate.rs::ensure_group_columns`）**

```sql
ALTER TABLE groups ADD COLUMN thinking_effort TEXT NOT NULL DEFAULT 'off'
```

- 老库缺列则加列，默认 `off`（零侵入）。取值域**不做 DB CHECK**，由应用层 `normalize_effort` 校验。

**领域层（`domain/group.rs`）**

```rust
pub struct Group { /* ... */ pub thinking_effort: String }
pub struct CreateGroupPayload { pub name: String, pub thinking_effort: Option<String>, pub items: Vec<GroupItemInput> }
pub struct UpdateGroupPayload { pub id: i64, pub name: String, pub thinking_effort: Option<String>, pub items: Vec<GroupItemInput> }

/// 归一化档位；未知值回退 off。None → "off"。
fn normalize_effort(raw: &str) -> &'static str
```

- `list_groups` / `get_group_by_name` 的 SELECT 必须含 `thinking_effort`。
- `create_group` / `update_group` 写入前用 `normalize_effort(payload.thinking_effort.as_deref().unwrap_or("off"))` 归一化。

**转发层（`proxy/forward.rs`）**

```rust
fn thinking_family(upstream_model: &str) -> ThinkingFamily
fn apply_thinking_effort(obj: &mut serde_json::Map<String, Value>, upstream_model: &str, effort: &str)
fn rewrite_model(body: &Value, upstream_model: &str, effort: &str) -> Value  // 顺序：insert model → strip_tool_strict → apply_thinking_effort
```

- `forward_with_failover` / `attempt_non_stream` / `attempt_stream_prime` 均带 `effort: &str` 参数透传。
- `server.rs::chat_completions` 读 `group.thinking_effort` 传入。

### 3. Contracts（家族识别 + 档位映射）

**家族识别**（对 `upstream_model.to_ascii_lowercase()`）：

| 家族 | 匹配规则 | supports_minimal |
|------|----------|------------------|
| OpenAiReasoning | 含 `gpt-5` / `gpt5` | true |
| OpenAiReasoning | 词界匹配 `o1`/`o3`/`o4`（`(^\|[-_/])o[134]([-_/]\|$)`）| false |
| ClaudeThinking | 含 `claude` 且含 `sonnet-4`/`sonnet4`/`opus-4`/`opus4`/`3-7`/`3.7` | - |
| QwenThinking | 含 `qwen3` 或（含 `qwen` 且含 `3`）| - |
| None | 其它（gpt-4o、claude haiku、deepseek、qwen-turbo 等）| - |

**档位映射**：

| effort | OpenAI `reasoning_effort` | Claude `thinking.budget_tokens` | Qwen `enable_thinking` |
|--------|---------------------------|--------------------------------|------------------------|
| off | 不注入 | 不注入 | 不注入 |
| minimal | `minimal`（否则 `low`）| 2048 | true |
| low | `low` | 4096 | true |
| medium | `medium` | 8192 | true |
| high | `high` | 16384 | true |
| auto | `medium` | 8192 | true |

**外部契约（已 web 核对，勿凭记忆改）**：

- **OpenAI**：`minimal` 仅原版 GPT-5/mini/nano 支持；o1/o3 只支持 `low/medium/high`。故 o 系 `supports_minimal=false`，minimal 降级 `low`。
- **Anthropic**：`budget_tokens` 下限 1024（API 强制）且需 `< max_tokens`。代理**不代抬 max_tokens**；客户端未给足则上游报错，属客户端责任。映射值 2048/4096/8192/16384 均 ≥ 1024。
- **Qwen**：DashScope OpenAI 兼容端点将 `enable_thinking` 序列化为**顶层**字段，故注入顶层正确。vLLM 用 `chat_template_kwargs.enable_thinking`，本实现不覆盖（会被忽略，不报错）。

### 4. Validation & Error Matrix

| 输入 | 行为 |
|------|------|
| `thinking_effort` 未知值 / None | `normalize_effort` 回退 `off` |
| `effort == "off"`（任意家族）| `apply_thinking_effort` 入口即 return，body 不改动 |
| 客户端已带 `reasoning_effort`/`thinking`/`enable_thinking` | 保留客户端值，不覆盖 |
| None 家族（非推理模型）| 不注入任何字段 |

### 5. Good / Base / Bad Cases

- **Good**：分组 `high` + upstream `gpt-5` → 注入 `reasoning_effort:"high"`。
- **Base**：分组 `off`（默认）→ 转发行为与改动前逐字节一致（除既有 `strip_tool_strict`）。
- **Bad（应避免）**：对 off 档为 Qwen 注入 `enable_thinking:false` —— 决策明确统一「off 纯不改动」，不做家族特判。

### 6. Tests Required（`forward.rs` 单测断言点）

- `family_gpt5_supports_minimal` / `family_o_series_no_minimal` / `family_claude_thinking` / `family_qwen3_thinking`：家族识别分支
- `openai_injects_reasoning_effort`：GPT-5 high → `reasoning_effort=="high"`
- `o_series_minimal_downgrades_to_low`：o3 minimal → `reasoning_effort=="low"`
- `claude_injects_thinking_budget`：sonnet-4 medium → `thinking.budget_tokens==8192`
- `qwen_injects_enable_thinking_true`：qwen3 auto → `enable_thinking==true`
- `off_never_injects_any_family`：off 各家族均不注入
- `non_reasoning_family_never_injects`：gpt-4o high → 无 `reasoning_effort`
- 客户端已带字段 → 不覆盖（断言值为客户端原值）
- `domain/group.rs`：创建带档位、更新档位、未知值回退 off

### 7. Wrong vs Correct

```rust
// ❌ 错：在入口（不知道具体 upstream_model）注入，同分组跨厂商会注错字段
fn chat_completions(...) { apply_thinking_effort(&mut body, effort); forward(...); }

// ✅ 对：在 rewrite_model 内注入，此处已知每个候选项的 upstream_model
fn rewrite_model(body, upstream_model, effort) -> Value {
    // insert model → strip_tool_strict → apply_thinking_effort(obj, upstream_model, effort)
}
```

```rust
// ❌ 错：off 对 Qwen 特判注入 false，破坏「off 逐字节不变」验收
if effort == "off" { obj.insert("enable_thinking", false.into()); }

// ✅ 对：off 入口统一 return，对所有家族不改动 body
fn apply_thinking_effort(obj, upstream_model, effort) {
    if effort == "off" { return; }
    // ...
}
```
