use std::fs;
use std::net::IpAddr;
use std::path::Path;

use serde::Deserialize;

use crate::error::GatewayError;

pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 8080;
pub const DEFAULT_CONFIG_PATH: &str = "data/config.json";

/// 网关进程配置（当前骨架仅消费 server）。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GatewayConfig {
    pub server: ServerConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub log: LogConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_database_type", rename = "type")]
    pub db_type: String,
    #[serde(default = "default_database_path")]
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

fn default_database_type() -> String {
    "sqlite".to_string()
}

fn default_database_path() -> String {
    "data/data.db".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            db_type: default_database_type(),
            path: default_database_path(),
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: DEFAULT_HOST.to_string(),
                port: DEFAULT_PORT,
            },
            database: DatabaseConfig::default(),
            log: LogConfig::default(),
        }
    }
}

impl GatewayConfig {
    /// 从 JSON 文件加载并校验配置。文件缺失时失败，不静默回退默认值。
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, GatewayError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(GatewayError::ConfigNotFound {
                path: path.to_path_buf(),
            });
        }

        let raw = fs::read_to_string(path).map_err(|source| GatewayError::ConfigRead {
            path: path.to_path_buf(),
            source,
        })?;

        let config: GatewayConfig =
            serde_json::from_str(&raw).map_err(|source| GatewayError::ConfigParse {
                path: path.to_path_buf(),
                source,
            })?;
        config.validate()?;
        Ok(config)
    }

    /// 校验 host/port；不静默改写配置值。
    pub fn validate(&self) -> Result<(), GatewayError> {
        let host = self.server.host.trim();
        if host.is_empty() {
            return Err(GatewayError::invalid_config(
                "server.host 不能为空；请配置本机 IP（默认 127.0.0.1）",
            ));
        }

        if self.server.port == 0 {
            return Err(GatewayError::invalid_config(
                "server.port 不能为 0；请配置 1..=65535 的端口",
            ));
        }

        let ip: IpAddr = host.parse().map_err(|_| {
            GatewayError::invalid_config(format!(
                "server.host 不是合法 IP 地址: {host}（当前骨架不解析域名）"
            ))
        })?;

        if ip.is_unspecified() {
            tracing::warn!(
                host = %host,
                "server.host 为全接口绑定（0.0.0.0/::），存在公网暴露风险；默认配置应使用 127.0.0.1"
            );
        }

        Ok(())
    }

    pub fn socket_addr(&self) -> Result<std::net::SocketAddr, GatewayError> {
        self.validate()?;
        let host = self.server.host.trim();
        let ip: IpAddr = host.parse().map_err(|_| {
            GatewayError::invalid_config(format!("server.host 不是合法 IP 地址: {host}"))
        })?;
        Ok(std::net::SocketAddr::from((ip, self.server.port)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_config(body: &str) -> (tempfile_path::TempPath, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "model-hub-gateway-config-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(body.as_bytes()).unwrap();
        (tempfile_path::TempPath { dir }, path)
    }

    mod tempfile_path {
        use std::path::PathBuf;

        pub struct TempPath {
            pub dir: PathBuf,
        }

        impl Drop for TempPath {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.dir);
            }
        }
    }

    #[test]
    fn default_config_uses_loopback_8080() {
        let config = GatewayConfig::default();
        assert_eq!(config.server.host, DEFAULT_HOST);
        assert_eq!(config.server.port, DEFAULT_PORT);
        assert_eq!(config.database.db_type, "sqlite");
        assert_eq!(config.database.path, "data/data.db");
        assert_eq!(config.log.level, "info");
        config.validate().unwrap();
    }

    #[test]
    fn load_valid_config() {
        let body = r#"{
            "server": { "host": "127.0.0.1", "port": 19080 },
            "database": { "type": "sqlite", "path": "data/data.db" },
            "log": { "level": "info" }
        }"#;
        let (_guard, path) = write_temp_config(body);
        let config = GatewayConfig::load_from_path(&path).unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 19080);
        assert_eq!(config.database.db_type, "sqlite");
    }

    #[test]
    fn missing_file_fails() {
        let path = std::env::temp_dir().join(format!(
            "model-hub-gateway-missing-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let err = GatewayConfig::load_from_path(&path).unwrap_err();
        assert!(matches!(err, GatewayError::ConfigNotFound { .. }));
    }

    #[test]
    fn corrupt_json_fails() {
        let (_guard, path) = write_temp_config("{ not-json");
        let err = GatewayConfig::load_from_path(&path).unwrap_err();
        assert!(matches!(err, GatewayError::ConfigParse { .. }));
    }

    #[test]
    fn empty_host_fails() {
        let config = GatewayConfig {
            server: ServerConfig {
                host: "   ".into(),
                port: 8080,
            },
            ..GatewayConfig::default()
        };
        let err = config.validate().unwrap_err();
        assert!(matches!(err, GatewayError::ConfigInvalid { .. }));
    }

    #[test]
    fn zero_port_fails() {
        let config = GatewayConfig {
            server: ServerConfig {
                host: "127.0.0.1".into(),
                port: 0,
            },
            ..GatewayConfig::default()
        };
        let err = config.validate().unwrap_err();
        assert!(matches!(err, GatewayError::ConfigInvalid { .. }));
    }

    #[test]
    fn non_ip_host_fails() {
        let config = GatewayConfig {
            server: ServerConfig {
                host: "localhost".into(),
                port: 8080,
            },
            ..GatewayConfig::default()
        };
        let err = config.validate().unwrap_err();
        assert!(matches!(err, GatewayError::ConfigInvalid { .. }));
    }

    #[test]
    fn unspecified_host_is_allowed_but_valid() {
        let config = GatewayConfig {
            server: ServerConfig {
                host: "0.0.0.0".into(),
                port: 8080,
            },
            ..GatewayConfig::default()
        };
        // 不静默改写；显式配置可校验通过
        config.validate().unwrap();
    }
}
