use std::{fs, path::Path};

use serde::Serialize;

use crate::error::AppError;

pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 8080;
/// 相对 gateway 工作目录的默认配置文件（对齐 octopus `start --config`）
pub const DEFAULT_CONFIG_RELATIVE: &str = "data/config.json";
/// 相对 gateway 工作目录的 SQLite 路径
pub const DEFAULT_DATABASE_RELATIVE: &str = "data/data.db";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GatewayRuntimeConfig {
    pub host: String,
    pub port: u16,
    pub database_type: String,
    /// 相对 gateway 工作目录，或绝对路径
    pub database_path: String,
    pub log_level: String,
    /// 传给 `octopus start --config`
    pub config_relative: String,
}

impl GatewayRuntimeConfig {
    pub fn default_local(_gateway_dir: &Path) -> Self {
        Self {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            database_type: "sqlite".to_string(),
            database_path: DEFAULT_DATABASE_RELATIVE.to_string(),
            log_level: "info".to_string(),
            config_relative: DEFAULT_CONFIG_RELATIVE.to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ConfigFile {
    server: ServerConfig,
    database: DatabaseConfig,
    log: LogConfig,
}

#[derive(Debug, Serialize)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Debug, Serialize)]
struct DatabaseConfig {
    #[serde(rename = "type")]
    db_type: String,
    path: String,
}

#[derive(Debug, Serialize)]
struct LogConfig {
    level: String,
}

pub fn write_config_file(
    gateway_dir: &Path,
    config: &GatewayRuntimeConfig,
) -> Result<(), AppError> {
    let config_path = gateway_dir.join(&config.config_relative);
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|source| AppError::CreateDirectory {
            path: parent.display().to_string(),
            source,
        })?;
    }

    // 确保数据库父目录存在（相对路径相对 gateway_dir）
    let db_path = Path::new(&config.database_path);
    let db_parent_rel = db_path.parent().unwrap_or_else(|| Path::new("."));
    let db_parent = if db_path.is_absolute() {
        db_parent_rel.to_path_buf()
    } else {
        gateway_dir.join(db_parent_rel)
    };
    fs::create_dir_all(&db_parent).map_err(|source| AppError::CreateDirectory {
        path: db_parent.display().to_string(),
        source,
    })?;

    let payload = ConfigFile {
        server: ServerConfig {
            host: config.host.clone(),
            port: config.port,
        },
        database: DatabaseConfig {
            db_type: config.database_type.clone(),
            path: config.database_path.clone(),
        },
        log: LogConfig {
            level: config.log_level.clone(),
        },
    };

    let body =
        serde_json::to_string_pretty(&payload).map_err(|source| AppError::SerializeConfig {
            path: config_path.display().to_string(),
            source,
        })?;
    fs::write(&config_path, body).map_err(|source| AppError::WriteConfig {
        path: config_path.display().to_string(),
        source,
    })?;
    Ok(())
}

pub fn env_overrides(config: &GatewayRuntimeConfig) -> Vec<(String, String)> {
    vec![
        ("OCTOPUS_SERVER_HOST".into(), config.host.clone()),
        ("OCTOPUS_SERVER_PORT".into(), config.port.to_string()),
        ("OCTOPUS_DATABASE_TYPE".into(), config.database_type.clone()),
        ("OCTOPUS_DATABASE_PATH".into(), config.database_path.clone()),
        ("OCTOPUS_LOG_LEVEL".into(), config.log_level.clone()),
    ]
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        env_overrides, write_config_file, GatewayRuntimeConfig, DEFAULT_CONFIG_RELATIVE,
        DEFAULT_DATABASE_RELATIVE, DEFAULT_HOST, DEFAULT_PORT,
    };

    #[test]
    fn default_config_binds_loopback_and_relative_sqlite() {
        let dir = PathBuf::from("C:/tmp/gateway");
        let config = GatewayRuntimeConfig::default_local(&dir);
        assert_eq!(config.host, DEFAULT_HOST);
        assert_eq!(config.port, DEFAULT_PORT);
        assert_eq!(config.database_type, "sqlite");
        assert_eq!(config.database_path, DEFAULT_DATABASE_RELATIVE);
        assert_eq!(config.config_relative, DEFAULT_CONFIG_RELATIVE);
    }

    #[test]
    fn env_overrides_cover_upstream_keys() {
        let config = GatewayRuntimeConfig::default_local(&PathBuf::from("g"));
        let env = env_overrides(&config);
        let keys: Vec<_> = env.iter().map(|(k, _)| k.as_str()).collect();
        assert!(keys.contains(&"OCTOPUS_SERVER_HOST"));
        assert!(keys.contains(&"OCTOPUS_SERVER_PORT"));
        assert!(keys.contains(&"OCTOPUS_DATABASE_PATH"));
    }

    #[test]
    fn writes_config_json_under_data_dir() {
        let dir =
            std::env::temp_dir().join(format!("model-hub-gateway-config-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let config = GatewayRuntimeConfig::default_local(&dir);
        write_config_file(&dir, &config).unwrap();
        let text = std::fs::read_to_string(dir.join("data/config.json")).unwrap();
        assert!(text.contains("127.0.0.1"));
        assert!(text.contains("8080"));
        assert!(text.contains("sqlite"));
        assert!(text.contains("data/data.db"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
