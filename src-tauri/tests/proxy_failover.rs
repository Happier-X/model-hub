//! 集成测试：鉴权 + 故障转移换源。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use model_hub_lib::db::open_db;
use model_hub_lib::domain::apikey::CreateApiKeyPayload;
use model_hub_lib::domain::group::{CreateGroupPayload, GroupItemInput};
use model_hub_lib::domain::provider::CreateProviderPayload;
use model_hub_lib::domain::Stores;
use model_hub_lib::proxy::circuit::CircuitRegistry;
use model_hub_lib::proxy::forward::UpstreamClients;
use model_hub_lib::proxy::server::{build_router, AppState};

struct Env {
    _dir: tempfile::TempDir,
    stores: Stores,
    router: axum::Router,
    raw_key: String,
}

fn setup() -> Env {
    let dir = tempfile::tempdir().unwrap();
    let db = open_db(&dir.path().join("t.db")).unwrap();
    let stores = Stores::new(db);
    let circuits = CircuitRegistry::new();
    let clients = UpstreamClients::new();
    let created = stores
        .create_api_key(CreateApiKeyPayload {
            name: "t".into(),
            enabled: true,
        })
        .unwrap();
    let state = AppState {
        stores: stores.clone(),
        circuits,
        clients,
    };
    Env {
        _dir: dir,
        stores,
        router: build_router(state),
        raw_key: created.raw_key,
    }
}

#[tokio::test]
async fn rejects_missing_api_key() {
    let env = setup();
    let res = env
        .router
        .oneshot(
            Request::builder()
                .uri("/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn models_lists_groups() {
    let env = setup();
    env.stores
        .create_group(CreateGroupPayload {
            name: "demo-group".into(),
            auto_failover: true,
            items: vec![],
        })
        .unwrap();

    let res = env
        .router
        .oneshot(
            Request::builder()
                .uri("/v1/models")
                .header("Authorization", format!("Bearer {}", env.raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["data"][0]["id"], "demo-group");
}

#[tokio::test]
async fn failover_from_5xx_to_success() {
    let bad = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&bad)
        .await;

    let good = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id":"chatcmpl-1",
            "object":"chat.completion",
            "choices":[{"message":{"role":"assistant","content":"ok"}}]
        })))
        .mount(&good)
        .await;

    let env = setup();
    let p_bad = env
        .stores
        .create_provider(CreateProviderPayload {
            name: "bad".into(),
            base_url: format!("{}/v1", bad.uri()),
            api_key: "k".into(),
            enabled: true,
        })
        .unwrap();
    let p_good = env
        .stores
        .create_provider(CreateProviderPayload {
            name: "good".into(),
            base_url: format!("{}/v1", good.uri()),
            api_key: "k".into(),
            enabled: true,
        })
        .unwrap();
    env.stores
        .create_group(CreateGroupPayload {
            name: "g1".into(),
            auto_failover: true,
            items: vec![
                GroupItemInput {
                    provider_id: p_bad.id,
                    upstream_model: "m-bad".into(),
                },
                GroupItemInput {
                    provider_id: p_good.id,
                    upstream_model: "m-good".into(),
                },
            ],
        })
        .unwrap();

    let body = json!({
        "model": "g1",
        "messages": [{"role":"user","content":"hi"}],
        "stream": false
    });
    let res = env
        .router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", env.raw_key))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["choices"][0]["message"]["content"], "ok");
}
