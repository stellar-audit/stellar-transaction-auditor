//! Integration tests for the backend API.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

// Helper: create a test app with seeded demo data.
fn test_app() -> axum::Router {
    let config = stellar_transaction_auditor_backend::state::Config {
        bind_addr: "0.0.0.0:0".parse().unwrap(),
        contract_id: String::new(),
        soroban_rpc_url: "http://localhost".into(),
        horizon_url: "http://localhost".into(),
        network_passphrase: "Test SDF Network ; September 2015".into(),
        network_name: "TESTNET".into(),
    };
    let state = stellar_transaction_auditor_backend::state::AppState::new(config);
    state.store.seed_demo_data();
    stellar_transaction_auditor_backend::app_router(state)
}

async fn body_to_string(body: Body) -> String {
    let bytes = body.collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn health_returns_ok() {
    let app = test_app();
    let res = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_to_string(res.into_body()).await;
    assert!(body.contains("\"status\":\"ok\""));
    assert!(body.contains("\"version\""));
}

#[tokio::test]
async fn status_returns_config_info() {
    let app = test_app();
    let res = app
        .oneshot(Request::builder().uri("/api/v1/status").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_to_string(res.into_body()).await;
    assert!(body.contains("\"network\":\"TESTNET\""));
    assert!(body.contains("\"entry_count\":5"));
}

#[tokio::test]
async fn list_entries_returns_seeded_data() {
    let app = test_app();
    let res = app
        .oneshot(Request::builder().uri("/api/v1/entries").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_to_string(res.into_body()).await;
    assert!(body.contains("\"total\":5"));
}

#[tokio::test]
async fn get_entry_by_id() {
    let app = test_app();
    let res = app
        .oneshot(Request::builder().uri("/api/v1/entries/0").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_to_string(res.into_body()).await;
    assert!(body.contains("\"id\":0"));
}

#[tokio::test]
async fn get_nonexistent_entry_returns_404() {
    let app = test_app();
    let res = app
        .oneshot(Request::builder().uri("/api/v1/entries/999").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn verify_chain_returns_true_for_valid_chain() {
    let app = test_app();
    let res = app
        .oneshot(Request::builder().uri("/api/v1/verify?from=0&to=4").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_to_string(res.into_body()).await;
    assert!(body.contains("\"verified\":true"));
}

#[tokio::test]
async fn entry_count_endpoint() {
    let app = test_app();
    let res = app
        .oneshot(Request::builder().uri("/api/v1/entries/count").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_to_string(res.into_body()).await;
    assert!(body.contains("\"count\":5"));
}

#[tokio::test]
async fn pagination_with_limit() {
    let app = test_app();
    let res = app
        .oneshot(Request::builder().uri("/api/v1/entries?start=0&limit=2").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_to_string(res.into_body()).await;
    assert!(body.contains("\"count\":2"));
    assert!(body.contains("\"limit\":2"));
}

#[tokio::test]
async fn invalid_limit_returns_400() {
    let app = test_app();
    let res = app
        .oneshot(Request::builder().uri("/api/v1/entries?limit=0").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn check_auditor_endpoint() {
    let app = test_app();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auditors/GA2C5RFPE6GCKMY3US5GAB29QZYE7DYXTZPGHLDJ5YBJO7VOZKAUBP2X")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_to_string(res.into_body()).await;
    assert!(body.contains("\"is_auditor\":true"));
}
