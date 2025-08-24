use axum::http::{Request, StatusCode};
use axum::body::Body;
use tower::{ServiceExt, ServiceBuilder, service_fn};

#[tokio::test]
async fn metrics_rate_limited_counter_increments() {
    let limits = solana_arbitrage_bot::web::RateLimitsConfig {
        default_per_ip_per_minute: Some(1000),
        control_per_ip_per_minute: Some(1000),
        config_per_ip_per_minute: Some(1000),
        status_per_ip_per_minute: Some(1),
    };

    // Build a tower Service chain to ensure shared state across requests
    let base = service_fn(|_req: Request<Body>| async move {
        Ok::<axum::response::Response<Body>, std::convert::Infallible>(
            axum::response::Response::new(Body::from("ok"))
        )
    });

    let svc = ServiceBuilder::new()
        .layer(solana_arbitrage_bot::web::metrics::MetricsLayer)
        .layer(solana_arbitrage_bot::web::rate_limit::RateLimitLayer::from_config(&Some(limits)))
        .service(base);

    let req = Request::builder().method("GET").uri("/api/status")
        .header("x-real-ip", "192.0.2.3")
        .body(Body::empty()).unwrap();
    let _ = svc.clone().oneshot(req).await.unwrap();

    // second request should be 429
    let req = Request::builder().method("GET").uri("/api/status")
        .header("x-real-ip", "192.0.2.3")
        .body(Body::empty()).unwrap();
    let resp = svc.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
}

