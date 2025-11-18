//! Event store for registry event persistence and querying
//!
//! This module provides event storage capabilities for audit trails,
//! event sourcing, and real-time event subscriptions.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use llm_registry_core::{AssetId, EventType, RegistryEvent};
use serde_json::Value as JsonValue;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::{debug, instrument};

use crate::error::{DbError, DbResult};

/// Query parameters for searching events
#[derive(Debug, Clone, Default)]
pub struct EventQuery {
    /// Filter by asset ID
    pub asset_id: Option<AssetId>,

    /// Filter by event types
    pub event_types: Vec<String>,

    /// Filter by actor
    pub actor: Option<String>,

    /// Filter events after this timestamp
    pub after: Option<DateTime<Utc>>,

    /// Filter events before this timestamp
    pub before: Option<DateTime<Utc>>,

    /// Maximum number of events to return
    pub limit: i64,

    /// Number of events to skip
    pub offset: i64,
}

impl EventQuery {
    /// Create a new event query with defaults
    pub fn new() -> Self {
        Self {
            limit: 100,
            offset: 0,
            ..Default::default()
        }
    }

    /// Filter by asset ID
    pub fn asset_id(mut self, id: AssetId) -> Self {
        self.asset_id = Some(id);
        self
    }

    /// Filter by event type
    pub fn event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_types.push(event_type.into());
        self
    }

    /// Filter by actor
    pub fn actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    /// Filter events after timestamp
    pub fn after(mut self, timestamp: DateTime<Utc>) -> Self {
        self.after = Some(timestamp);
        self
    }

    /// Filter events before timestamp
    pub fn before(mut self, timestamp: DateTime<Utc>) -> Self {
        self.before = Some(timestamp);
        self
    }

    /// Set pagination limit
    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = limit;
        self
    }

    /// Set pagination offset
    pub fn offset(mut self, offset: i64) -> Self {
        self.offset = offset;
        self
    }
}

/// Results from an event query
#[derive(Debug, Clone)]
pub struct EventQueryResults {
    /// Events matching the query
    pub events: Vec<RegistryEvent>,

    /// Total number of matching events (without pagination)
    pub total: i64,

    /// Current offset
    pub offset: i64,

    /// Current limit
    pub limit: i64,
}

impl EventQueryResults {
    /// Check if there are more events available
    pub fn has_more(&self) -> bool {
        (self.offset + self.events.len() as i64) < self.total
    }

    /// Get the number of events in this page
    pub fn count(&self) -> usize {
        self.events.len()
    }
}

/// Event store trait for persisting and querying registry events
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Append a new event to the store
    ///
    /// # Arguments
    /// * `event` - The event to append
    ///
    /// # Returns
    /// * The persisted event with any database-generated fields
    async fn append(&self, event: RegistryEvent) -> DbResult<RegistryEvent>;

    /// Append multiple events atomically
    ///
    /// # Arguments
    /// * `events` - Vector of events to append
    ///
    /// # Returns
    /// * Vector of persisted events
    async fn append_batch(&self, events: Vec<RegistryEvent>) -> DbResult<Vec<RegistryEvent>>;

    /// Query events with filters
    ///
    /// # Arguments
    /// * `query` - Query parameters for filtering and pagination
    ///
    /// # Returns
    /// * Query results with matching events
    async fn query(&self, query: &EventQuery) -> DbResult<EventQueryResults>;

    /// Get events for a specific asset
    ///
    /// # Arguments
    /// * `asset_id` - The asset ID
    /// * `limit` - Maximum number of events to return
    ///
    /// # Returns
    /// * Vector of events for the asset, ordered by timestamp descending
    async fn get_asset_events(&self, asset_id: &AssetId, limit: i64) -> DbResult<Vec<RegistryEvent>>;

    /// Get the latest event for an asset
    ///
    /// # Arguments
    /// * `asset_id` - The asset ID
    ///
    /// # Returns
    /// * The most recent event for the asset, if any
    async fn get_latest_event(&self, asset_id: &AssetId) -> DbResult<Option<RegistryEvent>>;

    /// Count total events in the store
    async fn count_events(&self) -> DbResult<i64>;

    /// Count events by type
    async fn count_by_type(&self, event_type: &str) -> DbResult<i64>;

    /// Health check for event store
    async fn health_check(&self) -> DbResult<()>;
}

