//! llm_benchmark 公共模型榜单：CSV 白名单解析、24h 文件缓存、stale 回退。
//!
//! 数据源为 [llm2014/llm_benchmark](https://github.com/llm2014/llm_benchmark) 仓库
//! GitHub Pages 托管的原始文件（无任何 API Key）：
//! 1. `docs/data/datasets.json` 列出各榜单 CSV 路径与月份；
//! 2. 取 `category == "logic"` 的最新月榜 CSV（展示名 + 极限分数）。
//!
//! 解析白名单：模型名（展示名）与「极限分数」列，其余列一律丢弃。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;

/// llm_benchmark raw 基础地址（GitHub Pages 原始文件，`docs/` 前缀）。
pub const LLM_BENCHMARK_BASE: &str =
    "https://raw.githubusercontent.com/llm2014/llm_benchmark/main/docs/";

/// datasets.json：列出各榜单 CSV 路径与月份。
pub const LLM_BENCHMARK_DATASETS_URL: &str = concat!(
    "https://raw.githubusercontent.com/llm2014/llm_benchmark/main/docs/data/datasets.json"
);

/// 使用的榜单分类（logic = 综合榜）。
pub const LLM_BENCHMARK_CATEGORY: &str = "logic";

/// 整次请求超时（连接 + 响应体）。
pub const LEADERBOARD_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
pub const LEADERBOARD_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// 缓存 TTL：24 小时。
pub const LEADERBOARD_CACHE_TTL_SECS: i64 = 24 * 60 * 60;

/// 缓存文件名（位于应用 `config_dir`）。
pub const LEADERBOARD_CACHE_FILE: &str = "model-leaderboard-llm-benchmark.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeaderboardModel {
    /// 榜单展示名（如 `GPT-5.5 (xhigh)`）；同时作为匹配用 id。
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_slug: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// llm_benchmark logic 榜「极限分数」（0-100）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intelligence_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coding_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agentic_score: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLeaderboardSnapshot {
    /// 固定为 `"llm_benchmark"`。
    pub source: String,
    /// 缓存写入 / 网络拉取成功时的 Unix 秒。
    pub fetched_at_unix: i64,
    /// 网络失败时使用旧缓存则为 true。
    pub stale: bool,
    /// 本次是否直接命中有效缓存（未发起网络）。
    pub cache_hit: bool,
    pub models: Vec<LeaderboardModel>,
}

/// 磁盘缓存结构（不含运行时 `stale` / `cache_hit`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LeaderboardCacheFile {
    source: String,
    fetched_at_unix: i64,
    models: Vec<LeaderboardModel>,
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn cache_path(config_dir: &Path) -> PathBuf {
    config_dir.join(LEADERBOARD_CACHE_FILE)
}

fn is_cache_fresh(fetched_at_unix: i64, now: i64) -> bool {
    fetched_at_unix > 0 && now.saturating_sub(fetched_at_unix) < LEADERBOARD_CACHE_TTL_SECS
}

/* ------------------------------------------------------------------ */
/* CSV 解析（手写，不引入 csv crate）                                   */
/* ------------------------------------------------------------------ */

/// 手写 CSV 行解析：支持引号包裹字段、引号内逗号、双引号转义（`""` → `"`）。
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            '"' => {
                in_quotes = true;
            }
            ',' if !in_quotes => {
                fields.push(std::mem::take(&mut current));
            }
            _ => current.push(c),
        }
    }
    fields.push(current);
    fields
}

