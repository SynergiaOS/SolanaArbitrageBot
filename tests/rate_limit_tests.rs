use axum::http::{Request, StatusCode};
use axum::body::Body;
use tower::ServiceExt;

#[tokio::test]
async fn rate_limit_exceeded_returns_429() {
    use std::convert::Infallible;
    use tower::{service_fn, ServiceBuilder};

    // Service: always returns 200 OK with body "ok"
    let base = service_fn(|_req: Request<Body>| async move {
        Ok::<axum::response::Response<Body>, Infallible>(axum::response::Response::new(Body::from("ok")))
    });

    // Custom limits: status <= 5/min for test
    let cfg = solana_arbitrage_bot::web::RateLimitsConfig {
        default_per_ip_per_minute: Some(1000),
        control_per_ip_per_minute: Some(1000),
        config_per_ip_per_minute: Some(1000),
        status_per_ip_per_minute: Some(5),
    };

    let svc = ServiceBuilder::new()
        .layer(solana_arbitrage_bot::web::rate_limit::RateLimitLayer::from_config(&Some(cfg)))
        .service(base);

    // Send 6 requests to a status endpoint within the same second from same IP
    for i in 0..5 {
        let req = Request::builder()
            .method("GET")
            .uri("/api/status")
            .header("x-real-ip", "1.2.3.4")
            .body(Body::empty())
            .unwrap();
        let resp = svc.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "request {} should be ok", i);
    }

    // The 6th request should be rate limited
    let req = Request::builder()
        .method("GET")
        .uri("/api/status")
        .header("x-real-ip", "1.2.3.4")
        .body(Body::empty())
        .unwrap();
    let resp = svc.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn different_ips_have_separate_budgets() {
    use std::convert::Infallible;
    use tower::{service_fn, ServiceBuilder};

    let base = service_fn(|_req: Request<Body>| async move {
        Ok::<axum::response::Response<Body>, Infallible>(axum::response::Response::new(Body::from("ok")))
    });

    let cfg = solana_arbitrage_bot::web::RateLimitsConfig {
        default_per_ip_per_minute: Some(5),
        control_per_ip_per_minute: Some(5),
        config_per_ip_per_minute: Some(5),
        status_per_ip_per_minute: Some(5),
    };

    let svc = ServiceBuilder::new()
        .layer(solana_arbitrage_bot::web::rate_limit::RateLimitLayer::from_config(&Some(cfg)))
        .service(base);

    for _ in 0..5 {
        let req1 = Request::builder().method("GET").uri("/api/status").header("x-real-ip", "1.1.1.1").body(Body::empty()).unwrap();
        let req2 = Request::builder().method("GET").uri("/api/status").header("x-real-ip", "2.2.2.2").body(Body::empty()).unwrap();
        let r1 = svc.clone().oneshot(req1).await.unwrap();
        let r2 = svc.clone().oneshot(req2).await.unwrap();
        assert_eq!(r1.status(), StatusCode::OK);
        assert_eq!(r2.status(), StatusCode::OK);
    }

    // Next request from 1.1.1.1 should be 429, 2.2.2.2 still OK if not exceeded
    let r1 = svc.clone().oneshot(
        Request::builder().method("GET").uri("/api/status").header("x-real-ip", "1.1.1.1").body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(r1.status(), StatusCode::TOO_MANY_REQUESTS);
}

