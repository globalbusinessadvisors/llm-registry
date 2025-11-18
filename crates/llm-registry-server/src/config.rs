//! Server configuration
//!
//! This module handles hierarchical configuration loading from multiple sources:
//! - Default configuration file
//! - Environment-specific configuration file
//! - Environment variables
//! - Command-line arguments

use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server settings
    pub server: HttpServerConfig,

    /// gRPC server settings
    #[serde(default)]
    pub grpc: GrpcServerConfig,

    /// Database settings
    pub database: DatabaseConfig,

    /// Logging settings
    pub logging: LoggingConfig,

    /// CORS settings
    #[serde(default)]
    pub cors: CorsConfig,
}

/// HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpServerConfig {
    /// Host to bind to
    #[serde(default = "default_host")]
    pub host: String,

    /// Port to bind to
    #[serde(default = "default_port")]
    pub port: u16,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Enable graceful shutdown
    #[serde(default = "default_true")]
    pub graceful_shutdown: bool,

    /// Graceful shutdown timeout in seconds
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_seconds: u64,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_timeout() -> u64 {
    30
}

fn default_true() -> bool {
    true
}

fn default_shutdown_timeout() -> u64 {
    30
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            timeout_seconds: default_timeout(),
            graceful_shutdown: default_true(),
            shutdown_timeout_seconds: default_shutdown_timeout(),
        }
    }
}

/// gRPC server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcServerConfig {
    /// Enable gRPC server
    #[serde(default)]
    pub enabled: bool,

    /// Host to bind to
    #[serde(default = "default_host")]
    pub host: String,

    /// Port to bind to
    #[serde(default = "default_grpc_port")]
    pub port: u16,
}

fn default_grpc_port() -> u16 {
    50051
}

impl Default for GrpcServerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: default_host(),
            port: default_grpc_port(),
        }
    }
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database connection URL
    pub url: String,

    /// Maximum number of connections in the pool
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Minimum number of connections in the pool
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,

    /// Connection timeout in seconds
    #[serde(default = "default_connection_timeout")]
    pub connect_timeout_seconds: u64,

    /// Idle timeout in seconds
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_seconds: u64,

    /// Maximum lifetime of a connection in seconds
    #[serde(default = "default_max_lifetime")]
    pub max_lifetime_seconds: u64,

    /// Run migrations on startup
    #[serde(default = "default_true")]
    pub run_migrations: bool,
}

fn default_max_connections() -> u32 {
    10
}

fn default_min_connections() -> u32 {
    2
}

fn default_connection_timeout() -> u64 {
    30
}

fn default_idle_timeout() -> u64 {
    600
}

fn default_max_lifetime() -> u64 {
    1800
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://localhost/llm_registry".to_string(),
            max_connections: default_max_connections(),
            min_connections: default_min_connections(),
            connect_timeout_seconds: default_connection_timeout(),
            idle_timeout_seconds: default_idle_timeout(),
            max_lifetime_seconds: default_max_lifetime(),
            run_migrations: default_true(),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Use JSON formatting
    #[serde(default)]
    pub json_format: bool,

    /// Include timestamps
    #[serde(default = "default_true")]
    pub include_timestamps: bool,

    /// Include thread IDs
    #[serde(default)]
    pub include_thread_ids: bool,

    /// Include target module
    #[serde(default = "default_true")]
    pub include_target: bool,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            json_format: false,
            include_timestamps: true,
            include_thread_ids: false,
            include_target: true,
        }
    }
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins (empty means all)
    #[serde(default)]
    pub allowed_origins: Vec<String>,

    /// Allow credentials
    #[serde(default)]
    pub allow_credentials: bool,

    /// Max age for preflight requests in seconds
    #[serde(default = "default_cors_max_age")]
    pub max_age_seconds: u64,
}

fn default_cors_max_age() -> u64 {
    3600
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec![],
            allow_credentials: false,
            max_age_seconds: default_cors_max_age(),
        }
    }
}

impl ServerConfig {
    /// Load configuration from files and environment
    ///
    /// Configuration is loaded in the following order (later sources override earlier):
    /// 1. Default configuration file (config/default.toml)
    /// 2. Environment-specific file (config/{env}.toml)
    /// 3. Environment variables (LLM_REGISTRY_*)
    ///
    /// # Arguments
    ///
    /// * `config_dir` - Directory containing configuration files
    /// * `environment` - Environment name (development, production, etc.)
    ///
    /// # Errors
    ///
    /// Returns an error if configuration cannot be loaded or parsed
    pub fn load(config_dir: impl Into<PathBuf>, environment: &str) -> Result<Self, ConfigError> {
        let config_dir = config_dir.into();

        let config = Config::builder()
            // Start with default config
            .add_source(File::from(config_dir.join("default.toml")).required(false))
            // Add environment-specific config
            .add_source(File::from(config_dir.join(format!("{}.toml", environment))).required(false))
            // Add environment variables with prefix LLM_REGISTRY
            // e.g., LLM_REGISTRY_SERVER__PORT=8080
            .add_source(
                Environment::with_prefix("LLM_REGISTRY")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        config.try_deserialize()
    }

    /// Load configuration with defaults if files don't exist
    pub fn load_or_default(config_dir: impl Into<PathBuf>, environment: &str) -> Self {
        Self::load(config_dir, environment).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to load configuration: {}", e);
            eprintln!("Using default configuration");
            Self::default()
        })
    }

    /// Get database connection string
    pub fn database_url(&self) -> &str {
        &self.database.url
    }

    /// Get server bind address
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: HttpServerConfig::default(),
            grpc: GrpcServerConfig::default(),
            database: DatabaseConfig::default(),
            logging: LoggingConfig::default(),
            cors: CorsConfig::default(),
        }
    }
}

/// Get the current environment name
///
/// Reads from the `ENVIRONMENT` or `ENV` environment variable,
/// defaulting to "development" if not set.
pub fn get_environment() -> String {
    std::env::var("ENVIRONMENT")
        .or_else(|_| std::env::var("ENV"))
        .unwrap_or_else(|_| "development".to_string())
}

/// Get the configuration directory
///
/// Reads from the `CONFIG_DIR` environment variable,
/// defaulting to "config" if not set.
pub fn get_config_dir() -> PathBuf {
    std::env::var("CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("config"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_server_config_bind_address() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_address(), "0.0.0.0:3000");
    }

    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 2);
    }

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, "info");
        assert!(!config.json_format);
        assert!(config.include_timestamps);
    }

    #[test]
    fn test_get_environment_default() {
        // Clear env var for test
        std::env::remove_var("ENVIRONMENT");
        std::env::remove_var("ENV");
        let env = get_environment();
        assert_eq!(env, "development");
    }
}