/// 解析 llm_benchmark logic 月榜 CSV。
/// 表头按列名定位「模型」与「极限分数」的索引（不依赖固定列序）。
pub fn parse_llm_benchmark_csv(body: &str) -> Result<Vec<LeaderboardModel>, AppError> {
    let mut lines = body.lines().map(str::trim).filter(|l| !l.is_empty());

    let header = lines
        .next()
        .ok_or_else(|| AppError::Business("无法解析 llm_benchmark 榜单：CSV 为空".into()))?;
    let header_fields = parse_csv_line(header);

    let model_col = header_fields
        .iter()
        .position(|h| h.trim() == "模型")
        .ok_or_else(|| AppError::Business("无法解析 llm_benchmark 榜单：缺少「模型」列".into()))?;
    let score_col = header_fields
        .iter()
        .position(|h| h.trim() == "极限分数")
        .ok_or_else(|| AppError::Business("无法解析 llm_benchmark 榜单：缺少「极限分数」列".into()))?;

    let mut out = Vec::new();
    for line in lines {
        let fields = parse_csv_line(line);
        let Some(model) = fields.get(model_col).map(|s| s.trim()).filter(|s| !s.is_empty())
        else {
            continue;
        };
        let Some(score_raw) = fields.get(score_col).map(|s| s.trim()) else {
            continue;
        };
        let Ok(score) = score_raw.parse::<f64>() else {
            continue;
        };
        if !score.is_finite() {
            continue;
        }
        out.push(LeaderboardModel {
            id: model.to_string(),
            canonical_slug: None,
            name: Some(model.to_string()),
            intelligence_score: Some(score),
            coding_score: None,
            agentic_score: None,
        });
    }

    if out.is_empty() {
        return Err(AppError::Business(
            "llm_benchmark 返回空榜单（无可解析的「模型/极限分数」行）。请稍后强制刷新。".into(),
        ));
    }
    Ok(out)
}

/// 从 datasets.json 定位 `category == logic` 且 reportDate 最新的 csv 相对路径。
/// reportDate 形如 `2026-08`，字符串字典序即时间序。
pub fn locate_latest_logic_csv(datasets_json: &str) -> Result<String, AppError> {
    let value: Value = serde_json::from_str(datasets_json).map_err(|_| {
        AppError::Business("无法解析 llm_benchmark datasets.json：响应不是有效 JSON".into())
    })?;

    let datasets = value
        .get("datasets")
        .and_then(|d| d.as_array())
        .ok_or_else(|| AppError::Business("无法解析 llm_benchmark datasets.json：缺少 datasets 数组".into()))?;

    let mut best: Option<(String, String)> = None; // (reportDate, csv)
    for item in datasets {
        let Some(category) = item.get("category").and_then(|v| v.as_str()) else {
            continue;
        };
        if category != LLM_BENCHMARK_CATEGORY {
            continue;
        }
        let Some(report_date) = item.get("reportDate").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(csv) = item.get("csv").and_then(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty())
        else {
            continue;
        };
        // 优先「月榜」标题；取最新 reportDate（字典序）。
        let is_monthly = item
            .get("title")
            .and_then(|v| v.as_str())
            .is_some_and(|t| t == "月榜");
        let replace = match &best {
            Some((best_date, _)) => {
                let better_date = report_date > best_date.as_str();
                let same_date_monthly = report_date == best_date.as_str() && is_monthly;
                better_date || same_date_monthly
            }
            None => true,
        };
        if replace {
            best = Some((report_date.to_string(), csv.to_string()));
        }
    }

    best.map(|(_, csv)| csv).ok_or_else(|| {
        AppError::Business(format!(
            "无法解析 llm_benchmark datasets.json：未找到 {} 分类的月榜",
            LLM_BENCHMARK_CATEGORY
        ))
    })
}

/* ------------------------------------------------------------------ */
/* 磁盘缓存                                                             */
/* ------------------------------------------------------------------ */

fn read_cache(config_dir: &Path) -> Result<Option<LeaderboardCacheFile>, AppError> {
    let path = cache_path(config_dir);
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|source| AppError::ReadShellConfig {
        path: path.display().to_string(),
        source,
    })?;
    let cache: LeaderboardCacheFile = serde_json::from_str(&text).map_err(|e| {
        AppError::Business(format!(
            "榜单缓存损坏（{}）：{}。可强制刷新重新拉取，或删除该文件后重试。",
            path.display(),
            e
        ))
    })?;
    if cache.models.is_empty() {
        return Ok(None);
    }
    Ok(Some(cache))
}

