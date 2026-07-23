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
    let home = dirs_home()
        .ok_or_else(|| AppError::Business("无法定位用户主目录，无法写入 Pi Agent 配置".into()))?;
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
    let text = fs::read_to_string(path).map_err(|source| {
        AppError::Business(format!("读取 Pi 配置失败（{}）：{source}", path.display()))
    })?;
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
    let pretty = serde_json::to_string_pretty(root)
        .map_err(|e| AppError::Business(format!("序列化 Pi models.json 失败：{e}")))?;
    // 直接覆盖写入即可（本机配置文件，体积小）。
    fs::write(path, format!("{pretty}\n")).map_err(|source| {
        AppError::Business(format!(
            "写入 Pi models.json 失败（{}）：{source}",
            path.display()
        ))
    })?;
    Ok(())
}

fn empty_root() -> Value {
    json!({ "providers": {} })
}

/// 在 `providers.model-hub` 中按模型 id upsert 一条（id/name = 分组名）；
/// 固定占位 apiKey，并刷新 baseUrl / api。保留其它 models 与其它 providers。
/// 返回写入后 model-hub.models 长度。
pub fn upsert_model_hub_group(
    path: &Path,
    base_url: &str,
    group_name: &str,
) -> Result<usize, AppError> {
    let name = group_name.trim();
    if name.is_empty() {
        return Err(AppError::Business("分组名不能为空".into()));
    }
    let normalized = normalize_openai_base_url(base_url);
    if normalized.is_empty() {
        return Err(AppError::Business(
            "代理 Base URL 无效。请先启动代理或检查端口配置。".into(),
        ));
    }

    let existing = read_models_json(path)?;
    let mut root = match existing {
        None => empty_root(),
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

    let providers_obj = providers
        .as_object_mut()
        .ok_or_else(|| AppError::Business("Pi models.json 的 providers 必须是对象".into()))?;

    let provider_entry = providers_obj
        .entry(PI_PROVIDER_ID.to_string())
        .or_insert_with(|| {
            json!({
                "baseUrl": normalized.clone(),
                "api": "openai-completions",
                "apiKey": DEFAULT_PLACEHOLDER_KEY,
                "models": [],
            })
        });

    let provider_obj = provider_entry.as_object_mut().ok_or_else(|| {
        AppError::Business("Pi models.json 的 providers.model-hub 必须是对象".into())
    })?;

    provider_obj.insert("baseUrl".into(), Value::String(normalized));
    provider_obj.insert("api".into(), Value::String("openai-completions".into()));
    provider_obj.insert(
        "apiKey".into(),
        Value::String(DEFAULT_PLACEHOLDER_KEY.into()),
    );

    let models_value = provider_obj
        .entry("models".to_string())
        .or_insert_with(|| Value::Array(vec![]));

    let models = models_value.as_array_mut().ok_or_else(|| {
        AppError::Business("Pi models.json 的 model-hub.models 必须是数组".into())
    })?;

    let new_model = json!({
        "id": name,
        "name": name,
    });

    if let Some(pos) = models.iter().position(|m| {
        m.get("id")
            .and_then(|v| v.as_str())
            .map(|id| id == name)
            .unwrap_or(false)
    }) {
        models[pos] = new_model;
    } else {
        models.push(new_model);
    }

    let model_count = models.len();
    write_models_json(path, &root)?;
    Ok(model_count)
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
    fn upsert_creates_provider_and_model() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("models.json");
        let count = upsert_model_hub_group(&path, "http://127.0.0.1:8080", "coding").unwrap();
        assert_eq!(count, 1);
        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            v["providers"][PI_PROVIDER_ID]["baseUrl"],
            "http://127.0.0.1:8080/v1"
        );
        assert_eq!(
            v["providers"][PI_PROVIDER_ID]["apiKey"],
            DEFAULT_PLACEHOLDER_KEY
        );
        assert_eq!(v["providers"][PI_PROVIDER_ID]["api"], "openai-completions");
        assert_eq!(v["providers"][PI_PROVIDER_ID]["models"][0]["id"], "coding");
        assert_eq!(
            v["providers"][PI_PROVIDER_ID]["models"][0]["name"],
            "coding"
        );
    }

    #[test]
    fn upsert_same_id_replaces_and_refreshes_base_url() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("models.json");
        upsert_model_hub_group(&path, "http://127.0.0.1:8080", "coding").unwrap();
        let count = upsert_model_hub_group(&path, "http://127.0.0.1:9090", "coding").unwrap();
        assert_eq!(count, 1);
        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            v["providers"][PI_PROVIDER_ID]["baseUrl"],
            "http://127.0.0.1:9090/v1"
        );
        assert_eq!(
            v["providers"][PI_PROVIDER_ID]["models"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(v["providers"][PI_PROVIDER_ID]["models"][0]["id"], "coding");
    }

    #[test]
    fn upsert_preserves_other_models_and_providers() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("models.json");
        fs::write(
            &path,
            r#"{
              "providers": {
                "ollama": {
                  "baseUrl": "http://localhost:11434/v1",
                  "api": "openai-completions",
                  "apiKey": "ollama",
                  "models": [{"id": "llama"}]
                },
                "model-hub": {
                  "baseUrl": "http://127.0.0.1:8080/v1",
                  "api": "openai-completions",
                  "apiKey": "sk-old-real-key",
                  "models": [{"id": "chat", "name": "chat"}]
                }
              }
            }"#,
        )
        .unwrap();

        let count = upsert_model_hub_group(&path, "http://127.0.0.1:8080", "coding").unwrap();
        assert_eq!(count, 2);

        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(v["providers"].get("ollama").is_some());
        assert_eq!(v["providers"]["ollama"]["models"][0]["id"], "llama");

        let models = v["providers"][PI_PROVIDER_ID]["models"].as_array().unwrap();
        assert_eq!(models.len(), 2);
        let ids: Vec<&str> = models
            .iter()
            .filter_map(|m| m.get("id").and_then(|x| x.as_str()))
            .collect();
        assert!(ids.contains(&"chat"));
        assert!(ids.contains(&"coding"));
        // 旧真实 Key 应被改回占位
        assert_eq!(
            v["providers"][PI_PROVIDER_ID]["apiKey"],
            DEFAULT_PLACEHOLDER_KEY
        );
    }

    #[test]
    fn upsert_rejects_empty_name() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("models.json");
        let err = upsert_model_hub_group(&path, "http://127.0.0.1:8080", "  ").unwrap_err();
        assert!(err.to_string().contains("分组名不能为空"));
    }

    #[test]
    fn upsert_rejects_empty_base_url() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("models.json");
        let err = upsert_model_hub_group(&path, "   ", "coding").unwrap_err();
        assert!(err.to_string().contains("Base URL"));
    }
}
