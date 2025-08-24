//! Per-IP rate limiting middleware with path categories and standard headers

use axum::extract::Request;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tower::{Layer, Service};

use super::metrics::record_rate_limited; // now without IP label
use super::RateLimitsConfig;

#[derive(Clone)]
pub struct RateLimitLayer {
    pub default_per_min: u64,
    pub control_per_min: u64,
    pub config_per_min: u64,
    pub status_per_min: u64,
    pub window: Duration,
}

impl RateLimitLayer {
    pub fn from_config(cfg: &Option<RateLimitsConfig>) -> Self {
        let d = cfg.as_ref();
        let default_per_min = d.and_then(|c| c.default_per_ip_per_minute).unwrap_or(100);
        let control_per_min = d.and_then(|c| c.control_per_ip_per_minute).unwrap_or(10);
        let config_per_min = d.and_then(|c| c.config_per_ip_per_minute).unwrap_or(20);
        let status_per_min = d.and_then(|c| c.status_per_ip_per_minute).unwrap_or(100);
        Self {
            default_per_min,
            control_per_min,
            config_per_min,
            status_per_min,
            window: Duration::from_secs(60),
        }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitMiddleware<S>;
    fn layer(&self, inner: S) -> Self::Service {
        RateLimitMiddleware::new(
            inner,
            self.default_per_min,
            self.control_per_min,
            self.config_per_min,
            self.status_per_min,
            self.window,
        )
    }
}

#[derive(Clone)]
pub struct RateLimitMiddleware<S> {
    inner: S,
    // (ip, category) -> state
    shards: Arc<Vec<Shard>>,
    default_per_min: u64,
    control_per_min: u64,
    config_per_min: u64,
    status_per_min: u64,
    window: Duration,
    last_cleanup: Arc<Mutex<Instant>>,
}

#[derive(Debug, Clone)]
struct Bucket {
    last_refill: Instant,
    tokens: f64, // token bucket (requests)
}

impl<S> RateLimitMiddleware<S> {
    pub fn new(
        inner: S,
        default_per_min: u64,
        control_per_min: u64,
        config_per_min: u64,
        status_per_min: u64,
        window: Duration,
    ) -> Self {
        Self {
            inner,
            shards: Arc::new(
                (0..SHARDS)
                    .map(|_| Shard {
                        map: Arc::new(Mutex::new(HashMap::new())),
                    })
                    .collect(),
            ),
            default_per_min,
            control_per_min,
            config_per_min,
            status_per_min,
            window,
            last_cleanup: Arc::new(Mutex::new(Instant::now())),
        }
    }
}

const SHARDS: usize = 64;

#[derive(Clone)]
struct Shard {
    map: Arc<Mutex<HashMap<(IpAddr, &'static str), Bucket>>>,
}

fn shard_index(ip: &IpAddr, endpoint: &'static str) -> usize {
    use std::collections::hash_map::DefaultHasher;
    let mut h = DefaultHasher::new();
    ip.hash(&mut h);
    endpoint.hash(&mut h);
    (h.finish() as usize) % SHARDS
}

fn endpoint_category(path: &str) -> &'static str {
    if path.starts_with("/api/control/") {
        "control"
    } else if path == "/api/config" {
        "config"
    } else if path == "/api/status" || path == "/api/transactions" {
        "status"
    } else if path == "/metrics" || path == "/health" || path.starts_with("/static") || path == "/"
    {
        "public"
    } else {
        "other"
    }
}

fn limit_for_category(
    mw: &RateLimitMiddleware<impl Service<Request, Response = Response>>,
    cat: &str,
) -> Option<u64> {
    match cat {
        "control" => Some(mw.control_per_min),
        "config" => Some(mw.config_per_min),
        "status" => Some(mw.status_per_min),
        "public" => None,
        _ => Some(mw.default_per_min),
    }
}

fn parse_first_ip_from_xff(header: &str) -> Option<IpAddr> {
    let first = header.split(',').next()?.trim();
    first.parse().ok()
}

pub(crate) fn extract_ip(headers: &HeaderMap, req: &Request) -> Option<IpAddr> {
    if let Some(v) = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok()) {
        if let Some(ip) = parse_first_ip_from_xff(v) {
            return Some(ip);
        }
    }
    if let Some(v) = headers.get("x-real-ip").and_then(|h| h.to_str().ok()) {
        if let Ok(ip) = v.parse() {
            return Some(ip);
        }
    }
    // Fallback to connection info if available
    if let Some(addr) = req.extensions().get::<std::net::SocketAddr>() {
        return Some(addr.ip());
    }
    None
}

