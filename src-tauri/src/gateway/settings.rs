use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use super::config::DEFAULT_PORT;
use crate::error::AppError;

const SHELL_CONFIG_FILE: &str = "shell.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShellConfig {
    pub gateway_port: u16,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            gateway_port: DEFAULT_PORT,
        }
    }
}

/// 读取壳配置。文件缺失、JSON 损坏或端口为 0 时安全回退默认值，
/// 以保证应用仍可打开并可通过下一次保存恢复配置。
pub fn load_shell_config(config_dir: &Path) -> Result<ShellConfig, AppError> {
    let path = config_dir.join(SHELL_CONFIG_FILE);
    let body = match fs::read_to_string(&path) {
        Ok(body) => body,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            // 替换中断或恢复失败时，旧配置仍保存在同目录备份中。
            let backup_path = path.with_extension("json.bak");
            match fs::read_to_string(&backup_path) {
                Ok(body) => body,
                Err(backup_source) if backup_source.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(ShellConfig::default());
                }
                Err(backup_source) => {
                    return Err(AppError::ReadShellConfig {
                        path: backup_path.display().to_string(),
                        source: backup_source,
                    });
                }
            }
        }
        Err(source) => {
            return Err(AppError::ReadShellConfig {
                path: path.display().to_string(),
                source,
            });
        }
    };

    let config = match serde_json::from_str::<ShellConfig>(&body) {
        Ok(config) if config.gateway_port != 0 => config,
        Ok(_) | Err(_) => ShellConfig::default(),
    };
    Ok(config)
}

fn replace_with_backup(
    path: &Path,
    temporary_path: &Path,
    replace: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let backup_path = path.with_extension("json.bak");
    let had_existing = path.exists();
    if had_existing {
        let _ = fs::remove_file(&backup_path);
        fs::rename(path, &backup_path)?;
    }

    if let Err(source) = replace(temporary_path, path) {
        let _ = fs::remove_file(temporary_path);
        if had_existing {
            fs::rename(&backup_path, path)?;
        }
        return Err(source);
    }
    if had_existing {
        let _ = fs::remove_file(backup_path);
    }
    Ok(())
}

/// 先写同目录临时文件再以备份恢复保护替换正式文件，避免中断或替换失败损坏旧配置。
pub fn save_shell_config(config_dir: &Path, config: &ShellConfig) -> Result<(), AppError> {
    if config.gateway_port == 0 {
        return Err(AppError::InvalidPort);
    }
    fs::create_dir_all(config_dir).map_err(|source| AppError::CreateDirectory {
        path: config_dir.display().to_string(),
        source,
    })?;

    let path = config_dir.join(SHELL_CONFIG_FILE);
    let temporary_path = config_dir.join(format!("{SHELL_CONFIG_FILE}.tmp"));
    let body =
        serde_json::to_string_pretty(config).map_err(|source| AppError::SerializeShellConfig {
            path: path.display().to_string(),
            source,
        })?;
    fs::write(&temporary_path, body).map_err(|source| AppError::WriteShellConfig {
        path: temporary_path.display().to_string(),
        source,
    })?;

    // Windows 的 rename 不能覆盖目标，不能先删除正式配置，否则替换失败会丢失旧配置。
    replace_with_backup(&path, &temporary_path, |from, to| fs::rename(from, to)).map_err(|source| {
        AppError::WriteShellConfig {
            path: path.display().to_string(),
            source,
        }
    })
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{load_shell_config, replace_with_backup, save_shell_config, ShellConfig};
    use crate::gateway::config::DEFAULT_PORT;

    fn temp_dir(case: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "model-hub-shell-config-{case}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn missing_config_uses_default_port() {
        let dir = temp_dir("missing");
        assert_eq!(load_shell_config(&dir).unwrap().gateway_port, DEFAULT_PORT);
    }

    #[test]
    fn saves_and_loads_port_and_can_replace_existing_file() {
        let dir = temp_dir("roundtrip");
        save_shell_config(
            &dir,
            &ShellConfig {
                gateway_port: 18080,
            },
        )
        .unwrap();
        save_shell_config(
            &dir,
            &ShellConfig {
                gateway_port: 19090,
            },
        )
        .unwrap();
        assert_eq!(load_shell_config(&dir).unwrap().gateway_port, 19090);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn replacement_failure_restores_readable_old_config() {
        let dir = temp_dir("restore");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("shell.json");
        let temporary_path = dir.join("shell.json.tmp");
        std::fs::write(&path, r#"{"gateway_port":18080}"#).unwrap();
        std::fs::write(&temporary_path, r#"{"gateway_port":19090}"#).unwrap();

        let result = replace_with_backup(&path, &temporary_path, |_, _| {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "模拟替换失败",
            ))
        });

        assert!(result.is_err());
        assert_eq!(load_shell_config(&dir).unwrap().gateway_port, 18080);
        assert!(!temporary_path.exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_primary_config_recovers_from_backup() {
        let dir = temp_dir("backup-load");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("shell.json.bak"), r#"{"gateway_port":18080}"#).unwrap();
        assert_eq!(load_shell_config(&dir).unwrap().gateway_port, 18080);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_zero_port_before_writing() {
        let dir = temp_dir("invalid-port");
        assert!(matches!(
            save_shell_config(&dir, &ShellConfig { gateway_port: 0 }),
            Err(crate::error::AppError::InvalidPort)
        ));
        assert!(!dir.exists());
    }

    #[test]
    fn damaged_or_invalid_config_safely_falls_back() {
        let dir = temp_dir("damaged");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("shell.json"), "not-json").unwrap();
        assert_eq!(load_shell_config(&dir).unwrap().gateway_port, DEFAULT_PORT);
        std::fs::write(dir.join("shell.json"), r#"{"gateway_port":0}"#).unwrap();
        assert_eq!(load_shell_config(&dir).unwrap().gateway_port, DEFAULT_PORT);
        std::fs::write(dir.join("shell.json"), r#"{"gateway_port":65536}"#).unwrap();
        assert_eq!(load_shell_config(&dir).unwrap().gateway_port, DEFAULT_PORT);
        let _ = std::fs::remove_dir_all(dir);
    }
}
