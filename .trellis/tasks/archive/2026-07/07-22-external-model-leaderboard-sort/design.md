# 技术设计

## 数据流

GroupsPage → `get_model_leaderboard(force_refresh)` IPC → Rust OpenRouter client → JSON 白名单解析 → 24h 文件缓存 → TS 模型匹配/混合评分 → 表单稳定排序。

## IPC 返回

```text
ModelLeaderboardSnapshot {
  source: "openrouter",
  fetched_at_unix: i64,
  stale: bool,
  cache_hit: bool,
  models: [{ id, canonical_slug, name, intelligence_score?, coding_score?, agentic_score? }]
}
```

## 排序合同

- 外部分数命中：统一排序 key = `1_000_000 + external_score * 1_000`，确保外部命中优先按同一指标比较。
- 外部无分/无匹配：key = 本地启发式 score。
- 同 key 稳定保持原顺序。
- `force_refresh=false` 优先 24h 缓存；true 尝试网络。
- 网络失败：有缓存返回 stale；无缓存报可行动错误。

## 安全

- URL 固定 OpenRouter，不接受前端任意 URL。
- 不携带任何供应商/客户端 Key。
- 缓存只存公开榜单元数据。
