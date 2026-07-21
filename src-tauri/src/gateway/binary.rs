use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::error::AppError;

pub const DEFAULT_GATEWAY_BINARY_NAME: &str = "model-hub-gateway.exe";
pub const BINARY_ENV_OVERRIDE: &str = "MODEL_HUB_GATEWAY_BIN";
pub const RUST_BINARY_ENV: &str = "MODEL_HUB_GATEWAY_RUST_BIN";
/// 安装包内嵌默认 Rust 网关相对 resource_dir 的路径。
pub const BUNDLED_GATEWAY_SIDECAR_RELATIVE: &str = "sidecar/model-hub-gateway.exe";

/// 解析网关二进制（仅 model-hub-gateway）。
///
/// 优先级：
/// 1. `MODEL_HUB_GATEWAY_BIN`
/// 2. `MODEL_HUB_GATEWAY_RUST_BIN`
/// 3. 安装资源 `sidecar/model-hub-gateway.exe` → 按哈希部署到 `bin_dir`
/// 4. 已有 `bin_dir/model-hub-gateway.exe`
#[allow(dead_code)] // 对外/测试便捷入口；生产路径多用 with_resource
pub fn resolve_binary_path(bin_dir: &Path) -> Result<PathBuf, AppError> {
    resolve_binary_path_with_resource(bin_dir, None)
}

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
                "环境变量 {BINARY_ENV_OVERRIDE} 指向的文件不存在。请先 `cargo build --manifest-path gateway-rust/Cargo.toml --release`，或依赖安装包内嵌网关 / 设置 {RUST_BINARY_ENV} / 将 {DEFAULT_GATEWAY_BINARY_NAME} 放到 bin_dir。"
            ),
        });
    }

    if let Ok(override_path) = env::var(RUST_BINARY_ENV) {
        let path = PathBuf::from(override_path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(AppError::BinaryMissing {
            path: path.display().to_string(),
            hint: format!(
                "环境变量 {RUST_BINARY_ENV} 指向的文件不存在。请执行 `cargo build --manifest-path gateway-rust/Cargo.toml --release`，将产物放到该路径，或依赖安装包内嵌网关 / 设置 {BINARY_ENV_OVERRIDE}。"
            ),
        });
    }

    let target = bin_dir.join(DEFAULT_GATEWAY_BINARY_NAME);

    if let Some(resource_root) = resource_dir {
        let source = resource_root.join(BUNDLED_GATEWAY_SIDECAR_RELATIVE);
        if source.is_file() {
            return ensure_bundled_deployed(&source, &target);
        }
    }

    if target.is_file() {
        return Ok(target);
    }

    Err(AppError::BinaryMissing {
        path: target.display().to_string(),
        hint: format!(
            "未找到网关 {DEFAULT_GATEWAY_BINARY_NAME}。正式安装包应自带 sidecar/{DEFAULT_GATEWAY_BINARY_NAME}；开发环境请运行 scripts/prepare-bundled-gateway-rust.ps1，将二进制复制到「{path}」，或设置 {RUST_BINARY_ENV}/{BINARY_ENV_OVERRIDE}。详见 gateway-rust/README.md。",
            path = target.display()
        ),
    })
}

/// 若目标不存在或与源哈希不同，则原子复制。
/// 返回实际应启动的路径（目标被占用时回退到按哈希命名的旁路文件）。
pub fn ensure_bundled_deployed(source: &Path, target: &Path) -> Result<PathBuf, AppError> {
    if !source.is_file() {
        return Err(AppError::BinaryMissing {
            path: source.display().to_string(),
            hint: format!(
                "安装资源中缺少网关二进制（源路径：{src}）。请重新安装应用，或开发环境运行 prepare 脚本。",
                src = source.display()
            ),
        });
    }

    let src_hash = file_sha256(source).map_err(|source_err| AppError::BinaryDeployFailed {
        path: source.display().to_string(),
        source: source_err,
    })?;

    if target.is_file() {
        if let Ok(dst_hash) = file_sha256(target) {
            if src_hash == dst_hash {
                return Ok(target.to_path_buf());
            }
        }
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|source_err| AppError::CreateDirectory {
            path: parent.display().to_string(),
            source: source_err,
        })?;
    }

    match atomic_copy(source, target) {
        Ok(()) => Ok(target.to_path_buf()),
        Err(source_err) if is_sharing_violation(&source_err) => {
            // 旧网关仍在运行时无法覆盖默认文件名：写到旁路路径并启动新副本。
            let short = hex8(&src_hash);
            let alt = target.with_file_name(format!("model-hub-gateway-{short}.exe"));
            if alt.is_file() {
                if let Ok(alt_hash) = file_sha256(&alt) {
                    if alt_hash == src_hash {
                        return Ok(alt);
                    }
                }
            }
            atomic_copy(source, &alt).map_err(|err| AppError::BinaryDeployFailed {
                path: alt.display().to_string(),
                source: err,
            })?;
            Ok(alt)
        }
        Err(source_err) => Err(AppError::BinaryDeployFailed {
            path: target.display().to_string(),
            source: source_err,
        }),
    }
}

fn atomic_copy(source: &Path, target: &Path) -> Result<(), std::io::Error> {
    let tmp = target.with_extension("exe.deploying");
    if tmp.exists() {
        let _ = fs::remove_file(&tmp);
    }
    fs::copy(source, &tmp)?;
    if target.exists() {
        fs::remove_file(target).map_err(|err| {
            let _ = fs::remove_file(&tmp);
            err
        })?;
    }
    fs::rename(&tmp, target).map_err(|err| {
        let _ = fs::remove_file(&tmp);
        err
    })?;
    Ok(())
}

