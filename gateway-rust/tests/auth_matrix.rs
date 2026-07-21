//! 本地开放模式：管理/客户端路径均无需凭证。

use std::net::SocketAddr;
use std::time::Duration;

use model_hub_gateway::{bind_listener, build_router, serve, AppState, DEFAULT_HOST};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;

async fn start_test_server() -> (SocketAddr, oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
    let listener = bind_listener(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind random loopback port");
    let addr = listener.local_addr().expect("local_addr");
    assert_eq!(addr.ip().to_string(), DEFAULT_HOST);
    assert_ne!(addr.port(), 0);
    assert_ne!(addr.port(), 8080);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let app = build_router(AppState::for_tests());
    let handle = tokio::spawn(async move {
        serve(listener, app, async move {
            let _ = shutdown_rx.await;
        })
        .await
        .expect("serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    (addr, shutdown_tx, handle)
}

struct HttpResult {
    status: u16,
    body: serde_json::Value,
}

async fn http_json(
    method: &str,
    base: &str,
    path: &str,
    body: Option<&str>,
    auth: Option<&str>,
) -> HttpResult {
    let url = format!("{base}{path}");
    let url = url.strip_prefix("http://").expect("http url");
    let (host_port, path_only) = url
        .split_once('/')
        .map(|(h, p)| (h, format!("/{p}")))
        .unwrap();
    let (host, port_str) = host_port.split_once(':').unwrap();
    let port: u16 = port_str.parse().unwrap();

    let mut stream = tokio::net::TcpStream::connect((host, port))
        .await
        .expect("connect");

    let mut request = format!(
        "{method} {path_only} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\nAccept: application/json\r\n"
    );
    if let Some(token) = auth {
        request.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    if let Some(body) = body {
        request.push_str("Content-Type: application/json\r\n");
        request.push_str(&format!("Content-Length: {}\r\n", body.len()));
        request.push_str("\r\n");
        request.push_str(body);
    } else {
        request.push_str("\r\n");
    }

    stream.write_all(request.as_bytes()).await.unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    let raw = String::from_utf8_lossy(&buf);
    let (header, body_raw) = raw.split_once("\r\n\r\n").expect("http response");
    let status_line = header.lines().next().unwrap();
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let json: serde_json::Value = if body_raw.trim().is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_str(body_raw.trim())
            .unwrap_or(serde_json::Value::String(body_raw.to_string()))
    };
    HttpResult { status, body: json }
}

#[tokio::test]
async fn open_local_api_without_credentials() {
    let (addr, shutdown_tx, handle) = start_test_server().await;
    let base = format!("http://{addr}");

    let health = http_json("GET", &base, "/health", None, None).await;
    assert_eq!(health.status, 200);
    assert_eq!(health.body["status"], "ok");

    // 管理路径无需 Token
    let status = http_json("GET", &base, "/api/v1/user/status", None, None).await;
    assert_eq!(status.status, 200);
    assert_eq!(status.body["data"], "ok");

    let list = http_json("GET", &base, "/api/v1/apikey/list", None, None).await;
    assert_eq!(list.status, 200);

    // 客户端路径无需 Key
    let models = http_json("GET", &base, "/v1/models", None, None).await;
    assert_eq!(models.status, 200);
    assert_eq!(models.body["object"], "list");

    // 带垃圾 Token 也不应 401（本地信任）
    let models_junk = http_json("GET", &base, "/v1/models", None, Some("junk")).await;
    assert_eq!(models_junk.status, 200);

    let _ = shutdown_tx.send(());
    let _ = handle.await;
}
