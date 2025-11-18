//! Database layer for LLM Registry
//!
//! This crate provides database persistence for the LLM Registry system, including:
//! - Connection pool management with deadpool
//! - Repository trait abstractions for assets
//! - PostgreSQL implementation with SQLx
//! - Event store for audit trails and event sourcing
//! - Database migrations
//! - Comprehensive error handling
//!
//! # Features
//!
//! - **Compile-time verified queries**: Using SQLx macros for type-safe SQL
//! - **Connection pooling**: Efficient connection management with configurable pools
//! - **Transaction support**: ACID guarantees for multi-step operations
//! - **Event sourcing**: Complete audit trail of all registry operations
//! - **Flexible search**: Full-text search and filtering capabilities
//! - **Dependency tracking**: Graph-based dependency management with cycle detection
//!
//! # Example
//!
//! ```rust,no_run
//! use llm_registry_db::{PoolConfig, create_pool, PostgresAssetRepository};
//! use llm_registry_core::Asset;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a connection pool
//! let config = PoolConfig::new("postgres://localhost/llm_registry")
//!     .max_connections(10);
//! let pool = create_pool(&config).await?;
//!
//! // Create repository
//! let repo = PostgresAssetRepository::new(pool);
//!
//! // Use the repository
//! // let asset = repo.create(my_asset).await?;
//! # Ok(())
//! # }
//! ```

// Re-export core domain types for convenience
pub use llm_registry_core;

// Public modules
pub mod cache;
pub mod error;
pub mod event_store;
pub mod nats_publisher;
pub mod pool;
pub mod postgres;
pub mod repository;

// Re-exports for convenience
pub use cache::{CacheConfig, CacheStats, RedisCache};
pub use error::{DbError, DbResult};
pub use event_store::{EventQuery, EventQueryResults, EventStore, PostgresEventStore};
pub use nats_publisher::{
    EventMessage, NatsEventPublisher, NatsPublisherConfig, NatsSubscriberConfig,
};
pub use pool::{
    close_pool, create_pool, get_pool_stats, run_migrations, verify_pool_health, PoolConfig,
    PoolStats,
};
pub use postgres::PostgresAssetRepository;
pub use repository::{AssetRepository, SearchQuery, SearchResults, SortField, SortOrder};

// Re-export sqlx types that users may need
pub use sqlx::postgres::PgPool;

/// Database layer version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default database URL environment variable name
pub const DEFAULT_DATABASE_URL_ENV: &str = "DATABASE_URL";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_default_env_var() {
        assert_eq!(DEFAULT_DATABASE_URL_ENV, "DATABASE_URL");
    }
}
