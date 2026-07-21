//! 请求日志 list/clear/分页/401 + mock chat 写入集成测试。

use std::net::SocketAddr;
use std::time::Duration;

use model_hub_gateway::{bind_listener, build_router, serve, AppState, DEFAULT_HOST};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn start_test_server() -> (
    SocketAddr,
    oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("data.db");
    let state = AppState::for_tests_with_db_path(db_path.to_str().unwrap());

    let listener = bind_listener(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    assert_eq!(addr.ip().to_string(), DEFAULT_HOST);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let app = build_router(state);
    let handle = tokio::spawn(async move {
        serve(listener, app, async move {
            let _ = shutdown_rx.await;
        })
        .await
        .expect("serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    (addr, shutdown_tx, handle, dir)
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

async fn admin_login(base: &str) -> String {
    let login = http_json(
        "POST",
        base,
        "/api/v1/user/login",
        Some(r#"{"username":"admin","password":"admin","expire":3600}"#),
        None,
    )
    .await;
    assert_eq!(login.status, 200);
    login.body["data"]["token"].as_str().unwrap().to_string()
}

async fn setup_chat_env(base: &str, mock_uri: &str, token: &str) -> String {
    let create_ch = http_json(
        "POST",
        base,
        "/api/v1/channel/create",
        Some(&format!(
            r#"{{
            "name": "log-channel",
            "type": 0,
            "enabled": true,
            "base_urls": [{{"url": "{mock_uri}/v1", "delay": 0}}],
            "keys": [{{"enabled": true, "channel_key": "sk-upstream-log", "remark": ""}}],
            "model": "gpt-4o-mini",
            "custom_model": "",
            "proxy": false,
            "auto_sync": false,
            "auto_group": 0,
            "custom_header": []
        }}"#
        )),
        Some(token),
    )
    .await;
    assert_eq!(create_ch.status, 200, "{:?}", create_ch.body);
    let channel_id = create_ch.body["data"]["id"].as_i64().unwrap();

    let create_g = http_json(
        "POST",
        base,
        "/api/v1/group/create",
        Some(&format!(
            r#"{{
            "name": "log-group",
            "mode": 1,
            "match_regex": "",
            "items": [{{
                "channel_id": {channel_id},
                "model_name": "gpt-4o-mini",
                "priority": 1,
                "weight": 1
            }}]
        }}"#
        )),
        Some(token),
    )
    .await;
    assert_eq!(create_g.status, 200, "{:?}", create_g.body);

    let create_key = http_json(
        "POST",
        base,
        "/api/v1/apikey/create",
        Some(r#"{"name":"log-client","enabled":true}"#),
        Some(token),
    )
    .await;
    assert_eq!(create_key.status, 200);
    create_key.body["data"]["api_key"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn log_list_requires_jwt() {
    let (addr, shutdown_tx, handle, _dir) = start_test_server().await;
    let base = format!("http://{addr}");

    let no_token = http_json(
        "GET",
        &base,
        "/api/v1/log/list?page=1&page_size=20",
        None,
        None,
    )
    .await;
    assert_eq!(no_token.status, 401);

    let clear_no_token = http_json("DELETE", &base, "/api/v1/log/clear", None, None).await;
    assert_eq!(clear_no_token.status, 401);

    shutdown_tx.send(()).unwrap();
    handle.await.unwrap();
}

#[tokio::test]
async fn chat_writes_log_list_clear_and_page_size_cap() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer sk-upstream-log"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl-log",
            "object": "chat.completion",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "pong"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 8,
                "total_tokens": 20
            }
        })))
        .mount(&mock)
        .await;

    let (addr, shutdown_tx, handle, _dir) = start_test_server().await;
    let base = format!("http://{addr}");
    let token = admin_login(&base).await;
    let raw_key = setup_chat_env(&base, &mock.uri(), &token).await;

    // 初始 list 为空
    let empty = http_json(
        "GET",
        &base,
        "/api/v1/log/list?page=1&page_size=20",
        None,
        Some(&token),
    )
    .await;
    assert_eq!(empty.status, 200);
    assert_eq!(empty.body["data"].as_array().unwrap().len(), 0);

    // 非流式 chat 成功
    let chat = http_json(
        "POST",
        &base,
        "/v1/chat/completions",
        Some(r#"{"model":"log-group","messages":[{"role":"user","content":"secret-hi"}]}"#),
        Some(&raw_key),
    )
    .await;
    assert_eq!(chat.status, 200, "{:?}", chat.body);

    // 路由失败也记日志
    let unknown = http_json(
        "POST",
        &base,
        "/v1/chat/completions",
        Some(r#"{"model":"nope-group","messages":[{"role":"user","content":"x"}]}"#),
        Some(&raw_key),
    )
    .await;
    assert_eq!(unknown.status, 404);

    // 401 不记业务日志：先清一次再测？此处先 list 应有 2 条（成功+未知分组）
    let listed = http_json(
        "GET",
        &base,
        "/api/v1/log/list?page=1&page_size=20",
        None,
        Some(&token),
    )
    .await;
    assert_eq!(listed.status, 200);
    let logs = listed.body["data"].as_array().unwrap();
    assert_eq!(logs.len(), 2, "logs: {logs:?}");

    // 倒序：最新在前（未知分组 id 更大）
    let first = &logs[0];
    let second = &logs[1];
    assert!(first["id"].as_i64().unwrap() > second["id"].as_i64().unwrap());

    // 成功日志字段
    let success = logs
        .iter()
        .find(|l| l["error"].as_str().unwrap_or("").is_empty())
        .expect("success log");
    assert_eq!(success["request_model_name"], "log-group");
    assert_eq!(success["channel_name"], "log-channel");
    assert_eq!(success["actual_model_name"], "gpt-4o-mini");
    assert_eq!(success["input_tokens"], 12);
    assert_eq!(success["output_tokens"], 8);
    assert_eq!(success["cost"], 0.0);
    assert!(success["time"].as_i64().unwrap() > 1_000_000_000);
    // 不含 messages / 密钥
    let dump = listed.body.to_string();
    assert!(!dump.contains("secret-hi"));
    assert!(!dump.contains("sk-upstream-log"));
    assert!(!dump.contains("sk-modelhub-"));

    // 失败日志 error 非空
    let failed = logs
        .iter()
        .find(|l| !l["error"].as_str().unwrap_or("").is_empty())
        .expect("error log");
    assert!(failed["error"].as_str().unwrap().contains("未知分组"));

    // 401 不增加日志
    let before = logs.len();
    let _ = http_json(
        "POST",
        &base,
        "/v1/chat/completions",
        Some(r#"{"model":"log-group","messages":[{"role":"user","content":"noauth"}]}"#),
        None,
    )
    .await;
    let after_401 = http_json(
        "GET",
        &base,
        "/api/v1/log/list?page=1&page_size=20",
        None,
        Some(&token),
    )
    .await;
    assert_eq!(after_401.body["data"].as_array().unwrap().len(), before);

    // page_size 上限 100：请求 999 仍最多 100（当前仅 2 条）
    let capped = http_json(
        "GET",
        &base,
        "/api/v1/log/list?page=1&page_size=999",
        None,
        Some(&token),
    )
    .await;
    assert_eq!(capped.status, 200);
    assert!(capped.body["data"].as_array().unwrap().len() <= 100);

    // 分页：page_size=1
    let page1 = http_json(
        "GET",
        &base,
        "/api/v1/log/list?page=1&page_size=1",
        None,
        Some(&token),
    )
    .await;
    let page2 = http_json(
        "GET",
        &base,
        "/api/v1/log/list?page=2&page_size=1",
        None,
        Some(&token),
    )
    .await;
    assert_eq!(page1.body["data"].as_array().unwrap().len(), 1);
    assert_eq!(page2.body["data"].as_array().unwrap().len(), 1);
    assert_ne!(page1.body["data"][0]["id"], page2.body["data"][0]["id"]);

    // clear
    let cleared = http_json("DELETE", &base, "/api/v1/log/clear", None, Some(&token)).await;
    assert_eq!(cleared.status, 200);
    assert!(cleared.body["data"].is_null());

    let after_clear = http_json(
        "GET",
        &base,
        "/api/v1/log/list?page=1&page_size=20",
        None,
        Some(&token),
    )
    .await;
    assert_eq!(after_clear.body["data"].as_array().unwrap().len(), 0);

    shutdown_tx.send(()).unwrap();
    handle.await.unwrap();
}
