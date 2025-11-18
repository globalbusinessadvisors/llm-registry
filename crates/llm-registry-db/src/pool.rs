//! Database connection pool management
//!
//! This module provides connection pooling for PostgreSQL using SQLx's built-in
//! pooling capabilities with additional configuration and health checking.

use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use sqlx::ConnectOptions;
use std::str::FromStr;
use std::time::Duration;
use tracing::{debug, info};

use crate::error::{DbError, DbResult};

/// Default minimum number of connections in the pool
pub const DEFAULT_MIN_CONNECTIONS: u32 = 2;

/// Default maximum number of connections in the pool
pub const DEFAULT_MAX_CONNECTIONS: u32 = 10;

/// Default connection timeout in seconds
pub const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Default idle timeout in seconds (10 minutes)
pub const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 600;

/// Default maximum lifetime for a connection (30 minutes)
pub const DEFAULT_MAX_LIFETIME_SECS: u64 = 1800;

/// Configuration for database connection pool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Database URL (e.g., postgres://user:pass@localhost/db)
    pub database_url: String,

    /// Minimum number of connections to maintain in the pool
    pub min_connections: u32,

    /// Maximum number of connections allowed in the pool
    pub max_connections: u32,

    /// Timeout for establishing a new connection
    pub connect_timeout: Duration,

    /// Idle timeout - connections idle for this duration will be closed
    pub idle_timeout: Duration,

    /// Maximum lifetime of a connection
    pub max_lifetime: Duration,

    /// Whether to enable SQL statement logging
    pub enable_logging: bool,

    /// Whether to run migrations on startup
    pub run_migrations: bool,
}

