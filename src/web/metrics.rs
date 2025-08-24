use axum::{extract::Request, response::Response};
use prometheus::{Encoder, HistogramVec, IntCounterVec, IntCounter, Opts, TextEncoder, register_histogram_vec, register_int_counter, register_int_counter_vec};
use std::task::{Context, Poll};
use tower::{Layer, Service};
use std::time::Instant;

#[derive(Clone)]
pub struct MetricsLayer;

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsMiddleware<S>;
    fn layer(&self, inner: S) -> Self::Service {
        MetricsMiddleware { inner }
    }
}

#[derive(Clone)]
pub struct MetricsMiddleware<S> {
    inner: S,
}

fn endpoint_category(path: &str) -> &'static str {
    if path.starts_with("/api/control/") { "control" }
    else if path == "/api/config" { "config" }
    else if path == "/api/status" || path == "/api/transactions" { "status" }
    else if path == "/metrics" || path == "/health" || path.starts_with("/static") || path == "/" { "public" }
    else { "other" }
}

// Global metrics (default registry)
lazy_static::lazy_static! {
    static ref HTTP_REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "http_requests_total",
        "Total HTTP requests",
        & ["method", "endpoint", "status"]
    ).unwrap();

    static ref HTTP_REQUEST_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "http_request_duration_seconds",
        "HTTP request duration by endpoint",
        & ["endpoint"],
        vec![0.005,0.01,0.025,0.05,0.1,0.25,0.5,1.0,2.5,5.0]
    ).unwrap();

    static ref HTTP_REQUESTS_RATE_LIMITED_TOTAL: IntCounterVec = register_int_counter_vec!(
        "http_requests_rate_limited_total",
        "Total rate limited HTTP requests",
        & ["endpoint"]
    ).unwrap();

    static ref METRICS_SCRAPES_TOTAL: IntCounter = register_int_counter!(
        Opts::new("metrics_scrapes_total", "Total /metrics scrapes")
    ).unwrap();
}

impl<S> Service<Request> for MetricsMiddleware<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let method = req.method().clone();
        let path = req.uri().path().to_string();
        let endpoint = endpoint_category(&path);
        let start = Instant::now();

        let fut = self.inner.call(req);
        Box::pin(async move {
            let resp = fut.await?;
            let status = resp.status().as_u16();
            HTTP_REQUESTS_TOTAL.with_label_values(&[method.as_str(), endpoint, &status.to_string()]).inc();
            HTTP_REQUEST_DURATION_SECONDS.with_label_values(&[endpoint]).observe(start.elapsed().as_secs_f64());
            Ok(resp)
        })
    }
}

pub async fn metrics_handler() -> Response {
    METRICS_SCRAPES_TOTAL.inc();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    Response::builder()
        .header("content-type", encoder.format_type())
        .body(axum::body::Body::from(buffer))
        .unwrap()
}

pub fn record_rate_limited(endpoint: &str) {
    HTTP_REQUESTS_RATE_LIMITED_TOTAL.with_label_values(&[endpoint]).inc();
}

