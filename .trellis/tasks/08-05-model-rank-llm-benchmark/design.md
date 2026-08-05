# 技术设计：模型能力排序改用 llm_benchmark 榜单

## 范围

- 层：backend（`leaderboard.rs` 数据源替换）+ frontend（匹配归一化增强 + 展示文案）+ spec 更新。
- 不做：多榜融合、别名表、后端 schema 迁移、分组其它行为。

## 数据流

```
用户点「强制刷新榜单」/ 排序时按需拉取
  → fetch_llm_benchmark():
      1) GET raw.github.../docs/data/datasets.json
         动态定位 category=="logic" 的最新 reportDate 的 csv 相对路径
      2) GET raw.github.../docs/{csv 相对路径}  → logic 月榜 CSV
  → parse_llm_benchmark_csv(body):
      手写 CSV 解析（无 csv crate，加依赖不值得）
      白名单取列：模型名、极限分数（第一列数值列）
      → Vec<LeaderboardModel> { id: 展示名, name: 展示名,
                                 intelligence_score: 极限分数,
                                 coding_score: None, agentic_score: None }
  → 24h 文件缓存 model-leaderboard-llm-benchmark.json（复用现有 read/write 结构）
  → ModelLeaderboardSnapshot { source: "llm_benchmark", ... }
```

前端链路（复用）：

```
buildExternalScoreIndex(models):
  对每条 entry 用 normalizeModelIdForMatch(id/name) 建索引
  → matchModelToLeaderboard(apiModelId, index) 分层匹配
  → sortQueueByLeaderboard 排序（语义不变）
```

## 后端改造点（leaderboard.rs）

1. **常量**：
   - 删 `OPENROUTER_MODELS_URL`；新增 `LLM_BENCHMARK_DATASETS_URL`（raw datasets.json）与 `LLM_BENCHMARK_BASE`（raw `docs/` 前缀）。
   - `LEADERBOARD_CACHE_FILE` 改为 `model-leaderboard-llm-benchmark.json`（旧 `-openrouter` 文件不再读取；残留无害）。
   - 新增 `LLM_BENCHMARK_CATEGORY = "logic"`。
2. **URL 解析**：
   - `datasets.json` 结构为数组：`[{category, reportDate, tableIndex, title, csv}]`。
   - 取 `category == "logic"` 且 `reportDate` 最大（字符串 YYYY-MM 可直接字典序比较）的 `csv` 字段（如 `data/logic/2026-08.csv`）。
   - 请求地址 = `LLM_BENCHMARK_BASE` + csv 路径。
3. **CSV 解析（手写）**：
   - 支持引号包裹字段（展示名可能含逗号/括号）；首行是表头，跳过。
   - 表头列定位：按列名找 `模型` 与 `极限分数` 的索引（不依赖固定列序，稳妥）。
   - 数值解析失败/非有限 → 该行跳过（与现有 json_number_to_f64 语义一致）。
   - 解析结果空 → 错误「llm_benchmark 返回空榜单…」。
4. **fetch 函数**：`fetch_llm_benchmark_models()`（两步请求；复用超时/错误 sanitize 逻辑，文案改为 llm_benchmark）。
5. **内部通用化**：`get_model_leaderboard_with_fetch` 已注入式，无需改；`source` 字符串改为 `"llm_benchmark"`。
6. **错误文案**：OpenRouter → llm_benchmark（超时/连接/HTTP 状态/空榜）。

## 前端改造点

### modelCapability.ts（M1 核心）

1. `normalizeModelIdForMatch` 增加**展示名净化**（对 API id 与榜单展示名两侧对称）：
   - 剥离括号及其内容：`(xhigh)` / `(max)` / `(high)` / `(think)` 等 → `GPT-5.5 (xhigh)` → `GPT-5.5`。
   - 剥离纯 4 位数字日期段（如 `0731` / `2507`）→ `DeepSeek V4 Flash 0731` → `DeepSeek V4 Flash`。
   - 现有空格→`-`、剥厂商前缀、剥已知渠道/量化后缀保留。
   - 注意：4 位数字剥离需在括号剥离后、与判别 token 判断不冲突（如 `gpt-4o` 的 `4o` 非纯数字段，不受影响；`gemma-4-31b` 的 `31b` 带 b 非纯数字）。
2. `sourceLabel`：`"OpenRouter"` → `"llm_benchmark"`（`buildExternalScoreIndex` 内）。
3. 注释/类型名 `ExternalLeaderboardEntry` 保留（字段仍 id/name/intelligence_score，语义兼容）。

### GroupsPage.vue

- `leaderboardStatusText`：`OpenRouter N 条` → `llm_benchmark N 条`；`更新于` 不变。
- 队列分数徽章：`OpenRouter · {分}` → `llm_benchmark · {分}`；title 文案同改。
- 排序提示文案「已按 OpenRouter 通用能力排序」→「已按 llm_benchmark 综合能力排序」。

### 单测

- `modelCapability.test.ts`：
  - 新增展示名净化用例：`GPT-5.5 (xhigh)` → `gpt-5-5`、`DeepSeek V4 Flash 0731 (max)` → `deepseek-v4-flash`、`Kimi-K3 (max)` → `kimi-k3`。
  - 匹配用例：API `openai/gpt-5.5` 命中展示名 `GPT-5.5 (xhigh)`；护栏用例保持（`gpt-4o` 不命中 `gpt-4o-mini`）。
  - sourceLabel 断言改 `llm_benchmark`。
- Rust：
  - `parse_llm_benchmark_csv`：表头定位、引号字段、缺列行跳过、数值解析、空榜错误。
  - datasets 定位：多 category 取 logic 最新 reportDate。
  - 缓存 roundtrip + source 断言改 `"llm_benchmark"`；URL 断言改 raw github 常量。

## 兼容与回滚

- IPC 结构与前端类型不变（`ModelLeaderboardSnapshot` / `LeaderboardModel`），仅 source 字符串变化。
- 旧 `model-leaderboard-openrouter.json` 残留无害；新文件名避免旧结构解析失败。
- 回滚：git revert 前端 + 后端文件；无 DB 迁移。

## 风险

- 展示名与 API 名差异大的模型（厂商自定义命名）可能漏配 → 沉底（符合现有契约，非错误）。
- 手写 CSV 需覆盖引号内逗号；表头列名假设固定（`模型` / `极限分数`），若上游改列名需同步（datasets.json 已动态化，列名风险低）。

## spec 更新

- `.trellis/spec/frontend/model-queue-sort.md`：来源表述 OpenRouter → llm_benchmark；匹配说明加「展示名净化（剥括号档位/日期）」；示例更新。
- `.trellis/spec/backend/upstream-access.md`：允许的公共 URL 条目 OpenRouter 榜单 → llm_benchmark raw GitHub（仍为固定公共 URL、无用户 Key）。
