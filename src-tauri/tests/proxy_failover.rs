//! 集成测试：本机无鉴权访问 + 故障转移换源 + 流式静默超时日志。

use std::time::Duration;

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
use model_hub_lib::proxy::circuit::CircuitRegistry;
use model_hub_lib::proxy::forward::{ForwardPolicy, UpstreamClients};
use model_hub_lib::proxy::server::{build_router, AppState};

struct Env {
    _dir: tempfile::TempDir,
    stores: Stores,
    circuits: CircuitRegistry,
    router: axum::Router,
}

fn setup_with_policy(policy: ForwardPolicy) -> Env {
    let dir = tempfile::tempdir().unwrap();
    let db = open_db(&dir.path().join("t.db")).unwrap();
    let stores = Stores::new(db);
    let circuits = CircuitRegistry::new();
    let clients = UpstreamClients::new();
    let state = AppState {
        stores: stores.clone(),
        circuits: circuits.clone(),
        clients,
        forward_policy: policy,
    };
    Env {
        _dir: dir,
        stores,
        circuits,
        router: build_router(state),
    }
}

fn setup() -> Env {
    setup_with_policy(ForwardPolicy::default())
}

#[tokio::test]
async fn allows_missing_api_key() {
    let env = setup();
    env.stores
        .create_group(CreateGroupPayload {
            name: "open-local".into(),
            auto_failover: true,
            items: vec![],
        })
        .unwrap();
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
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn ignores_arbitrary_authorization_header() {
    let env = setup();
    let res = env
        .router
        .oneshot(
            Request::builder()
                .uri("/v1/models")
                .header("Authorization", "Bearer arbitrary-value")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
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

/// 简易上游：立刻返回 HTTP 头 + 首段 body，然后挂起（用于触发静默超时）。
async fn spawn_hanging_after_first_chunk_upstream() -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let Ok((mut sock, _)) = listener.accept().await else {
            return;
        };
        let mut buf = vec![0u8; 8192];
        let mut req = Vec::new();
        loop {
            match sock.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    req.extend_from_slice(&buf[..n]);
                    if req.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
                Err(_) => return,
            }
        }
        let payload = b"data: {\"partial\":true}\n\n";
        // 声明更长 body，只写出首段，迫使下游在后续 chunk 上等待。
        let declared = payload.len() + 4096;
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {declared}\r\nConnection: close\r\n\r\n"
        );
        let _ = sock.write_all(head.as_bytes()).await;
        let _ = sock.write_all(payload).await;
        tokio::time::sleep(Duration::from_secs(120)).await;
    });
    format!("http://{addr}")
}

