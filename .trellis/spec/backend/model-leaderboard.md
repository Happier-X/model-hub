# 外部模型榜单（OpenRouter）

> 公共榜单拉取、白名单解析、文件缓存与 IPC 契约。

---

## Scenario: get_model_leaderboard

### 1. Scope / Trigger

- Trigger：分组队列按「外部通用/编码能力」排序时，管理面需获取公开模型分数；跨层 IPC + 外网 + 文件缓存，必须写清可执行合同。

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

### 3. Contracts

**固定外网**

| 项 | 值 |
|----|-----|
| URL | `https://openrouter.ai/api/v1/models?sort=intelligence-high-to-low`（`OPENROUTER_MODELS_URL`，禁止前端自定义） |
| 鉴权 | **不**携带任何 API Key / 供应商 Key |
| 请求头 | 仅 `Accept: application/json` |
| 超时 | 整次 15s；连接 10s |

**缓存**

| 项 | 值 |
|----|-----|
| 路径 | `{config_dir}/model-leaderboard-openrouter.json` |
| TTL | 24 小时（`LEADERBOARD_CACHE_TTL_SECS`） |
| 新鲜缓存 | `force_refresh=false` 时直接返回，`cache_hit=true`、`stale=false` |
| 强制刷新 | `force_refresh=true` 跳过新鲜缓存，尝试网络 |

**白名单字段（仅这些进入缓存与 IPC）**

| JSON 路径 | 输出字段 |
|-----------|----------|
| `data[].id` | `id`（必填，空则跳过条目） |
| `data[].canonical_slug` | `canonical_slug?` |
| `data[].name` | `name?` |
| `data[].benchmarks.artificial_analysis.intelligence_index` | `intelligence_score?` |
| `...coding_index` | `coding_score?` |
| `...agentic_index` | `agentic_score?` |

禁止把 pricing、description 等任意上游字段透传或落盘。

**响应 `ModelLeaderboardSnapshot`**

| 字段 | 类型 | 说明 |
|------|------|------|
| `source` | string | 固定 `"openrouter"` |
| `fetched_at_unix` | i64 | 成功拉取/写入缓存时的 Unix 秒 |
| `stale` | bool | 网络失败回退旧缓存时为 true |
| `cache_hit` | bool | 本次未发起网络、直接用有效缓存 |
| `models` | `LeaderboardModel[]` | 白名单解析结果 |

### 4. Validation & Error Matrix

| 条件 | 行为 |
|------|------|
| 缓存存在且未过期且 `force_refresh=false` | 返回缓存，`cache_hit=true` |
| 网络成功 | 写缓存，`stale=false`、`cache_hit=false` |
| 网络失败 / 超时 / 非 JSON / 缺 `data`，**有**旧缓存 | 返回旧缓存，`stale=true` |
| 同上且 **无** 缓存 | `Err`，可行动中文 message（检查网络/稍后重试） |
| 条目缺 `id` 或 id 为空 | 跳过该条目，不整单失败 |
| 分数非数字 | 对应 score 为 `None`，不崩 |

### 5. Good/Base/Bad Cases

- **Good**：24h 内二次打开 → `cache_hit=true`，无网络。
- **Base**：无缓存首次拉取 → 网络成功 → 落盘并返回 models。
- **Bad**：断网且无缓存 → 业务错误；断网有缓存 → `stale=true` 仍可用。

### 6. Tests Required

- 白名单解析：含多余字段时输出不含 pricing/description。
- 新鲜缓存命中：`force_refresh=false` 不调 fetch。
- 强制刷新：即使缓存新鲜也调 fetch。
- 网络失败 + 有缓存 → `stale=true`。
- 网络失败 + 无缓存 → `Err`。
- 超时常量 / URL 常量存在且为 15s 与固定 OpenRouter URL。

### 7. Wrong vs Correct

#### Wrong

```rust
// 接受前端任意 URL，或带上用户供应商 Key
let url = user_provided_url;
req.header("Authorization", format!("Bearer {}", provider_key));
```

#### Correct

```rust
// 固定 URL、无 Key；只解析白名单字段后写 config_dir 缓存
const OPENROUTER_MODELS_URL: &str =
    "https://openrouter.ai/api/v1/models?sort=intelligence-high-to-low";
// 失败有缓存 → stale 快照；无缓存才 Err
```

---

## Anti-Patterns

- 抓 LMSYS / 网页 HTML 当榜单源。
- 把完整上游响应 JSON 原样缓存或透传前端。
- 外网失败时清空已有缓存。
- 在错误信息中带入任何 Key。
