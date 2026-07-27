# 模型排序改用 OpenRouter 榜单并支持模糊匹配

## Goal

让分组「故障转移队列」的能力排序更准、更好懂：排名完全以 OpenRouter 榜单（通用能力 intelligence）为准，上游模型名与 OpenRouter id 不一致时能通过分层模糊匹配命中，匹配不上的模型明确标「未匹配」并沉底；同时把用户看到的排序交互与分数展示大幅简化，不再区分本地分/外部分。

## Background / 现状

- 后端 `src-tauri/src/domain/leaderboard.rs`（本次**不改**）已实现 OpenRouter 榜单拉取：
  - 固定 `GET https://openrouter.ai/api/v1/models?sort=intelligence-high-to-low`，无 API Key。
  - 每条模型提取 `id / canonical_slug / name` 与 `benchmarks.artificial_analysis.{intelligence_index, coding_index, agentic_index}` → `intelligence_score / coding_score / agentic_score`。
  - 24h 文件缓存 + 网络失败 stale 回退。前端只消费 `intelligence_score`。
- 前端 `src/utils/modelCapability.ts`（本次重写核心）：
  - `scoreModelCapability` = 本地启发式硬编码打分（Claude/GPT/Gemini/DeepSeek/Qwen/Llama/Mistral），覆盖窄，其余模型 `score:0 / 未识别`。
  - `matchExternalScore` = 归一化后**完全相等**匹配，差一点就命中不了。
  - `hybridSortKey` / `sortByHybridCapability` = 外部命中优先、否则回退本地启发式。
  - `QueueSortMode = "local" | "external_intelligence" | "external_coding"`。
- 前端 `src/pages/GroupsPage.vue`（本次改交互）：排序方式三档下拉 + 每条模型「本地/OpenRouter · 家族 · 分数」标签 + tooltip 并列两套分 + 一堆缓存/回退文案。
- 引用范围：`modelCapability.ts` 仅被 `GroupsPage.vue` 与 `modelCapability.test.ts` 使用。spec `frontend/model-queue-sort.md` 描述旧混合排序契约，本次需同步重写。

## 用户反馈的痛点

1. 自动排序「识别不到」很多模型：本地启发式覆盖窄（Grok/Kimi/GLM/MiniMax/自定义名等一律 0 分沉底），外部严格匹配又太死（差一个后缀/前缀就漏）。
2. 交互太复杂、用户看不明白：三档排序模式 + 双来源分数 + 一堆缓存/回退状态文案。

## Requirements

### R1 排名完全以 OpenRouter 为准，砍掉本地启发式排名

- 移除本地启发式打分作为**排序依据**：不再维护 `scoreModelCapability` 的家族硬编码分。
- 能力排序只用 OpenRouter `intelligence_score`。
- `QueueSortMode` 三档收敛为单一 intelligence 排序，UI 不再提供排序模式切换（D1）。

### R2 上游模型名 → OpenRouter 分层模糊匹配（D3）

匹配分三层，越靠后越宽松，但都带防错配护栏，**不做纯相似度/编辑距离匹配**：

1. **精确**：归一化后 key 完全相等（保留现有 `normalizeModelIdForMatch`）。
2. **归一化增强**：补齐去噪——厂商前缀、`-latest/-instruct/-chat/-it` 等渠道后缀、日期后缀、量化后缀（`fp8/int4/awq/gptq/gguf` 等）后再精确比。
3. **受控近似**（新增，带护栏）：仅当一侧归一化 key 是另一侧的**前缀**，且被截掉的剩余部分**不含改变模型档位的判别 token** 时才命中。
   - 判别 token：`mini / nano / small / large / pro / flash / lite / tiny / haiku / sonnet / opus / turbo / plus / max` 及参数量段（如 `7b / 72b / 405b`）。
   - 命中多个候选时，取 `intelligence_score` 最高者。
   - 反例护栏：`gpt-4o` **不得**命中 `gpt-4o-mini`（差 `mini` 是判别 token）；裸 `claude` 不得命中具体版本；`claude-sonnet-3` 不得配到其它 Claude。

### R3 简化排序交互与分数展示

- 移除排序方式下拉，改为单一「按能力排序」按钮（内部固定 intelligence）。
- 每条模型标签只呈现一套 OpenRouter 能力信号：命中显示分数（可含匹配层级/来源进 tooltip），未命中显示「未匹配」。不再并列本地分/外部分。
- 未匹配模型（D2）：排序时**统一沉底 + 保持彼此原有相对顺序**（稳定排序），不显示假分数。
- 精简榜单状态文案：保留「更新时间 / 条数 / 缓存或陈旧 / 刷新失败」的必要信息，去掉与「本地/外部混合」相关的解释。
- 保留：排序只改当前表单 `form.items`、不自动保存、用户可拖拽微调（延续 `component-guidelines.md`）。

## Acceptance Criteria

- [ ] AC1 队列点「按能力排序」后，能命中 OpenRouter 榜单的模型按 `intelligence_score` 降序在前；未匹配模型沉底且彼此保持原相对顺序。
- [ ] AC2 UI 不再有排序方式下拉；仅一个「按能力排序」按钮 + 榜单状态/刷新入口。
- [ ] AC3 每条模型标签命中时显示 OpenRouter 分数，未命中显示「未匹配」；不再出现「本地 · 家族 · 分数」双来源展示。
- [ ] AC4 分层匹配单测覆盖：精确命中、去噪后命中（日期/渠道/量化后缀）、前缀受控近似命中；且 `gpt-4o` 不命中 `gpt-4o-mini`、裸 `claude` 不命中、`claude-sonnet-3` 不误配。
- [ ] AC5 榜单不可用（无缓存且网络失败）时，点排序给出可行动中文提示并保持原顺序，不伪造空成功。
- [ ] AC6 `modelCapability.ts` 不再导出/使用本地启发式打分作为排序依据；`QueueSortMode` 三档相关死代码清除；旧测试同步更新。
- [ ] AC7 前端类型检查、lint、`node --test` 单测全绿；spec `frontend/model-queue-sort.md` 同步为新契约。

## Out of Scope

- 后端 `leaderboard.rs` 拉取/缓存逻辑（保持不变，仅前端消费方式变化）。
- 编码能力（coding）/ agentic 指标排序（本次砍掉，后端字段保留不用）。
- 纯相似度/编辑距离/AI 语义匹配。
- 绑定型分组（source_provider_id 托管、只读队列）的排序交互（本就无排序 UI）。

## 已定决策

- D1（Q1）排序指标收敛为**单一「通用能力(intelligence)」**，砍「编码能力」「本地启发式」档，UI 不再提供排序模式切换。
- D2（Q2）模糊匹配失败模型：**统一沉底 + 保持彼此原有相对顺序（稳定排序）**，标签标「未匹配」，不显示假分数。
- D3（Q3）模糊匹配采用**分层（精确 → 归一化增强 → 前缀 + 判别 token 护栏）**，不做纯相似度/编辑距离匹配；多候选取分最高。

## Risks

- 前缀近似仍可能有个别边界错配（如两个正当模型恰好前缀包含关系且剩余无判别 token）；用判别 token 清单 + 单测反例守住主要档位错配，边界 case 由用户拖拽微调兜底。
- 判别 token 清单需覆盖主流档位词，遗漏会漏配或误配；清单集中定义、便于后续扩充。