fn write_cache(
    config_dir: &Path,
    models: &[LeaderboardModel],
    fetched_at_unix: i64,
) -> Result<(), AppError> {
    fs::create_dir_all(config_dir).map_err(|source| AppError::CreateDirectory {
        path: config_dir.display().to_string(),
        source,
    })?;
    let path = cache_path(config_dir);
    let tmp = config_dir.join(format!("{LEADERBOARD_CACHE_FILE}.tmp"));
    let file = LeaderboardCacheFile {
        source: "llm_benchmark".into(),
        fetched_at_unix,
        models: models.to_vec(),
    };
    let text =
        serde_json::to_string_pretty(&file).map_err(|source| AppError::SerializeShellConfig {
            path: path.display().to_string(),
            source,
        })?;
    {
        let mut f = fs::File::create(&tmp).map_err(|source| AppError::WriteShellConfig {
            path: tmp.display().to_string(),
            source,
        })?;
        use std::io::Write;
        f.write_all(text.as_bytes())
            .map_err(|source| AppError::WriteShellConfig {
                path: tmp.display().to_string(),
                source,
            })?;
        f.sync_all().map_err(|source| AppError::WriteShellConfig {
            path: tmp.display().to_string(),
            source,
        })?;
    }
    // Windows 不允许 rename 覆盖已有目标；先移除旧缓存再替换。
    if path.exists() {
        fs::remove_file(&path).map_err(|source| AppError::WriteShellConfig {
            path: path.display().to_string(),
            source,
        })?;
    }
    fs::rename(&tmp, &path).map_err(|source| AppError::WriteShellConfig {
        path: path.display().to_string(),
        source,
    })?;
    Ok(())
}

/* ------------------------------------------------------------------ */
/* 网络拉取                                                             */
/* ------------------------------------------------------------------ */

fn map_reqwest_error(err: reqwest::Error) -> AppError {
    if err.is_timeout() {
        return AppError::Business(
            "请求 llm_benchmark 榜单超时（15 秒）。请检查网络后强制刷新，或使用本地启发式排序。"
                .into(),
        );
    }
    if err.is_connect() {
        return AppError::Business(
            "无法连接 llm_benchmark。请检查网络后强制刷新，或使用本地启发式排序。".into(),
        );
    }
    AppError::Business(format!(
        "请求 llm_benchmark 失败：{}。可强制刷新重试，或使用本地启发式排序。",
        sanitize_network_message(&err.to_string())
    ))
}

fn sanitize_network_message(msg: &str) -> String {
    let mut s = msg.to_string();
    for marker in ["Bearer ", "bearer ", "api_key=", "api-key=", "key="] {
        if let Some(idx) = s.find(marker) {
            let start = idx + marker.len();
            let rest = &s[start..];
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == '}')
                .unwrap_or(rest.len());
            if end > 4 {
                s.replace_range(start..start + end, "***");
            }
        }
    }
    s
}

async fn get_text(url: &str, client: &reqwest::Client) -> Result<String, AppError> {
    let response = client
        .get(url)
        .header("Accept", "application/json,text/csv,text/plain")
        .send()
        .await
        .map_err(map_reqwest_error)?;

    let status = response.status();
    if !status.is_success() {
        return Err(AppError::Business(format!(
            "llm_benchmark 返回 HTTP {}，无法获取榜单。请稍后强制刷新，或使用本地启发式排序。",
            status.as_u16()
        )));
    }

    response.text().await.map_err(|e| {
        AppError::Business(format!(
            "读取 llm_benchmark 响应失败：{}。请稍后强制刷新，或使用本地启发式排序。",
            sanitize_network_message(&e.to_string())
        ))
    })
}