/// 流式：首包后静默超时只写一条失败日志（无误导性 200 空 error），并记熔断失败。
#[tokio::test]
async fn stream_idle_timeout_single_failure_log() {
    let base = spawn_hanging_after_first_chunk_upstream().await;

    let env = setup_with_policy(ForwardPolicy {
        stream_idle_timeout: Duration::from_millis(80),
    });
    let provider = env
        .stores
        .create_provider(CreateProviderPayload {
            name: "slow-stream".into(),
            base_url: format!("{base}/v1"),
            api_key: "k".into(),
            enabled: true,
        })
        .unwrap();
    env.stores
        .create_group(CreateGroupPayload {
            name: "g-stream".into(),
            auto_failover: true,
            items: vec![GroupItemInput {
                provider_id: provider.id,
                upstream_model: "m-stream".into(),
            }],
        })
        .unwrap();

    let body = json!({
        "model": "g-stream",
        "messages": [{"role":"user","content":"hi"}],
        "stream": true
    });
    let res = env
        .router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // HTTP 头在 prime 成功时仍可能是 200；关键是消费 body 后日志语义。
    assert_eq!(res.status(), StatusCode::OK);
    // 静默超时会使 body 以错误结束；忽略 to_bytes 错误，回调应在 yield Err 前已写入日志。
    let _ = axum::body::to_bytes(res.into_body(), usize::MAX).await;

    // 等待终态回调落库（兜底）
    tokio::time::sleep(Duration::from_millis(100)).await;

    let logs = env
        .stores
        .list_logs(model_hub_lib::domain::log::LogQuery {
            page: 1,
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    // 静默超时路径：不得留下「仅 200 且 error 为空」作为结论；应为单条失败。
    assert_eq!(logs.items.len(), 1, "期望仅一条最终日志，实际: {:?}", logs);
    let log = &logs.items[0];
    assert_eq!(log.status_code, 504);
    assert_eq!(log.error, "流式静默超时");
    assert_eq!(log.provider_name, "slow-stream");
    assert_eq!(log.upstream_model, "m-stream");
    assert!(!log.error.contains("sk-"));
    assert!(!log.error.contains("messages"));

    // 首包 record_success 后 idle 会 record_failure → consecutive_failures >= 1
    assert!(
        env.circuits.consecutive_failures(provider.id) >= 1,
        "静默超时应记录熔断失败"
    );
}

/// 流式正常结束：记一条 200 成功日志。
#[tokio::test]
async fn stream_success_single_ok_log() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(
            "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\ndata: [DONE]\n\n",
        ))
        .mount(&upstream)
        .await;

    let env = setup();
    let provider = env
        .stores
        .create_provider(CreateProviderPayload {
            name: "ok-stream".into(),
            base_url: format!("{}/v1", upstream.uri()),
            api_key: "k".into(),
            enabled: true,
        })
        .unwrap();
    env.stores
        .create_group(CreateGroupPayload {
            name: "g-ok".into(),
            auto_failover: false,
            items: vec![GroupItemInput {
                provider_id: provider.id,
                upstream_model: "m-ok".into(),
            }],
        })
        .unwrap();

    let body = json!({
        "model": "g-ok",
        "messages": [{"role":"user","content":"hi"}],
        "stream": true
    });
    let res = env
        .router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let _ = axum::body::to_bytes(res.into_body(), usize::MAX).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let logs = env
        .stores
        .list_logs(model_hub_lib::domain::log::LogQuery {
            page: 1,
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(logs.items.len(), 1, "期望单条成功日志: {:?}", logs);
    assert_eq!(logs.items[0].status_code, 200);
    assert!(logs.items[0].error.is_empty());
    assert_eq!(logs.items[0].provider_name, "ok-stream");
}

/// 非流式成功：server 侧立即写一条日志，统计 total 同步增加。
#[tokio::test]
async fn non_stream_success_writes_log_and_stats() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id":"chatcmpl-1",
            "object":"chat.completion",
            "choices":[{"message":{"role":"assistant","content":"ok"}}]
        })))
        .mount(&upstream)
        .await;

    let env = setup();
    let provider = env
        .stores
        .create_provider(CreateProviderPayload {
            name: "p-ok".into(),
            base_url: format!("{}/v1", upstream.uri()),
            api_key: "k".into(),
            enabled: true,
        })
        .unwrap();
    env.stores
        .create_group(CreateGroupPayload {
            name: "g-nonstream".into(),
            auto_failover: false,
            items: vec![GroupItemInput {
                provider_id: provider.id,
                upstream_model: "m".into(),
            }],
        })
        .unwrap();

    let body = json!({
        "model": "g-nonstream",
        "messages": [{"role":"user","content":"hi"}],
        "stream": false
    });
    let res = env
        .router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let logs = env
        .stores
        .list_logs(model_hub_lib::domain::log::LogQuery {
            page: 1,
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(logs.total, 1, "非流式成功应有日志: {:?}", logs);
    assert_eq!(logs.items[0].status_code, 200);
    assert_eq!(logs.items[0].group_name, "g-nonstream");

    let stats = env.stores.request_stats_today().unwrap();
    assert_eq!(stats.total, 1);
    assert_eq!(stats.success, 1);
}

/// 流式 body 被 drop（模拟客户端提前断开）：仍写入中断日志，供 UI 统计。
#[tokio::test]
async fn stream_abort_on_drop_writes_log() {
    let base = spawn_hanging_after_first_chunk_upstream().await;

    let env = setup_with_policy(ForwardPolicy {
        stream_idle_timeout: Duration::from_secs(30),
    });
    let provider = env
        .stores
        .create_provider(CreateProviderPayload {
            name: "hang".into(),
            base_url: format!("{base}/v1"),
            api_key: "k".into(),
            enabled: true,
        })
        .unwrap();
    env.stores
        .create_group(CreateGroupPayload {
            name: "g-abort".into(),
            auto_failover: false,
            items: vec![GroupItemInput {
                provider_id: provider.id,
                upstream_model: "m".into(),
            }],
        })
        .unwrap();

    let body = json!({
        "model": "g-abort",
        "messages": [{"role":"user","content":"hi"}],
        "stream": true
    });
    let res = env
        .router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // 不读 body，直接 drop → 触发 StreamState::Drop 写 abort 日志
    drop(res);

    tokio::time::sleep(Duration::from_millis(100)).await;

    let logs = env
        .stores
        .list_logs(model_hub_lib::domain::log::LogQuery {
            page: 1,
            page_size: 50,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(logs.items.len(), 1, "drop 中断应有日志: {:?}", logs);
    assert_eq!(logs.items[0].status_code, 499);
    assert!(logs.items[0].error.contains("断开"));
    assert_eq!(logs.items[0].provider_name, "hang");
    // 不记熔断失败
    assert_eq!(env.circuits.consecutive_failures(provider.id), 0);
}
