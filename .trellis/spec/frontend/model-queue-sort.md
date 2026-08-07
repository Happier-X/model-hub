# 分组队列模型排序

> llm_benchmark code_v3（Agentic）榜行序排序与分层模糊匹配契约。

---

## Scenario: 队列按能力排序

### 1. Scope / Trigger

- Trigger：GroupsPage 对故障转移队列按模型能力重排；完全依赖 IPC 透传的 llm_benchmark 榜单（code_v3 Agentic 榜），不设本地硬编码打分。

### 2. Signatures

```ts
// src/api/tauri.ts
export interface LeaderboardModel {
  id: string
  canonical_slug?: string | null
  name?: string | null
  intelligence_score?: number | null
  coding_score?: number | null
  agentic_score?: number | null
}

export interface ModelLeaderboardSnapshot {
  source: string
  fetched_at_unix: number
  stale: boolean
  cache_hit: boolean
  models: LeaderboardModel[]
}
```

```ts
// src/utils/modelCapability.ts
export type MatchTier = "exact" | "normalized" | "prefix";

export interface MatchedExternalScore {
  score: number; // agentic_score（code_v3 行序倒排分）
  leaderboardId: string;
  sourceLabel: string;
  tier: MatchTier;
}

export function sortQueueByLeaderboard<T>(
  items: readonly T[],
  getModelId: (item: T) => string,
  index: Map<string, MatchedExternalScore> | null,
): T[]
```

### 3. Contracts

**排序方式**

- 仅单一指标：按 llm_benchmark code_v3（Agentic）榜行序倒排分降序排序（首行分最高，与网站 Agentic 标签页展示顺序一致）。
- **未命中沉底**：匹配不到榜单的模型，整体排在所有命中模型之后。
- **稳定排序**：同分项、或都是未匹配项时，彼此保持原下标相对顺序。

**分层匹配**

上游模型名映射到 llm_benchmark 榜单条目，按顺序采取三层尝试（命中即返回，若单层多候选则取分数最高者）：

1. **精确（exact）**：双侧归一化 key 完全相等。
2. **归一化增强（normalized）**：两侧剥离厂商前缀、日期、渠道（`-latest` / `-instruct` / `-chat` 等）、量化后缀（`fp8` / `gguf` 等），再比较是否完全相等。
3. **受控近似（prefix）**：仅当一侧的增强 key 是另一侧前缀（以 `-` 为边界），且**被截掉的剩余部分不含档位判别 token** 时，才允许命中。
   - 判别 token 包含：`mini`、`nano`、`pro`、`large`、`haiku`、`sonnet`、`opus`、`max` 等词，以及参数量段（`7b`、`72b`）。
   - 目的：宁可未匹配，绝不错配。如 `gpt-4o` 绝不得前缀命中 `gpt-4o-mini`。

**展示名净化（llm_benchmark 特有）**

- 榜单条目是展示名（如 `Claude Fable 5 (high)`、`DeepSeek V4 Flash 0731 (max)`），无 API id。
- 归一化时先剥离 `(...)` 档位/后缀内容，再剥离纯 4 位数字段（MMDD/YYMM 日期），两侧对称（API 名与展示名走同一归一化）。
- 例：`Claude Fable 5 (high)` → `claude-fable-5`；`DeepSeek V4 Flash 0731 (max)` → `deepseek-v4-flash`。

**UI / 表单**

- 排序**编辑态**成功后自动 `updateGroup` 落库并留在页面（提示「已保存，可继续拖拽微调」）；**新建态**（无分组 id）只改表单，提示保存后生效，不自动创建分组。
- 展示：命中时显示 `llm_benchmark · {分数}`，未命中时灰底显示 `未匹配`。
- 榜单状态含更新时间、条数、缓存或陈旧；刷新失败时附中文错误提示。
- 用户仍可拖拽微调顺序。

### 4. Validation & Error Matrix

| 条件 | 行为 |
|------|------|
| 点击排序且尚无榜单 | 先 `getModelLeaderboard(false)`；如果失败（断网且无缓存），toast 中文提示，队列保持原顺序 |
| 模型未匹配 | 标签显示「未匹配」，排序时沉底且保持原相对顺序 |
| `invoke` 失败 | 不伪造空成功快照；抛出真实可行动的错误信息 |

### 5. Good/Base/Bad Cases

- **Good (Exact)**：API `claude-fable-5` → 归一化 `claude-fable-5` 命中展示名 `Claude Fable 5 (high)`。
- **Good (Prefix)**：`gpt-4o-custom` → 剥离 `custom`（非判别 token）后前缀命中 `gpt-4o`。
- **Base (Miss)**：`company-internal-model` 不在榜单 → 未匹配沉底。
- **Bad (Guardrail)**：`gpt-4o-mini` → 剩余部分含 `mini`（判别 token）→ 不得命中榜单的 `gpt-4o`。

### 6. Tests Required

- `matchModelToLeaderboard`：精确命中、归一化去噪命中、前缀命中、护栏拦截错配案例（必须包含 `mini/sonnet` 等档位词防线测试）。
- `sortQueueByLeaderboard`：命中降序在前、未匹配沉底、同分或同状态时稳定保持原序。

### 7. Wrong vs Correct

#### Wrong

```ts
// 模糊包含匹配 → 极易发生档位错配
if (leaderboardId.includes(localId)) match()
// 排序后立刻 saveGroup
await saveGroup(form)
// 回退到本地硬编码打分兜底
if (!hit) useLocalScore()
// 直接拿展示名比较（没剥 `(xhigh)` 档位与 4 位日期）
if (entry.name === apiModelName) match()
```

#### Correct

```ts
// 受控匹配：前缀且无判别 token 冲突
const hit = matchModelToLeaderboard(modelId, index)
// 只改 form.items，提示用户保存
applySortedItems(sorted, "已按能力排序...")
// 未匹配模型统一沉底
if (!a.match && b.match) return 1
```

---

## 与组件规范关系

- 排序编辑态自动保存（`autoSaveAfterSort`：updateGroup + 留在页面），新建态仅改表单；仍可拖拽微调。
- 外部模式直接作为唯一指标，精简交互理解成本。
