# 外部模型榜单（llm_benchmark）

> 公共榜单拉取（datasets.json 定位 + logic 月榜 CSV）、白名单解析、文件缓存与 IPC 契约。

---

## Scenario: get_model_leaderboard

### 1. Scope / Trigger

- Trigger：分组队列按「外部综合能力」排序时，管理面需获取公开模型分数；跨层 IPC + 外网 + 文件缓存，必须写清可执行合同。
- 数据源：[llm2014/llm_benchmark](https://github.com/llm2014/llm_benchmark)（GitHub Pages raw），logic 综合榜「极限分数」。

### 2. Signatures

- 模块：`src-tauri/src/domain/leaderboard.rs`
- Tauri command：

```rust
#[tauri::command]
pub async fn get_model_leaderboard(
    app: AppHandle,
    force_refresh: Option<bool>,
) -> Result<ModelLeaderboardSnapshot, InvokeError>
```

- 领域入口：

```rust
pub async fn get_model_leaderboard(
    config_dir: &Path,
    force_refresh: bool,
) -> Result<ModelLeaderboardSnapshot, AppError>
```

- 解析/定位纯函数：

```rust
pub fn parse_llm_benchmark_csv(body: &str) -> Result<Vec<LeaderboardModel>, AppError>
pub fn locate_latest_logic_csv(datasets_json: &str) -> Result<String, AppError>
```

### 3. Contracts

**固定外网**

| 项 | 值 |
|----|-----|
| datasets URL | `https://raw.githubusercontent.com/llm2014/llm_benchmark/main/docs/data/datasets.json`（`LLM_BENCHMARK_DATASETS_URL`） |
| CSV URL | `LLM_BENCHMARK_BASE` + datasets 中 `category=="logic"` 最新 `reportDate` 的 `csv` 相对路径（如 `data/logic/2026-08.csv`） |
| 鉴权 | **不**携带任何 API Key / 供应商 Key |
| 请求头 | `Accept: application/json,text/csv,text/plain` |
| 超时 | 整次 15s；连接 10s |

**缓存**

| 项 | 值 |
|----|-----|
| 路径 | `{config_dir}/model-leaderboard-llm-benchmark.json` |
| TTL | 24 小时（`LEADERBOARD_CACHE_TTL_SECS`） |
| 新鲜缓存 | `force_refresh=false` 时直接返回，`cache_hit=true`、`stale=false` |
| 强制刷新 | `force_refresh=true` 跳过新鲜缓存，尝试网络 |

**白名单字段（仅这些进入缓存与 IPC）**

- CSV 表头按列名定位「模型」与「极限分数」（不依赖固定列序）。
- 每行仅输出：

| CSV 列 | 输出字段 |
|--------|----------|
| `模型` | `id`（展示名，如 `GPT-5.5 (xhigh)`；必填，空则跳过） |
| `模型` | `name?`（同展示名） |
| `极限分数` | `intelligence_score?`（0-100；非有限数字则跳过该行） |

- `canonical_slug` / `coding_score` / `agentic_score` 恒为 `None`。
- 手写 CSV 解析（无 csv crate）：支持引号包裹字段、引号内逗号、`""` 转义。
- 禁止把其它 CSV 列（耗时/价格/Token 等）透传或落盘。

**响应 `ModelLeaderboardSnapshot`**

| 字段 | 类型 | 说明 |
|------|------|------|
| `source` | string | 固定 `"llm_benchmark"` |
| `fetched_at_unix` | i64 | 成功拉取/写入缓存时的 Unix 秒 |
| `stale` | bool | 网络失败回退旧缓存时为 true |
| `cache_hit` | bool | 本次未发起网络、直接用有效缓存 |
| `models` | `LeaderboardModel[]` | 白名单解析结果 |

### 4. Validation & Error Matrix

| 条件 | 行为 |
|------|------|
| 缓存存在且未过期且 `force_refresh=false` | 返回缓存，`cache_hit=true` |
| 网络成功 | 写缓存，`stale=false`、`cache_hit=false` |
| 网络失败 / 超时 / datasets 非 JSON / 无 logic 月榜 / CSV 缺列 / 空榜，**有**旧缓存 | 返回旧缓存，`stale=true` |
| 同上且 **无** 缓存 | `Err`，可行动中文 message（检查网络/稍后重试） |
| datasets 顶层必须是 `{"datasets": [...]}` 对象 | 非数组顶层报「无法解析」 |
| 行缺「模型」或「极限分数」 | 跳过该行，不整单失败 |
| 分数非数字 / 非有限 | 跳过该行 |

### 5. Good/Base/Bad Cases

- **Good**：24h 内二次打开 → `cache_hit=true`，无网络。
- **Base**：无缓存首次拉取 → datasets 定位最新 logic 月榜 → 拉 CSV → 落盘并返回 models。
- **Bad**：断网且无缓存 → 业务错误；断网有缓存 → `stale=true` 仍可用。

### 6. Tests Required

- 白名单解析：只输出「模型/极限分数」，跳过坏分数行，空榜报错。
- datasets 定位：多 category 取 logic 最新 `reportDate`；缺 logic 报错。
- 新鲜缓存命中：`force_refresh=false` 不调 fetch。
- 强制刷新：即使缓存新鲜也调 fetch。
- 网络失败 + 有缓存 → `stale=true`。
- 网络失败 + 无缓存 → `Err`。
- 超时 / URL 常量存在且指向 raw.githubusercontent.com/llm2014/llm_benchmark（不含 openrouter.ai）。

### 7. Wrong vs Correct

#### Wrong

```rust
// 接受前端任意 URL，或带上用户供应商 Key
let url = user_provided_url;
req.header("Authorization", format!("Bearer {}", provider_key));
// 直接依赖固定 CSV 路径（每月失效，需改代码）
let csv = get("https://raw.githubusercontent.com/llm2014/llm_benchmark/main/docs/data/logic/2026-08.csv");
```

#### Correct

```rust
// 固定 datasets URL、无 Key；动态定位最新 logic 月榜 CSV
let datasets = get_text(LLM_BENCHMARK_DATASETS_URL, &client).await?;
let csv_rel = locate_latest_logic_csv(&datasets)?;
let models = parse_llm_benchmark_csv(&get_text(&format!("{LLM_BENCHMARK_BASE}{csv_rel}"), &client).await?)?;
// 失败有缓存 → stale 快照；无缓存才 Err
```

---

## Anti-Patterns

- 抓网页 HTML / 非稳定接口当榜单源。
- 把完整 CSV 或上游 JSON 原样缓存或透传前端。
- 外网失败时清空已有缓存。
- 在错误信息中带入任何 Key。
- 直接写死月榜 CSV 路径（应走 datasets.json 动态定位）。
