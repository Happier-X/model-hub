# 修复：榜单缓存分类不匹配导致排序全未匹配

## Goal

切换 llm_benchmark 分类（logic → code_v3）后，磁盘缓存仍是旧 logic 格式（`agentic_score` 全 None），且缓存未记录自身分类，TTL 内一直被当作有效缓存使用，导致前端 `buildExternalScoreIndex` 索引为空、队列排序**全部未匹配**。

## Background（已核实）

- 用户缓存 `model-leaderboard-llm-benchmark.json`：43 个 logic 榜模型，`agentic_score` 全 None（旧格式），`fetched_at_unix` 在 TTL 24h 内。
- 根因链：缓存 schema 无「分类」标识 → 新代码（只看 agentic_score）读旧缓存 → 索引空 → 全未匹配。
- 附带事实：即使刷新，code_v3 仅 16 个模型，队列中真实模型大量不命中属预期（上任务已确认接受覆盖变小）。

## Decisions

| # | 决策 | 结论 |
|---|------|------|
| 1 | 缓存加分类标识 | `LeaderboardCacheFile` 增加 `category: String` 字段（序列化必填；反序列化用 `#[serde(default = ...)]` 兼容旧文件 → 缺省 `"logic"`） |
| 2 | 读取校验 | `read_cache` 返回前校验 `cache.category == LLM_BENCHMARK_CATEGORY`；不匹配 → 返回 `Ok(None)`（视为无缓存，走联网刷新）并 log warning |
| 3 | 写入 | `write_cache` 写入当前 `LLM_BENCHMARK_CATEGORY` |
| 4 | 旧文件兼容 | 旧文件（无 category 字段）经 serde default 视为 `"logic"` → 与当前 `"code_v3"` 不匹配 → 自动失效重拉，无需手动删文件 |
| 5 | 副作用 | 首次切换后会自动重拉一次 code_v3 榜（一次性；此后缓存带 code_v3 标识正常复用） |

## Requirements

### R1 缓存 schema
- `LeaderboardCacheFile` 增加 `category: String`，反序列化 default = `"logic"`（旧格式迁移）

### R2 读写校验
- `write_cache`：`category: LLM_BENCHMARK_CATEGORY.into()`
- `read_cache`：`cache.category != LLM_BENCHMARK_CATEGORY` → warn + 返回 None（不视为损坏，不报错）

### R3 测试
- 旧格式（无 category 字段 JSON）反序列化 default = `"logic"`，且 read_cache 时因分类不匹配返回 None
- 新格式（category=code_v3）正常读回；缓存命中/回退测试改用新字段
- 现有测试同步（write_cache 调用点无需改签名；断言可加 category 校验）

## Out of Scope

- 不引入缓存版本号（category 已足够区分本次切换；未来再改分类同样生效）
- 不改 TTL / force_refresh / stale 回退逻辑
- 不动 code_v3 解析与前端匹配（上任务已完成）

## Acceptance Criteria

- [ ] AC1：`cargo test --lib` 全绿（含旧格式兼容与分类不匹配测试）
- [ ] AC2：`pnpm typecheck/lint/test:unit/build` 不受影响全绿（前端无改动预期）
- [ ] AC3：用用户实际旧缓存（无 category 字段）启动 → 触发排序时自动重拉 code_v3 榜，不再全未匹配
- [ ] AC4：重拉后缓存带 `category: "code_v3"`，再次启动命中缓存正常

## Notes

- 一次性自动重拉依赖排序时 `ensureLeaderboardForExternalSort` 自动拉取逻辑（上上任务已移除手动按钮）。
