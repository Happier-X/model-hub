//! Model Hub Rust 原生网关实验骨架。
//!
//! 提供可测试的配置、HTTP 路由与优雅退出 API，供后续鉴权/业务模块扩展。
//! **本 crate 不能替代当前发布版内嵌的 octopus 侧车。**

pub mod config;
pub mod error;
pub mod http;
pub mod server;

pub use config::{GatewayConfig, DEFAULT_CONFIG_PATH, DEFAULT_HOST, DEFAULT_PORT};
pub use error::GatewayError;
pub use http::{build_router, AppState, ErrorBody, HealthResponse};
pub use server::{bind_listener, run, run_with_shutdown, run_with_state, serve, shutdown_signal};
