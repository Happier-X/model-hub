# 执行计划

## 阶段 1：后端定位与解析（leaderboard.rs）
- [ ] `LLM_BENCHMARK_CATEGORY` → `"code_v3"`，常量注释改「Agentic 榜」
- [ ] `locate_latest_logic_csv` 泛化为 `locate_latest_csv(datasets_json, category)`（函数体原样，category 参数化）；更新调用点与测试
- [ ] 新增 `parse_code_v3_csv`：表头定位 `Model` 列；收集有效行 → 行序倒排 `agentic_score`；`intelligence_score/coding_score/canonical_slug=None`
- [ ] `fetch_llm_benchmark_models` 改用 `locate_latest_csv(…, "code_v3")` + `parse_code_v3_csv`
- [ ] `parse_llm_benchmark_csv`（logic 版）保留不删（回退用）

## 阶段 2：后端测试
- [ ] 新增 `parse_code_v3_csv` 测试：3 有效行 + 坏行 → 行序分 3/2/1；None 字段断言
- [ ] `locate_latest_csv` 分类测试：code_v3 取最新月榜
- [ ] 现有缓存/回退测试确认仍绿（不依赖分类）
- [ ] `cargo test` 全绿

## 阶段 3：前端排序指标
- [ ] `modelCapability.ts`：`ExternalLeaderboardEntry` 加 `agentic_score`；`buildExternalScoreIndex` 消费 `agentic_score`
- [ ] `tauri.ts`：`LeaderboardModel` 注释同步
- [ ] `modelCapability.test.ts`：字段换 agentic_score，断言值不变
- [ ] 全库 grep 清理「logic 综合榜/极限分数」残留表述

## 阶段 4：质量检查
- [ ] `pnpm typecheck` / `pnpm lint` / `pnpm test:unit` / `pnpm build` 全绿
- [ ] `cargo test` 全绿
- [ ] 手工核对 AC3（code_v3 行序：Claude Fable 5 居首）

## 阶段 5：spec 更新
- [ ] `model-leaderboard.md`：分类/表头/解析/白名单重写为 code_v3
- [ ] `model-queue-sort.md`：排序依据改「code_v3 Agentic 行序」
- [ ] `upstream-access.md`：相关表述同步
