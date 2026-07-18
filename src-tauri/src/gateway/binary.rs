use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::error::AppError;

pub const DEFAULT_WINDOWS_BINARY_NAME: &str = "octopus.exe";
pub const BINARY_ENV_OVERRIDE: &str = "MODEL_HUB_GATEWAY_BIN";
/// 安装包内嵌侧车相对 resource_dir 的路径。
pub const BUNDLED_SIDECAR_RELATIVE: &str = "sidecar/octopus.exe";
/// 产品文档用的内置网关版本钉扎。
pub const BUNDLED_GATEWAY_VERSION: &str = "v0.9.28";

/// 解析侧车二进制：开发覆盖 → app data 已部署副本 → 资源内嵌并部署。
pub fn resolve_binary_path(bin_dir: &Path) -> Result<PathBuf, AppError> {
    resolve_binary_path_with_resource(bin_dir, None)
}

/// 带可选 resource_dir 的解析（安装态可从内嵌资源部署）。
pub fn resolve_binary_path_with_resource(
    bin_dir: &Path,
    resource_dir: Option<&Path>,
) -> Result<PathBuf, AppError> {
    if let Ok(override_path) = env::var(BINARY_ENV_OVERRIDE) {
        let path = PathBuf::from(override_path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(AppError::BinaryMissing {
            path: path.display().to_string(),
            hint: format!(
                "环境变量 {BINARY_ENV_OVERRIDE} 指向的文件不存在。请检查路径，或依赖安装包内置网关 {BUNDLED_GATEWAY_VERSION}。"
            ),
        });
    }

    let target = bin_dir.join(DEFAULT_WINDOWS_BINARY_NAME);

    if let Some(resource_root) = resource_dir {
        let source = resource_root.join(BUNDLED_SIDECAR_RELATIVE);
        if source.is_file() {
            ensure_bundled_deployed(&source, &target)?;
            return Ok(target);
        }
    }

    if target.is_file() {
        return Ok(target);
    }

    Err(AppError::BinaryMissing {
        path: target.display().to_string(),
        hint: format!(
            "未找到内置网关 {BUNDLED_GATEWAY_VERSION}。正式安装包应自带侧车；开发环境请运行 scripts/prepare-bundled-octopus.ps1，将 {DEFAULT_WINDOWS_BINARY_NAME} 放到「{path}」，或设置 {BINARY_ENV_OVERRIDE}。详见 gateway/README.md。",
            path = target.display()
        ),
    })
}

/// 若目标不存在或与源哈希不同，则原子复制。
pub fn ensure_bundled_deployed(source: &Path, target: &Path) -> Result<(), AppError> {
    if !source.is_file() {
        return Err(AppError::BinaryMissing {
            path: source.display().to_string(),
            hint: format!(
                "安装资源中缺少内置网关 {BUNDLED_GATEWAY_VERSION}（期望路径 sidecar/octopus.exe）。请重新安装应用。"
            ),
        });
    }

    if target.is_file() {
        match (file_sha256(source), file_sha256(target)) {
            (Ok(src_hash), Ok(dst_hash)) if src_hash == dst_hash => return Ok(()),
            _ => {
                // 哈希不同或读取失败：尝试覆盖
            }
        }
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|source_err| AppError::CreateDirectory {
            path: parent.display().to_string(),
            source: source_err,
        })?;
    }

    let tmp = target.with_extension("exe.deploying");
    if tmp.exists() {
        let _ = fs::remove_file(&tmp);
    }

    fs::copy(source, &tmp).map_err(|source_err| AppError::BinaryDeployFailed {
        path: target.display().to_string(),
        source: source_err,
    })?;

    // Windows 上若目标被占用，rename 可能失败；给出可行动提示。
    if target.exists() {
        if let Err(source_err) = fs::remove_file(target) {
            let _ = fs::remove_file(&tmp);
            return Err(AppError::BinaryDeployFailed {
                path: target.display().to_string(),
                source: source_err,
            });
        }
    }

    fs::rename(&tmp, target).map_err(|source_err| {
        let _ = fs::remove_file(&tmp);
        AppError::BinaryDeployFailed {
            path: target.display().to_string(),
            source: source_err,
        }
    })?;

    Ok(())
}

fn file_sha256(path: &Path) -> Result<[u8; 32], std::io::Error> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let digest = hasher.finalize();
    let mut out = [0_u8; 32];
    out.copy_from_slice(&digest);
    Ok(out)
}

