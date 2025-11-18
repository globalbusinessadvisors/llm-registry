//! Telemetry and tracing configuration
//!
//! This module configures structured logging and distributed tracing
//! for the LLM Registry server.

use tracing::Level;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Telemetry configuration
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Log level
    pub log_level: String,

    /// Whether to use JSON formatting
    pub json_format: bool,

    /// Whether to include timestamps
    pub include_timestamps: bool,

    /// Whether to include thread IDs
    pub include_thread_ids: bool,

    /// Whether to include target module
    pub include_target: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            json_format: false,
            include_timestamps: true,
            include_thread_ids: false,
            include_target: true,
        }
    }
}

impl TelemetryConfig {
    /// Create a new telemetry config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set log level
    pub fn with_log_level(mut self, level: impl Into<String>) -> Self {
        self.log_level = level.into();
        self
    }

    /// Enable JSON formatting
    pub fn with_json_format(mut self, enabled: bool) -> Self {
        self.json_format = enabled;
        self
    }

    /// Configure timestamp inclusion
    pub fn with_timestamps(mut self, enabled: bool) -> Self {
        self.include_timestamps = enabled;
        self
    }

    /// Configure thread ID inclusion
    pub fn with_thread_ids(mut self, enabled: bool) -> Self {
        self.include_thread_ids = enabled;
        self
    }

    /// Configure target module inclusion
    pub fn with_target(mut self, enabled: bool) -> Self {
        self.include_target = enabled;
        self
    }
}

/// Initialize telemetry with default configuration
///
/// This sets up tracing with sensible defaults for development.
///
/// # Example
///
/// ```rust,no_run
/// use llm_registry_server::telemetry;
///
/// telemetry::init();
/// ```
pub fn init() {
    init_with_config(TelemetryConfig::default());
}

/// Initialize telemetry with custom configuration
///
/// # Arguments
///
/// * `config` - Telemetry configuration
///
/// # Example
///
/// ```rust,no_run
/// use llm_registry_server::telemetry::{init_with_config, TelemetryConfig};
///
/// let config = TelemetryConfig::new()
///     .with_log_level("debug")
///     .with_json_format(true);
///
/// init_with_config(config);
/// ```
pub fn init_with_config(config: TelemetryConfig) {
    // Build environment filter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    if config.json_format {
        // JSON formatting for production
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .json()
                    .with_current_span(true)
                    .with_span_list(false)
                    .with_timer(fmt::time::SystemTime)
                    .with_target(config.include_target)
                    .with_thread_ids(config.include_thread_ids),
            )
            .init();
    } else {
        // Pretty formatting for development
        let mut fmt_layer = fmt::layer()
            .with_target(config.include_target)
            .with_thread_ids(config.include_thread_ids);

        // Always include timestamps (SystemTime is always available)
        fmt_layer = fmt_layer.with_timer(fmt::time::SystemTime);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    }
}

/// Initialize telemetry from environment variables
///
/// This reads configuration from:
/// - `RUST_LOG` - Log level filter
/// - `LOG_FORMAT` - "json" for JSON formatting, anything else for pretty
/// - `LOG_TIMESTAMPS` - "true" or "false"
/// - `LOG_THREAD_IDS` - "true" or "false"
/// - `LOG_TARGET` - "true" or "false"
///
/// # Example
///
/// ```bash
/// RUST_LOG=debug LOG_FORMAT=json cargo run
/// ```
pub fn init_from_env() {
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let json_format = std::env::var("LOG_FORMAT")
        .map(|v| v.to_lowercase() == "json")
        .unwrap_or(false);
    let include_timestamps = std::env::var("LOG_TIMESTAMPS")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(true);
    let include_thread_ids = std::env::var("LOG_THREAD_IDS")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);
    let include_target = std::env::var("LOG_TARGET")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(true);

    let config = TelemetryConfig {
        log_level,
        json_format,
        include_timestamps,
        include_thread_ids,
        include_target,
    };

    init_with_config(config);
}

/// Get the current log level
pub fn get_log_level() -> Level {
    std::env::var("RUST_LOG")
        .ok()
        .and_then(|level| match level.to_lowercase().as_str() {
            "trace" => Some(Level::TRACE),
            "debug" => Some(Level::DEBUG),
            "info" => Some(Level::INFO),
            "warn" => Some(Level::WARN),
            "error" => Some(Level::ERROR),
            _ => None,
        })
        .unwrap_or(Level::INFO)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_config_default() {
        let config = TelemetryConfig::default();
        assert_eq!(config.log_level, "info");
        assert!(!config.json_format);
        assert!(config.include_timestamps);
        assert!(!config.include_thread_ids);
        assert!(config.include_target);
    }

    #[test]
    fn test_telemetry_config_builder() {
        let config = TelemetryConfig::new()
            .with_log_level("debug")
            .with_json_format(true)
            .with_timestamps(false);

        assert_eq!(config.log_level, "debug");
        assert!(config.json_format);
        assert!(!config.include_timestamps);
    }

    #[test]
    fn test_get_log_level_default() {
        // This might be affected by environment, so we just test it doesn't panic
        let _level = get_log_level();
    }
}
