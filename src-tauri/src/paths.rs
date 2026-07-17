use std::{fs, path::Path};

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::error::{AppError, InvokeError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppPaths {
    pub app_data_dir: String,
    pub config_dir: String,
    pub gateway_dir: String,
    pub bin_dir: String,
}

fn create_directory(path: &Path) -> Result<(), AppError> {
    fs::create_dir_all(path).map_err(|source| AppError::CreateDirectory {
        path: path.display().to_string(),
        source,
    })
}

fn build_paths(app_data_dir: &Path) -> AppPaths {
    AppPaths {
        app_data_dir: app_data_dir.display().to_string(),
        config_dir: app_data_dir.join("config").display().to_string(),
        gateway_dir: app_data_dir.join("gateway").display().to_string(),
        bin_dir: app_data_dir.join("bin").display().to_string(),
    }
}

pub fn resolve_paths(app: &AppHandle) -> Result<AppPaths, AppError> {
    let app_data_dir = app.path().app_data_dir()?;
    let paths = build_paths(&app_data_dir);
    let config_dir = app_data_dir.join("config");
    let gateway_dir = app_data_dir.join("gateway");
    let bin_dir = app_data_dir.join("bin");

    for path in [&app_data_dir, &config_dir, &gateway_dir, &bin_dir] {
        create_directory(path)?;
    }

    Ok(paths)
}

#[tauri::command]
pub fn get_paths(app: AppHandle) -> Result<AppPaths, InvokeError> {
    resolve_paths(&app).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{build_paths, AppPaths};

    #[test]
    fn builds_the_documented_subdirectory_contract() {
        let root = PathBuf::from("model-hub-test-data");

        assert_eq!(
            build_paths(&root),
            AppPaths {
                app_data_dir: root.display().to_string(),
                config_dir: root.join("config").display().to_string(),
                gateway_dir: root.join("gateway").display().to_string(),
                bin_dir: root.join("bin").display().to_string(),
            }
        );
    }
}
