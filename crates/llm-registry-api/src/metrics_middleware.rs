//! Metrics middleware for HTTP request tracking
//!
//! This module provides middleware for automatic collection of HTTP request metrics
//! including request counts, durations, and status codes.

use axum::{
    body::Body,
    extract::MatchedPath,
    http::{Request, Response},
    middleware::Next,
};
use std::time::Instant;
use tracing::info;

/// Middleware for collecting HTTP request metrics
///
/// Records:
/// - Request count by method, path, and status
/// - Request duration by method and path
///
/// Note: Actual metric recording is done by the server binary which has access
/// to the Prometheus registry. This middleware just logs the information.
pub async fn metrics_middleware(
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let start = Instant::now();
    let method = req.method().to_string();

    // Try to get the matched path template (e.g., "/api/v1/assets/:id")
    // If not available, use the URI path
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|mp| mp.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());

    // Process the request
    let response = next.run(req).await;

    let duration = start.elapsed();
    let status = response.status().as_u16();

    // Log metrics data (can be picked up by telemetry layer)
    info!(
        method = %method,
        path = %path,
        status = status,
        duration_ms = duration.as_millis() as u64,
        "http_request_completed"
    );

    response
}

/// Span creation for HTTP requests
///
/// Creates a tracing span for each HTTP request with relevant context
pub async fn create_request_span(
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let method = req.method().to_string();
    let uri = req.uri().to_string();
    let version = format!("{:?}", req.version());

    // Get matched path if available
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|mp| mp.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());

    // Create span with HTTP semantic conventions
    let span = tracing::info_span!(
        "http_request",
        http.method = %method,
        http.target = %uri,
        http.route = %path,
        http.version = %version,
        otel.kind = "server",
        otel.status_code = tracing::field::Empty,
    );

    let _enter = span.enter();

    let response = next.run(req).await;

    // Record status in span
    span.record("otel.status_code", response.status().as_u16());

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        response::Response,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "OK"
    }

    #[tokio::test]
    async fn test_metrics_middleware() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn(metrics_middleware));

        let request = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_request_span_middleware() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn(create_request_span));

        let request = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
