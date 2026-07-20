use std::net::SocketAddr;
use std::time::Duration;

use model_hub_gateway::{
    bind_listener, build_router, serve, AppState, GatewayConfig, DEFAULT_HOST,
};
use tokio::sync::oneshot;

async fn start_test_server() -> (SocketAddr, oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
    let listener = bind_listener(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind random loopback port");
    let addr = listener.local_addr().expect("local_addr");
    assert_eq!(addr.ip().to_string(), DEFAULT_HOST);
    // 请求端口 0，由操作系统分配；不会主动占用用户默认的 8080 实例。
    assert_ne!(addr.port(), 0);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let app = build_router(AppState::default());
    let handle = tokio::spawn(async move {
        serve(listener, app, async move {
            let _ = shutdown_rx.await;
        })
        .await
        .expect("serve");
    });

    // 短暂等待 accept 就绪
    tokio::time::sleep(Duration::from_millis(50)).await;
    (addr, shutdown_tx, handle)
}

async fn get_json(url: &str) -> (u16, serde_json::Value) {
    let client = reqwest_like_get(url).await;
    client
}

/// 使用 tokio TcpStream + 极简 HTTP/1.1 客户端，避免为集成测试额外钉依赖。
async fn reqwest_like_get(url: &str) -> (u16, serde_json::Value) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let url = url.strip_prefix("http://").expect("http url");
    let (host_port, path) = url
        .split_once('/')
        .map(|(h, p)| (h, format!("/{p}")))
        .unwrap();
    let (host, port_str) = host_port.split_once(':').unwrap();
    let port: u16 = port_str.parse().unwrap();

    let mut stream = tokio::net::TcpStream::connect((host, port))
        .await
        .expect("connect");
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\nAccept: application/json\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).await.unwrap();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    let raw = String::from_utf8_lossy(&buf);
    let (header, body) = raw.split_once("\r\n\r\n").expect("http response");
    let status_line = header.lines().next().unwrap();
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(body.trim()).unwrap();
    (status, json)
}

#[tokio::test]
async fn health_and_not_found_then_graceful_shutdown() {
    let (addr, shutdown_tx, handle) = start_test_server().await;
    let base = format!("http://{addr}");

    let (status, health) = get_json(&format!("{base}/health")).await;
    assert_eq!(status, 200);
    assert_eq!(health["status"], "ok");
    assert_eq!(health["service"], "model-hub-gateway");
    assert_eq!(health["version"], env!("CARGO_PKG_VERSION"));

    let (status, err) = get_json(&format!("{base}/unknown-path")).await;
    assert_eq!(status, 404);
    assert_eq!(err["error"]["code"], "NOT_FOUND");
    assert_eq!(err["error"]["message"], "未找到请求的接口");

    shutdown_tx.send(()).expect("send shutdown");
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("server task timed out")
        .expect("server task join");
}

#[tokio::test]
async fn default_config_socket_is_loopback() {
    let config = GatewayConfig::default();
    let addr = config.socket_addr().unwrap();
    assert_eq!(addr.ip().to_string(), "127.0.0.1");
    assert_eq!(addr.port(), 8080);
}
