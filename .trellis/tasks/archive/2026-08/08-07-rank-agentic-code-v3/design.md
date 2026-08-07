# 设计：队列排序改用 llm_benchmark Agentic 榜（code_v3）

## 数据流

```
llm_benchmark raw
  └─ datasets.json（固定 URL）
       └─ locate_latest_csv(datasets_json, "code_v3")  // 泛化自 locate_latest_logic_csv
            └─ data/code_v3/2026-08.csv（英文表头、等级制）
                 └─ parse_code_v3_csv(body)
                      └─ LeaderboardModel { id, name, agentic_score=行序倒排, intelligence=None }
                           └─ 24h 缓存 / force_refresh / stale 回退（沿用）
                                └─ IPC get_model_leaderboard
                                     └─ 前端 buildExternalScoreIndex 用 agentic_score
                                          └─ sortQueueByLeaderboard 降序（不变）
```

## code_v3 CSV 结构（实测 2026-08）

```
表头: "Model","MacOS App(C)","Web(E)","Game(F)","Rust App(G)","Simple Model(H)","iOS+Server(I)","Animation(J)","Unprompted","Scaffold","Think"
行:   "Claude Fable 5 (high)","Pass","Pass","Pass","Pending","2/A+","3/A+","Pending","1","Claude Code","1"
      "Claude Opus 5 (max)","Pass","Pending","Pass","Pending","4/A","1/A+","12/B+","3","Claude Code","1"
      ...
```

要点：
- 表头英文，Model 列名固定 `Model`（与 logic 中文「模型」不同）。
- 等级列值（`Pass/Pending/Skip/Failed/2/A+`）**不解析**，仅用于人读。
- 网站对 code_v3 无「中位分数」列 → 不排序 → 展示即 CSV 行序 = 作者能力序。
- 行序倒排分：首行 `score = N`，末行 `score = 1`（N=有效行数）。

## 后端改造（leaderboard.rs）

### 常量
```rust
pub const LLM_BENCHMARK_CATEGORY: &str = "code_v3";
```

### 定位函数泛化
```rust
pub fn locate_latest_csv(datasets_json: &str, category: &str) -> Result<String, AppError>
// 原 locate_latest_logic_csv 逻辑，category 参数化；保留内部「月榜 title 优先 + 最新 reportDate」

pub fn locate_latest_logic_csv(datasets_json: &str) -> Result<String, AppError> {
    locate_latest_csv(datasets_json, LLM_BENCHMARK_CATEGORY)
}
// 或直接改调用点，测试同步。倾向直接改函数签名（无外部调用者）。
```

### code_v3 解析
```rust
pub fn parse_code_v3_csv(body: &str) -> Result<Vec<LeaderboardModel>, AppError> {
    // 行级解析沿用 parse_csv_line（引号/转义）
    // 表头定位 "Model" 列（trim 后精确匹配；不存在报错）
    // 遍历数据行：Model 非空 → LeaderboardModel {
    //     id: model, canonical_slug: None, name: Some(model),
    //     intelligence_score: None, coding_score: None,
    //     agentic_score: Some((total_rows - index) as f64)
    // }
    // 空榜（无有效行）报错，与 logic 一致
}
```

注意：行序分必须在「过滤空行之后」计算，且需要先收集全部行再回填分（两次遍历或先收集 Vec<Model> 再 map）。

### fetch 入口
```rust
pub async fn fetch_llm_benchmark_models() -> ... {
    let csv_rel = locate_latest_csv(&datasets_json, LLM_BENCHMARK_CATEGORY)?;
    parse_code_v3_csv(&csv_body)
}
```

## 前端改造

### modelCapability.ts
```ts
export interface ExternalLeaderboardEntry {
  id: string;
  canonical_slug?: string | null;
  name?: string | null;
  /** llm_benchmark code_v3（Agentic）榜行序倒排分；首行最高。 */
  agentic_score?: number | null;
}

// buildExternalScoreIndex 内：
const raw = entry.agentic_score;  // 原 intelligence_score
```
其余（归一化、前缀护栏、排序、未匹配沉底）全部不变。

### tauri.ts
`LeaderboardModel` 注释更新：agentic_score 为排序分（行序倒排），intelligence_score 恒 None。

## 风险与回退

- 榜单模型数变少（~18 vs ~49）：匹配率下降是预期行为（用户选择 Agentic 榜）；未匹配沉底机制兜底。
- 上游 code_v3 若某月行序非能力序：忠实于上游展示，不额外加工（用户明确要求与网站一致）。
- 回退：改回 `LLM_BENCHMARK_CATEGORY = "logic"` + 恢复调用 `parse_llm_benchmark_csv`（旧函数保留不删，可并存，便于回退与未来多榜并存）。
- code_v3 CSV 格式若上游调整表头（如 Model 改名）：`parse_code_v3_csv` 报错 → stale 回退旧缓存，不崩。

## 测试设计

### 后端（leaderboard.rs tests）
- `parse_code_v3_csv_whitelist_rows`：SAMPLE_CODE_V3_CSV 3 行 + 1 坏行 → 解析 3 行；首行 agentic_score=3、次行=2、末行=1；intelligence/coding/canonical_slug 全 None。
- `locate_latest_csv_category`：SAMPLE_DATASETS 中取 code_v3 最新（2026-08，优先月榜 title）。
- 缓存/TTL/force_refresh/stale：沿用现有测试（不依赖分类）。
- 空榜报错：纯表头 → Business 错误。

### 前端（modelCapability.test.ts）
- 现有用例将 `intelligence_score` 字段替换为 `agentic_score`，断言分数值不变（纯字段切换，匹配/排序逻辑未动）。