/// 从 llm_benchmark 拉取并白名单解析（无 Key）：
/// 1. datasets.json 定位最新 logic 月榜 CSV；2. 拉取并解析 CSV。
pub async fn fetch_llm_benchmark_models() -> Result<Vec<LeaderboardModel>, AppError> {
    let client = reqwest::Client::builder()
        .timeout(LEADERBOARD_REQUEST_TIMEOUT)
        .connect_timeout(LEADERBOARD_CONNECT_TIMEOUT)
        .build()
        .map_err(|e| AppError::Business(format!("无法创建 HTTP 客户端：{e}")))?;

    let datasets_json = get_text(LLM_BENCHMARK_DATASETS_URL, &client).await?;
    let csv_rel_path = locate_latest_logic_csv(&datasets_json)?;
    let csv_url = format!("{LLM_BENCHMARK_BASE}{csv_rel_path}");
    let csv_body = get_text(&csv_url, &client).await?;
    parse_llm_benchmark_csv(&csv_body)
}

/// 获取榜单快照：优先 24h 缓存；`force_refresh` 时尝试网络；失败时 stale 回退。
pub async fn get_model_leaderboard(
    config_dir: &Path,
    force_refresh: bool,
) -> Result<ModelLeaderboardSnapshot, AppError> {
    get_model_leaderboard_with_fetch(config_dir, force_refresh, fetch_llm_benchmark_models).await
}

/// 可注入 fetch 的内部实现（单测与生产共用）。
async fn get_model_leaderboard_with_fetch<F, Fut>(
    config_dir: &Path,
    force_refresh: bool,
    fetch: F,
) -> Result<ModelLeaderboardSnapshot, AppError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<Vec<LeaderboardModel>, AppError>>,
{
    let now = now_unix();
    // 损坏/不可读缓存不应阻止联网刷新；它不算可用于 stale 回退的缓存。
    let cached = match read_cache(config_dir) {
        Ok(cache) => cache,
        Err(error) => {
            tracing::warn!(error = %error, "读取榜单缓存失败，将尝试联网刷新");
            None
        }
    };

    if !force_refresh {
        if let Some(ref cache) = cached {
            if is_cache_fresh(cache.fetched_at_unix, now) {
                return Ok(ModelLeaderboardSnapshot {
                    source: "llm_benchmark".into(),
                    fetched_at_unix: cache.fetched_at_unix,
                    stale: false,
                    cache_hit: true,
                    models: cache.models.clone(),
                });
            }
        }
    }

    match fetch().await {
        Ok(models) if !models.is_empty() => {
            let fetched_at = now_unix();
            if let Err(e) = write_cache(config_dir, &models, fetched_at) {
                tracing::warn!(error = %e, "写入榜单缓存失败");
            }
            Ok(ModelLeaderboardSnapshot {
                source: "llm_benchmark".into(),
                fetched_at_unix: fetched_at,
                stale: false,
                cache_hit: false,
                models,
            })
        }
        Ok(_) => stale_or_error(
            cached,
            AppError::Business(
                "llm_benchmark 返回空榜单。请稍后强制刷新，或使用本地启发式排序。".into(),
            ),
        ),
        Err(err) => stale_or_error(cached, err),
    }
}

