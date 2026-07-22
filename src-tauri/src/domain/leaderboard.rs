//! OpenRouter 公共模型榜单：白名单解析、24h 文件缓存、stale 回退。
//!
//! 固定请求 `GET https://openrouter.ai/api/v1/models?sort=intelligence-high-to-low`，
//! 不携带任何 API Key；仅提取 `id/canonical_slug/name` 与
//! `benchmarks.artificial_analysis.{intelligence_index,coding_index,agentic_index}`。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;

/// OpenRouter 公共 Models API（固定 URL，不接受前端自定义）。
pub const OPENROUTER_MODELS_URL: &str =
    "https://openrouter.ai/api/v1/models?sort=intelligence-high-to-low";

/// 整次请求超时（连接 + 响应体）。
pub const LEADERBOARD_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
pub const LEADERBOARD_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// 缓存 TTL：24 小时。
pub const LEADERBOARD_CACHE_TTL_SECS: i64 = 24 * 60 * 60;

/// 缓存文件名（位于应用 `config_dir`）。
pub const LEADERBOARD_CACHE_FILE: &str = "model-leaderboard-openrouter.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeaderboardModel {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_slug: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intelligence_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coding_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agentic_score: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLeaderboardSnapshot {
    /// 固定为 `"openrouter"`。
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

/// 从 JSON 白名单解析榜单模型列表（禁止透传未知字段）。
pub fn parse_openrouter_models_json(body: &str) -> Result<Vec<LeaderboardModel>, AppError> {
    let value: Value = serde_json::from_str(body)
        .map_err(|_| AppError::Business("无法解析 OpenRouter 榜单：响应不是有效 JSON".into()))?;

    let data = value
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| AppError::Business("无法解析 OpenRouter 榜单：缺少 data 数组".into()))?;

    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(id) = item.get("id").and_then(|v| v.as_str()).map(str::trim) else {
            continue;
        };
        if id.is_empty() {
            continue;
        }

        let canonical_slug = item
            .get("canonical_slug")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        let aa = item
            .get("benchmarks")
            .and_then(|b| b.get("artificial_analysis"));

        let intelligence_score = aa
            .and_then(|a| a.get("intelligence_index"))
            .and_then(json_number_to_f64);
        let coding_score = aa
            .and_then(|a| a.get("coding_index"))
            .and_then(json_number_to_f64);
        let agentic_score = aa
            .and_then(|a| a.get("agentic_index"))
            .and_then(json_number_to_f64);

        out.push(LeaderboardModel {
            id: id.to_string(),
            canonical_slug,
            name,
            intelligence_score,
            coding_score,
            agentic_score,
        });
    }
    Ok(out)
}

fn json_number_to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok().filter(|x| x.is_finite()),
        _ => None,
    }
    .filter(|x| x.is_finite())
}

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
        source: "openrouter".into(),
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

