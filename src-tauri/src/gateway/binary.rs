use std::{
    env,
    path::{Path, PathBuf},
};

use crate::error::AppError;

pub const DEFAULT_WINDOWS_BINARY_NAME: &str = "octopus.exe";
pub const BINARY_ENV_OVERRIDE: &str = "MODEL_HUB_GATEWAY_BIN";

pub fn resolve_binary_path(bin_dir: &Path) -> Result<PathBuf, AppError> {
    if let Ok(override_path) = env::var(BINARY_ENV_OVERRIDE) {
        let path = PathBuf::from(override_path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(AppError::BinaryMissing {
            path: path.display().to_string(),
            hint: format!(
                "环境变量 {BINARY_ENV_OVERRIDE} 指向的文件不存在。请检查路径，或将 {DEFAULT_WINDOWS_BINARY_NAME} 放到 bin 目录。"
            ),
        });
    }

    let candidate = bin_dir.join(DEFAULT_WINDOWS_BINARY_NAME);
    if candidate.is_file() {
        return Ok(candidate);
    }

    Err(AppError::BinaryMissing {
        path: candidate.display().to_string(),
        hint: format!(
            "未找到网关程序。请按 gateway/README.md 下载 Windows 版本，放到「{path}」，或设置环境变量 {BINARY_ENV_OVERRIDE}。",
            path = candidate.display()
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::{resolve_binary_path, BINARY_ENV_OVERRIDE, DEFAULT_WINDOWS_BINARY_NAME};

    #[test]
    fn missing_binary_returns_actionable_error() {
        let dir =
            std::env::temp_dir().join(format!("model-hub-bin-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Ensure override is not interfering in this test process if set empty-ish
        let previous = std::env::var_os(BINARY_ENV_OVERRIDE);
        std::env::remove_var(BINARY_ENV_OVERRIDE);

        let err = resolve_binary_path(&dir).unwrap_err().to_string();
        assert!(err.contains(DEFAULT_WINDOWS_BINARY_NAME) || err.contains("未找到"));

        match previous {
            Some(value) => std::env::set_var(BINARY_ENV_OVERRIDE, value),
            None => std::env::remove_var(BINARY_ENV_OVERRIDE),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
