//! Authentication middleware for the web dashboard

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    response::Response,
};
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Authentication middleware layer
#[derive(Clone)]
pub struct AuthLayer {
    auth_token: Option<String>,
}

impl AuthLayer {
    pub fn new(auth_token: Option<String>) -> Self {
        Self { auth_token }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            inner,
            auth_token: self.auth_token.clone(),
        }
    }
}

/// Authentication middleware service
#[derive(Clone)]
pub struct AuthMiddleware<S> {
    inner: S,
    auth_token: Option<String>,
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
        let path = request.uri().path();
        if is_public_endpoint(path) {
            let future = self.inner.call(request);
            return Box::pin(future);
        }

        // Check authentication if token is configured
        if let Some(ref expected_token) = self.auth_token {
            if !is_authenticated(&request.headers(), expected_token) {
                return Box::pin(async move {
                    Ok(Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .body("Unauthorized".into())
                        .unwrap())
                });
            }
        }

        let future = self.inner.call(request);
        Box::pin(future)
    }
}

/// Check if the endpoint is public (doesn't require authentication)
fn is_public_endpoint(path: &str) -> bool {
    matches!(path, "/" | "/static" | "/health")
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
pub fn auth_middleware(auth_token: Option<String>) -> AuthLayer {
    AuthLayer::new(auth_token)
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
