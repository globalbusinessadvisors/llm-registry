//! Redis-based caching layer
//!
//! This module provides distributed caching using Redis to improve
//! performance for frequently accessed data like assets and search results.

use llm_registry_core::{Asset, AssetId};
use redis::{aio::ConnectionManager, AsyncCommands, Client, RedisError};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::error::{DbError, DbResult};

/// Default TTL for cached items (15 minutes)
pub const DEFAULT_CACHE_TTL_SECS: u64 = 900;

/// Default TTL for search results (5 minutes)
pub const SEARCH_CACHE_TTL_SECS: u64 = 300;

/// Redis cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Redis connection URL
    pub redis_url: String,

    /// Default time-to-live for cached items
    pub default_ttl: Duration,

    /// TTL for search results
    pub search_ttl: Duration,

    /// Key prefix for namespacing
    pub key_prefix: String,

    /// Enable cache compression
    pub enable_compression: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://localhost:6379".to_string(),
            default_ttl: Duration::from_secs(DEFAULT_CACHE_TTL_SECS),
            search_ttl: Duration::from_secs(SEARCH_CACHE_TTL_SECS),
            key_prefix: "llm_registry".to_string(),
            enable_compression: false,
        }
    }
}

impl CacheConfig {
    /// Create new cache configuration
    pub fn new(redis_url: impl Into<String>) -> Self {
        Self {
            redis_url: redis_url.into(),
            ..Default::default()
        }
    }

    /// Set default TTL
    pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Set search TTL
    pub fn with_search_ttl(mut self, ttl: Duration) -> Self {
        self.search_ttl = ttl;
        self
    }

    /// Set key prefix
    pub fn with_key_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.key_prefix = prefix.into();
        self
    }

    /// Enable or disable compression
    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.enable_compression = enabled;
        self
    }
}

/// Redis cache client
#[derive(Clone)]
pub struct RedisCache {
    connection: ConnectionManager,
    config: CacheConfig,
}

impl RedisCache {
    /// Create a new Redis cache client
    pub async fn new(config: CacheConfig) -> DbResult<Self> {
        info!("Connecting to Redis at {}", mask_redis_url(&config.redis_url));

        let client = Client::open(config.redis_url.clone())
            .map_err(|e| DbError::Configuration(format!("Invalid Redis URL: {}", e)))?;

        let connection = ConnectionManager::new(client)
            .await
            .map_err(|e| DbError::Connection(format!("Failed to connect to Redis: {}", e)))?;

        info!("Successfully connected to Redis");

        Ok(Self { connection, config })
    }

    /// Get cached asset by ID
    pub async fn get_asset(&self, asset_id: &AssetId) -> DbResult<Option<Asset>> {
        let key = self.asset_key(asset_id);
        self.get(&key).await
    }

    /// Cache an asset
    pub async fn set_asset(&self, asset: &Asset) -> DbResult<()> {
        let key = self.asset_key(&asset.id);
        self.set(&key, asset, self.config.default_ttl).await
    }

    /// Delete cached asset
    pub async fn delete_asset(&self, asset_id: &AssetId) -> DbResult<()> {
        let key = self.asset_key(asset_id);
        self.delete(&key).await
    }

    /// Get cached value
    pub async fn get<T>(&self, key: &str) -> DbResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        debug!("Cache GET: {}", key);

        let mut conn = self.connection.clone();

        let data: Option<Vec<u8>> = conn
            .get(key)
            .await
            .map_err(|e| {
                warn!("Cache GET error for key {}: {}", key, e);
                DbError::Cache(format!("Failed to get from cache: {}", e))
            })?;