/// 仅供测试：写入固定字节并返回路径。
#[cfg(test)]
pub fn write_fake_binary(path: &Path, content: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_bundled_deployed, file_sha256, resolve_binary_path_with_resource, write_fake_binary,
        BINARY_ENV_OVERRIDE, BUNDLED_SIDECAR_RELATIVE, DEFAULT_WINDOWS_BINARY_NAME,
    };
    use std::{
        path::PathBuf,
        sync::{Mutex, OnceLock},
    };

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn unique_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "model-hub-bin-{}-{}-{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn with_cleared_override<T>(f: impl FnOnce() -> T) -> T {
        // 环境变量属于进程全局状态，串行化相关单测，避免并发互相污染。
        let _guard = env_lock().lock().unwrap();
        let previous = std::env::var_os(BINARY_ENV_OVERRIDE);
        std::env::remove_var(BINARY_ENV_OVERRIDE);
        let result = f();
        match previous {
            Some(value) => std::env::set_var(BINARY_ENV_OVERRIDE, value),
            None => std::env::remove_var(BINARY_ENV_OVERRIDE),
        }
        result
    }

    #[test]
    fn missing_binary_returns_actionable_error() {
        with_cleared_override(|| {
            let dir = unique_dir("missing");
            let err = resolve_binary_path_with_resource(&dir, None)
                .unwrap_err()
                .to_string();
            assert!(err.contains(DEFAULT_WINDOWS_BINARY_NAME) || err.contains("未找到"));
            let _ = std::fs::remove_dir_all(&dir);
        });
    }

    #[test]
    fn prefers_env_override_when_file_exists() {
        with_cleared_override(|| {
            let dir = unique_dir("override");
            let override_bin = dir.join("custom-gateway.exe");
            write_fake_binary(&override_bin, b"override-bin");
            std::env::set_var(BINARY_ENV_OVERRIDE, &override_bin);

            let resolved = resolve_binary_path_with_resource(&dir.join("bin"), None).unwrap();
            assert_eq!(resolved, override_bin);

            std::env::remove_var(BINARY_ENV_OVERRIDE);
            let _ = std::fs::remove_dir_all(&dir);
        });
    }

    #[test]
    fn deploys_bundled_sidecar_when_target_missing() {
        with_cleared_override(|| {
            let root = unique_dir("deploy");
            let resource = root.join("resources");
            let bin_dir = root.join("bin");
            let source = resource.join(BUNDLED_SIDECAR_RELATIVE);
            write_fake_binary(&source, b"bundled-v1");

            let resolved = resolve_binary_path_with_resource(&bin_dir, Some(&resource)).unwrap();
            let target = bin_dir.join(DEFAULT_WINDOWS_BINARY_NAME);
            assert_eq!(resolved, target);
            assert_eq!(std::fs::read(&target).unwrap(), b"bundled-v1");

            let _ = std::fs::remove_dir_all(&root);
        });
    }

    #[test]
    fn skips_copy_when_hash_matches() {
        with_cleared_override(|| {
            let root = unique_dir("hash-match");
            let resource = root.join("resources");
            let bin_dir = root.join("bin");
            let source = resource.join(BUNDLED_SIDECAR_RELATIVE);
            let target = bin_dir.join(DEFAULT_WINDOWS_BINARY_NAME);
            write_fake_binary(&source, b"same-bytes");
            write_fake_binary(&target, b"same-bytes");

            let before = std::fs::metadata(&target).unwrap().modified().unwrap();
            ensure_bundled_deployed(&source, &target).unwrap();
            let after = std::fs::metadata(&target).unwrap().modified().unwrap();
            assert_eq!(before, after);
            assert_eq!(file_sha256(&source).unwrap(), file_sha256(&target).unwrap());

            let _ = std::fs::remove_dir_all(&root);
        });
    }

    #[test]
    fn overwrites_when_hash_differs() {
        with_cleared_override(|| {
            let root = unique_dir("hash-diff");
            let resource = root.join("resources");
            let bin_dir = root.join("bin");
            let source = resource.join(BUNDLED_SIDECAR_RELATIVE);
            let target = bin_dir.join(DEFAULT_WINDOWS_BINARY_NAME);
            write_fake_binary(&source, b"new-version");
            write_fake_binary(&target, b"old-version");

            ensure_bundled_deployed(&source, &target).unwrap();
            assert_eq!(std::fs::read(&target).unwrap(), b"new-version");

            let _ = std::fs::remove_dir_all(&root);
        });
    }

    #[test]
    fn falls_back_to_bin_dir_when_resource_absent() {
        with_cleared_override(|| {
            let root = unique_dir("bin-fallback");
            let bin_dir = root.join("bin");
            let target = bin_dir.join(DEFAULT_WINDOWS_BINARY_NAME);
            write_fake_binary(&target, b"dev-local-bin");

            let resolved = resolve_binary_path_with_resource(&bin_dir, None).unwrap();
            assert_eq!(resolved, target);
            assert_eq!(std::fs::read(&resolved).unwrap(), b"dev-local-bin");

            let _ = std::fs::remove_dir_all(&root);
        });
    }
}