/// PostgreSQL implementation of EventStore
#[derive(Debug, Clone)]
pub struct PostgresEventStore {
    pool: PgPool,
}

impl PostgresEventStore {
    /// Create a new PostgreSQL event store
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl EventStore for PostgresEventStore {
    #[instrument(skip(self, event))]
    async fn append(&self, event: RegistryEvent) -> DbResult<RegistryEvent> {
        debug!("Appending event to store");

        let event_type_str = event.event_type.event_name();
        let asset_id = event.event_type.asset_id();

        sqlx::query(
            r#"
            INSERT INTO registry_events (
                event_id, event_type, asset_id, timestamp,
                actor, payload, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(ulid::Ulid::new().to_string())
        .bind(event_type_str)
        .bind(asset_id.map(|id| id.to_string()))
        .bind(&event.timestamp)
        .bind(&event.actor.as_deref().unwrap_or("system"))
        .bind(serde_json::to_value(&event.event_type)?)
        .bind(serde_json::to_value(&event.context)?)
        .execute(&self.pool)
        .await?;

        debug!("Event appended successfully");
        Ok(event)
    }

    #[instrument(skip(self, events), fields(count = events.len()))]
    async fn append_batch(&self, events: Vec<RegistryEvent>) -> DbResult<Vec<RegistryEvent>> {
        debug!("Appending batch of events");

        if events.is_empty() {
            return Ok(Vec::new());
        }

        let mut tx = self.pool.begin().await?;

        for event in &events {
            let event_type_str = event.event_type.event_name();
            let asset_id = event.event_type.asset_id();

            sqlx::query(
                r#"
                INSERT INTO registry_events (
                    event_id, event_type, asset_id, timestamp,
                    actor, payload, metadata
                ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(ulid::Ulid::new().to_string())
            .bind(event_type_str)
            .bind(asset_id.map(|id| id.to_string()))
            .bind(&event.timestamp)
            .bind(&event.actor.as_deref().unwrap_or("system"))
            .bind(serde_json::to_value(&event.event_type)?)
            .bind(serde_json::to_value(&event.context)?)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        debug!("Event batch appended successfully");
        Ok(events)
    }

    #[instrument(skip(self, query))]
    async fn query(&self, query: &EventQuery) -> DbResult<EventQueryResults> {
        debug!("Querying events");

        let mut sql = String::from(
            r#"
            SELECT
                event_id, event_type, asset_id, timestamp,
                actor, payload, metadata
            FROM registry_events
            WHERE 1=1
            "#,
        );

        let mut conditions = Vec::new();

        if query.asset_id.is_some() {
            conditions.push("asset_id = $ASSET_ID");
        }

        if !query.event_types.is_empty() {
            conditions.push("event_type = ANY($EVENT_TYPES)");
        }

        if query.actor.is_some() {
            conditions.push("actor = $ACTOR");
        }

        if query.after.is_some() {
            conditions.push("timestamp > $AFTER");
        }

        if query.before.is_some() {
            conditions.push("timestamp < $BEFORE");
        }

        if !conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" ORDER BY timestamp DESC");
        sql.push_str(&format!(" LIMIT {} OFFSET {}", query.limit, query.offset));

        // Create string bindings before building query to ensure proper lifetimes
        let asset_id_str = query.asset_id.as_ref().map(|id| id.to_string());

        // Build query dynamically
        let mut db_query = sqlx::query(&sql);

        if let Some(ref id_str) = asset_id_str {
            db_query = db_query.bind(id_str);
        }

        if !query.event_types.is_empty() {
            db_query = db_query.bind(&query.event_types);
        }

        if let Some(ref actor) = query.actor {
            db_query = db_query.bind(actor);
        }

        if let Some(after) = query.after {
            db_query = db_query.bind(after);
        }

        if let Some(before) = query.before {
            db_query = db_query.bind(before);
        }

        let rows = db_query.fetch_all(&self.pool).await?;

        let events: Result<Vec<RegistryEvent>, DbError> =
            rows.into_iter().map(row_to_event).collect();

        let events = events?;

        let total = self.count_query_results(query).await?;

        Ok(EventQueryResults {
            events,
            total,
            offset: query.offset,
            limit: query.limit,
        })
    }

    #[instrument(skip(self), fields(asset_id = %asset_id))]
    async fn get_asset_events(&self, asset_id: &AssetId, limit: i64) -> DbResult<Vec<RegistryEvent>> {
        debug!("Getting events for asset");

        let rows = sqlx::query(
            r#"
            SELECT
                event_id, event_type, asset_id, timestamp,
                actor, payload, metadata
            FROM registry_events
            WHERE asset_id = $1
            ORDER BY timestamp DESC
            LIMIT $2
            "#,
        )
        .bind(&asset_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let events: Result<Vec<RegistryEvent>, DbError> =
            rows.into_iter().map(row_to_event).collect();

        events
    }

    #[instrument(skip(self), fields(asset_id = %asset_id))]
    async fn get_latest_event(&self, asset_id: &AssetId) -> DbResult<Option<RegistryEvent>> {
        debug!("Getting latest event for asset");

        let row = sqlx::query(
            r#"
            SELECT
                event_id, event_type, asset_id, timestamp,
                actor, payload, metadata
            FROM registry_events
            WHERE asset_id = $1
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
        )
        .bind(&asset_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_event(row)?)),
            None => Ok(None),
        }
    }

    async fn count_events(&self) -> DbResult<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM registry_events")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("count"))
    }

    async fn count_by_type(&self, event_type: &str) -> DbResult<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM registry_events WHERE event_type = $1")
            .bind(event_type)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("count"))
    }

    async fn health_check(&self) -> DbResult<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(Into::into)
    }
}