fn set_headers(
    mut resp: Response,
    limit: Option<u64>,
    remaining: Option<u64>,
    reset_ts: u64,
    retry_after: Option<u64>,
) -> Response {
    let headers = resp.headers_mut();
    if let Some(l) = limit {
        headers.insert(
            "X-RateLimit-Limit",
            HeaderValue::from_str(&l.to_string()).unwrap(),
        );
    } else {
        headers.insert("X-RateLimit-Limit", HeaderValue::from_static("0"));
    }
    headers.insert(
        "X-RateLimit-Remaining",
        HeaderValue::from_str(&remaining.unwrap_or(0).to_string()).unwrap(),
    );
    headers.insert(
        "X-RateLimit-Reset",
        HeaderValue::from_str(&reset_ts.to_string()).unwrap(),
    );
    if let Some(secs) = retry_after {
        headers.insert(
            "Retry-After",
            HeaderValue::from_str(&secs.to_string()).unwrap(),
        );
    }
    resp
}

impl<S> Service<Request> for RateLimitMiddleware<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let path = req.uri().path().to_string();
        let endpoint = endpoint_category(&path);
        let headers = req.headers().clone();
        let ip = extract_ip(&headers, &req);

        // For public endpoints do nothing but set headers with no limit
        if endpoint == "public" {
            let fut = self.inner.call(req);
            let reset_ts = (std::time::SystemTime::now() + self.window)
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            return Box::pin(async move {
                let resp = fut.await?;
                let resp = set_headers(resp, None, None, reset_ts, None);
                Ok(resp)
            });
        }

        let limit = limit_for_category(self, endpoint).unwrap_or(0);
        let now = Instant::now();
        let reset_at = now + self.window;
        let reset_ts = (std::time::SystemTime::now() + self.window)
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let remaining: u64;
        let mut over_limit = false;

        if let Some(ip) = ip {
            let refill_per_sec = (limit as f64) / (self.window.as_secs_f64());
            let shard_idx = shard_index(&ip, endpoint);
            let shard = &self.shards[shard_idx];
            let mut map = shard.map.lock().expect("rate limit lock");
            let entry = map.entry((ip, endpoint)).or_insert(Bucket {
                last_refill: now,
                tokens: limit as f64,
            });
            let elapsed = now.duration_since(entry.last_refill).as_secs_f64();
            entry.tokens = (entry.tokens + elapsed * refill_per_sec).min(limit as f64);
            entry.last_refill = now;

            if entry.tokens >= 1.0 {
                entry.tokens -= 1.0;
                remaining = entry.tokens.floor() as u64;
            } else {
                over_limit = true;
                remaining = 0;
            }
        } else {
            // No IP -> treat as over limit to be safe
            over_limit = true;
            remaining = 0;
        }

        if over_limit {
            record_rate_limited(endpoint);
            let mut resp = Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body(axum::body::Body::from("Too Many Requests"))
                .unwrap();
            let retry_after = reset_at.duration_since(now).as_secs();
            resp = set_headers(
                resp,
                Some(limit),
                Some(remaining),
                reset_ts,
                Some(retry_after),
            );
            // Periodic cleanup of inactive buckets
            {
                let mut last = self.last_cleanup.lock().unwrap();
                if last.elapsed() > Duration::from_secs(60) {
                    *last = Instant::now();
                    for shard in self.shards.iter() {
                        let mut map = shard.map.lock().unwrap();
                        let nowi = Instant::now();
                        map.retain(|_, b| {
                            nowi.duration_since(b.last_refill) < Duration::from_secs(600)
                        });
                    }
                }
            }

            return Box::pin(async move { Ok(resp) });
        }

        let fut = self.inner.call(req);
        Box::pin(async move {
            let resp = fut.await?;
            let resp = set_headers(resp, Some(limit), Some(remaining), reset_ts, None);
            Ok(resp)
        })
    }
}
