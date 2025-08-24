use axum::http::Request;
use axum::{routing::get, Router};
use tower::ServiceExt;

#[tokio::test]
async fn xff_and_ipv6_precedence() {
    let app = Router::new()
        .route("/status", get(|| async { "ok" }))
        .layer(solana_arbitrage_bot::web::rate_limit::RateLimitLayer::from_config(&None));

    // X-Forwarded-For multiple entries -> pick first
    let req = Request::builder()
        .method("GET")
        .uri("/status")
        .header("x-forwarded-for", "203.0.113.10, 10.0.0.1")
        .body(axum::body::Body::empty())
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();

    // X-Real-IP overrides fallback; IPv6 should parse
    let req = Request::builder()
        .method("GET")
        .uri("/status")
        .header("x-real-ip", "2001:db8::1")
        .body(axum::body::Body::empty())
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();
}
