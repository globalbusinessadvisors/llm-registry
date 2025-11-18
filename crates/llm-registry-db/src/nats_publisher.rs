//! NATS event publisher
//!
//! This module provides NATS-based event publishing for the LLM Registry.
//! Events stored in PostgreSQL are also published to NATS for real-time
//! notifications and event-driven integrations.

use async_nats::{Client, ConnectOptions};
use llm_registry_core::{EventType, RegistryEvent};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::error::{DbError, DbResult};

/// NATS event publisher configuration
#[derive(Debug, Clone)]
pub struct NatsPublisherConfig {
    /// NATS server URL
    pub server_url: String,

    /// Client name for identification
    pub client_name: String,

    /// Connection timeout
    pub connect_timeout: Duration,

    /// Reconnect attempts
    pub max_reconnect_attempts: usize,

    /// Reconnect delay
    pub reconnect_delay: Duration,

    /// Enable JetStream
    pub enable_jetstream: bool,
}

impl Default for NatsPublisherConfig {
    fn default() -> Self {
        Self {
            server_url: "nats://localhost:4222".to_string(),
            client_name: "llm-registry".to_string(),
            connect_timeout: Duration::from_secs(5),
            max_reconnect_attempts: 10,
            reconnect_delay: Duration::from_secs(1),
            enable_jetstream: true,
        }
    }
}

impl NatsPublisherConfig {
    /// Create new configuration
    pub fn new(server_url: impl Into<String>) -> Self {
        Self {
            server_url: server_url.into(),
            ..Default::default()
        }
    }

    /// Set client name
    pub fn with_client_name(mut self, name: impl Into<String>) -> Self {
        self.client_name = name.into();
        self
    }

    /// Set connection timeout
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Enable or disable JetStream
    pub fn with_jetstream(mut self, enabled: bool) -> Self {
        self.enable_jetstream = enabled;
        self
    }
}

/// NATS event publisher
#[derive(Clone)]
pub struct NatsEventPublisher {
    client: Client,
    config: NatsPublisherConfig,
}

impl NatsEventPublisher {
    /// Create a new NATS event publisher
    pub async fn new(config: NatsPublisherConfig) -> DbResult<Self> {
        info!(
            "Connecting to NATS server at {}",
            config.server_url
        );

        let connect_options = ConnectOptions::new()
            .name(&config.client_name)
            .connection_timeout(config.connect_timeout)
            .reconnect_delay_callback(move |attempts| {
                if attempts > 5 {
                    warn!("NATS reconnection attempt #{}", attempts);
                }
                config.reconnect_delay
            });

        let client = connect_options
            .connect(&config.server_url)
            .await
            .map_err(|e| DbError::Configuration(format!("Failed to connect to NATS: {}", e)))?;

        info!("Successfully connected to NATS");

        Ok(Self {
            client,
            config,
        })
    }

    /// Publish an event to NATS
    pub async fn publish(&self, event: &RegistryEvent) -> DbResult<()> {
        let subject = self.build_subject(event);

        debug!(
            "Publishing event {} to subject: {}",
            event.event_name(),
            subject
        );

        // Serialize event to JSON
        let payload = serde_json::to_vec(&EventMessage::from(event))
            .map_err(|e| DbError::Serialization(format!("Failed to serialize event: {}", e)))?;

        // Publish to NATS
        self.client
            .publish(subject.clone(), payload.into())
            .await
            .map_err(|e| {
                error!("Failed to publish event to NATS: {}", e);
                DbError::Connection(format!("NATS publish failed: {}", e))
            })?;

        debug!("Event published successfully to {}", subject);
        Ok(())
    }

    /// Publish multiple events in batch
    pub async fn publish_batch(&self, events: &[RegistryEvent]) -> DbResult<Vec<DbResult<()>>> {
        let mut results = Vec::with_capacity(events.len());

        for event in events {
            results.push(self.publish(event).await);
        }

        Ok(results)
    }

    /// Build NATS subject for an event
    fn build_subject(&self, event: &RegistryEvent) -> String {
        // Subject format: registry.events.{event_type}.{asset_id}
        match event.asset_id() {
            Some(asset_id) => format!(
                "registry.events.{}.{}",
                event_type_to_subject(&event.event_type),
                asset_id.to_string()
            ),
            None => format!(
                "registry.events.{}",
                event_type_to_subject(&event.event_type)
            ),
        }
    }

