# 增加模型思考强度设置与自动最佳档位

## 目标

给每个分组一个"思考强度（thinking effort）"配置，在代理转发时按 upstream 模型家族自动翻译成对应厂商字段（OpenAI `reasoning_effort` / Anthropic `thinking.budget_tokens` / Qwen `enable_thinking` 等）。用户可选"自动"档位，由后端按 upstream_model 家族识别选择当前最佳档位；用户也可显式指定 `off/minimal/low/medium/high`。

## 背景与动机

当前 model-hub 只做无参数的透明转发。GPT-5、Claude Sonnet 4、DeepSeek R1、Qwen3 等新一代模型都有可控的推理/思考档位，但不同厂商字段和取值域完全不同（`reasoning_effort` vs `thinking.budget_tokens` vs `enable_thinking`）。让客户端逐个适配成本高，且当同一分组内跨厂商故障转移时，客户端不可能预知会切到哪家。合适的落点是把"思考强度"抽象成分组级配置，由代理在写入 upstream 请求前按目标模型家族翻译。

## 用户价值

- **零改客户端**：客户端只发 `model=<分组名>`，思考强度由后台统一注入
- **跨厂商一致**：同一分组内故障转移时不用担心字段兼容
- **自动档位**：不懂各家字段的用户选 `auto`，系统按模型家族推荐最佳档位

## 需求

### 功能需求

1. **分组级配置**：每个分组新增字段 `thinking_effort`，取值 `off | minimal | low | medium | high | auto`，默认 `off`（保持零侵入的原有行为）
2. **前端配置入口**：分组新建 / 编辑对话框内新增下拉选择器；分组列表卡片展示当前档位
3. **转发层注入**：转发前根据分组配置和 `upstream_model` 家族翻译并注入对应字段
4. **家族识别 → 字段映射**（后端）：
   - **OpenAI GPT-5 / o1 / o3 / o4 系列**：注入 `reasoning_effort: <档位>`（`minimal` 仅 GPT-5 支持，其余家族退回 `low`）
   - **Claude Sonnet 4 / Opus 4 / 3.7 系列**：注入 `thinking: {type: "enabled", budget_tokens: <档位对应预算>}`
   - **Qwen3 系列**：注入 `enable_thinking: true`（off 时不注入，交由模型默认，保持 off 的纯粹"不改动"语义）
   - **DeepSeek Reasoner (R1)**：模型自身即推理模型，无档位参数，不注入
   - **未识别家族 / 非推理模型（GPT-4o、Claude Haiku 3.5、Qwen Turbo 等）**：不注入
5. **auto 档位策略**：按模型家族给一个"稳妥的默认"—— GPT-5 → `medium`；o 系列 → `medium`；Claude 推理系列 → `medium`（budget_tokens 约 8000）；Qwen3 → `enable_thinking: true`；其余不注入
6. **客户端优先级**：若客户端请求体内已带 `reasoning_effort` / `thinking` / `enable_thinking`，保留客户端值不覆盖（客户端显式声明高于分组默认）
7. **数据迁移**：老库无 `thinking_effort` 列时自动加列，默认 `off`；沿用现有 `ensure_group_columns` 迁移模式

### 非功能需求

- **零侵入**：`thinking_effort=off`（默认）时转发行为与当前完全一致，对**所有家族**都不改动 body（含 Qwen——off 不注入 `enable_thinking:false`，只靠模型默认）
- **可测**：家族识别与档位映射逻辑独立成纯函数，Rust 单测覆盖每个家族分支
- **可观测**：请求日志无需新列，但转发层在注入字段时打点（`tracing::debug`）便于排障

### 范围外（本次不做）

- 每个 `group_item`（provider-model）独立档位：目前 auto 已按 upstream_model 家族区分，用户 A/B 场景不常见
- 按任务复杂度动态选档位（消息长度 / 是否含代码等启发式）：先给稳定的家族默认，动态策略后续再迭代
- 自定义 budget_tokens 数值：档位映射固定，用户不填数字
- 修改 `/v1/models` 响应结构：分组名对外表达不变

## 验收标准

- [ ] `groups` 表新增 `thinking_effort` 列（TEXT NOT NULL DEFAULT 'off'），旧库迁移后已有分组保持 `off`
- [ ] `CreateGroupPayload` / `UpdateGroupPayload` / `Group` 结构含 `thinking_effort` 字段；前端 `Group` TS 类型同步
- [ ] 分组新建 / 编辑对话框内可选择档位（下拉：关闭 / 极简 / 低 / 中 / 高 / 自动）；保存后回显正确
- [ ] 分组列表卡片显示当前档位（非 `off` 时可见徽章）
- [ ] 转发单测覆盖：
  - GPT-5 + `high` → body 含 `reasoning_effort: "high"`
  - Claude Sonnet 4 + `medium` → body 含 `thinking: {type: "enabled", budget_tokens: 8192}`
  - Qwen3 + `auto` → body 含 `enable_thinking: true`
  - Qwen3 + `off` → body 不含 `enable_thinking`（off 统一不注入）
  - GPT-4o + `high` → body 不含 `reasoning_effort`（非推理模型）
  - `auto` + GPT-5 → body 含 `reasoning_effort: "medium"`
  - 客户端已带 `reasoning_effort` → 保留客户端值
  - o3 + `minimal` → body 含 `reasoning_effort: "low"`（o 系列不支持 minimal，降级）
- [ ] `thinking_effort=off` 时 `rewrite_model` 输出与本次改动前逐字节相同（除既有的 `strip_tool_strict` 行为）
- [ ] `pnpm typecheck` / `pnpm lint` / `cargo test` / `cargo check` 全部通过
