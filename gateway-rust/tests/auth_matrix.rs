//! 随机端口集成鉴权矩阵：登录 / API Key CRUD / /v1/models。

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
    // 不占用用户默认 8080
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
    extra_headers: &[(&str, &str)],
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

    let mut request = format!("{method} {path_only} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\nAccept: application/json\r\n");
    if let Some(token) = auth {
        request.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    for (k, v) in extra_headers {
        request.push_str(&format!("{k}: {v}\r\n"));
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
async fn full_auth_matrix() {
    let (addr, shutdown_tx, handle) = start_test_server().await;
    let base = format!("http://{addr}");

    // /health 无凭证 200
    let health = http_json("GET", &base, "/health", None, None, &[]).await;
    assert_eq!(health.status, 200);
    assert_eq!(health.body["status"], "ok");

    // 错误密码 401
    let bad_login = http_json(
        "POST",
        &base,
        "/api/v1/user/login",
        Some(r#"{"username":"admin","password":"nope"}"#),
        None,
        &[],
    )
    .await;
    assert_eq!(bad_login.status, 401);
    assert_eq!(bad_login.body["message"], "用户名或密码错误");
    assert_eq!(bad_login.body["error"]["code"], "UNAUTHORIZED");

    // 默认 admin 登录成功
    let login = http_json(
        "POST",
        &base,
        "/api/v1/user/login",
        Some(r#"{"username":"admin","password":"admin","expire":3600}"#),
        None,
        &[],
    )
    .await;
    assert_eq!(login.status, 200);
    let admin_token = login.body["data"]["token"].as_str().unwrap().to_string();
    assert!(!admin_token.is_empty());
    assert!(login.body["data"]["expire_at"].is_string());

    // 无 token / 坏 token 访问 status 与 apikey
    let no_status = http_json("GET", &base, "/api/v1/user/status", None, None, &[]).await;
    assert_eq!(no_status.status, 401);
    assert!(!no_status.body["message"].as_str().unwrap().is_empty());

    let bad_status = http_json(
        "GET",
        &base,
        "/api/v1/user/status",
        None,
        Some("bad.token.value"),
        &[],
    )
    .await;
    assert_eq!(bad_status.status, 401);

    let ok_status = http_json(
        "GET",
        &base,
        "/api/v1/user/status",
        None,
        Some(&admin_token),
        &[],
    )
    .await;
    assert_eq!(ok_status.status, 200);
    assert_eq!(ok_status.body["data"], "ok");

    let no_list = http_json("GET", &base, "/api/v1/apikey/list", None, None, &[]).await;
    assert_eq!(no_list.status, 401);

    // 创建 API Key
    let create = http_json(
        "POST",
        &base,
        "/api/v1/apikey/create",
        Some(r#"{"name":"local-client","enabled":true}"#),
        Some(&admin_token),
        &[],
    )
    .await;
    assert_eq!(create.status, 200);
    let raw_key = create.body["data"]["api_key"].as_str().unwrap().to_string();
    let key_id = create.body["data"]["id"].as_i64().unwrap();
    assert!(raw_key.starts_with("sk-octopus-"));
    assert_eq!(create.body["data"]["name"], "local-client");

    // list 脱敏
    let list = http_json(
        "GET",
        &base,
        "/api/v1/apikey/list",
        None,
        Some(&admin_token),
        &[],
    )
    .await;
    assert_eq!(list.status, 200);
    let items = list.body["data"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    let listed_key = items[0]["api_key"].as_str().unwrap();
    assert_ne!(listed_key, raw_key);
    assert!(listed_key.contains("****"));

    // update
    let update = http_json(
        "POST",
        &base,
        "/api/v1/apikey/update",
        Some(&format!(r#"{{"id":{key_id},"name":"renamed"}}"#)),
        Some(&admin_token),
        &[],
    )
    .await;
    assert_eq!(update.status, 200);
    assert_eq!(update.body["data"]["name"], "renamed");
    // update 不得返回完整 key
    assert_ne!(update.body["data"]["api_key"].as_str().unwrap(), raw_key);

    // /v1/models 鉴权矩阵
    let no_key = http_json("GET", &base, "/v1/models", None, None, &[]).await;
    assert_eq!(no_key.status, 401);
    assert!(no_key.body["message"].as_str().is_some());

    let bad_key = http_json(
        "GET",
        &base,
        "/v1/models",
        None,
        Some("sk-placeholder"),
        &[],
    )
    .await;
    assert_eq!(bad_key.status, 401);

    let admin_as_client =
        http_json("GET", &base, "/v1/models", None, Some(&admin_token), &[]).await;
    assert_eq!(admin_as_client.status, 401);

    let good = http_json("GET", &base, "/v1/models", None, Some(&raw_key), &[]).await;
    assert_eq!(good.status, 200);
    assert_eq!(good.body["object"], "list");
    assert!(good.body["data"].as_array().unwrap().is_empty());

    // x-api-key 头
    let via_header = http_json(
        "GET",
        &base,
        "/v1/models",
        None,
        None,
        &[("x-api-key", raw_key.as_str())],
    )
    .await;
    assert_eq!(via_header.status, 200);

    // 禁用后 401
    let disable = http_json(
        "POST",
        &base,
        "/api/v1/apikey/update",
        Some(&format!(r#"{{"id":{key_id},"enabled":false}}"#)),
        Some(&admin_token),
        &[],
    )
    .await;
    assert_eq!(disable.status, 200);
    let disabled = http_json("GET", &base, "/v1/models", None, Some(&raw_key), &[]).await;
    assert_eq!(disabled.status, 401);

    // 重新启用再删除
    let _ = http_json(
        "POST",
        &base,
        "/api/v1/apikey/update",
        Some(&format!(r#"{{"id":{key_id},"enabled":true}}"#)),
        Some(&admin_token),
        &[],
    )
    .await;
    let del = http_json(
        "DELETE",
        &base,
        &format!("/api/v1/apikey/delete/{key_id}"),
        None,
        Some(&admin_token),
        &[],
    )
    .await;
    assert_eq!(del.status, 200);
    let after_del = http_json("GET", &base, "/v1/models", None, Some(&raw_key), &[]).await;
    assert_eq!(after_del.status, 401);

    // 未知路径仍 JSON 404
    let nf = http_json("GET", &base, "/nope", None, None, &[]).await;
    assert_eq!(nf.status, 404);
    assert_eq!(nf.body["error"]["code"], "NOT_FOUND");

    shutdown_tx.send(()).expect("send shutdown");
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("server task timed out")
        .expect("server task join");
}
