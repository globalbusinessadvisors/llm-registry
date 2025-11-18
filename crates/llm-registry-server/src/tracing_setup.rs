//! OpenTelemetry tracing setup
//!
//! This module configures OpenTelemetry tracing with Jaeger exporter
//! for distributed tracing across the LLM Registry services.

use opentelemetry::{
    global,
    trace::{TraceError, Tracer},
    KeyValue,
};
use opentelemetry_sdk::{
    trace::{self as sdktrace, RandomIdGenerator, Sampler},
    Resource,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// OpenTelemetry configuration
#[derive(Debug, Clone)]
pub struct OtelConfig {
    /// Service name
    pub service_name: String,

    /// Service version
    pub service_version: String,

    /// Environment (development, staging, production)
    pub environment: String,

    /// Jaeger agent endpoint (e.g., "localhost:6831")
    pub jaeger_agent_endpoint: Option<String>,

    /// OTLP exporter endpoint (e.g., "http://localhost:4317")
    pub otlp_endpoint: Option<String>,

    /// Trace sampling ratio (0.0 to 1.0)
    pub sampling_ratio: f64,

    /// Enable tracing
    pub enabled: bool,
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            service_name: "llm-registry".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            environment: "development".to_string(),
            jaeger_agent_endpoint: Some("localhost:6831".to_string()),
            otlp_endpoint: None,
            sampling_ratio: 1.0,
            enabled: true,
        }
    }
}

impl OtelConfig {
    /// Create new configuration
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            ..Default::default()
        }
    }

    /// Set service version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.service_version = version.into();
        self
    }

    /// Set environment
    pub fn with_environment(mut self, env: impl Into<String>) -> Self {
        self.environment = env.into();
        self
    }

    /// Set Jaeger endpoint
    pub fn with_jaeger_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.jaeger_agent_endpoint = Some(endpoint.into());
        self
    }

    /// Set OTLP endpoint
    pub fn with_otlp_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.otlp_endpoint = Some(endpoint.into());
        self
    }

    /// Set sampling ratio
    pub fn with_sampling_ratio(mut self, ratio: f64) -> Self {
        self.sampling_ratio = ratio.clamp(0.0, 1.0);
        self
    }

    /// Enable or disable tracing
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Initialize OpenTelemetry tracing
pub fn init_tracing(config: OtelConfig) -> Result<(), TraceError> {
    if !config.enabled {
        info!("OpenTelemetry tracing is disabled");
        return Ok(());
    }

    info!(
        "Initializing OpenTelemetry tracing for service: {}",
        config.service_name
    );

    // Create resource with service information
    let resource = Resource::new(vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("service.version", config.service_version.clone()),
        KeyValue::new("deployment.environment", config.environment.clone()),
    ]);

    // Configure tracer provider
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name(&config.service_name)
        .with_trace_config(
            sdktrace::config()
                .with_sampler(Sampler::TraceIdRatioBased(config.sampling_ratio))
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    // Create tracing layer
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Create env filter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Initialize subscriber with OpenTelemetry layer
    tracing_subscriber::registry()
        .with(env_filter)
        .with(telemetry_layer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
        )
        .try_init()
        .map_err(|e| TraceError::Other(Box::new(e)))?;

    info!("OpenTelemetry tracing initialized successfully");

    Ok(())
}

/// Shutdown OpenTelemetry tracing gracefully
pub fn shutdown_tracing() {
    info!("Shutting down OpenTelemetry tracing");
    global::shutdown_tracer_provider();
}

/// Get current tracer
pub fn current_tracer() -> impl Tracer {
    global::tracer("llm-registry")
}

/// Create a span for database operations
#[macro_export]
macro_rules! db_span {
    ($operation:expr) => {
        tracing::info_span!("database", operation = $operation, db.system = "postgresql")
    };
    ($operation:expr, $($key:tt = $value:expr),+) => {
        tracing::info_span!("database", operation = $operation, db.system = "postgresql", $($key = $value),+)
    };
}

/// Create a span for cache operations
#[macro_export]
macro_rules! cache_span {
    ($operation:expr) => {
        tracing::info_span!("cache", operation = $operation, cache.system = "redis")
    };
    ($operation:expr, $($key:tt = $value:expr),+) => {
        tracing::info_span!("cache", operation = $operation, cache.system = "redis", $($key = $value),+)
    };
}

/// Create a span for event operations
#[macro_export]
macro_rules! event_span {
    ($event_type:expr) => {
        tracing::info_span!("event", event.type = $event_type, messaging.system = "nats")
    };
    ($event_type:expr, $($key:tt = $value:expr),+) => {
        tracing::info_span!("event", event.type = $event_type, messaging.system = "nats", $($key = $value),+)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otel_config_builder() {
        let config = OtelConfig::new("test-service")
            .with_version("1.0.0")
            .with_environment("testing")
            .with_sampling_ratio(0.5)
            .with_enabled(true);

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.service_version, "1.0.0");
        assert_eq!(config.environment, "testing");
        assert_eq!(config.sampling_ratio, 0.5);
        assert!(config.enabled);
    }

    #[test]
    fn test_sampling_ratio_clamping() {
        let config = OtelConfig::new("test")
            .with_sampling_ratio(1.5); // Should clamp to 1.0

        assert_eq!(config.sampling_ratio, 1.0);

        let config2 = OtelConfig::new("test")
            .with_sampling_ratio(-0.5); // Should clamp to 0.0

        assert_eq!(config2.sampling_ratio, 0.0);
    }
}
