//! API middleware
//!
//! This module provides middleware layers for request processing including
//! logging, CORS, compression, and request ID generation.

use axum::http::{HeaderValue, Method, Request};
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::{MakeRequestId, RequestId},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::Level;
use uuid::Uuid;

/// Request ID generator using UUIDs
#[derive(Clone, Default)]
pub struct UuidRequestIdGenerator;

impl MakeRequestId for UuidRequestIdGenerator {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let request_id = Uuid::new_v4().to_string();
        Some(RequestId::new(
            HeaderValue::from_str(&request_id).unwrap(),
        ))
    }
}

/// Build trace layer
pub fn trace_layer() -> TraceLayer<tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>> {
    TraceLayer::new_for_http()
        .make_span_with(
            DefaultMakeSpan::new()
                .include_headers(true)
                .level(Level::INFO),
        )
        .on_response(
            DefaultOnResponse::new()
                .include_headers(true)
                .latency_unit(LatencyUnit::Millis)
                .level(Level::INFO),
        )
}

/// Build CORS layer
pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        // Allow requests from any origin
        // In production, configure this based on environment
        .allow_origin(Any)
        // Allow common HTTP methods
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        // Allow common headers
        .allow_headers(Any)
        // Expose request ID header
        .expose_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::HeaderName::from_static("x-request-id"),
        ])
        // Allow credentials
        .allow_credentials(false)
}

/// CORS configuration options
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// Allowed origins (empty means any)
    pub allowed_origins: Vec<String>,

    /// Whether to allow credentials
    pub allow_credentials: bool,

    /// Max age for preflight cache
    pub max_age_seconds: Option<u64>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec![],
            allow_credentials: false,
            max_age_seconds: Some(3600),
        }
    }
}

impl CorsConfig {
    /// Build CORS layer from config
    pub fn into_layer(self) -> CorsLayer {
        let mut layer = CorsLayer::new()
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers(Any)
            .expose_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::HeaderName::from_static("x-request-id"),
            ])
            .allow_credentials(self.allow_credentials);

        // Configure origins
        if self.allowed_origins.is_empty() {
            layer = layer.allow_origin(Any);
        } else {
            // Parse origins
            let origins: Vec<HeaderValue> = self
                .allowed_origins
                .iter()
                .filter_map(|o| o.parse().ok())
                .collect();
            layer = layer.allow_origin(origins);
        }

        // Configure max age
        if let Some(max_age) = self.max_age_seconds {
            layer = layer.max_age(std::time::Duration::from_secs(max_age));
        }

        layer
    }
}

/// Middleware configuration
#[derive(Debug, Clone)]
pub struct MiddlewareConfig {
    /// CORS configuration
    pub cors: CorsConfig,

    /// Enable compression
    pub enable_compression: bool,

    /// Enable request tracing
    pub enable_tracing: bool,

    /// Request timeout in seconds
    pub request_timeout_seconds: Option<u64>,
}

impl Default for MiddlewareConfig {
    fn default() -> Self {
        Self {
            cors: CorsConfig::default(),
            enable_compression: true,
            enable_tracing: true,
            request_timeout_seconds: Some(30),
        }
    }
}

impl MiddlewareConfig {
    /// Create a new middleware config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set CORS config
    pub fn with_cors(mut self, cors: CorsConfig) -> Self {
        self.cors = cors;
        self
    }

    /// Enable/disable compression
    pub fn with_compression(mut self, enable: bool) -> Self {
        self.enable_compression = enable;
        self
    }

    /// Enable/disable tracing
    pub fn with_tracing(mut self, enable: bool) -> Self {
        self.enable_tracing = enable;
        self
    }

    /// Set request timeout
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.request_timeout_seconds = Some(timeout_seconds);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_request_id_generator() {
        let mut generator = UuidRequestIdGenerator::default();
        let request = Request::new(());

        let request_id = generator.make_request_id(&request);
        assert!(request_id.is_some());

        // RequestId is generated successfully (internal format verification not possible)
    }

    #[test]
    fn test_cors_config_default() {
        let config = CorsConfig::default();
        assert!(config.allowed_origins.is_empty());
        assert!(!config.allow_credentials);
        assert_eq!(config.max_age_seconds, Some(3600));
    }

    #[test]
    fn test_middleware_config_default() {
        let config = MiddlewareConfig::default();
        assert!(config.enable_compression);
        assert!(config.enable_tracing);
        assert_eq!(config.request_timeout_seconds, Some(30));
    }

    #[test]
    fn test_middleware_config_builder() {
        let config = MiddlewareConfig::new()
            .with_compression(false)
            .with_tracing(false)
            .with_timeout(60);

        assert!(!config.enable_compression);
        assert!(!config.enable_tracing);
        assert_eq!(config.request_timeout_seconds, Some(60));
    }
}
