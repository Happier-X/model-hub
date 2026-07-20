use std::future::Future;
use std::io;
use std::net::SocketAddr;

use axum::Router;
use tokio::net::TcpListener;

use crate::config::GatewayConfig;
use crate::error::GatewayError;
use crate::http::{build_router, AppState};

/// 使用预绑定 listener 与外部 shutdown 信号提供 HTTP 服务。
pub async fn serve(
    listener: TcpListener,
    app: Router,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> io::Result<()> {
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
}

/// 按配置绑定并运行网关，直到 shutdown 完成。
pub async fn run_with_shutdown(
    config: GatewayConfig,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), GatewayError> {
    run_with_state(config, AppState::default(), shutdown).await
}

/// 按配置和共享状态绑定并运行，供后续业务模块注入依赖。
pub async fn run_with_state(
    config: GatewayConfig,
    state: AppState,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), GatewayError> {
    let addr = config.socket_addr()?;
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|source| GatewayError::Bind {
            addr: addr.to_string(),
            source,
        })?;

    let local = listener.local_addr().map_err(|source| GatewayError::Bind {
        addr: addr.to_string(),
        source,
    })?;

    tracing::info!(%local, "网关 HTTP 服务已监听");

    let app = build_router(state);
    serve(listener, app, shutdown)
        .await
        .map_err(|source| GatewayError::Serve { source })?;

    tracing::info!("网关 HTTP 服务已停止");
    Ok(())
}

/// 按配置绑定并运行网关，直到 Ctrl-C。
pub async fn run(config: GatewayConfig) -> Result<(), GatewayError> {
    run_with_shutdown(config, shutdown_signal()).await
}

/// 等待进程级退出信号。
#[cfg(not(windows))]
pub async fn shutdown_signal() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => tracing::info!("收到 Ctrl-C，开始优雅退出"),
        Err(err) => tracing::error!(error = %err, "监听 Ctrl-C 失败"),
    }
}

/// Windows 同时处理 Ctrl-C 与 Ctrl-Break。
#[cfg(windows)]
pub async fn shutdown_signal() {
    use tokio::signal::windows::{ctrl_break, ctrl_c};

    let mut ctrl_c_stream = match ctrl_c() {
        Ok(stream) => stream,
        Err(err) => {
            tracing::error!(error = %err, "注册 Ctrl-C 监听失败");
            return;
        }
    };
    let mut ctrl_break_stream = match ctrl_break() {
        Ok(stream) => stream,
        Err(err) => {
            tracing::error!(error = %err, "注册 Ctrl-Break 监听失败");
            return;
        }
    };

    tokio::select! {
        _ = ctrl_c_stream.recv() => tracing::info!("收到 Ctrl-C，开始优雅退出"),
        _ = ctrl_break_stream.recv() => tracing::info!("收到 Ctrl-Break，开始优雅退出"),
    }
}

/// 绑定指定地址，供集成测试获取随机端口。
pub async fn bind_listener(addr: SocketAddr) -> Result<TcpListener, GatewayError> {
    TcpListener::bind(addr)
        .await
        .map_err(|source| GatewayError::Bind {
            addr: addr.to_string(),
            source,
        })
}
