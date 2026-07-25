# 技术设计：模型思考强度设置

## 架构总览

思考强度是分组级配置，落在数据层（`groups.thinking_effort`），在转发层（`forward.rs`）写入 upstream 请求前按 `upstream_model` 家族翻译成对应厂商字段。前端在分组编辑对话框提供档位选择。

```
客户端 ──model=分组名──▶ chat_completions
                            │  读 group.thinking_effort
                            ▼
                  forward_with_failover(candidates, body, effort)
                            │  每个候选项：rewrite_model 时
                            ▼
              apply_thinking_effort(body, upstream_model, effort)
                            │  家族识别 → 字段注入（客户端已声明则跳过）
                            ▼
                        上游 provider
```

关键点：档位是**分组级**，但字段翻译依赖**每个候选项的 upstream_model**（同分组内 GPT-5 与 Claude 会翻译成不同字段），因此注入必须发生在 `rewrite_model`（已知具体 `upstream_model`）而非入口。

## 数据层

### schema 变更

`groups` 表新增列：

```sql
thinking_effort TEXT NOT NULL DEFAULT 'off'
```

迁移沿用 `migrate.rs` 现有的 `ensure_group_columns` 加列模式（老库缺列则 `ALTER TABLE groups ADD COLUMN thinking_effort TEXT NOT NULL DEFAULT 'off'`）。取值域不做数据库层 CHECK 约束，由应用层校验（未知值回退 `off`）。

### 领域模型（`domain/group.rs`）

```rust
pub struct Group {
    // ...现有字段
    pub thinking_effort: String,  // off|minimal|low|medium|high|auto
}

pub struct CreateGroupPayload {
    pub name: String,
    pub thinking_effort: Option<String>,  // None → "off"
    pub items: Vec<GroupItemInput>,
}

pub struct UpdateGroupPayload {
    pub id: i64,
    pub name: String,
    pub thinking_effort: Option<String>,
    pub items: Vec<GroupItemInput>,
}
```

- `load_items` 所在的 group 查询 SELECT 增加 `thinking_effort`
- `create_group` / `update_group` 的 INSERT/UPDATE 写入该列；写入前用 `normalize_effort` 归一化（未知值 → `off`）
- `get_group_by_name` 返回值携带该字段（转发层要用）

### 档位归一化

```rust
/// 归一化档位；未知值回退 off。
fn normalize_effort(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "minimal" => "minimal",
        "low" => "low",
        "medium" => "medium",
        "high" => "high",
        "auto" => "auto",
        _ => "off",
    }
}
```

## 转发层（核心）

### 新增纯函数 `apply_thinking_effort`

放在 `forward.rs`（与 `strip_tool_strict` 同层，都是 body 改写辅助）。签名：

```rust
/// 按 upstream 模型家族翻译思考强度档位为对应厂商字段。
/// - effort == "off" → 入口直接 return，对所有家族都不改动 body
/// - 客户端已显式声明对应字段 → 保留，不覆盖
/// - 未识别家族 / 非推理模型 → 不注入
fn apply_thinking_effort(obj: &mut serde_json::Map<String, Value>, upstream_model: &str, effort: &str)
```

调用点：`rewrite_model` 内，`strip_tool_strict` 之后。`rewrite_model` 签名扩展为 `rewrite_model(body, upstream_model, effort)`。

### 家族识别

复用「小写 + 子串匹配」的轻量判定（不引入前端 `modelCapability.ts` 逻辑，Rust 侧独立实现最小识别）：

```rust
enum ThinkingFamily {
    OpenAiReasoning { supports_minimal: bool }, // gpt-5*, o1/o3/o4
    ClaudeThinking,                              // claude sonnet4/opus4/3.7
    QwenThinking,                                // qwen3*
    None,                                        // 其它一律不注入
}
```

识别规则（对 `upstream_model.to_lowercase()`）：

| 家族 | 匹配 | supports_minimal |
|------|------|------------------|
| OpenAiReasoning | 含 `gpt-5` / `gpt5` | true |
| OpenAiReasoning | 匹配 `o1`/`o3`/`o4` 词界（正则 `(^|[-_/])o[134]([-_/]|$)`）| false |
| ClaudeThinking | 含 `claude` 且含 `sonnet-4`/`sonnet4`/`opus-4`/`opus4`/`3-7`/`3.7` | - |
| QwenThinking | 含 `qwen3` 或（含 `qwen` 且含 `3`）| - |
| None | 其它（含 gpt-4o、claude haiku、deepseek、qwen-turbo 等）| - |

> 明确排除：Claude Haiku（不支持 extended thinking）、DeepSeek R1（推理内建无档位）、GPT-4o/4.1（非推理）。这些走 None，不注入。

### 字段映射

档位 → 各家族的具体值：

**OpenAiReasoning** → `reasoning_effort`（字符串）
| 档位 | 注入值 |
|------|--------|
| minimal | `"minimal"`（仅 supports_minimal；否则 `"low"`）|
| low | `"low"` |
| medium | `"medium"` |
| high | `"high"` |
| auto | `"medium"` |

**ClaudeThinking** → `thinking: {type: "enabled", budget_tokens: N}`
| 档位 | budget_tokens |
|------|---------------|
| minimal | 2048 |
| low | 4096 |
| medium | 8192 |
| high | 16384 |
| auto | 8192 |