fn map_reqwest_error(err: reqwest::Error) -> AppError {
    if err.is_timeout() {
        return AppError::Business(
            "请求 OpenRouter 榜单超时（15 秒）。请检查网络后强制刷新，或使用本地启发式排序。"
                .into(),
        );
    }
    if err.is_connect() {
        return AppError::Business(
            "无法连接 OpenRouter。请检查网络后强制刷新，或使用本地启发式排序。".into(),
        );
    }
    AppError::Business(format!(
        "请求 OpenRouter 失败：{}。可强制刷新重试，或使用本地启发式排序。",
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

/// 从 OpenRouter 拉取并白名单解析（无 Key）。
pub async fn fetch_openrouter_models() -> Result<Vec<LeaderboardModel>, AppError> {
    let client = reqwest::Client::builder()
        .timeout(LEADERBOARD_REQUEST_TIMEOUT)
        .connect_timeout(LEADERBOARD_CONNECT_TIMEOUT)
        .build()
        .map_err(|e| AppError::Business(format!("无法创建 HTTP 客户端：{e}")))?;

    let response = client
        .get(OPENROUTER_MODELS_URL)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(map_reqwest_error)?;

    let status = response.status();
    if !status.is_success() {
        return Err(AppError::Business(format!(
            "OpenRouter 返回 HTTP {}，无法获取榜单。请稍后强制刷新，或使用本地启发式排序。",
            status.as_u16()
        )));
    }

    let body = response.text().await.map_err(|e| {
        AppError::Business(format!(
            "读取 OpenRouter 响应失败：{}。请稍后强制刷新，或使用本地启发式排序。",
            sanitize_network_message(&e.to_string())
        ))
    })?;

    parse_openrouter_models_json(&body)
}

/// 获取榜单快照：优先 24h 缓存；`force_refresh` 时尝试网络；失败时 stale 回退。
pub async fn get_model_leaderboard(
    config_dir: &Path,
    force_refresh: bool,
) -> Result<ModelLeaderboardSnapshot, AppError> {
    get_model_leaderboard_with_fetch(config_dir, force_refresh, fetch_openrouter_models).await
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
                    source: "openrouter".into(),
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
                source: "openrouter".into(),
                fetched_at_unix: fetched_at,
                stale: false,
                cache_hit: false,
                models,
            })
        }
        Ok(_) => stale_or_error(
            cached,
            AppError::Business(
                "OpenRouter 返回空榜单。请稍后强制刷新，或使用本地启发式排序。".into(),
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
        tracing::warn!(error = %error, "拉取 OpenRouter 榜单失败，使用旧缓存");
        return Ok(ModelLeaderboardSnapshot {
            source: "openrouter".into(),
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

    const SAMPLE_BODY: &str = r#"{
      "data": [
        {
          "id": "anthropic/claude-sonnet-4",
          "canonical_slug": "anthropic/claude-sonnet-4",
          "name": "Claude Sonnet 4",
          "description": "should be ignored",
          "pricing": {"prompt": "0.001"},
          "benchmarks": {
            "artificial_analysis": {
              "intelligence_index": 72.5,
              "coding_index": 68.0,
              "agentic_index": 55.1
            }
          }
        },
        {
          "id": "openai/gpt-5",
          "name": "GPT-5",
          "benchmarks": {
            "artificial_analysis": {
              "intelligence_index": "90.1",
              "coding_index": 88
            }
          }
        },
        {
          "id": "vendor/no-bench",
          "canonical_slug": "vendor/no-bench"
        },
        {
          "object": "model"
        }
      ]
    }"#;

    #[test]
    fn parse_whitelist_only_fields() {
        let models = parse_openrouter_models_json(SAMPLE_BODY).unwrap();
        assert_eq!(models.len(), 3);

        assert_eq!(models[0].id, "anthropic/claude-sonnet-4");
        assert_eq!(
            models[0].canonical_slug.as_deref(),
            Some("anthropic/claude-sonnet-4")
        );
        assert_eq!(models[0].name.as_deref(), Some("Claude Sonnet 4"));
        assert_eq!(models[0].intelligence_score, Some(72.5));
        assert_eq!(models[0].coding_score, Some(68.0));
        assert_eq!(models[0].agentic_score, Some(55.1));

        assert_eq!(models[1].intelligence_score, Some(90.1));
        assert_eq!(models[1].coding_score, Some(88.0));
        assert!(models[1].agentic_score.is_none());

        assert!(models[2].intelligence_score.is_none());
        assert!(models[2].coding_score.is_none());
    }

    #[test]
    fn parse_rejects_non_json() {
        let err = parse_openrouter_models_json("not-json").unwrap_err();
        assert!(err.to_string().contains("无法解析"));
    }

    #[test]
    fn parse_rejects_missing_data() {
        let err = parse_openrouter_models_json(r#"{"object":"list"}"#).unwrap_err();
        assert!(err.to_string().contains("data"));
    }

    #[test]
    fn cache_roundtrip_and_freshness() {
        let dir = TempDir::new().unwrap();
        let models = parse_openrouter_models_json(SAMPLE_BODY).unwrap();
        let now = 1_700_000_000_i64;
        write_cache(dir.path(), &models, now).unwrap();

        let loaded = read_cache(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.fetched_at_unix, now);
        assert_eq!(loaded.models.len(), 3);
        assert_eq!(loaded.source, "openrouter");

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
        let models = parse_openrouter_models_json(SAMPLE_BODY).unwrap();
        let now = now_unix();
        write_cache(dir.path(), &models, now).unwrap();

        let snap = get_model_leaderboard_with_fetch(dir.path(), false, || async {
            panic!("fresh cache must not call network");
        })
        .await
        .unwrap();
        assert!(snap.cache_hit);
        assert!(!snap.stale);
        assert_eq!(snap.source, "openrouter");
        assert_eq!(snap.models.len(), 3);
    }

    #[tokio::test]
    async fn get_leaderboard_stale_when_network_fails_and_cache_exists() {
        let dir = TempDir::new().unwrap();
        let models = parse_openrouter_models_json(SAMPLE_BODY).unwrap();
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
        let models = parse_openrouter_models_json(SAMPLE_BODY).unwrap();
        write_cache(dir.path(), &models[..1], 1).unwrap();

        let snap = get_model_leaderboard_with_fetch(dir.path(), true, || async {
            Ok(parse_openrouter_models_json(SAMPLE_BODY).unwrap())
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
    fn openrouter_url_is_fixed() {
        assert!(OPENROUTER_MODELS_URL.contains("openrouter.ai"));
        assert!(OPENROUTER_MODELS_URL.contains("sort=intelligence-high-to-low"));
    }
}