    /// Check if connected to NATS
    pub fn is_connected(&self) -> bool {
        matches!(
            self.client.connection_state(),
            async_nats::connection::State::Connected
        )
    }

    /// Get client reference
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Close the connection
    pub async fn close(self) -> DbResult<()> {
        info!("Closing NATS connection");

        // Wait for pending messages to flush
        self.client
            .flush()
            .await
            .map_err(|e| DbError::Connection(format!("Failed to flush NATS: {}", e)))?;

        Ok(())
    }
}

/// Event message wrapper for NATS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMessage {
    /// Event type/name
    pub event_type: String,

    /// Asset ID (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,

    /// Event timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Correlation ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,

    /// Actor (user or service)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,

    /// Source system
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Event data (the EventType as JSON)
    pub data: serde_json::Value,
}

impl From<&RegistryEvent> for EventMessage {
    fn from(event: &RegistryEvent) -> Self {
        Self {
            event_type: event.event_name().to_string(),
            asset_id: event.asset_id().map(|id| id.to_string()),
            timestamp: event.timestamp,
            correlation_id: event.correlation_id.clone(),
            actor: event.actor.clone(),
            source: event.source.clone(),
            data: serde_json::to_value(&event.event_type).unwrap_or(serde_json::json!({})),
        }
    }
}

/// Convert EventType to subject part
fn event_type_to_subject(event_type: &EventType) -> &'static str {
    match event_type {
        EventType::AssetRegistered { .. } => "asset.registered",
        EventType::AssetUpdated { .. } => "asset.updated",
        EventType::AssetDeleted { .. } => "asset.deleted",
        EventType::AssetStatusChanged { .. } => "asset.status_changed",
        EventType::AssetDownloaded { .. } => "asset.downloaded",
        EventType::ChecksumVerified { .. } => "checksum.verified",
        EventType::ChecksumFailed { .. } => "checksum.failed",
        EventType::PolicyValidated { .. } => "policy.validated",
        EventType::DependencyAdded { .. } => "dependency.added",
        EventType::CircularDependencyDetected { .. } => "circular_dependency.detected",
        EventType::Custom { .. } => "custom",
    }
}

/// NATS event subscriber configuration
#[derive(Debug, Clone)]
pub struct NatsSubscriberConfig {
    /// Subject pattern to subscribe to
    pub subject: String,

    /// Queue group name (for load balancing)
    pub queue_group: Option<String>,

    /// Maximum pending messages
    pub max_pending: usize,
}

impl NatsSubscriberConfig {
    /// Create new subscriber configuration
    pub fn new(subject: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
            queue_group: None,
            max_pending: 1000,
        }
    }

    /// Set queue group
    pub fn with_queue_group(mut self, group: impl Into<String>) -> Self {
        self.queue_group = Some(group.into());
        self
    }

    /// Set max pending messages
    pub fn with_max_pending(mut self, max: usize) -> Self {
        self.max_pending = max;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_registry_core::{AssetId, EventType};

    #[test]
    fn test_config_builder() {
        let config = NatsPublisherConfig::new("nats://localhost:4222")
            .with_client_name("test-client")
            .with_jetstream(true);

        assert_eq!(config.server_url, "nats://localhost:4222");
        assert_eq!(config.client_name, "test-client");
        assert!(config.enable_jetstream);
    }

    #[test]
    fn test_event_message_serialization() {
        let event = RegistryEvent::new(EventType::AssetRegistered {
            asset_id: AssetId::new(),
            asset_name: "test-asset".to_string(),
            asset_version: "1.0.0".to_string(),
            asset_type: "model".to_string(),
        });

        let message = EventMessage::from(&event);
        let json = serde_json::to_string(&message).unwrap();

        assert!(json.contains("event_type"));
        assert!(json.contains("asset_registered"));
    }

    #[test]
    fn test_event_type_to_subject() {
        let event_type = EventType::AssetRegistered {
            asset_id: AssetId::new(),
            asset_name: "test".to_string(),
            asset_version: "1.0.0".to_string(),
            asset_type: "model".to_string(),
        };

        assert_eq!(event_type_to_subject(&event_type), "asset.registered");
    }

    #[test]
    fn test_subscriber_config() {
        let config = NatsSubscriberConfig::new("registry.events.>")
            .with_queue_group("workers")
            .with_max_pending(5000);

        assert_eq!(config.subject, "registry.events.>");
        assert_eq!(config.queue_group, Some("workers".to_string()));
        assert_eq!(config.max_pending, 5000);
    }
}
