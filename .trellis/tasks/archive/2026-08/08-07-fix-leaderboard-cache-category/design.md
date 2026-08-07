# 设计：缓存分类不匹配修复

## 数据流（修复后）

```
read_cache(config_dir)
  → 解析 LeaderboardCacheFile（category 缺省 "logic" 兼容旧文件）
  → cache.category == LLM_BENCHMARK_CATEGORY("code_v3") ?
       ├─ 是 → 正常返回，走 TTL 判断
       └─ 否 → warn + Ok(None) → 触发联网刷新
                          ↓
write_cache → category: LLM_BENCHMARK_CATEGORY（此后缓存自带分类，正常复用）
```

## 关键代码

### schema（leaderboard.rs）
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LeaderboardCacheFile {
    source: String,
    /// 缓存所属榜单分类；旧文件缺省 "logic"（迁移兼容）。
    #[serde(default = "default_cache_category")]
    category: String,
    fetched_at_unix: i64,
    models: Vec<LeaderboardModel>,
}

fn default_cache_category() -> String {
    "logic".into()
}
```
- 旧文件无 `category` 字段 → serde 注入 `"logic"` → 与当前 `"code_v3"` 不等 → 失效。
- 若未来再改分类，同一机制自动失效，无需手动删缓存。

### read_cache 校验
```rust
if cache.category != LLM_BENCHMARK_CATEGORY {
    tracing::warn!(
        cache_category = %cache.category,
        expected = LLM_BENCHMARK_CATEGORY,
        "榜单缓存分类与当前配置不符，忽略缓存并重新拉取"
    );
    return Ok(None);
}
```
放在 `models.is_empty()` 检查附近，位置不重要（都是返回 None 的路径）。

### write_cache
```rust
let file = LeaderboardCacheFile {
    source: "llm_benchmark".into(),
    category: LLM_BENCHMARK_CATEGORY.into(),
    fetched_at_unix,
    models: models.to_vec(),
};
```

## 兼容性

- 旧文件（无 category）：serde default 生效，不报「损坏」。
- 读取顺序：先解析（可能损坏报错）→ 再校验分类 → 再校验空榜。
- stale 回退：分类不匹配时返回 None，若网络也失败则无缓存可回退 → 报错（符合语义：旧榜不该在新榜失败时冒充数据）。
  - 注意：`stale_or_error(cached, ...)` 的 cached 此时是 None，走错误分支——这是正确行为。

## 测试设计（leaderboard.rs tests）

1. `read_cache_mismatch_category_returns_none`：写一个 `category="logic"` 的缓存（用旧格式 JSON 字符串直接写文件，模拟用户真实旧缓存），`read_cache` 返回 None。
2. `read_cache_missing_category_defaults_logic`：写无 category 字段的 JSON → 反序列化成功且 category=="logic"（serde default 生效）。
3. `cache_roundtrip_with_category`：现有 roundtrip 测试的 `loaded.category` 断言 == "code_v3"。
4. 现有 `get_leaderboard_uses_fresh_cache_without_network` 等：`write_cache` 已写 code_v3 分类 → 分类匹配 → 行为不变，仍绿。

## 前端

无改动（本次纯后端缓存层修复）。
