//! Model Hub Rust 原生网关实验实现。
//!
//! 提供配置、HTTP 路由、SQLite 持久化、渠道/分组 CRUD、请求日志 list/clear、
//! 管理 JWT / 客户端 API Key 鉴权、非流式 Chat 转发、SSE 流式透明代理与优雅退出 API。
//! **本 crate 不能替代当前发布版内嵌的 octopus 侧车。**

pub mod apikey;
pub mod auth;
pub mod channel;
pub mod config;
pub mod db;
pub mod error;
pub mod group;
pub mod http;
pub mod log;
pub mod response;
pub mod router;
pub mod routes;
pub mod server;
pub mod upstream;

pub use config::{AuthConfig, GatewayConfig, DEFAULT_CONFIG_PATH, DEFAULT_HOST, DEFAULT_PORT};
pub use error::GatewayError;
pub use http::{build_router, AppState, ErrorBody, HealthResponse};
pub use server::{bind_listener, run, run_with_shutdown, run_with_state, serve, shutdown_signal};
