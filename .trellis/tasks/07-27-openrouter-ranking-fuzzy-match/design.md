# 技术设计：OpenRouter 榜单排序 + 分层模糊匹配

## 边界与影响面

| 层 | 文件 | 改动 |
|----|------|------|
| 匹配/排序核心 | `src/utils/modelCapability.ts` | 重写：删本地启发式排序依据，新增分层匹配 + 单一 intelligence 排序 |
| 单测 | `src/utils/modelCapability.test.ts` | 重写：删本地打分/三档相关用例，补分层匹配 + 未匹配沉底用例 |
| 交互 | `src/pages/GroupsPage.vue` | 删排序方式下拉、`QueueSortMode` 状态；单一「按能力排序」按钮；标签只显 OpenRouter 分/未匹配；精简文案 |
| spec | `.trellis/spec/frontend/model-queue-sort.md` | 重写为新契约 |
| 后端 | `src-tauri/src/domain/leaderboard.rs` | **不动**（继续返回三指标，前端只取 intelligence） |
| IPC 类型 | `src/api/tauri.ts` | 不动（`LeaderboardModel` 已含 `intelligence_score`） |

## 核心数据流

```
getModelLeaderboard(force) → ModelLeaderboardSnapshot { models[] }
   ↓ buildExternalScoreIndex(models)      // 只索引 intelligence_score
Map<normKey, { score, leaderboardId, sourceLabel }>
   ↓ matchModelToLeaderboard(upstreamModel, index)   // 分层匹配
MatchResult { score, leaderboardId, tier } | null
   ↓ sortQueueByLeaderboard(items, index)
命中项按 score 降序在前；未匹配项沉底且保持原序
```

## 关键类型（改后）

```ts
export interface ExternalLeaderboardEntry {
  id: string;
  canonical_slug?: string | null;
  name?: string | null;
  intelligence_score?: number | null;
  // coding/agentic 字段仍可存在于 IPC，但本模块不消费
}

/** 命中层级，仅用于展示/调试与多候选择优时的可解释性。 */
export type MatchTier = "exact" | "normalized" | "prefix";

export interface MatchedExternalScore {
  score: number;          // intelligence_score
  leaderboardId: string;
  sourceLabel: string;    // "OpenRouter"
  tier: MatchTier;
}
```

移除：`ModelCapability` 的本地打分语义、`scoreModelCapability` 作为排序依据、`QueueSortMode`、`ExternalSortMetric`、`hybridSortKey`、`sortByHybridCapability`、`EXTERNAL_SORT_BASE`、`sortByModelCapability`。

> 若确有极小工具（如展示用家族标签）需要保留，评估后再定；默认整体删除本地启发式，保持模块聚焦。

## 分层匹配算法（D3）

`matchModelToLeaderboard(modelId, index)`：

1. **精确**：`key = normalizeModelIdForMatch(modelId)`，`index.get(key)` 命中 → `tier: "exact"`。
2. **归一化增强**：`normalizeModelIdForMatch` 内补齐去噪（渠道/量化/日期后缀），再 `index.get` → `tier: "normalized"`。
   - 增强点集中在 `normalizeModelIdForMatch`，让 index 与查询两侧一致归一，避免只在一侧去噪导致不对称。
3. **受控近似（前缀 + 判别 token 护栏）**：仅当精确/增强都未命中时启用。
   - 遍历 index 的 key（或预排序候选），对每个 `entryKey`：
     - 判断 `key` 与 `entryKey` 是否构成前缀关系（一侧是另一侧加 `-<suffix>`）。
     - 取被截掉的 `remainder`，按 `-` 切 token，若任一 token 落在**判别 token 清单** → 拒绝该候选。
   - 收集所有通过护栏的候选，取 `score` 最高者，`tier: "prefix"`。

### 判别 token 清单（集中常量）

```ts
const TIER_TOKENS = new Set([
  "mini","nano","small","large","pro","flash","lite","tiny",
  "haiku","sonnet","opus","turbo","plus","max",
]);
// 参数量：正则 /^\d+(\.\d+)?b$/ 视为判别 token（7b/72b/405b）
```

### 护栏正确性要点

- 前缀关系必须以 `-` 边界切分，避免 `gpt-4` 前缀命中 `gpt-40`（无 `-` 边界不算前缀）。
- 双向前缀都检查（上游可能更长或更短）。
- 判别 token 命中即整体拒绝该候选，宁可未匹配也不错配（D3 原则）。

## 排序（D2）

`sortQueueByLeaderboard(items, getModelId, index)`：

- 计算每项 `match = matchModelToLeaderboard(...)`。
- 命中项：按 `match.score` 降序；同分保持输入原序（稳定）。
- 未匹配项：整体排在所有命中项之后，彼此保持输入原序。
- 实现：`stableSort`，比较器 `(a,b)`：
  - 两者都命中 → `b.score - a.score`，同分回退原 index。
  - 一命中一未命中 → 命中在前。
  - 都未命中 → 原 index 升序。

## UI 简化（R3）

- 删除 `sortMode` ref、`sortModeOptions`、排序方式 `HSelect`、`ExternalSortMetric` 分支。
- `externalIndex` 恒基于 intelligence 构建（榜单存在时）。
- 「按能力排序」按钮：加载/确保榜单 → 无榜单且失败则中文提示保持原序（AC5）。
- 每条模型标签：
  - 命中 → `OpenRouter · {分数}`（tooltip 可加匹配层级/leaderboardId）。
  - 未命中 → `未匹配`（灰底），排序时沉底。
- 榜单状态文案精简：更新时间 / 条数 / 缓存命中 / 陈旧 / 刷新失败；去掉「本地/外部回退」解释。
- 保留：不自动保存、可拖拽微调、`applySortedItems` 的 options/fetching 重映射逻辑。

## 兼容性 / 迁移

- 无持久化数据结构变化：排序只作用于表单内存态，保存的仍是 `items` 顺序。
- 已保存分组的队列顺序不受影响（不迁移）。
- 后端与 IPC 契约不变，纯前端行为变更。

## 测试策略

- 单测（`node --test`）：分层匹配各层 + 反例护栏 + 未匹配沉底 + 稳定排序 + 归一化去噪补充。
- 手动/构建校验：`GroupsPage` 排序按钮、标签展示、无榜单提示。