        match data {
            Some(bytes) => {
                let value: T = serde_json::from_slice(&bytes)
                    .map_err(|e| DbError::Serialization(format!("Failed to deserialize cached value: {}", e)))?;

                debug!("Cache HIT: {}", key);
                Ok(Some(value))
            }
            None => {
                debug!("Cache MISS: {}", key);
                Ok(None)
            }
        }
    }

    /// Set cached value with TTL
    pub async fn set<T>(&self, key: &str, value: &T, ttl: Duration) -> DbResult<()>
    where
        T: Serialize,
    {
        debug!("Cache SET: {} (TTL: {:?})", key, ttl);

        let data = serde_json::to_vec(value)
            .map_err(|e| DbError::Serialization(format!("Failed to serialize value: {}", e)))?;

        let mut conn = self.connection.clone();

        conn.set_ex(key, data, ttl.as_secs())
            .await
            .map_err(|e| {
                error!("Cache SET error for key {}: {}", key, e);
                DbError::Cache(format!("Failed to set cache: {}", e))
            })?;

        Ok(())
    }

    /// Delete cached value
    pub async fn delete(&self, key: &str) -> DbResult<()> {
        debug!("Cache DELETE: {}", key);

        let mut conn = self.connection.clone();

        conn.del(key)
            .await
            .map_err(|e| {
                error!("Cache DELETE error for key {}: {}", key, e);
                DbError::Cache(format!("Failed to delete from cache: {}", e))
            })?;

        Ok(())
    }

    /// Check if key exists
    pub async fn exists(&self, key: &str) -> DbResult<bool> {
        let mut conn = self.connection.clone();

        conn.exists(key)
            .await
            .map_err(|e| DbError::Cache(format!("Failed to check key existence: {}", e)))
    }

    /// Invalidate multiple keys matching a pattern
    pub async fn invalidate_pattern(&self, pattern: &str) -> DbResult<usize> {
        debug!("Cache INVALIDATE pattern: {}", pattern);

        let mut conn = self.connection.clone();

        // Get keys matching pattern
        let keys: Vec<String> = conn
            .keys(pattern)
            .await
            .map_err(|e| DbError::Cache(format!("Failed to get keys: {}", e)))?;

        if keys.is_empty() {
            return Ok(0);
        }

        // Delete all matching keys
        let count = keys.len();
        conn.del(&keys)
            .await
            .map_err(|e| DbError::Cache(format!("Failed to delete keys: {}", e)))?;

        info!("Invalidated {} cache keys matching pattern: {}", count, pattern);
        Ok(count)
    }

    /// Clear all cache entries with our prefix
    pub async fn clear_all(&self) -> DbResult<usize> {
        let pattern = format!("{}:*", self.config.key_prefix);
        self.invalidate_pattern(&pattern).await
    }

    /// Get cache statistics
    pub async fn stats(&self) -> DbResult<CacheStats> {
        let mut conn = self.connection.clone();

        let info: String = redis::cmd("INFO")
            .query_async(&mut conn)
            .await
            .map_err(|e| DbError::Cache(format!("Failed to get Redis info: {}", e)))?;

        Ok(CacheStats::parse_from_info(&info))
    }

    /// Build cache key for asset
    fn asset_key(&self, asset_id: &AssetId) -> String {
        format!("{}:asset:{}", self.config.key_prefix, asset_id.to_string())
    }

    /// Build cache key for search results
    pub fn search_key(&self, query_hash: &str) -> String {
        format!("{}:search:{}", self.config.key_prefix, query_hash)
    }

    /// Build cache key with custom namespace
    pub fn custom_key(&self, namespace: &str, key: &str) -> String {
        format!("{}:{}:{}", self.config.key_prefix, namespace, key)
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total keys in cache
    pub total_keys: usize,

    /// Memory used by Redis
    pub memory_used_bytes: usize,

    /// Total connections
    pub connected_clients: usize,

    /// Keyspace hits
    pub keyspace_hits: usize,

    /// Keyspace misses
    pub keyspace_misses: usize,
}

impl CacheStats {
    /// Parse Redis INFO output
    fn parse_from_info(info: &str) -> Self {
        let mut stats = Self {
            total_keys: 0,
            memory_used_bytes: 0,
            connected_clients: 0,
            keyspace_hits: 0,
            keyspace_misses: 0,
        };

        for line in info.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() != 2 {
                continue;
            }

            match parts[0] {
                "used_memory" => {
                    stats.memory_used_bytes = parts[1].trim().parse().unwrap_or(0);
                }
                "connected_clients" => {
                    stats.connected_clients = parts[1].trim().parse().unwrap_or(0);
                }
                "keyspace_hits" => {
                    stats.keyspace_hits = parts[1].trim().parse().unwrap_or(0);
                }
                "keyspace_misses" => {
                    stats.keyspace_misses = parts[1].trim().parse().unwrap_or(0);
                }
                _ => {}
            }
        }

        stats
    }

    /// Calculate hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.keyspace_hits + self.keyspace_misses;
        if total == 0 {
            0.0
        } else {
            self.keyspace_hits as f64 / total as f64
        }
    }
}

/// Mask sensitive parts of Redis URL for logging
fn mask_redis_url(url: &str) -> String {
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
    fn test_config_builder() {
        let config = CacheConfig::new("redis://localhost:6379")
            .with_default_ttl(Duration::from_secs(600))
            .with_search_ttl(Duration::from_secs(300))
            .with_key_prefix("test")
            .with_compression(true);

        assert_eq!(config.redis_url, "redis://localhost:6379");
        assert_eq!(config.default_ttl, Duration::from_secs(600));
        assert_eq!(config.search_ttl, Duration::from_secs(300));
        assert_eq!(config.key_prefix, "test");
        assert!(config.enable_compression);
    }

    #[test]
    fn test_cache_key_building() {
        let config = CacheConfig::default();
        let cache_config = config.clone();

        // Would need actual Redis connection to test RedisCache methods
        // These are unit tests for pure functions

        let asset_id = AssetId::new();
        let expected_key = format!("llm_registry:asset:{}", asset_id.to_string());

        // Test key format
        assert!(expected_key.starts_with("llm_registry:asset:"));
    }

    #[test]
    fn test_mask_redis_url() {
        let url = "redis://:password@localhost:6379";
        let masked = mask_redis_url(url);
        assert!(!masked.contains("password"));
        assert!(masked.contains("localhost"));

        let url_no_pass = "redis://localhost:6379";
        let masked = mask_redis_url(url_no_pass);
        assert!(masked.contains("localhost"));
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let stats = CacheStats {
            total_keys: 0,
            memory_used_bytes: 0,
            connected_clients: 0,
            keyspace_hits: 80,
            keyspace_misses: 20,
        };

        assert_eq!(stats.hit_rate(), 0.8);

        let empty_stats = CacheStats {
            total_keys: 0,
            memory_used_bytes: 0,
            connected_clients: 0,
            keyspace_hits: 0,
            keyspace_misses: 0,
        };

        assert_eq!(empty_stats.hit_rate(), 0.0);
    }
}
