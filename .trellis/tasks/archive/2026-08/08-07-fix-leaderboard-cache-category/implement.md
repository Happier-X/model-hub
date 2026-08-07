# 执行计划

## 阶段 1：schema 与默认值（leaderboard.rs）
- [ ] `LeaderboardCacheFile` 增加 `category: String` + `#[serde(default = "default_cache_category")]`；新增 `fn default_cache_category() -> String { "logic".into() }`

## 阶段 2：读写校验
- [ ] `write_cache`：`category: LLM_BENCHMARK_CATEGORY.into()`
- [ ] `read_cache`：解析后校验 `cache.category != LLM_BENCHMARK_CATEGORY` → warn + `Ok(None)`

## 阶段 3：测试
- [ ] `read_cache_category_mismatch_returns_none`（旧格式 JSON 直接写文件）
- [ ] `read_cache_missing_category_defaults_logic`（serde default 生效）
- [ ] roundtrip 测试加 `loaded.category == "code_v3"` 断言
- [ ] 现有缓存测试（fresh/stale/force_refresh）确认仍绿
- [ ] `cargo test --lib` 全绿

## 阶段 4：前端回归
- [ ] `pnpm typecheck / lint / test:unit / build` 全绿（预期无改动）

## 阶段 5：spec 更新
- [ ] `model-leaderboard.md`：缓存结构表加 `category` 字段 + 分类不匹配失效语义

## 阶段 6：收尾
- [ ] 用真实旧缓存验证 AC3（可手动触发 get_model_leaderboard 单测模拟）
- [ ] journal + archive
