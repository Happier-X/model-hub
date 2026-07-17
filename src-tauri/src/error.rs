use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("无法解析应用数据目录：{0}")]
    Path(#[from] tauri::Error),
    #[error("无法创建数据目录“{path}”：{source}")]
    CreateDirectory {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvokeError {
    pub code: &'static str,
    pub message: String,
}

impl From<AppError> for InvokeError {
    fn from(error: AppError) -> Self {
        Self {
            code: "PATH_INITIALIZATION_FAILED",
            message: format!("{error}。请检查目录权限后重试。"),
        }
    }
}