impl PostgresEventStore {
    /// Count query results without pagination
    async fn count_query_results(&self, query: &EventQuery) -> DbResult<i64> {
        let mut sql = String::from("SELECT COUNT(*) as count FROM registry_events WHERE 1=1");

        if query.asset_id.is_some() {
            sql.push_str(" AND asset_id = $1");
        }

        if query.actor.is_some() {
            sql.push_str(" AND actor = $2");
        }

        // Create string bindings before building query to ensure proper lifetimes
        let asset_id_str = query.asset_id.as_ref().map(|id| id.to_string());

        let mut db_query = sqlx::query(&sql);

        if let Some(ref id_str) = asset_id_str {
            db_query = db_query.bind(id_str);
        }

        if let Some(ref actor) = query.actor {
            db_query = db_query.bind(actor);
        }

        let row = db_query.fetch_one(&self.pool).await?;

        Ok(row.get("count"))
    }
}

/// Convert database row to RegistryEvent
fn row_to_event(row: PgRow) -> DbResult<RegistryEvent> {
    let payload: JsonValue = row.get("payload");
    let event_type: EventType = serde_json::from_value(payload)
        .map_err(|e| DbError::Serialization(format!("Failed to parse event type: {}", e)))?;

    let timestamp: DateTime<Utc> = row.get("timestamp");
    let actor: Option<String> = row.get("actor");

    let metadata_json: JsonValue = row.get("metadata");
    let context = serde_json::from_value(metadata_json)
        .unwrap_or_else(|_| std::collections::HashMap::new());

    Ok(RegistryEvent {
        event_type,
        timestamp,
        correlation_id: None,
        actor,
        source: None,
        context,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_query_builder() {
        let asset_id = AssetId::from_string("01HN9XWZP8XQYZVJ4KFQY6XQZV").unwrap();
        let query = EventQuery::new()
            .asset_id(asset_id)
            .event_type("asset_registered")
            .actor("test-user")
            .limit(50);

        assert!(query.asset_id.is_some());
        assert_eq!(query.event_types.len(), 1);
        assert_eq!(query.actor.as_deref(), Some("test-user"));
        assert_eq!(query.limit, 50);
    }

    #[test]
    fn test_event_query_results_has_more() {
        let results = EventQueryResults {
            events: vec![],
            total: 100,
            offset: 0,
            limit: 50,
        };

        assert_eq!(results.count(), 0);
        // With offset 0 and 0 events, offset + count (0) < total (100), so has_more = true
        assert!(results.has_more());
    }
}
