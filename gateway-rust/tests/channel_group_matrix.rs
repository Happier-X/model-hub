//! 随机端口集成：login → channel → group → apikey → models。

use std::net::SocketAddr;
use std::time::Duration;

use model_hub_gateway::{bind_listener, build_router, serve, AppState, DEFAULT_HOST};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;

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
        .expect("bind random loopback port");
    let addr = listener.local_addr().expect("local_addr");
    assert_eq!(addr.ip().to_string(), DEFAULT_HOST);
    assert_ne!(addr.port(), 0);
    assert_ne!(addr.port(), 8080);

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

#[tokio::test]
async fn channel_group_apikey_full_flow() {
    let (addr, shutdown_tx, handle, _dir) = start_test_server().await;
    let base = format!("http://{addr}");

    // 无 JWT 访问渠道 401
    let no_auth = http_json("GET", &base, "/api/v1/channel/list", None, None).await;
    assert_eq!(no_auth.status, 401);
    assert!(!no_auth.body["message"].as_str().unwrap().is_empty());

    // 登录
    let login = http_json(
        "POST",
        &base,
        "/api/v1/user/login",
        Some(r#"{"username":"admin","password":"admin","expire":3600}"#),
        None,
    )
    .await;
    assert_eq!(login.status, 200);
    let token = login.body["data"]["token"].as_str().unwrap().to_string();

    // 创建渠道
    let create_ch = http_json(
        "POST",
        &base,
        "/api/v1/channel/create",
        Some(
            r#"{
            "name": "smoke-openai",
            "type": 0,
            "enabled": true,
            "base_urls": [{"url": "https://api.openai.com/v1", "delay": 0}],
            "keys": [{"enabled": true, "channel_key": "sk-test-fake", "remark": "smoke"}],
            "model": "gpt-4o-mini",
            "custom_model": "",
            "proxy": false,
            "auto_sync": false,
            "auto_group": 0,
            "custom_header": []
        }"#,
        ),
        Some(&token),
    )
    .await;
    assert_eq!(
        create_ch.status, 200,
        "create channel: {:?}",
        create_ch.body
    );
    let channel_id = create_ch.body["data"]["id"].as_i64().unwrap();
    assert_eq!(create_ch.body["data"]["type"], 0);
    assert_eq!(create_ch.body["data"]["name"], "smoke-openai");
    let key_id = create_ch.body["data"]["keys"][0]["id"].as_i64().unwrap();

    // list
    let list_ch = http_json("GET", &base, "/api/v1/channel/list", None, Some(&token)).await;
    assert_eq!(list_ch.status, 200);
    assert_eq!(list_ch.body["data"].as_array().unwrap().len(), 1);

    // update：改名 + keys_to_update + keys_to_add
    let update_ch = http_json(
        "POST",
        &base,
        "/api/v1/channel/update",
        Some(&format!(
            r#"{{
            "id": {channel_id},
            "name": "smoke-renamed",
            "type": 0,
            "base_urls": [{{"url": "https://example.com/v1", "delay": 0}}],
            "model": "gpt-4o",
            "keys_to_update": [{{"id": {key_id}, "channel_key": "sk-rotated"}}],
            "keys_to_add": [{{"enabled": true, "channel_key": "sk-extra", "remark": ""}}]
        }}"#
        )),
        Some(&token),
    )
    .await;
    assert_eq!(update_ch.status, 200);
    assert_eq!(update_ch.body["data"]["name"], "smoke-renamed");
    assert_eq!(update_ch.body["data"]["model"], "gpt-4o");
    assert_eq!(update_ch.body["data"]["keys"].as_array().unwrap().len(), 2);
    assert_eq!(
        update_ch.body["data"]["keys"][0]["channel_key"],
        "sk-rotated"
    );

    // enable
    let enable = http_json(
        "POST",
        &base,
        "/api/v1/channel/enable",
        Some(&format!(r#"{{"id":{channel_id},"enabled":false}}"#)),
        Some(&token),
    )
    .await;
    assert_eq!(enable.status, 200);
    assert_eq!(enable.body["data"]["enabled"], false);

    // 重新启用以便分组引用
    let _ = http_json(
        "POST",
        &base,
        "/api/v1/channel/enable",
        Some(&format!(r#"{{"id":{channel_id},"enabled":true}}"#)),
        Some(&token),
    )
    .await;

    // 创建分组
    let create_g = http_json(
        "POST",
        &base,
        "/api/v1/group/create",
        Some(&format!(
            r#"{{
            "name": "smoke-group",
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
    assert_eq!(create_g.status, 200, "create group: {:?}", create_g.body);
    let group_id = create_g.body["data"]["id"].as_i64().unwrap();
    assert_eq!(create_g.body["data"]["mode"], 1);
    let item_id = create_g.body["data"]["items"][0]["id"].as_i64().unwrap();

    let list_g = http_json("GET", &base, "/api/v1/group/list", None, Some(&token)).await;
    assert_eq!(list_g.status, 200);
    assert_eq!(list_g.body["data"].as_array().unwrap().len(), 1);

    // update：items_to_delete + items_to_add
    let update_g = http_json(
        "POST",
        &base,
        "/api/v1/group/update",
        Some(&format!(
            r#"{{
            "id": {group_id},
            "name": "group-renamed",
            "items_to_delete": [{item_id}],
            "items_to_add": [{{
                "channel_id": {channel_id},
                "model_name": "gpt-4o",
                "priority": 1,
                "weight": 1
            }}]
        }}"#
        )),
        Some(&token),
    )
    .await;
    assert_eq!(update_g.status, 200);
    assert_eq!(update_g.body["data"]["name"], "group-renamed");
    assert_eq!(update_g.body["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(update_g.body["data"]["items"][0]["model_name"], "gpt-4o");

    // API Key + models
    let create_key = http_json(
        "POST",
        &base,
        "/api/v1/apikey/create",
        Some(r#"{"name":"local-client","enabled":true}"#),
        Some(&token),
    )
    .await;
    assert_eq!(create_key.status, 200);
    let raw_key = create_key.body["data"]["api_key"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(raw_key.starts_with("sk-octopus-"));

    let models = http_json("GET", &base, "/v1/models", None, Some(&raw_key)).await;
    assert_eq!(models.status, 200);
    assert_eq!(models.body["object"], "list");

    // 删除分组与渠道
    let del_g = http_json(
        "DELETE",
        &base,
        &format!("/api/v1/group/delete/{group_id}"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(del_g.status, 200);

    let del_ch = http_json(
        "DELETE",
        &base,
        &format!("/api/v1/channel/delete/{channel_id}"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(del_ch.status, 200);

    let list_empty = http_json("GET", &base, "/api/v1/channel/list", None, Some(&token)).await;
    assert!(list_empty.body["data"].as_array().unwrap().is_empty());

    shutdown_tx.send(()).expect("send shutdown");
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("server task timed out")
        .expect("server task join");
}

#[tokio::test]
async fn apikey_survives_new_state_instance() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("persist.db");
    let path = db_path.to_str().unwrap();

    let state1 = AppState::for_tests_with_db_path(path);
    let created = state1
        .api_keys
        .create(model_hub_gateway::apikey::CreateApiKeyRequest {
            name: "persist-me".into(),
            enabled: true,
            expire_at: None,
            max_cost: None,
            supported_models: None,
        })
        .unwrap();
    let raw = created.api_key.clone();

    // 新 state 打开同一文件
    let state2 = AppState::for_tests_with_db_path(path);
    let found = state2.api_keys.find_by_raw_key(&raw).unwrap();
    assert_eq!(found.id, created.id);
    assert_eq!(found.name, "persist-me");
    let list = state2.api_keys.list();
    assert_eq!(list.len(), 1);
    assert_ne!(list[0].api_key, raw);
}
