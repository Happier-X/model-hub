# 队列排序改用 llm_benchmark Agentic 榜（code_v3）

## Goal

分组「按模型能力排序」的榜单依据从 llm_benchmark **logic（推理）** 综合榜切换为 **code_v3（Agentic）** 榜，排名与网站「Agentic」标签页（`#category=code_v3`）一致。

## Background（已核实事实）

- 用户指定目标页面：`https://llm2014.github.io/llm_benchmark/#category=code_v3&dataset=code_v3|2026-08|0`（「Agentic」标签页）。
- llm_benchmark 数据源分类：`logic`（网站展示名「推理」）/ `code_v3`（展示名「Agentic」）/ `code`（废弃）/ `vision`。`code_v3` 即用户所指 Agentic 榜。
- **code_v3 CSV 结构与 logic 完全不同**：
  - 表头英文：`Model, MacOS App(C), Web(E), Game(F), Rust App(G), Simple Model(H), iOS+Server(I), Animation(J), Unprompted, Scaffold, Think`。
  - 值为等级制：`Pass` / `Pending` / `Skip` / `Failed(n/m)` / `排名/等级`（如 `2/A+`、`15/B+`、`43/C`），**没有单一数值分**。
  - 网站对 code_v3 **不排序**（无「中位分数」列则保持 CSV 行序），CSV 行序即作者按能力排的展示顺序（Claude Fable 5 → Claude Opus 5 → GPT-5.6 Sol → Kimi-K3 → …）。
- 旧实现（8-05 切换前）用 OpenRouter artificial_analysis 的 `agentic_index`，与本次需求无关；本次不改数据源，只切 llm_benchmark 分类。
- 当前实现：`LLM_BENCHMARK_CATEGORY="logic"`；`parse_llm_benchmark_csv` 按中文表头定位「模型/极限分数」→ `intelligence_score`；`agentic_score` 恒 None。

## Decisions（待用户确认）

| # | 决策 | 结论 |
|---|------|------|
| 1 | 排名依据 | **按 code_v3 CSV 行序**（与网站 Agentic 页展示顺序一致），而非自定义「等级→分数」映射求和（映射主观、与网站排序不一致） |
| 2 | 分数表达 | 解析时给每个模型 `agentic_score = (总行数 - 行索引)`（首行分最高）；`intelligence_score` 置 None；排序仍走现有「降序」逻辑无需改 |
| 3 | 匹配 | 前端归一化匹配层（剥 `(...)` 档位、纯 4 位日期、厂商前缀）**不变**——code_v3 展示名如 `Claude Fable 5 (high)` 同样适用 |
| 4 | 兼容 | `canonical_slug` / `coding_score` 恒 None；`source` 仍 `"llm_benchmark"`；缓存文件/TTL/force_refresh/stale 回退全部沿用 |
| 5 | UI 文案 | 无需改（排序按钮无榜单名文案残留）；展示分来自 `agentic_score` |

## Requirements

### R1 后端：分类与定位
- `LLM_BENCHMARK_CATEGORY` 常量改为 `"code_v3"`；`locate_latest_logic_csv` → 泛化为按 category 定位最新月榜（或改名 `locate_latest_csv(datasets_json, category)`），测试同步改

### R2 后端：code_v3 CSV 解析
- 新增 `parse_code_v3_csv(body) -> Result<Vec<LeaderboardModel>>`：
  - 表头按列名定位 `Model` 列（英文表头）；`Model` 缺失报错
  - 每行输出 `LeaderboardModel { id=Model, name=Model, agentic_score=行序倒排分, intelligence_score=None, coding_score=None, canonical_slug=None }`
  - 行序分：`agentic_score = (有效行数 - 当前行索引) as f64`（首行最高，降序即行序）
  - 跳过空 Model 行；等级列（Pass/A+ 等）不解析不消费
- `fetch_llm_benchmark_models` 改调 `parse_code_v3_csv`

### R3 前端：排序指标切换
- `modelCapability.ts`：`ExternalLeaderboardEntry` 增加 `agentic_score`；`buildExternalScoreIndex` 改用 `entry.agentic_score`（intelligence_score 不再消费）
- `tauri.ts`：`LeaderboardModel` 注释同步（agentic_score 为排序分）
- 注释/文档中「logic 综合榜极限分数」表述全部替换为「code_v3 Agentic 榜」

### R4 测试与 spec
- `leaderboard.rs` 测试：SAMPLE 换 code_v3 CSV；行序分断言（首行分最高）；幂等/缓存测试沿用
- `modelCapability.test.ts`：分数字段改用 agentic_score，断言值不变（逻辑不变）
- spec：`model-leaderboard.md`（表头/解析/白名单重写）、`model-queue-sort.md`（排序依据改 Agentic）、`upstream-access.md` 相关行

## Out of Scope

- 不改 OpenRouter / 不改数据源
- 不做「等级→分数」映射求和
- 不改前端匹配归一化逻辑
- 不改缓存/回退机制

## Acceptance Criteria

- [ ] AC1：`cargo test`（含新 code_v3 解析测试）全绿
- [ ] AC2：`pnpm typecheck` / `pnpm lint` / `pnpm test:unit` / `pnpm build` 全绿
- [ ] AC3：排序结果与网站 Agentic 标签页（2026-08）行序一致：Claude Fable 5 排最前，依行序递减
- [ ] AC4：未匹配模型仍沉底且保持原相对顺序；稳定排序不回归
- [ ] AC5：spec 同步（model-leaderboard / model-queue-sort / upstream-access）

## Notes

- 切换后榜单模型数从 logic 的 ~49 行变为 code_v3 的 ~18 行，覆盖模型变少，但这是用户明确选择（Agentic 榜）。
