use axum::{routing::get, Router};
use tower::ServiceExt;

#[tokio::test]
async fn metrics_endpoint_works() {
    let app = Router::new().route("/metrics", get(solana_arbitrage_bot::web::metrics::metrics_handler));
    let req = axum::http::Request::builder().method("GET").uri("/metrics").body(axum::body::Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
}

