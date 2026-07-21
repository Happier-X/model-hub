//! 随机端口 + wiremock 上游：Chat 非流式与 SSE 流式转发集成测试。

use std::net::SocketAddr;
use std::time::Duration;

use model_hub_gateway::{bind_listener, build_router, serve, AppState, DEFAULT_HOST};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn start_test_server() -> (
    SocketAddr,
    oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
    tempfile::TempDir,
    AppState,
) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("data.db");
    let state = AppState::for_tests_with_db_path(db_path.to_str().unwrap());

    let listener = bind_listener(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind random loopback port");
    let addr = listener.local_addr().expect("local_addr");
    assert_eq!(addr.ip().to_string(), DEFAULT_HOST);
    assert_ne!(addr.port(), 0);
    assert_ne!(addr.port(), 8080);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let app = build_router(state.clone());
    let handle = tokio::spawn(async move {
        serve(listener, app, async move {
            let _ = shutdown_rx.await;
        })
        .await
        .expect("serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    (addr, shutdown_tx, handle, dir, state)
}

struct HttpResult {
    status: u16,
    body: serde_json::Value,
}

struct HttpRawResult {
    status: u16,
    content_type: String,
    body_raw: String,
}

async fn http_exchange(
    method: &str,
    base: &str,
    path: &str,
    body: Option<&str>,
    auth: Option<&str>,
    accept: &str,
) -> (u16, String, String) {
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
        "{method} {path_only} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\nAccept: {accept}\r\n"
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
    let content_type = header
        .lines()
        .find_map(|line| {
            let (k, v) = line.split_once(':')?;
            if k.eq_ignore_ascii_case("content-type") {
                Some(v.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_default();
    (status, content_type, body_raw.to_string())
}

async fn http_json(
    method: &str,
    base: &str,
    path: &str,
    body: Option<&str>,
    auth: Option<&str>,
) -> HttpResult {
    let (status, _ct, body_raw) =
        http_exchange(method, base, path, body, auth, "application/json").await;
    let json: serde_json::Value = if body_raw.trim().is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_str(body_raw.trim())
            .unwrap_or(serde_json::Value::String(body_raw.to_string()))
    };
    HttpResult { status, body: json }
}

async fn http_raw(
    method: &str,
    base: &str,
    path: &str,
    body: Option<&str>,
    auth: Option<&str>,
) -> HttpRawResult {
    let (status, content_type, body_raw) =
        http_exchange(method, base, path, body, auth, "text/event-stream").await;
    HttpRawResult {
        status,
        content_type,
        body_raw,
    }
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

#[tokio::test]
async fn chat_forward_to_mock_upstream() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer sk-upstream-mock"))
        .and(body_partial_json(json!({
            "model": "gpt-4o-mini"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl-mock",
            "object": "chat.completion",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "pong"},
                "finish_reason": "stop"
            }]
        })))
        .mount(&mock)
        .await;

    let (addr, shutdown_tx, handle, _dir, _state) = start_test_server().await;
    let base = format!("http://{addr}");
    let token = admin_login(&base).await;

    let create_ch = http_json(
        "POST",
        &base,
        "/api/v1/channel/create",
        Some(&format!(
            r#"{{
            "name": "mock-openai",
            "type": 0,
            "enabled": true,
            "base_urls": [{{"url": "{}/v1", "delay": 0}}],
            "keys": [{{"enabled": true, "channel_key": "sk-upstream-mock", "remark": ""}}],
            "model": "gpt-4o-mini",
            "custom_model": "",
            "proxy": false,
            "auto_sync": false,
            "auto_group": 0,
            "custom_header": []
        }}"#,
            mock.uri()
        )),
        Some(&token),
    )
    .await;
    assert_eq!(create_ch.status, 200, "{:?}", create_ch.body);
    let channel_id = create_ch.body["data"]["id"].as_i64().unwrap();

    let create_g = http_json(
        "POST",
        &base,
        "/api/v1/group/create",
        Some(&format!(
            r#"{{
            "name": "demo-group",
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
        Some(&token),
    )
    .await;
    assert_eq!(create_g.status, 200, "{:?}", create_g.body);

    let create_key = http_json(
        "POST",
        &base,
        "/api/v1/apikey/create",
        Some(r#"{"name":"chat-client","enabled":true}"#),
        Some(&token),
    )
    .await;
    assert_eq!(create_key.status, 200);
    let raw_key = create_key.body["data"]["api_key"]
        .as_str()
        .unwrap()
        .to_string();

    // models 列出分组名
    let models = http_json("GET", &base, "/v1/models", None, Some(&raw_key)).await;
    assert_eq!(models.status, 200);
    assert_eq!(models.body["object"], "list");
    let data = models.body["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["id"], "demo-group");
    assert_eq!(data[0]["object"], "model");

    // 无 Key / 管理 JWT → 401
    let no_key = http_json(
        "POST",
        &base,
        "/v1/chat/completions",
        Some(r#"{"model":"demo-group","messages":[{"role":"user","content":"hi"}]}"#),
        None,
    )
    .await;
    assert_eq!(no_key.status, 401);

    let admin_as_client = http_json(
        "POST",
        &base,
        "/v1/chat/completions",
        Some(r#"{"model":"demo-group","messages":[{"role":"user","content":"hi"}]}"#),
        Some(&token),
    )
    .await;
    assert_eq!(admin_as_client.status, 401);

    // 正常转发
    let chat = http_json(
        "POST",
        &base,
        "/v1/chat/completions",
        Some(r#"{"model":"demo-group","messages":[{"role":"user","content":"hi"}]}"#),
        Some(&raw_key),
    )
    .await;
    assert_eq!(chat.status, 200, "chat body: {:?}", chat.body);
    assert_eq!(chat.body["id"], "chatcmpl-mock");
    assert_eq!(chat.body["choices"][0]["message"]["content"], "pong");

    // stream=true 无 mock 上游 SSE 时会 502 或上游错误，但绝非 401 / STREAM_NOT_SUPPORTED
    // 完整 SSE 见 chat_sse_stream_proxy；此处保证非流式路径仍正常且未知分组非 401

    // 未知分组
    let unknown = http_json(
        "POST",
        &base,
        "/v1/chat/completions",
        Some(r#"{"model":"nope","messages":[{"role":"user","content":"hi"}]}"#),
        Some(&raw_key),
    )
    .await;
    assert_eq!(unknown.status, 404);
    assert_ne!(unknown.status, 401);

    shutdown_tx.send(()).expect("send shutdown");
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("server task timed out")
        .expect("server task join");
}

#[tokio::test]
async fn chat_sse_stream_proxy() {
    let mock = MockServer::start().await;
    let sse_body = concat!(
        "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n",
        "\n",
        "data: [DONE]\n",
        "\n",
    );
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer sk-upstream-sse"))
        .and(body_partial_json(json!({
            "model": "gpt-4o-mini",
            "stream": true
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(sse_body.as_bytes().to_vec(), "text/event-stream"),
        )
        .mount(&mock)
        .await;

    let (addr, shutdown_tx, handle, _dir, _state) = start_test_server().await;
    let base = format!("http://{addr}");
    let token = admin_login(&base).await;

    let create_ch = http_json(
        "POST",
        &base,
        "/api/v1/channel/create",
        Some(&format!(
            r#"{{
            "name": "sse-openai",
            "type": 0,
            "enabled": true,
            "base_urls": [{{"url": "{}/v1", "delay": 0}}],
            "keys": [{{"enabled": true, "channel_key": "sk-upstream-sse", "remark": ""}}],
            "model": "gpt-4o-mini",
            "custom_model": "",
            "proxy": false,
            "auto_sync": false,
            "auto_group": 0,
            "custom_header": []
        }}"#,
            mock.uri()
        )),
        Some(&token),
    )
    .await;
    assert_eq!(create_ch.status, 200, "{:?}", create_ch.body);
    let channel_id = create_ch.body["data"]["id"].as_i64().unwrap();

    let create_g = http_json(
        "POST",
        &base,
        "/api/v1/group/create",
        Some(&format!(
            r#"{{
            "name": "sse-group",
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
        Some(&token),
    )
    .await;
    assert_eq!(create_g.status, 200, "{:?}", create_g.body);

    let create_key = http_json(
        "POST",
        &base,
        "/api/v1/apikey/create",
        Some(r#"{"name":"sse-client","enabled":true}"#),
        Some(&token),
    )
    .await;
    assert_eq!(create_key.status, 200);
    let raw_key = create_key.body["data"]["api_key"]
        .as_str()
        .unwrap()
        .to_string();

    // 无 Key → 401
    let no_key = http_raw(
        "POST",
        &base,
        "/v1/chat/completions",
        Some(r#"{"model":"sse-group","messages":[{"role":"user","content":"hi"}],"stream":true}"#),
        None,
    )
    .await;
    assert_eq!(no_key.status, 401);

    // 管理 JWT → 401
    let admin_as_client = http_raw(
        "POST",
        &base,
        "/v1/chat/completions",
        Some(r#"{"model":"sse-group","messages":[{"role":"user","content":"hi"}],"stream":true}"#),
        Some(&token),
    )
    .await;
    assert_eq!(admin_as_client.status, 401);

    // 未知分组 → 非 401
    let unknown = http_raw(
        "POST",
        &base,
        "/v1/chat/completions",
        Some(r#"{"model":"nope-sse","messages":[{"role":"user","content":"hi"}],"stream":true}"#),
        Some(&raw_key),
    )
    .await;
    assert_eq!(unknown.status, 404);
    assert_ne!(unknown.status, 401);
    assert!(!unknown.body_raw.contains("STREAM_NOT_SUPPORTED"));

    // 有效 Key + stream → event-stream 含 data 与 [DONE]
    let stream = http_raw(
        "POST",
        &base,
        "/v1/chat/completions",
        Some(r#"{"model":"sse-group","messages":[{"role":"user","content":"hi"}],"stream":true}"#),
        Some(&raw_key),
    )
    .await;
    assert_eq!(stream.status, 200, "stream body: {}", stream.body_raw);
    assert!(
        stream
            .content_type
            .to_ascii_lowercase()
            .contains("text/event-stream"),
        "content-type: {}",
        stream.content_type
    );
    assert!(
        stream.body_raw.contains("data:"),
        "body: {}",
        stream.body_raw
    );
    assert!(
        stream.body_raw.contains("[DONE]"),
        "body: {}",
        stream.body_raw
    );
    assert!(stream.body_raw.contains("hi"), "body: {}", stream.body_raw);
    assert!(!stream.body_raw.contains("STREAM_NOT_SUPPORTED"));

    // stream=false 仍走整包 JSON
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(body_partial_json(json!({
            "model": "gpt-4o-mini"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl-nonstream",
            "object": "chat.completion",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "json"},
                "finish_reason": "stop"
            }]
        })))
        .mount(&mock)
        .await;

    let non_stream = http_json(
        "POST",
        &base,
        "/v1/chat/completions",
        Some(r#"{"model":"sse-group","messages":[{"role":"user","content":"hi"}],"stream":false}"#),
        Some(&raw_key),
    )
    .await;
    assert_eq!(non_stream.status, 200, "{:?}", non_stream.body);
    assert_eq!(non_stream.body["id"], "chatcmpl-nonstream");

    shutdown_tx.send(()).expect("send shutdown");
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("server task timed out")
        .expect("server task join");
}

#[tokio::test]
async fn round_robin_switches_upstream_model() {
    let mock = MockServer::start().await;

    // 两个 model_name 分别命中
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_partial_json(json!({ "model": "model-a" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "a",
            "choices": [{"message": {"content": "from-a"}}]
        })))
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_partial_json(json!({ "model": "model-b" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "b",
            "choices": [{"message": {"content": "from-b"}}]
        })))
        .mount(&mock)
        .await;

    let (addr, shutdown_tx, handle, _dir, _state) = start_test_server().await;
    let base = format!("http://{addr}");
    let token = admin_login(&base).await;

    let create_ch = http_json(
        "POST",
        &base,
        "/api/v1/channel/create",
        Some(&format!(
            r#"{{
            "name": "rr-channel",
            "type": 0,
            "enabled": true,
            "base_urls": [{{"url": "{}", "delay": 0}}],
            "keys": [{{"enabled": true, "channel_key": "sk-rr", "remark": ""}}],
            "model": "x",
            "custom_model": "",
            "proxy": false,
            "auto_sync": false,
            "auto_group": 0,
            "custom_header": []
        }}"#,
            mock.uri()
        )),
        Some(&token),
    )
    .await;
    assert_eq!(create_ch.status, 200);
    let channel_id = create_ch.body["data"]["id"].as_i64().unwrap();

    let create_g = http_json(
        "POST",
        &base,
        "/api/v1/group/create",
        Some(&format!(
            r#"{{
            "name": "rr-group",
            "mode": 1,
            "match_regex": "",
            "items": [
                {{"channel_id": {channel_id}, "model_name": "model-a", "priority": 1, "weight": 1}},
                {{"channel_id": {channel_id}, "model_name": "model-b", "priority": 1, "weight": 1}}
            ]
        }}"#
        )),
        Some(&token),
    )
    .await;
    assert_eq!(create_g.status, 200);

    let create_key = http_json(
        "POST",
        &base,
        "/api/v1/apikey/create",
        Some(r#"{"name":"rr-client","enabled":true}"#),
        Some(&token),
    )
    .await;
    let raw_key = create_key.body["data"]["api_key"]
        .as_str()
        .unwrap()
        .to_string();

    let body = r#"{"model":"rr-group","messages":[{"role":"user","content":"hi"}]}"#;
    let first = http_json(
        "POST",
        &base,
        "/v1/chat/completions",
        Some(body),
        Some(&raw_key),
    )
    .await;
    let second = http_json(
        "POST",
        &base,
        "/v1/chat/completions",
        Some(body),
        Some(&raw_key),
    )
    .await;
    assert_eq!(first.status, 200, "{:?}", first.body);
    assert_eq!(second.status, 200, "{:?}", second.body);
    assert_eq!(first.body["id"], "a");
    assert_eq!(second.body["id"], "b");

    shutdown_tx.send(()).expect("send shutdown");
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("server task timed out")
        .expect("server task join");
}
