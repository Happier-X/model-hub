use std::env;

/// 壳侧网关实现选择：默认 octopus，实验路径可选 rust。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayImpl {
    Octopus,
    Rust,
}

pub const IMPL_ENV: &str = "MODEL_HUB_GATEWAY_IMPL";

impl GatewayImpl {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Octopus => "octopus",
            Self::Rust => "rust",
        }
    }
}

/// 从环境变量解析实现：仅 `rust`（大小写不敏感）启用 Rust 网关，其余一律 octopus。
pub fn resolve_gateway_impl() -> GatewayImpl {
    match env::var(IMPL_ENV) {
        Ok(value) if value.eq_ignore_ascii_case("rust") => GatewayImpl::Rust,
        _ => GatewayImpl::Octopus,
    }
}

/// 按实现构造启动参数（不含二进制路径）。供单测快照与 process 共用。
pub fn command_args(impl_kind: GatewayImpl, config_relative: &str) -> Vec<String> {
    match impl_kind {
        GatewayImpl::Octopus => vec![
            "start".to_string(),
            "--config".to_string(),
            config_relative.to_string(),
        ],
        GatewayImpl::Rust => vec!["--config".to_string(), config_relative.to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::{command_args, resolve_gateway_impl, GatewayImpl, IMPL_ENV};
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn with_impl_env<T>(value: Option<&str>, f: impl FnOnce() -> T) -> T {
        let _guard = env_lock().lock().unwrap();
        let previous = std::env::var_os(IMPL_ENV);
        match value {
            Some(v) => std::env::set_var(IMPL_ENV, v),
            None => std::env::remove_var(IMPL_ENV),
        }
        let result = f();
        match previous {
            Some(v) => std::env::set_var(IMPL_ENV, v),
            None => std::env::remove_var(IMPL_ENV),
        }
        result
    }

    #[test]
    fn default_and_unknown_resolve_to_octopus() {
        with_impl_env(None, || {
            assert_eq!(resolve_gateway_impl(), GatewayImpl::Octopus);
        });
        with_impl_env(Some("octopus"), || {
            assert_eq!(resolve_gateway_impl(), GatewayImpl::Octopus);
        });
        with_impl_env(Some("unknown"), || {
            assert_eq!(resolve_gateway_impl(), GatewayImpl::Octopus);
        });
        with_impl_env(Some(""), || {
            assert_eq!(resolve_gateway_impl(), GatewayImpl::Octopus);
        });
    }

    #[test]
    fn rust_is_case_insensitive() {
        for value in ["rust", "Rust", "RUST", "rUsT"] {
            with_impl_env(Some(value), || {
                assert_eq!(resolve_gateway_impl(), GatewayImpl::Rust);
            });
        }
    }

    #[test]
    fn octopus_command_args_keep_start_subcommand() {
        assert_eq!(
            command_args(GatewayImpl::Octopus, "data/config.json"),
            vec![
                "start".to_string(),
                "--config".to_string(),
                "data/config.json".to_string()
            ]
        );
    }

    #[test]
    fn rust_command_args_have_no_start_subcommand() {
        assert_eq!(
            command_args(GatewayImpl::Rust, "data/config.json"),
            vec!["--config".to_string(), "data/config.json".to_string()]
        );
    }

    #[test]
    fn impl_name_strings() {
        assert_eq!(GatewayImpl::Octopus.as_str(), "octopus");
        assert_eq!(GatewayImpl::Rust.as_str(), "rust");
    }
}
