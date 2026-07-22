# 接入外部模型榜单排序

## Goal

分组队列可从 OpenRouter 公共 Models API 获取外部榜单分数，优先按外部通用/编码能力排序；未匹配模型回退本地启发式，避免新模型必须手工维护。

## Requirements

- R1：后端请求 OpenRouter `GET /api/v1/models?sort=intelligence-high-to-low`，不需要用户 Key。
- R2：仅解析 `id/canonical_slug/name/benchmarks.artificial_analysis.{intelligence_index,coding_index,agentic_index}`，禁止把外部任意数据直接透传。
- R3：缓存到应用 `config_dir/model-leaderboard-openrouter.json`，TTL 24 小时；缓存有效直接返回；可强制刷新。
- R4：网络失败但有旧缓存时返回 stale 缓存并提示；无缓存才报错。请求超时 15 秒。
- R5：前端支持排序方式：本地启发式、外部通用能力、外部编码能力；外部缺分回退本地。
- R6：模型名匹配归一化：小写、厂商前缀、日期后缀、分隔符/常见变体；仅高置信匹配，避免错配。
- R7：UI 显示来源（OpenRouter/本地）、分数、更新时间/缓存状态；排序仅改表单，需保存。
- R8：不抓网页、不依赖 LMSYS 非公开 API；OpenRouter 失败不影响本地排序。

## Acceptance Criteria

- [ ] 能拉取并缓存 OpenRouter 榜单
- [ ] 通用/编码指标可排序
- [ ] 未匹配模型回退本地启发式
- [ ] 离线时旧缓存可用
- [ ] typecheck/lint/cargo test/单测通过
