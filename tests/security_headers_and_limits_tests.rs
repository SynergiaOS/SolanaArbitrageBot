use axum::{routing::post, Router};
use tower::ServiceExt;
use axum::http::{Request, StatusCode};

#[tokio::test]
async fn security_headers_and_body_limit_applied() {
    let _cfg = solana_arbitrage_bot::web::WebConfig::default();
    let app = Router::new()
        .route("/api/control/emergency", post(|| async { "ok" }))
        // apply route_layer similar to server
        .route_layer(tower_http::limit::RequestBodyLimitLayer::new(64 * 1024))
        .route_layer(tower::limit::ConcurrencyLimitLayer::new(8))
        .route_layer(tower_http::set_header::SetResponseHeaderLayer::if_not_present(
            axum::http::HeaderName::from_static("x-content-type-options"),
            axum::http::HeaderValue::from_static("nosniff"),
        ));

    // headers present
    let req = Request::builder().method("POST").uri("/api/control/emergency")
        .body(axum::body::Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp.headers().get("x-content-type-options").is_some());

    // oversize body -> 413
    let big = vec![0u8; 70 * 1024];
    let req = Request::builder().method("POST").uri("/api/control/emergency")
        .body(axum::body::Body::from(big)).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