fn is_sharing_violation(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::ResourceBusy
    ) || err.raw_os_error() == Some(5) // ERROR_ACCESS_DENIED
        || err.raw_os_error() == Some(32) // ERROR_SHARING_VIOLATION
}

fn hex8(hash: &[u8; 32]) -> String {
    hash[..4].iter().map(|b| format!("{b:02x}")).collect()
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
        BINARY_ENV_OVERRIDE, BUNDLED_GATEWAY_SIDECAR_RELATIVE, DEFAULT_GATEWAY_BINARY_NAME,
        RUST_BINARY_ENV,
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

    fn with_cleared_overrides<T>(f: impl FnOnce() -> T) -> T {
        let _guard = env_lock().lock().unwrap();
        let prev_bin = std::env::var_os(BINARY_ENV_OVERRIDE);
        let prev_rust = std::env::var_os(RUST_BINARY_ENV);
        std::env::remove_var(BINARY_ENV_OVERRIDE);
        std::env::remove_var(RUST_BINARY_ENV);
        let result = f();
        match prev_bin {
            Some(v) => std::env::set_var(BINARY_ENV_OVERRIDE, v),
            None => std::env::remove_var(BINARY_ENV_OVERRIDE),
        }
        match prev_rust {
            Some(v) => std::env::set_var(RUST_BINARY_ENV, v),
            None => std::env::remove_var(RUST_BINARY_ENV),
        }
        result
    }

    #[test]
    fn missing_binary_returns_actionable_error() {
        with_cleared_overrides(|| {
            let dir = unique_dir("missing");
            let err = resolve_binary_path_with_resource(&dir, None)
                .unwrap_err()
                .to_string();
            assert!(err.contains(DEFAULT_GATEWAY_BINARY_NAME) || err.contains("未找到"));
            let _ = std::fs::remove_dir_all(dir);
        });
    }

    #[test]
    fn env_override_wins() {
        with_cleared_overrides(|| {
            let root = unique_dir("override");
            let override_bin = root.join("custom-gateway.exe");
            write_fake_binary(&override_bin, b"override-bin");
            std::env::set_var(BINARY_ENV_OVERRIDE, &override_bin);
            let resolved = resolve_binary_path_with_resource(&root.join("bin"), None).unwrap();
            assert_eq!(resolved, override_bin);
            let _ = std::fs::remove_dir_all(root);
        });
    }

    #[test]
    fn deploys_bundled_sidecar_when_target_missing() {
        with_cleared_overrides(|| {
            let root = unique_dir("deploy");
            let resource = root.join("resources");
            let source = resource.join(BUNDLED_GATEWAY_SIDECAR_RELATIVE);
            write_fake_binary(&source, b"bundled-v1");
            let bin_dir = root.join("bin");
            let resolved = resolve_binary_path_with_resource(&bin_dir, Some(&resource)).unwrap();
            assert_eq!(resolved, bin_dir.join(DEFAULT_GATEWAY_BINARY_NAME));
            assert_eq!(std::fs::read(&resolved).unwrap(), b"bundled-v1");
            let _ = std::fs::remove_dir_all(root);
        });
    }

    #[test]
    fn skips_copy_when_hash_matches() {
        with_cleared_overrides(|| {
            let root = unique_dir("skip-hash");
            let resource = root.join("resources");
            let source = resource.join(BUNDLED_GATEWAY_SIDECAR_RELATIVE);
            let bin_dir = root.join("bin");
            let target = bin_dir.join(DEFAULT_GATEWAY_BINARY_NAME);
            write_fake_binary(&source, b"same-bytes");
            write_fake_binary(&target, b"same-bytes");
            let before = file_sha256(&target).unwrap();
            let path = ensure_bundled_deployed(&source, &target).unwrap();
            assert_eq!(path, target);
            assert_eq!(file_sha256(&target).unwrap(), before);
            let _ = std::fs::remove_dir_all(root);
        });
    }

    #[test]
    fn overwrites_when_hash_differs() {
        with_cleared_overrides(|| {
            let root = unique_dir("overwrite");
            let resource = root.join("resources");
            let source = resource.join(BUNDLED_GATEWAY_SIDECAR_RELATIVE);
            let bin_dir = root.join("bin");
            let target = bin_dir.join(DEFAULT_GATEWAY_BINARY_NAME);
            write_fake_binary(&source, b"new-version");
            write_fake_binary(&target, b"old-version");
            let path = ensure_bundled_deployed(&source, &target).unwrap();
            assert_eq!(path, target);
            assert_eq!(std::fs::read(&target).unwrap(), b"new-version");
            let _ = std::fs::remove_dir_all(root);
        });
    }

    #[test]
    fn uses_existing_bin_dir_when_no_resource() {
        with_cleared_overrides(|| {
            let root = unique_dir("existing-bin");
            let bin_dir = root.join("bin");
            let target = bin_dir.join(DEFAULT_GATEWAY_BINARY_NAME);
            write_fake_binary(&target, b"dev-local-bin");
            let resolved = resolve_binary_path_with_resource(&bin_dir, None).unwrap();
            assert_eq!(resolved, target);
            let _ = std::fs::remove_dir_all(root);
        });
    }

    #[test]
    fn rust_env_override_wins_over_bin_dir() {
        with_cleared_overrides(|| {
            let root = unique_dir("rust-env");
            let custom = root.join("from-env.exe");
            write_fake_binary(&custom, b"from-env");
            write_fake_binary(
                &root.join("bin").join(DEFAULT_GATEWAY_BINARY_NAME),
                b"bin-dir",
            );
            std::env::set_var(RUST_BINARY_ENV, &custom);
            let resolved = resolve_binary_path_with_resource(&root.join("bin"), None).unwrap();
            assert_eq!(resolved, custom);
            let _ = std::fs::remove_dir_all(root);
        });
    }
}
