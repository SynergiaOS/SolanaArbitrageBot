//! Authentication middleware for the web dashboard

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    response::Response,
};
use std::task::{Context, Poll};
use tower::{Layer, Service};

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Role {
    Admin,
    Config,
    Status,
    Other,
}

pub fn required_role(path: &str) -> Role {
    if path.starts_with("/api/control/") {
        Role::Admin
    } else if path == "/api/config" {
        Role::Config
    } else if path == "/api/status" || path == "/api/transactions" {
        Role::Status
    } else {
        Role::Other
    }
}

/// Authentication middleware layer
#[derive(Clone)]
pub struct AuthLayer {
    default_token: Option<String>,
    admin_api_key: Option<String>,
    config_api_key: Option<String>,
    status_api_key: Option<String>,
    control_ip_allowlist: Option<Vec<cidr::AnyIpCidr>>,
}

impl AuthLayer {
    pub fn new(
        default_token: Option<String>,
        admin_api_key: Option<String>,
        config_api_key: Option<String>,
        status_api_key: Option<String>,
        control_ip_allowlist: Option<Vec<cidr::AnyIpCidr>>,
    ) -> Self {
        Self {
            default_token,
            admin_api_key,
            config_api_key,
            status_api_key,
            control_ip_allowlist,
        }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            inner,
            default_token: self.default_token.clone(),
            admin_api_key: self.admin_api_key.clone(),
            config_api_key: self.config_api_key.clone(),
            status_api_key: self.status_api_key.clone(),
            control_ip_allowlist: self.control_ip_allowlist.clone(),
        }
    }
}

/// Authentication middleware service
#[derive(Clone)]
pub struct AuthMiddleware<S> {
    inner: S,
    default_token: Option<String>,
    admin_api_key: Option<String>,
    config_api_key: Option<String>,
    status_api_key: Option<String>,
    control_ip_allowlist: Option<Vec<cidr::AnyIpCidr>>,
}

impl<S> Service<Request> for AuthMiddleware<S>
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

    fn call(&mut self, request: Request) -> Self::Future {
        // Skip auth for public endpoints
        let path = request.uri().path().to_string();
        if is_public_endpoint(&path) {
            let future = self.inner.call(request);
            return Box::pin(future);
        }

        // Role-based auth: determine required role and key
        let headers = request.headers().clone();
        let required = required_role(&path);

        // Optional: enforce IP allowlist for control endpoints
        if required == Role::Admin {
            if let Some(ref cidrs) = self.control_ip_allowlist {
                // Get IP via X-Real-IP/XFF fallback (same as limiter logic)
                let ip = crate::web::rate_limit::extract_ip(&headers, &request);
                let allowed = ip
                    .map(|ip| cidrs.iter().any(|c| c.contains(&ip)))
                    .unwrap_or(false);
                if !allowed {
                    return Box::pin(async move {
                        Ok(Response::builder()
                            .status(StatusCode::FORBIDDEN)
                            .body("Forbidden".into())
                            .unwrap())
                    });
                }
            }
        }

        let authed = match required {
            Role::Admin => self
                .admin_api_key
                .as_deref()
                .map(|key| is_authenticated(&headers, key))
                .unwrap_or(false),
            Role::Config => self
                .config_api_key
                .as_deref()
                .map(|key| is_authenticated(&headers, key))
                .or_else(|| {
                    self.default_token
                        .as_deref()
                        .map(|k| is_authenticated(&headers, k))
                })
                .unwrap_or(false),
            Role::Status => self
                .status_api_key
                .as_deref()
                .map(|key| is_authenticated(&headers, key))
                .or_else(|| {
                    self.default_token
                        .as_deref()
                        .map(|k| is_authenticated(&headers, k))
                })
                .unwrap_or(false),

            // fallthrough for match continues below
            Role::Other => self
                .default_token
                .as_deref()
                .map(|k| is_authenticated(&headers, k))
                .unwrap_or(true),
        };

        if !authed {
            return Box::pin(async move {
                Ok(Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body("Unauthorized".into())
                    .unwrap())
            });
        }

        let future = self.inner.call(request);
        Box::pin(future)
    }
}

/// Check if the endpoint is public (doesn't require authentication)
fn is_public_endpoint(path: &str) -> bool {
    if path == "/" || path == "/health" || path == "/metrics" {
        return true;
    }
    if path.starts_with("/static") {
        return true;
    }
    false
}

/// Check if the request is authenticated
fn is_authenticated(headers: &HeaderMap, expected_token: &str) -> bool {
    // Check Authorization header
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                // Remove "Bearer " prefix
                return token == expected_token;
            }
        }
    }

    // Check X-API-Key header
    if let Some(api_key) = headers.get("x-api-key") {
        if let Ok(key_str) = api_key.to_str() {
            return key_str == expected_token;
        }
    }

    false
}

/// Helper function to create auth middleware
pub fn auth_middleware_from_config(cfg: &crate::web::WebConfig) -> AuthLayer {
    let ak = cfg.api_keys.as_ref();
    let admin = ak.and_then(|a| a.admin_api_key.clone());
    let config = ak.and_then(|a| a.config_api_key.clone());
    let status = ak.and_then(|a| a.status_api_key.clone());
    let default = cfg.auth_token.clone();

    let allowlist = cfg.control_ip_allowlist.as_ref().map(|v| {
        v.iter()
            .filter_map(|s| s.parse::<cidr::AnyIpCidr>().ok())
            .collect::<Vec<_>>()
    });

    AuthLayer::new(default, admin, config, status, allowlist)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_is_public_endpoint() {
        assert!(is_public_endpoint("/"));
        assert!(is_public_endpoint("/static"));
        assert!(is_public_endpoint("/health"));
        assert!(!is_public_endpoint("/api/status"));
        assert!(!is_public_endpoint("/ws"));
    }

    #[test]
    fn test_is_authenticated() {
        let mut headers = HeaderMap::new();
        let token = "test-token-123";

        // Test with no headers
        assert!(!is_authenticated(&headers, token));

        // Test with Bearer token
        headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
        );
        assert!(is_authenticated(&headers, token));

        // Test with wrong Bearer token
        headers.insert(
            "authorization",
            HeaderValue::from_str("Bearer wrong-token").unwrap(),
        );
        assert!(!is_authenticated(&headers, token));

        // Test with API key
        headers.clear();
        headers.insert("x-api-key", HeaderValue::from_str(token).unwrap());
        assert!(is_authenticated(&headers, token));

        // Test with wrong API key
        headers.insert("x-api-key", HeaderValue::from_str("wrong-key").unwrap());
        assert!(!is_authenticated(&headers, token));
    }
}
