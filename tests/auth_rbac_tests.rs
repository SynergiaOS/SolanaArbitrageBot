use axum::{routing::get, Router};
use tower::ServiceExt;
use axum::http::{Request, StatusCode};

#[tokio::test]
async fn control_requires_admin_key_and_allowlisted_ip() {
    let cfg = solana_arbitrage_bot::web::WebConfig {
        enabled: true,
        host: "127.0.0.1".into(),
        port: 0,
        auth_token: Some("default".into()),
        database_path: "./data/test.db".into(),
        allowed_origins: None,
        rate_limits: None,
        api_keys: Some(solana_arbitrage_bot::web::ApiKeysConfig {
            admin_api_key: Some("admin123".into()),
            config_api_key: Some("config123".into()),
            status_api_key: Some("status123".into()),
        }),
        control_ip_allowlist: Some(vec!["127.0.0.1/32".into()]),
    };

    let app = Router::new()
        .route("/api/control/stop", get(|| async { "ok" }))
        .layer(solana_arbitrage_bot::web::auth::auth_middleware_from_config(&cfg));

    // Wrong IP (no X-Real-IP) -> forbidden
    let req = Request::builder().method("GET").uri("/api/control/stop")
        .header("x-api-key", "admin123")
        .body(axum::body::Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Allowlisted IP + admin key -> OK
    let req = Request::builder().method("GET").uri("/api/control/stop")
        .header("x-api-key", "admin123")
        .header("x-real-ip", "127.0.0.1")
        .body(axum::body::Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn config_requires_config_or_default_token() {
    let cfg = solana_arbitrage_bot::web::WebConfig::default();
    let app = Router::new()
        .route("/api/config", get(|| async { "ok" }))
        .layer(solana_arbitrage_bot::web::auth::auth_middleware_from_config(&cfg));

    // No token/key -> unauthorized
    let req = Request::builder().method("GET").uri("/api/config").body(axum::body::Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

