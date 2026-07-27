# 执行计划：OpenRouter 榜单排序 + 分层模糊匹配

## 顺序清单

### 1. 重写核心模块 `src/utils/modelCapability.ts`
- [ ] 删除本地启发式排序依据：`scoreModelCapability`（作为排序 key）、`parameterBonus`、`variantPenalty`、`result`、`sortByModelCapability`、`hybridSortKey`、`sortByHybridCapability`、`EXTERNAL_SORT_BASE`、`QueueSortMode`、`ExternalSortMetric`。
- [ ] 保留并增强 `normalizeModelIdForMatch`：补齐渠道/量化/日期去噪，保证 index 与查询两侧一致归一。
- [ ] `buildExternalScoreIndex(models)`：只索引 `intelligence_score`，key 来自 `id/canonical_slug/name` 的归一化，冲突取高分。
- [ ] 新增 `MatchTier`、更新 `MatchedExternalScore`（加 `tier`）。
- [ ] 新增 `matchModelToLeaderboard(modelId, index)`：精确 → 归一化 → 前缀+判别 token 护栏；多候选取分最高。
- [ ] 新增判别 token 常量 `TIER_TOKENS` + 参数量正则判定。
- [ ] 新增 `sortQueueByLeaderboard(items, getModelId, index)`：命中降序、未匹配沉底、稳定同序。

### 2. 重写单测 `src/utils/modelCapability.test.ts`
- [ ] 删除本地打分/三档/hybrid 相关用例。
- [ ] 归一化去噪：日期/厂商前缀/`-latest`/量化后缀。
- [ ] 分层匹配：exact / normalized / prefix 各命中一例。
- [ ] 护栏反例：`gpt-4o` 不得命中 `gpt-4o-mini`；`claude-3-5-sonnet` 不得命中 `claude-3-7-sonnet`；`gpt-4` 不得前缀命中 `gpt-40`。
- [ ] 多候选取分最高。
- [ ] `sortQueueByLeaderboard`：命中降序 + 未匹配沉底保持原序 + 同分稳定 + 无 index 时全部视为未匹配保持原序。

### 3. 简化 `src/pages/GroupsPage.vue`
- [ ] 删除 `sortMode` / `sortModeOptions` / 排序方式 `HSelect` / `ExternalSortMetric` 引用。
- [ ] `externalIndex` 改为榜单存在即按 intelligence 构建。
- [ ] `displayScoreOf` → 基于 `matchModelToLeaderboard`，返回命中分或「未匹配」。
- [ ] `sortQueueByCapability` 用 `sortQueueByLeaderboard`；无榜单且拉取失败时中文提示保持原序。
- [ ] 更新 import；模型标签模板：命中 `OpenRouter · 分数`，未匹配灰底「未匹配」。
- [ ] 精简 `leaderboardStatusText` 与队列说明文案，去掉本地/回退解释。
- [ ] `ensureLeaderboardForExternalSort` 简化为「按能力排序」总需榜单。

### 4. 重写 spec `.trellis/spec/frontend/model-queue-sort.md`
- [ ] 契约改为单一 intelligence 排序 + 分层匹配 + 未匹配沉底；更新签名、Contracts、错误矩阵、Good/Bad、Tests Required。

## 验证命令
```bash
npm run test:unit   # node --experimental-strip-types --test src/utils/*.test.ts
npm run typecheck   # vue-tsc --noEmit
npm run build       # vue-tsc --noEmit && vite build
```

## 风险文件 / 回滚点
- `src/utils/modelCapability.ts`：核心逻辑重写，回滚点＝改前版本；分层护栏是错配防线，单测须全绿。
- `src/pages/GroupsPage.vue`：模板与状态删减多，注意 `applySortedItems`、拖拽、options 重映射不被误删。

## start 前检查
- [ ] prd/design/implement 一致，AC 可测。
- [x] 已确认脚本名：`test:unit` / `typecheck` / `build`。