fn stale_or_error(
    cached: Option<LeaderboardCacheFile>,
    error: AppError,
) -> Result<ModelLeaderboardSnapshot, AppError> {
    if let Some(cache) = cached {
        tracing::warn!(error = %error, "拉取 llm_benchmark 榜单失败，使用旧缓存");
        return Ok(ModelLeaderboardSnapshot {
            source: "llm_benchmark".into(),
            fetched_at_unix: cache.fetched_at_unix,
            stale: true,
            cache_hit: false,
            models: cache.models,
        });
    }
    Err(error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const SAMPLE_CSV: &str = "\"模型\",\"极限分数\",\"中位分数\",\"中位差距\",\"变更\",\"平均耗时(秒)\",\"Token\",\"测试成本(元)\",\"价格(元/百万)\",\"发布时间\",\"Think\"\n\
\"GPT-5.5 (xhigh)\",\"83.80\",\"77.46\",\"7.57%\",\"+3.4%\",\"494\",\"30811\",\"¥178.58\",\"¥207.00\",\"26-04-24\",\"1\"\n\
\"Kimi-K3 (max)\",\"82.91\",\"74.80\",\"9.78%\",\"+32.6%\",\"1095\",\"39912\",\"¥117.34\",\"¥105.00\",\"26-07-16\",\"1\"\n\
\"DeepSeek V4 Flash 0731 (max)\",\"68.12\",\"58.80\",\"13.68%\",\"+34.5%\",\"907\",\"74801\",\"¥4.19\",\"¥2.00\",\"26-07-31\",\"1\"\n\
\"vendor,no-score\",\"not-a-number\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\"\n";

    const SAMPLE_DATASETS: &str = r#"{
      "datasets": [
        {"category": "code", "reportDate": "2026-07", "tableIndex": 0, "title": "月榜", "csv": "data/code/2026-07.csv"},
        {"category": "logic", "reportDate": "2026-06", "tableIndex": 0, "title": "月榜", "csv": "data/logic/2026-06.csv"},
        {"category": "logic", "reportDate": "2026-08", "tableIndex": 0, "title": "月榜", "csv": "data/logic/2026-08.csv"},
        {"category": "logic", "reportDate": "2026-08", "tableIndex": 1, "title": "副榜", "csv": "data/logic/2026-08-extra.csv"}
      ]
    }"#;

    #[test]
    fn parse_csv_whitelist_columns() {
        let models = parse_llm_benchmark_csv(SAMPLE_CSV).unwrap();
        assert_eq!(models.len(), 3);

        assert_eq!(models[0].id, "GPT-5.5 (xhigh)");
        assert_eq!(models[0].name.as_deref(), Some("GPT-5.5 (xhigh)"));
        assert_eq!(models[0].intelligence_score, Some(83.80));
        assert!(models[0].coding_score.is_none());
        assert!(models[0].agentic_score.is_none());

        assert_eq!(models[1].intelligence_score, Some(82.91));
        assert_eq!(models[2].intelligence_score, Some(68.12));
    }

    #[test]
    fn parse_csv_skips_bad_score_rows() {
        // 无「极限分数」数值的行（含引号内逗号的模型名）不产生条目。
        let models = parse_llm_benchmark_csv(SAMPLE_CSV).unwrap();
        assert_eq!(models.len(), 3);
    }

    #[test]
    fn parse_csv_rejects_non_csv() {
        let err = parse_llm_benchmark_csv("not csv at all").unwrap_err();
        assert!(err.to_string().contains("无法解析"));
    }

    #[test]
    fn parse_csv_rejects_missing_columns() {
        let err = parse_llm_benchmark_csv("\"模型\",\"中位分数\"\n\"a\",\"1\"").unwrap_err();
        assert!(err.to_string().contains("极限分数"));
    }

    #[test]
    fn parse_csv_rejects_empty_models() {
        let err = parse_llm_benchmark_csv("\"模型\",\"极限分数\"\n\"\",\"1\"").unwrap_err();
        assert!(err.to_string().contains("空榜单"));
    }

    #[test]
    fn locate_latest_logic_picks_newest_monthly() {
        let csv = locate_latest_logic_csv(SAMPLE_DATASETS).unwrap();
        assert_eq!(csv, "data/logic/2026-08.csv");
    }

    #[test]
    fn locate_latest_logic_rejects_missing_category() {
        let err = locate_latest_logic_csv(r#"{"datasets":[{"category":"code","reportDate":"2026-08","csv":"x.csv"}]}"#).unwrap_err();
        assert!(err.to_string().contains("未找到"));
    }

    #[test]
    fn locate_latest_logic_rejects_non_json() {
        let err = locate_latest_logic_csv("not json").unwrap_err();
        assert!(err.to_string().contains("无法解析"));
    }

    #[test]
    fn cache_roundtrip_and_freshness() {
        let dir = TempDir::new().unwrap();
        let models = parse_llm_benchmark_csv(SAMPLE_CSV).unwrap();
        let now = 1_700_000_000_i64;
        write_cache(dir.path(), &models, now).unwrap();

        let loaded = read_cache(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.fetched_at_unix, now);
        assert_eq!(loaded.models.len(), 3);
        assert_eq!(loaded.source, "llm_benchmark");

        assert!(is_cache_fresh(now, now + 3600));
        assert!(!is_cache_fresh(now, now + LEADERBOARD_CACHE_TTL_SECS + 1));
    }

    #[test]
    fn missing_cache_returns_none() {
        let dir = TempDir::new().unwrap();
        assert!(read_cache(dir.path()).unwrap().is_none());
    }

    #[tokio::test]
    async fn get_leaderboard_uses_fresh_cache_without_network() {
        let dir = TempDir::new().unwrap();
        let models = parse_llm_benchmark_csv(SAMPLE_CSV).unwrap();
        let now = now_unix();
        write_cache(dir.path(), &models, now).unwrap();

        let snap = get_model_leaderboard_with_fetch(dir.path(), false, || async {
            panic!("fresh cache must not call network");
        })
        .await
        .unwrap();
        assert!(snap.cache_hit);
        assert!(!snap.stale);
        assert_eq!(snap.source, "llm_benchmark");
        assert_eq!(snap.models.len(), 3);
    }

    #[tokio::test]
    async fn get_leaderboard_stale_when_network_fails_and_cache_exists() {
        let dir = TempDir::new().unwrap();
        let models = parse_llm_benchmark_csv(SAMPLE_CSV).unwrap();
        write_cache(dir.path(), &models, 1).unwrap();

        let snap = get_model_leaderboard_with_fetch(dir.path(), true, || async {
            Err(AppError::Business("模拟网络失败".into()))
        })
        .await
        .unwrap();
        assert!(snap.stale);
        assert!(!snap.cache_hit);
        assert_eq!(snap.fetched_at_unix, 1);
        assert_eq!(snap.models.len(), 3);
    }

    #[tokio::test]
    async fn get_leaderboard_errors_when_network_fails_without_cache() {
        let dir = TempDir::new().unwrap();
        let err = get_model_leaderboard_with_fetch(dir.path(), true, || async {
            Err(AppError::Business("模拟网络失败".into()))
        })
        .await
        .unwrap_err();
        assert!(err.to_string().contains("模拟网络失败"));
    }

    #[tokio::test]
    async fn get_leaderboard_force_refresh_writes_cache() {
        let dir = TempDir::new().unwrap();
        let models = parse_llm_benchmark_csv(SAMPLE_CSV).unwrap();
        write_cache(dir.path(), &models[..1], 1).unwrap();

        let snap = get_model_leaderboard_with_fetch(dir.path(), true, || async {
            Ok(parse_llm_benchmark_csv(SAMPLE_CSV).unwrap())
        })
        .await
        .unwrap();
        assert!(!snap.stale);
        assert!(!snap.cache_hit);
        assert_eq!(snap.models.len(), 3);

        let reloaded = read_cache(dir.path()).unwrap().unwrap();
        assert_eq!(reloaded.models.len(), 3);
        assert!(reloaded.fetched_at_unix > 1);
    }

    #[test]
    fn sanitize_masks_bearer() {
        let s = sanitize_network_message("error Bearer sk-secret-value more");
        assert!(!s.contains("sk-secret-value"));
        assert!(s.contains("***"));
    }

    #[test]
    fn urls_point_to_llm_benchmark() {
        assert!(LLM_BENCHMARK_DATASETS_URL.contains("raw.githubusercontent.com/llm2014/llm_benchmark"));
        assert!(LLM_BENCHMARK_DATASETS_URL.contains("datasets.json"));
        assert!(LLM_BENCHMARK_BASE.contains("raw.githubusercontent.com/llm2014/llm_benchmark"));
        assert!(!LLM_BENCHMARK_DATASETS_URL.contains("openrouter.ai"));
    }
}
