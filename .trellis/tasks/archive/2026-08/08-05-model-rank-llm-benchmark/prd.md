# 模型能力排序改用 llm_benchmark 榜单

## Goal

将分组「按模型能力排序」（现基于 OpenRouter 榜单）的数据源替换为 [llm2014/llm_benchmark](https://github.com/llm2014/llm_benchmark) 的榜单，匹配/排序契约保持现有语义（单一分数降序、未匹配沉底、稳定排序、仅改表单不自动保存）。

## 背景与现状

### 当前实现（OpenRouter 数据源）

- **后端** `src-tauri/src/domain/leaderboard.rs`：
  - 固定 `GET https://openrouter.ai/api/v1/models?sort=intelligence-high-to-low`（无 Key）。
  - JSON 白名单解析 → `LeaderboardModel { id, canonical_slug, name, intelligence_score, coding_score, agentic_score }`。
  - 24h 文件缓存 `model-leaderboard-openrouter.json`（config_dir），force_refresh / stale 回退逻辑完整。
- **前端** `src/utils/modelCapability.ts`：
  - `matchModelToLeaderboard` 分层匹配（exact → normalized → prefix+判别 token 护栏），查询侧是**上游 API 模型名**，榜单侧是 **OpenRouter 模型 id**。
  - `sortQueueByLeaderboard` 按 `intelligence_score` 降序、未命中沉底、同分稳定。
  - UI 展示 `OpenRouter · {分数}` / `未匹配`。
- **spec** `.trellis/spec/frontend/model-queue-sort.md` 定义了上述契约。

### llm_benchmark 数据形态（已调研）

- 仓库 GitHub Pages 托管：`https://raw.githubusercontent.com/llm2014/llm_benchmark/main/docs/data/...`。
- `docs/data/datasets.json`：列出各榜单 CSV 路径与月份（category: logic / code / code_v3 / vision）。
- **logic 榜** CSV 列：`模型, 极限分数, 中位分数, 中位差距, 变更, 平均耗时(秒), Token, 测试成本(元), 价格(元/百万), 发布时间, Think`。
- **code_v3 榜** CSV：各场景 Pass/Pending/Failed + 评分（非统一分数）。
- **模型名是展示名**（如 `GPT-5.5 (xhigh)`、`DeepSeek V4 Flash 0731 (max)`），**无 API id** —— 与用户队列里的 API 模型名（如 `deepseek/deepseek-v4`）差异大，匹配难度显著高于 OpenRouter 场景。

## 明确不做

- 不引入多榜单权重融合（除非 D1 决策要求）。
- 不改分组队列语义、故障转移、即时保存等既有行为。

## Open Decisions

- **D1** 用哪个榜 → **已定：仅 logic 综合榜**。
- **D2** 匹配策略 → **已定：M1**（展示名净化：剥 `(...)` 档位后缀/日期/厂商名，两侧对称归一化后走现有 exact/normalized/prefix+护栏 分层匹配；不维护别名表）。
- **D3** 后端解析与缓存 → **已定**：拉 `docs/data/datasets.json` 动态定位 `category=logic` 最新月榜 CSV 路径再拉 CSV（避免每月改代码）；缓存文件换新名 `model-leaderboard-llm-benchmark.json`（旧 `-openrouter` 文件不再读取，残留无害）。
- **D4** 排序指标 → **已定：极限分数**（榜单即按此降序，排序结果 = 榜单名次）。

## Acceptance Criteria

- [ ] AC1：按能力排序数据源切换为 llm_benchmark logic 榜，不再请求 OpenRouter 榜单。
- [ ] AC2：排序语义保持：单一分数降序、未匹配沉底、同分稳定、仅改表单不自动保存。
- [ ] AC3：后端支持 24h 缓存 / force_refresh / stale 回退（沿用现有快照结构）。
- [ ] AC4：前端展示来源标签与分数来自 llm_benchmark（不再显示 `OpenRouter · 分`）。
- [ ] AC5：`pnpm typecheck` / lint / `cargo test` 与相关单测通过。

## Notes

- 复杂任务：需 `design.md` / `implement.md`。
- 涉及后端（Rust 解析+缓存）+ 前端（匹配策略+展示）+ spec 更新（`model-queue-sort.md` / `upstream-access.md` 提及的 OpenRouter 来源表述）。
