//! 将本机 Model Hub 分组导出为 Pi Agent 的 models.json 配置。

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Map, Value};

use crate::error::AppError;

pub const PI_PROVIDER_ID: &str = "model-hub";
pub const DEFAULT_PLACEHOLDER_KEY: &str = "model-hub";

/// 将代理 Base URL（如 `http://127.0.0.1:8080`）规范为 Pi 需要的 OpenAI 兼容根（含 `/v1`）。
pub fn normalize_openai_base_url(base_url: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        return String::new();
    }
    if base.ends_with("/v1") {
        base.to_string()
    } else {
        format!("{base}/v1")
    }
}

pub fn default_pi_models_path() -> Result<PathBuf, AppError> {
    let home = dirs_home().ok_or_else(|| {
        AppError::Business("无法定位用户主目录，无法写入 Pi Agent 配置".into())
    })?;
    Ok(home.join(".pi").join("agent").join("models.json"))
}

fn dirs_home() -> Option<PathBuf> {
    // 优先标准环境变量，避免额外依赖。
    if let Ok(h) = std::env::var("USERPROFILE") {
        if !h.trim().is_empty() {
            return Some(PathBuf::from(h));
        }
    }
    if let Ok(h) = std::env::var("HOME") {
        if !h.trim().is_empty() {
            return Some(PathBuf::from(h));
        }
    }
    None
}

/// 构建 `providers.model-hub` 节点。
pub fn build_model_hub_provider(base_url: &str, api_key: &str, group_names: &[String]) -> Value {
    let models: Vec<Value> = group_names
        .iter()
        .map(|name| {
            json!({
                "id": name,
                "name": name,
            })
        })
        .collect();
    json!({
        "baseUrl": normalize_openai_base_url(base_url),
        "api": "openai-completions",
        "apiKey": if api_key.trim().is_empty() {
            DEFAULT_PLACEHOLDER_KEY
        } else {
            api_key.trim()
        },
        "models": models,
    })
}

/// 将 model-hub provider 合并进既有 models.json 根对象。
pub fn merge_model_hub_into_root(existing: Option<Value>, provider: Value) -> Result<Value, AppError> {
    let mut root = match existing {
        None => json!({ "providers": {} }),
        Some(Value::Object(map)) => Value::Object(map),
        Some(other) => {
            return Err(AppError::Business(format!(
                "Pi models.json 根节点必须是对象，当前为 {}",
                type_name(&other)
            )));
        }
    };

    let obj = root
        .as_object_mut()
        .ok_or_else(|| AppError::Business("Pi models.json 解析失败".into()))?;

    let providers = obj
        .entry("providers".to_string())
        .or_insert_with(|| Value::Object(Map::new()));

    let providers_obj = providers.as_object_mut().ok_or_else(|| {
        AppError::Business("Pi models.json 的 providers 必须是对象".into())
    })?;

    providers_obj.insert(PI_PROVIDER_ID.to_string(), provider);
    Ok(root)
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

pub fn read_models_json(path: &Path) -> Result<Option<Value>, AppError> {
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).map_err(|source| AppError::Business(format!(
        "读取 Pi 配置失败（{}）：{source}",
        path.display()
    )))?;
    if text.trim().is_empty() {
        return Ok(None);
    }
    let value: Value = serde_json::from_str(&text).map_err(|e| {
        AppError::Business(format!(
            "Pi models.json 不是合法 JSON（{}）：{e}",
            path.display()
        ))
    })?;
    Ok(Some(value))
}

pub fn write_models_json(path: &Path, root: &Value) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| AppError::CreateDirectory {
            path: parent.display().to_string(),
            source,
        })?;
    }
    let pretty = serde_json::to_string_pretty(root).map_err(|e| {
        AppError::Business(format!("序列化 Pi models.json 失败：{e}"))
    })?;
    // 直接覆盖写入即可（本机配置文件，体积小）。
    fs::write(path, format!("{pretty}\n")).map_err(|source| {
        AppError::Business(format!(
            "写入 Pi models.json 失败（{}）：{source}",
            path.display()
        ))
    })?;
    Ok(())
}

/// 导出到指定路径（测试可注入路径）。
pub fn export_model_hub_to_path(
    path: &Path,
    base_url: &str,
    api_key: &str,
    group_names: &[String],
) -> Result<PathBuf, AppError> {
    if group_names.is_empty() {
        return Err(AppError::Business(
            "当前没有分组。请先在「分组」页创建至少一个分组，再配置到 Pi。".into(),
        ));
    }
    if normalize_openai_base_url(base_url).is_empty() {
        return Err(AppError::Business("代理 Base URL 无效".into()));
    }
    let provider = build_model_hub_provider(base_url, api_key, group_names);
    let existing = read_models_json(path)?;
    let merged = merge_model_hub_into_root(existing, provider)?;
    write_models_json(path, &merged)?;
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn normalize_base_url_appends_v1() {
        assert_eq!(
            normalize_openai_base_url("http://127.0.0.1:8080"),
            "http://127.0.0.1:8080/v1"
        );
        assert_eq!(
            normalize_openai_base_url("http://127.0.0.1:8080/v1/"),
            "http://127.0.0.1:8080/v1"
        );
    }

    #[test]
    fn merge_preserves_other_providers() {
        let existing = json!({
            "providers": {
                "ollama": {
                    "baseUrl": "http://localhost:11434/v1",
                    "api": "openai-completions",
                    "apiKey": "ollama",
                    "models": [{"id": "llama"}]
                }
            }
        });
        let provider = build_model_hub_provider(
            "http://127.0.0.1:8080",
            "",
            &["g1".into(), "g2".into()],
        );
        let merged = merge_model_hub_into_root(Some(existing), provider).unwrap();
        let providers = merged["providers"].as_object().unwrap();
        assert!(providers.contains_key("ollama"));
        assert!(providers.contains_key(PI_PROVIDER_ID));
        assert_eq!(
            merged["providers"][PI_PROVIDER_ID]["baseUrl"],
            "http://127.0.0.1:8080/v1"
        );
        assert_eq!(
            merged["providers"][PI_PROVIDER_ID]["apiKey"],
            DEFAULT_PLACEHOLDER_KEY
        );
        assert_eq!(
            merged["providers"][PI_PROVIDER_ID]["models"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn export_roundtrip_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("models.json");
        fs::write(
            &path,
            r#"{ "providers": { "keep": { "baseUrl": "x", "api": "openai-completions", "models": [] } } }"#,
        )
        .unwrap();
        export_model_hub_to_path(
            &path,
            "http://127.0.0.1:9090",
            "sk-modelhub-test",
            &["chat".into()],
        )
        .unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let v: Value = serde_json::from_str(&text).unwrap();
        assert!(v["providers"].get("keep").is_some());
        assert_eq!(
            v["providers"][PI_PROVIDER_ID]["apiKey"],
            "sk-modelhub-test"
        );
        assert_eq!(v["providers"][PI_PROVIDER_ID]["models"][0]["id"], "chat");
    }

    #[test]
    fn export_rejects_empty_groups() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("models.json");
        let err = export_model_hub_to_path(&path, "http://127.0.0.1:8080", "", &[]).unwrap_err();
        assert!(err.to_string().contains("没有分组"));
    }
}
