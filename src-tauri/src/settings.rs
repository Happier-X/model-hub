//! 壳配置：`{config_dir}/shell.json` 持久化监听端口。

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub const DEFAULT_PORT: u16 = 8080;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    pub gateway_port: u16,
    /// 启动后是否自动检查应用更新（默认关闭；发现更新仍须用户确认安装）。
    #[serde(default)]
    pub check_update_on_startup: bool,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            gateway_port: DEFAULT_PORT,
            check_update_on_startup: false,
        }
    }
}

fn shell_path(config_dir: &Path) -> PathBuf {
    config_dir.join("shell.json")
}

fn shell_bak_path(config_dir: &Path) -> PathBuf {
    config_dir.join("shell.json.bak")
}

pub fn load_shell_config(config_dir: &Path) -> Result<ShellConfig, AppError> {
    let path = shell_path(config_dir);
    let bak = shell_bak_path(config_dir);

    let read = |p: &Path| -> Result<ShellConfig, AppError> {
        let text = fs::read_to_string(p).map_err(|source| AppError::ReadShellConfig {
            path: p.display().to_string(),
            source,
        })?;
        let cfg: ShellConfig = serde_json::from_str(&text).unwrap_or_default();
        if cfg.gateway_port == 0 {
            return Ok(ShellConfig::default());
        }
        Ok(cfg)
    };

    if path.exists() {
        match read(&path) {
            Ok(cfg) => Ok(cfg),
            Err(_) if bak.exists() => read(&bak).or_else(|_| Ok(ShellConfig::default())),
            Err(_) => Ok(ShellConfig::default()),
        }
    } else if bak.exists() {
        read(&bak).or_else(|_| Ok(ShellConfig::default()))
    } else {
        Ok(ShellConfig::default())
    }
}

pub fn save_shell_config(config_dir: &Path, config: &ShellConfig) -> Result<(), AppError> {
    if config.gateway_port == 0 {
        return Err(AppError::InvalidPort);
    }
    fs::create_dir_all(config_dir).map_err(|source| AppError::CreateDirectory {
        path: config_dir.display().to_string(),
        source,
    })?;

    let path = shell_path(config_dir);
    let bak = shell_bak_path(config_dir);
    let tmp = config_dir.join("shell.json.tmp");

    let json =
        serde_json::to_string_pretty(config).map_err(|source| AppError::SerializeShellConfig {
            path: path.display().to_string(),
            source,
        })?;

    {
        let mut file = fs::File::create(&tmp).map_err(|source| AppError::WriteShellConfig {
            path: tmp.display().to_string(),
            source,
        })?;
        file.write_all(json.as_bytes())
            .map_err(|source| AppError::WriteShellConfig {
                path: tmp.display().to_string(),
                source,
            })?;
        file.sync_all()
            .map_err(|source| AppError::WriteShellConfig {
                path: tmp.display().to_string(),
                source,
            })?;
    }

    if path.exists() {
        let _ = fs::remove_file(&bak);
        if let Err(source) = fs::rename(&path, &bak) {
            let _ = fs::remove_file(&tmp);
            return Err(AppError::WriteShellConfig {
                path: path.display().to_string(),
                source,
            });
        }
    }

    if let Err(source) = fs::rename(&tmp, &path) {
        if bak.exists() {
            let _ = fs::rename(&bak, &path);
        }
        return Err(AppError::WriteShellConfig {
            path: path.display().to_string(),
            source,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_missing_defaults_to_8080() {
        let dir = tempdir().unwrap();
        let cfg = load_shell_config(dir.path()).unwrap();
        assert_eq!(cfg.gateway_port, 8080);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        save_shell_config(
            dir.path(),
            &ShellConfig {
                gateway_port: 19090,
                check_update_on_startup: true,
            },
        )
        .unwrap();
        let cfg = load_shell_config(dir.path()).unwrap();
        assert_eq!(cfg.gateway_port, 19090);
        assert!(cfg.check_update_on_startup);
    }

    #[test]
    fn missing_check_update_field_defaults_false() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("shell.json");
        fs::write(&path, r#"{ "gateway_port": 9090 }"#).unwrap();
        let cfg = load_shell_config(dir.path()).unwrap();
        assert_eq!(cfg.gateway_port, 9090);
        assert!(!cfg.check_update_on_startup);
    }

    #[test]
    fn save_port_preserves_check_update_flag() {
        let dir = tempdir().unwrap();
        save_shell_config(
            dir.path(),
            &ShellConfig {
                gateway_port: 8080,
                check_update_on_startup: true,
            },
        )
        .unwrap();
        let mut cfg = load_shell_config(dir.path()).unwrap();
        cfg.gateway_port = 18080;
        save_shell_config(dir.path(), &cfg).unwrap();
        let cfg = load_shell_config(dir.path()).unwrap();
        assert_eq!(cfg.gateway_port, 18080);
        assert!(cfg.check_update_on_startup);
    }
}