impl PoolConfig {
    /// Create a new pool configuration with sensible defaults
    pub fn new(database_url: impl Into<String>) -> Self {
        Self {
            database_url: database_url.into(),
            min_connections: DEFAULT_MIN_CONNECTIONS,
            max_connections: DEFAULT_MAX_CONNECTIONS,
            connect_timeout: Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS),
            idle_timeout: Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECS),
            max_lifetime: Duration::from_secs(DEFAULT_MAX_LIFETIME_SECS),
            enable_logging: false,
            run_migrations: true,
        }
    }

    /// Set minimum connections
    pub fn min_connections(mut self, min: u32) -> Self {
        self.min_connections = min;
        self
    }

    /// Set maximum connections
    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    /// Set connection timeout
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set idle timeout
    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    /// Set maximum connection lifetime
    pub fn max_lifetime(mut self, lifetime: Duration) -> Self {
        self.max_lifetime = lifetime;
        self
    }

    /// Enable or disable SQL logging
    pub fn enable_logging(mut self, enabled: bool) -> Self {
        self.enable_logging = enabled;
        self
    }

    /// Enable or disable automatic migrations
    pub fn run_migrations(mut self, run: bool) -> Self {
        self.run_migrations = run;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> DbResult<()> {
        if self.database_url.is_empty() {
            return Err(DbError::Configuration(
                "Database URL cannot be empty".to_string(),
            ));
        }

        if self.min_connections > self.max_connections {
            return Err(DbError::Configuration(format!(
                "min_connections ({}) cannot be greater than max_connections ({})",
                self.min_connections, self.max_connections
            )));
        }

        if self.max_connections == 0 {
            return Err(DbError::Configuration(
                "max_connections must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self::new("postgres://localhost/llm_registry")
    }
}

/// Create a PostgreSQL connection pool from configuration
pub async fn create_pool(config: &PoolConfig) -> DbResult<PgPool> {
    config.validate()?;

    info!(
        "Creating database connection pool: min={}, max={}, database={}",
        config.min_connections,
        config.max_connections,
        mask_password(&config.database_url)
    );

    // Parse connection options
    let mut connect_opts = PgConnectOptions::from_str(&config.database_url)
        .map_err(|e| DbError::Configuration(format!("Invalid database URL: {}", e)))?;

    // Configure statement logging
    if config.enable_logging {
        connect_opts = connect_opts.log_statements(tracing::log::LevelFilter::Debug);
    } else {
        connect_opts = connect_opts.log_statements(tracing::log::LevelFilter::Off);
    }

    // Build pool with options
    let pool = PgPoolOptions::new()
        .min_connections(config.min_connections)
        .max_connections(config.max_connections)
        .acquire_timeout(config.connect_timeout)
        .idle_timeout(config.idle_timeout)
        .max_lifetime(config.max_lifetime)
        .connect_with(connect_opts)
        .await
        .map_err(|e| DbError::Connection(format!("Failed to create pool: {}", e)))?;

    info!("Database connection pool created successfully");

    // Run migrations if enabled
    if config.run_migrations {
        run_migrations(&pool).await?;
    }

    // Verify pool health
    verify_pool_health(&pool).await?;

    Ok(pool)
}

/// Run database migrations
pub async fn run_migrations(pool: &PgPool) -> DbResult<()> {
    info!("Running database migrations");

    sqlx::migrate!("../../migrations")
        .run(pool)
        .await
        .map_err(|e| DbError::Migration(format!("Migration failed: {}", e)))?;

    info!("Database migrations completed successfully");
    Ok(())
}

/// Verify that the connection pool is healthy
pub async fn verify_pool_health(pool: &PgPool) -> DbResult<()> {
    debug!("Verifying database pool health");

    // Execute a simple query to verify connectivity
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .map_err(|e| DbError::Connection(format!("Health check failed: {}", e)))?;

    debug!("Database pool health check passed");
    Ok(())
}

/// Get pool statistics
pub async fn get_pool_stats(pool: &PgPool) -> PoolStats {
    PoolStats {
        total_connections: pool.size() as u32,
        idle_connections: pool.num_idle() as u32,
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Total number of connections in the pool
    pub total_connections: u32,
    /// Number of idle connections
    pub idle_connections: u32,
}

impl PoolStats {
    /// Get number of active (in-use) connections
    pub fn active_connections(&self) -> u32 {
        self.total_connections.saturating_sub(self.idle_connections)
    }

    /// Check if pool is near capacity
    pub fn is_near_capacity(&self, threshold: f32) -> bool {
        if self.total_connections == 0 {
            return false;
        }
        let utilization = self.active_connections() as f32 / self.total_connections as f32;
        utilization >= threshold
    }
}

/// Gracefully close the connection pool
pub async fn close_pool(pool: PgPool) {
    info!("Closing database connection pool");
    pool.close().await;
    info!("Database connection pool closed");
}

/// Mask password in database URL for logging
fn mask_password(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        let mut masked = parsed.clone();
        if parsed.password().is_some() {
            let _ = masked.set_password(Some("***"));
        }
        masked.to_string()
    } else {
        // If parsing fails, just show the scheme and host
        url.split('@')
            .last()
            .map(|s| format!("***@{}", s))
            .unwrap_or_else(|| "***".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_validation() {
        let config = PoolConfig::new("postgres://localhost/test");
        assert!(config.validate().is_ok());

        let bad_config = PoolConfig::new("").min_connections(5).max_connections(2);
        assert!(bad_config.validate().is_err());

        let bad_config = PoolConfig::new("postgres://localhost/test")
            .min_connections(10)
            .max_connections(5);
        assert!(bad_config.validate().is_err());
    }

    #[test]
    fn test_mask_password() {
        let url = "postgres://user:secret@localhost:5432/db";
        let masked = mask_password(url);
        assert!(!masked.contains("secret"));
        assert!(masked.contains("localhost"));

        let url_no_pass = "postgres://localhost/db";
        let masked = mask_password(url_no_pass);
        assert!(masked.contains("localhost"));
    }

    #[test]
    fn test_pool_stats() {
        let stats = PoolStats {
            total_connections: 10,
            idle_connections: 3,
        };
        assert_eq!(stats.active_connections(), 7);
        assert!(stats.is_near_capacity(0.6));
        assert!(!stats.is_near_capacity(0.8));
    }

    #[test]
    fn test_pool_config_builder() {
        let config = PoolConfig::new("postgres://localhost/test")
            .min_connections(5)
            .max_connections(20)
            .connect_timeout(Duration::from_secs(5))
            .enable_logging(true);

        assert_eq!(config.min_connections, 5);
        assert_eq!(config.max_connections, 20);
        assert_eq!(config.connect_timeout, Duration::from_secs(5));
        assert!(config.enable_logging);
    }
}
