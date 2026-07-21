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
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            gateway_port: DEFAULT_PORT,
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

    let json = serde_json::to_string_pretty(config).map_err(|source| {
        AppError::SerializeShellConfig {
            path: path.display().to_string(),
            source,
        }
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
        save_shell_config(dir.path(), &ShellConfig { gateway_port: 19090 }).unwrap();
        let cfg = load_shell_config(dir.path()).unwrap();
        assert_eq!(cfg.gateway_port, 19090);
    }
}