> 注意 Anthropic 要求 `budget_tokens >= 1024` 且 `< max_tokens`。本设计不改 `max_tokens`；若客户端未给足够大的 `max_tokens`，上游会报错——这是客户端责任，代理仅注入 thinking。文档需提示。

**QwenThinking** → `enable_thinking`（布尔，顶层字段）
| 档位 | 注入值 |
|------|--------|
| minimal/low/medium/high/auto | `true` |

> **off 统一不注入**（见下方 off 语义）：off 时 Qwen 也不注入 `enable_thinking`，交由模型默认行为处理。这样 off 对所有家族都是"纯不改动"，语义干净、验收易守。
>
> **顶层 vs extra_body**：DashScope OpenAI 兼容端点期望 `enable_thinking` 出现在**请求体顶层**（OpenAI SDK 的 `extra_body` 最终就是序列化进顶层）。本代理直接注入顶层字段，对 DashScope 正确。**分歧点**：vLLM 的 OpenAI 兼容端点用 `chat_template_kwargs.enable_thinking` 而非顶层字段——若上游是自建 vLLM 部署，本注入不生效（但不会报错，vLLM 会忽略未知顶层字段）。本次仅覆盖 DashScope 形态。

### 客户端优先级（不覆盖）

注入前检查目标字段是否已存在：

```rust
// OpenAI
if obj.contains_key("reasoning_effort") { return; }
// Claude
if obj.contains_key("thinking") { return; }
// Qwen
if obj.contains_key("enable_thinking") { return; }
```

### off 档语义汇总

| effort | 行为 |
|--------|------|
| off | 所有家族：不注入（body 不变，除既有 `strip_tool_strict`）|
| 其它 | 按家族映射注入（客户端已声明则跳过）|

> **off 统一为纯不改动**：`apply_thinking_effort` 入口即 `if effort == "off" { return; }`。这是用户拍板的决策——off 对所有家族一致，Qwen 想关思考靠模型默认，不再对 off 做家族特判。这样验收标准「off 时 rewrite_model 输出逐字节相同」可稳守。

## 转发调用链改动

- `Candidate` 不变
- `forward_with_failover(...)` 新增参数 `effort: &str`（分组档位）
- `attempt_non_stream` / `attempt_stream_prime` 新增 `effort` 参数，透传给 `rewrite_model`
- `rewrite_model(body, upstream_model, effort)`：先 `insert model` → `strip_tool_strict` → `apply_thinking_effort`
- `server.rs` 的 `chat_completions`：读 `group.thinking_effort`，传入 `forward_with_failover`

## 前端改动

### 类型（`api/tauri.ts`）

```ts
export type ThinkingEffort = "off" | "minimal" | "low" | "medium" | "high" | "auto";
export interface Group { /* ... */ thinking_effort: ThinkingEffort; }
```

`createGroup` / `updateGroup` payload 增加 `thinking_effort`。

### GroupsPage.vue

- `GroupFormValues` 增加 `thinking_effort`
- 表单默认值 `"off"`；`startEdit` 时从 `g.thinking_effort` 回填
- 对话框内新增 `HSelect`（在分组名旁），选项：
  - 关闭 / 极简 / 低 / 中 / 高 / 自动（自动最佳）
- `onSubmit` payload 带上 `thinking_effort`
- 分组列表卡片：`thinking_effort !== "off"` 时显示徽章（如 `思考·高`）

## 兼容性与回滚

- **向后兼容**：默认 `off`，行为与当前逐字节一致。旧库迁移只加列不改数据
- **回滚**：新列保留无害（默认 off）；代码回退后该列被忽略
- **跨厂商**：同分组多候选项各自按 upstream_model 翻译，故障转移安全

## 外部契约（已 web 核对）

实现前已核对三个厂商字段规范：

- **OpenAI `reasoning_effort`**：取值 `none/minimal/low/medium/high/xhigh`（模型相关）。`minimal` 仅原版 GPT-5 / GPT-5-mini / GPT-5-nano 支持，GPT-5.1+ 不支持；o1 / o3 只支持 `low/medium/high`（无 minimal）。因此本设计中 GPT-5 家族 `supports_minimal=true`、o 系 `false`，且 o 系 minimal 档降级为 `low` 是安全的。
- **Anthropic `thinking`**：`{type:"enabled", budget_tokens:N}`，N 下限 1024（API 强制），上限可至 128000。本设计映射 2048/4096/8192/16384 均 ≥ 1024。约束 `budget_tokens < max_tokens`：**代理不改 `max_tokens`**，若客户端 `max_tokens` 不够大上游会报错，这是客户端责任（用户拍板：纯注入 thinking，不代抬 max_tokens）。文档需提示这一点。
- **Qwen `enable_thinking`**：非标准 OpenAI 参数。DashScope OpenAI 兼容端点通过 `extra_body` 注入，OpenAI SDK 会将其序列化为**顶层**字段——所以注入顶层 `enable_thinking` 对 DashScope 正确。vLLM 用 `chat_template_kwargs.enable_thinking`，本次不覆盖（已在上方标注）。
