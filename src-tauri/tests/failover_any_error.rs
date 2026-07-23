//! “响应提交前任意错误均换源”的验收测试。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use model_hub_lib::db::open_db;
use model_hub_lib::domain::group::{CreateGroupPayload, GroupItemInput};
use model_hub_lib::domain::provider::CreateProviderPayload;
use model_hub_lib::domain::Stores;
use model_hub_lib::proxy::forward::{ForwardPolicy, UpstreamClients};
use model_hub_lib::proxy::server::{build_router, AppState};

struct Env {
    _dir: tempfile::TempDir,
    stores: Stores,
    router: axum::Router,
}

fn setup() -> Env {
    let dir = tempfile::tempdir().unwrap();
    let stores = Stores::new(open_db(&dir.path().join("t.db")).unwrap());
    let router = build_router(AppState {
        stores: stores.clone(),
        clients: UpstreamClients::new(),
        forward_policy: ForwardPolicy::default(),
    });
    Env {
        _dir: dir,
        stores,
        router,
    }
}

fn add_two_candidates(env: &Env, first_url: &str, second_url: &str, group: &str) {
    let first = env
        .stores
        .create_provider(CreateProviderPayload {
            name: format!("{group}-first"),
            base_url: format!("{first_url}/v1"),
            api_key: "test-key-first".into(),
            enabled: true,
        })
        .unwrap();
    let second = env
        .stores
        .create_provider(CreateProviderPayload {
            name: format!("{group}-second"),
            base_url: format!("{second_url}/v1"),
            api_key: "test-key-second".into(),
            enabled: true,
        })
        .unwrap();
    env.stores
        .create_group(CreateGroupPayload {
            name: group.into(),
            items: vec![
                GroupItemInput {
                    provider_id: first.id,
                    upstream_model: "first-model".into(),
                },
                GroupItemInput {
                    provider_id: second.id,
                    upstream_model: "second-model".into(),
                },
            ],
        })
        .unwrap();
}

async fn chat(router: axum::Router, group: &str, stream: bool) -> axum::response::Response {
    let body = json!({
        "model": group,
        "messages": [{"role":"user","content":"测试"}],
        "stream": stream
    });
    router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn assert_http_error_failover(status: u16, error_body: serde_json::Value) {
    let first = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(status).set_body_json(error_body))
        .expect(1)
        .mount(&first)
        .await;
    let second = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id":"ok", "object":"chat.completion",
            "choices":[{"message":{"role":"assistant","content":"ok"}}]
        })))
        .expect(1)
        .mount(&second)
        .await;

    let env = setup();
    add_two_candidates(&env, &first.uri(), &second.uri(), "any-http-error");
    let response = chat(env.router, "any-http-error", false).await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn model_unsupported_400_failovers() {
    assert_http_error_failover(
        400,
        json!({"error":"当前 API 不支持所选模型 gpt-5.6-sol","type":"error"}),
    )
    .await;
}

#[tokio::test]
async fn ordinary_400_failovers() {
    assert_http_error_failover(400, json!({"message":"参数错误"})).await;
}

#[tokio::test]
async fn ordinary_404_failovers() {
    assert_http_error_failover(404, json!({"message":"not found"})).await;
}

#[tokio::test]
async fn structured_2xx_error_failovers_non_stream() {
    assert_http_error_failover(200, json!({"error":{"message":"invalid model"}})).await;
}

#[tokio::test]
async fn structured_2xx_error_failovers_before_stream_commit() {
    let first = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_json(json!({"error":"model unavailable","type":"error"})),
        )
        .expect(1)
        .mount(&first)
        .await;
    let second = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(
                    "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\ndata: [DONE]\n\n",
                ),
        )
        .expect(1)
        .mount(&second)
        .await;

    let env = setup();
    add_two_candidates(&env, &first.uri(), &second.uri(), "stream-envelope");
    let response = chat(env.router, "stream-envelope", true).await;
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("ok"));
}

#[tokio::test]
async fn network_error_failovers() {
    // 先绑定再释放端口，得到当前没有监听者的本机地址。
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let dead_url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);

    let second = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object":"chat.completion", "choices":[{"message":{"content":"ok"}}]
        })))
        .expect(1)
        .mount(&second)
        .await;

    let env = setup();
    add_two_candidates(&env, &dead_url, &second.uri(), "network-failover");
    let response = chat(env.router, "network-failover", false).await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn every_request_starts_from_first_candidate() {
    let first = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .expect(2)
        .mount(&first)
        .await;
    let second = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object":"chat.completion", "choices":[{"message":{"content":"ok"}}]
        })))
        .expect(2)
        .mount(&second)
        .await;

    let env = setup();
    add_two_candidates(&env, &first.uri(), &second.uri(), "always-first");
    assert_eq!(
        chat(env.router.clone(), "always-first", false)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        chat(env.router, "always-first", false).await.status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn exhausted_http_candidates_forward_last_raw_response() {
    let first = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(400).set_body_string("first-error"))
        .expect(1)
        .mount(&first)
        .await;
    let second = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(404)
                .insert_header("x-upstream-marker", "last")
                .set_body_string("last-raw-error"),
        )
        .expect(1)
        .mount(&second)
        .await;

    let env = setup();
    add_two_candidates(&env, &first.uri(), &second.uri(), "last-http");
    let response = chat(env.router, "last-http", false).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.headers()["x-upstream-marker"], "last");
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&bytes[..], b"last-raw-error");
}

#[tokio::test]
async fn exhausted_transport_errors_return_gateway_error() {
    // 两个都是未监听端口 → 最后一次无上游响应，应返回明确网关错误（502）。
    let first_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let first_url = format!("http://{}", first_listener.local_addr().unwrap());
    drop(first_listener);
    let second_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let second_url = format!("http://{}", second_listener.local_addr().unwrap());
    drop(second_listener);

    let env = setup();
    add_two_candidates(&env, &first_url, &second_url, "all-transport");
    let response = chat(env.router, "all-transport", false).await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8_lossy(&bytes);
    assert!(
        body.contains("上游网络错误") || body.contains("message"),
        "应返回明确网关错误体: {body}"
    );
    assert!(!body.contains("test-key"));
}
