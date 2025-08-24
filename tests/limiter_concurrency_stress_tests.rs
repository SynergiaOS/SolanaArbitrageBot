use axum::{routing::get, Router};
use tower::ServiceExt;
use axum::http::Request;

#[tokio::test]
async fn limiter_concurrency_stress() {
    // limit status to 20/min for test
    let limits = solana_arbitrage_bot::web::RateLimitsConfig {
        default_per_ip_per_minute: Some(1000),
        control_per_ip_per_minute: Some(1000),
        config_per_ip_per_minute: Some(1000),
        status_per_ip_per_minute: Some(20),
    };

    let app = Router::new()
        .route("/api/status", get(|| async { "ok" }))
        .layer(solana_arbitrage_bot::web::rate_limit::RateLimitLayer::from_config(&Some(limits)));

    let mut handles = vec![];
    for _ in 0..30 {
        let appc = app.clone();
        handles.push(tokio::spawn(async move {
            let req = Request::builder().method("GET").uri("/api/status")
                .header("x-real-ip", "192.0.2.1")
                .body(axum::body::Body::empty()).unwrap();
            let _ = appc.oneshot(req).await.unwrap();
        }));
    }

    for h in handles { let _ = h.await; }
}

