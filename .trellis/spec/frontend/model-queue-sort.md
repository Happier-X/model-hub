# 分组队列模型排序

> 本地启发式 + OpenRouter 外部分数混合排序、匹配与 UI 约定。

---

## Scenario: 队列按能力排序

### 1. Scope / Trigger

- Trigger：GroupsPage 对故障转移队列按模型能力重排；依赖 IPC 榜单与本地启发式，跨层合同与错配风险需固定。

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

export const getModelLeaderboard = (forceRefresh = false) =>
  invoke<ModelLeaderboardSnapshot>("get_model_leaderboard", {
    force_refresh: forceRefresh,
  })
```

```ts
// src/utils/modelCapability.ts
export type QueueSortMode =
  | "local"
  | "external_intelligence"
  | "external_coding"

export function hybridSortKey(
  modelId: string,
  index: Map<string, MatchedExternalScore> | null,
): { key: number; external: MatchedExternalScore | null; local: ModelCapability }

export function sortByHybridCapability<T>(
  items: readonly T[],
  getModelId: (item: T) => string,
  index: Map<string, MatchedExternalScore> | null,
): T[]
```

### 3. Contracts

**排序方式**

| 模式 | 外部分字段 | 未命中 |
|------|------------|--------|
| `local` | 不用榜单 | 本地启发式 `scoreModelCapability` |
| `external_intelligence` | `intelligence_score` | 回退本地 |
| `external_coding` | `coding_score` | 回退本地 |

**混合 key**

- 外部命中且有分：`key = 1_000_000 + external_score * 1_000`（保证外部命中整体高于本地分带）。
- 未匹配 / 无对应分 / 无 index：`key = local.score`。
- 稳定排序：`b.key - a.key`，同 key 保持原下标顺序。

**匹配（高置信）**

- 归一化：小写、去厂商前缀、去日期后缀、统一分隔符/常见变体（见 `normalizeModelIdForMatch`）。
- **仅**归一化后 key **完全相等** 命中；禁止子串/模糊匹配。
- 索引可由 `id`、`canonical_slug`、`name` 建 key；同 key 冲突时取更高分（有意策略）。

**UI / 表单**

- 排序 **只改当前表单** `form.items`，**不得**自动 `save`。
- 展示：来源（OpenRouter/本地）、系列标签、分数；榜单状态含更新时间、条数、缓存命中/陈旧；强制刷新失败时状态行可附错误。
- 用户仍可拖拽微调顺序。

### 4. Validation & Error Matrix

| 条件 | 行为 |
|------|------|
| 选外部排序且尚无榜单 | 先 `getModelLeaderboard(false)`；失败 toast/状态文案，可继续本地排序 |
| 强制刷新失败但有旧快照 | 保留旧快照，状态提示「刷新失败：…」 |
| 模型未匹配 | 标签显示本地启发式，按本地分排序 |
| `invoke` 失败 | 不伪造空成功快照；错误可行动中文 |

### 5. Good/Base/Bad Cases

- **Good**：`deepseek-r1` 与榜单 id 归一化相等 → 用外部分，key ≥ `1_000_000`。
- **Base**：仅本地模式 → 不依赖网络，与既有启发式一致。
- **Bad**：`claude-sonnet-3` 不得误配成其它 Claude；裸 `claude` 不得高置信命中具体版本。

### 6. Tests Required

- `matchExternalScore`：精确命中 / 不误配邻近名。
- `hybridSortKey`：命中外部分带；未命中等于本地分。
- `sortByHybridCapability`：外部优先、未命中回退、同 key 稳定。
- 既有本地启发式单测保持通过。

### 7. Wrong vs Correct

#### Wrong

```ts
// 模糊包含匹配 → 错配风险
if (leaderboardId.includes(localId)) useExternal()
// 排序后立刻 saveGroup
await saveGroup(form)
```

#### Correct

```ts
const key = normalizeModelIdForMatch(modelId)
const hit = index.get(key) // 仅全等
// 只改 form.items，提示用户保存
form.items = sortByHybridCapability(...)
```

---

## 与组件规范关系

- 延续 `component-guidelines.md`：能力排序不自动保存、未知模型稳定偏后、可拖拽微调。
- 外部模式增加来源标注与缓存状态，评分仍须可读。
