//! Event system for tracking registry operations
//!
//! This module provides types for representing events that occur in the registry,
//! enabling audit trails, notifications, and event-driven architectures.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::types::{AssetId, AssetStatus};

/// Types of events that can occur in the registry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventType {
    /// A new asset was registered
    AssetRegistered {
        /// ID of the registered asset
        asset_id: AssetId,
        /// Name of the asset
        asset_name: String,
        /// Version of the asset
        asset_version: String,
        /// Type of the asset
        asset_type: String,
    },

    /// An existing asset was updated
    AssetUpdated {
        /// ID of the updated asset
        asset_id: AssetId,
        /// Name of the asset
        asset_name: String,
        /// Fields that were updated
        updated_fields: Vec<String>,
    },

    /// An asset was deleted/removed
    AssetDeleted {
        /// ID of the deleted asset
        asset_id: AssetId,
        /// Name of the asset
        asset_name: String,
        /// Version of the asset
        asset_version: String,
    },

    /// Asset status changed
    AssetStatusChanged {
        /// ID of the asset
        asset_id: AssetId,
        /// Name of the asset
        asset_name: String,
        /// Previous status
        old_status: AssetStatus,
        /// New status
        new_status: AssetStatus,
    },

    /// Asset was downloaded
    AssetDownloaded {
        /// ID of the asset
        asset_id: AssetId,
        /// Name of the asset
        asset_name: String,
        /// Version of the asset
        asset_version: String,
        /// Optional user or service that downloaded it
        #[serde(skip_serializing_if = "Option::is_none")]
        downloader: Option<String>,
    },

    /// Checksum verification was performed
    ChecksumVerified {
        /// ID of the asset
        asset_id: AssetId,
        /// Whether verification succeeded
        success: bool,
        /// Hash algorithm used
        algorithm: String,
    },

    /// Checksum verification failed
    ChecksumFailed {
        /// ID of the asset
        asset_id: AssetId,
        /// Expected checksum
        expected: String,
        /// Actual checksum
        actual: String,
    },

    /// Policy validation was performed
    PolicyValidated {
        /// ID of the asset
        asset_id: AssetId,
        /// Policy that was validated
        policy_name: String,
        /// Whether validation passed
        passed: bool,
        /// Optional validation message
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },

    /// A dependency was added to an asset
    DependencyAdded {
        /// ID of the asset
        asset_id: AssetId,
        /// ID of the dependency (if by ID)
        #[serde(skip_serializing_if = "Option::is_none")]
        dependency_id: Option<AssetId>,
        /// Name of the dependency (if by name/version)
        #[serde(skip_serializing_if = "Option::is_none")]
        dependency_name: Option<String>,
    },

    /// Circular dependency was detected
    CircularDependencyDetected {
        /// IDs involved in the cycle
        cycle_asset_ids: Vec<AssetId>,
    },

    /// Custom event type for extensibility
    Custom {
        /// Event name
        name: String,
        /// Additional event data
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        data: HashMap<String, String>,
    },
}

impl EventType {
    /// Get a human-readable name for the event type
    pub fn event_name(&self) -> &str {
        match self {
            EventType::AssetRegistered { .. } => "asset_registered",
            EventType::AssetUpdated { .. } => "asset_updated",
            EventType::AssetDeleted { .. } => "asset_deleted",
            EventType::AssetStatusChanged { .. } => "asset_status_changed",
            EventType::AssetDownloaded { .. } => "asset_downloaded",
            EventType::ChecksumVerified { .. } => "checksum_verified",
            EventType::ChecksumFailed { .. } => "checksum_failed",
            EventType::PolicyValidated { .. } => "policy_validated",
            EventType::DependencyAdded { .. } => "dependency_added",
            EventType::CircularDependencyDetected { .. } => "circular_dependency_detected",
            EventType::Custom { name, .. } => name.as_str(),
        }
    }

    /// Get the asset ID associated with this event, if any
    pub fn asset_id(&self) -> Option<AssetId> {
        match self {
            EventType::AssetRegistered { asset_id, .. }
            | EventType::AssetUpdated { asset_id, .. }
            | EventType::AssetDeleted { asset_id, .. }
            | EventType::AssetStatusChanged { asset_id, .. }
            | EventType::AssetDownloaded { asset_id, .. }
            | EventType::ChecksumVerified { asset_id, .. }
            | EventType::ChecksumFailed { asset_id, .. }
            | EventType::PolicyValidated { asset_id, .. }
            | EventType::DependencyAdded { asset_id, .. } => Some(*asset_id),
            _ => None,
        }
    }

    /// Check if this is a critical/error event
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            EventType::ChecksumFailed { .. } | EventType::CircularDependencyDetected { .. }
        )
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.event_name())
    }
}

/// A registry event with metadata
///
/// Represents an event that occurred in the registry, with timestamp and context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryEvent {
    /// Event type and details
    #[serde(flatten)]
    pub event_type: EventType,

    /// When the event occurred
    pub timestamp: DateTime<Utc>,

    /// Optional correlation ID for tracking related events
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,

    /// Optional user or service that triggered the event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,

    /// Optional source system that generated the event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Additional context data
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub context: HashMap<String, String>,
}

impl RegistryEvent {
    /// Create a new event with the current timestamp
    pub fn new(event_type: EventType) -> Self {
        Self {
            event_type,
            timestamp: Utc::now(),
            correlation_id: None,
            actor: None,
            source: None,
            context: HashMap::new(),
        }
    }

    /// Create a builder for constructing events
    pub fn builder(event_type: EventType) -> RegistryEventBuilder {
        RegistryEventBuilder::new(event_type)
    }

    /// Get the event name
    pub fn event_name(&self) -> &str {
        self.event_type.event_name()
    }

    /// Get the associated asset ID, if any
    pub fn asset_id(&self) -> Option<AssetId> {
        self.event_type.asset_id()
    }

    /// Check if this is a critical event
    pub fn is_critical(&self) -> bool {
        self.event_type.is_critical()
    }

    /// Add context data
    pub fn add_context(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.context.insert(key.into(), value.into());
    }

    /// Get context data
    pub fn get_context(&self, key: &str) -> Option<&String> {
        self.context.get(key)
    }
}

impl fmt::Display for RegistryEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RegistryEvent({} at {}",
            self.event_name(),
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        )?;

        if let Some(ref actor) = self.actor {
            write!(f, ", actor={}", actor)?;
        }

        if let Some(asset_id) = self.asset_id() {
            write!(f, ", asset_id={}", asset_id)?;
        }

        write!(f, ")")
    }
}

/// Builder for constructing RegistryEvent instances
pub struct RegistryEventBuilder {
    event: RegistryEvent,
}

impl RegistryEventBuilder {
    /// Create a new builder
    pub fn new(event_type: EventType) -> Self {
        Self {
            event: RegistryEvent::new(event_type),
        }
    }

    /// Set the timestamp
    pub fn timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.event.timestamp = timestamp;
        self
    }

    /// Set the correlation ID
    pub fn correlation_id(mut self, id: impl Into<String>) -> Self {
        self.event.correlation_id = Some(id.into());
        self
    }

    /// Set the actor
    pub fn actor(mut self, actor: impl Into<String>) -> Self {
        self.event.actor = Some(actor.into());
        self
    }

    /// Set the source
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.event.source = Some(source.into());
        self
    }

    /// Add context data
    pub fn context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.event.context.insert(key.into(), value.into());
        self
    }

    /// Add multiple context entries
    pub fn with_context(mut self, context: HashMap<String, String>) -> Self {
        self.event.context.extend(context);
        self
    }

    /// Build the event
    pub fn build(self) -> RegistryEvent {
        self.event
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AssetStatus;

    #[test]
    fn test_event_type_asset_registered() {
        let asset_id = AssetId::new();
        let event_type = EventType::AssetRegistered {
            asset_id,
            asset_name: "gpt-2".to_string(),
            asset_version: "1.0.0".to_string(),
            asset_type: "model".to_string(),
        };

        assert_eq!(event_type.event_name(), "asset_registered");
        assert_eq!(event_type.asset_id(), Some(asset_id));
        assert!(!event_type.is_critical());
    }

    #[test]
    fn test_event_type_asset_status_changed() {
        let asset_id = AssetId::new();
        let event_type = EventType::AssetStatusChanged {
            asset_id,
            asset_name: "gpt-2".to_string(),
            old_status: AssetStatus::Active,
            new_status: AssetStatus::Deprecated,
        };

        assert_eq!(event_type.event_name(), "asset_status_changed");
        assert_eq!(event_type.asset_id(), Some(asset_id));
    }

    #[test]
    fn test_event_type_checksum_failed() {
        let asset_id = AssetId::new();
        let event_type = EventType::ChecksumFailed {
            asset_id,
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };

        assert_eq!(event_type.event_name(), "checksum_failed");
        assert!(event_type.is_critical());
    }

    #[test]
    fn test_event_type_circular_dependency() {
        let id1 = AssetId::new();
        let id2 = AssetId::new();
        let event_type = EventType::CircularDependencyDetected {
            cycle_asset_ids: vec![id1, id2],
        };

        assert!(event_type.is_critical());
        assert_eq!(event_type.asset_id(), None);
    }

    #[test]
    fn test_event_type_custom() {
        let event_type = EventType::Custom {
            name: "custom_event".to_string(),
            data: HashMap::new(),
        };

        assert_eq!(event_type.event_name(), "custom_event");
        assert!(!event_type.is_critical());
    }

    #[test]
    fn test_registry_event_creation() {
        let asset_id = AssetId::new();
        let event_type = EventType::AssetRegistered {
            asset_id,
            asset_name: "gpt-2".to_string(),
            asset_version: "1.0.0".to_string(),
            asset_type: "model".to_string(),
        };

        let event = RegistryEvent::new(event_type);
        assert_eq!(event.event_name(), "asset_registered");
        assert_eq!(event.asset_id(), Some(asset_id));
        assert!(event.correlation_id.is_none());
        assert!(event.actor.is_none());
    }

    #[test]
    fn test_registry_event_builder() {
        let asset_id = AssetId::new();
        let event_type = EventType::AssetRegistered {
            asset_id,
            asset_name: "gpt-2".to_string(),
            asset_version: "1.0.0".to_string(),
            asset_type: "model".to_string(),
        };

        let event = RegistryEvent::builder(event_type)
            .correlation_id("corr-123")
            .actor("user@example.com")
            .source("api-server")
            .context("request_id", "req-456")
            .build();

        assert_eq!(event.correlation_id.as_deref(), Some("corr-123"));
        assert_eq!(event.actor.as_deref(), Some("user@example.com"));
        assert_eq!(event.source.as_deref(), Some("api-server"));
        assert_eq!(event.get_context("request_id"), Some(&"req-456".to_string()));
    }

    #[test]
    fn test_registry_event_add_context() {
        let asset_id = AssetId::new();
        let event_type = EventType::AssetDownloaded {
            asset_id,
            asset_name: "gpt-2".to_string(),
            asset_version: "1.0.0".to_string(),
            downloader: Some("user@example.com".to_string()),
        };

        let mut event = RegistryEvent::new(event_type);
        event.add_context("download_size", "1024");
        event.add_context("download_duration_ms", "150");

        assert_eq!(event.get_context("download_size"), Some(&"1024".to_string()));
        assert_eq!(event.get_context("download_duration_ms"), Some(&"150".to_string()));
    }

    #[test]
    fn test_registry_event_is_critical() {
        let asset_id = AssetId::new();

        let normal_event = RegistryEvent::new(EventType::AssetRegistered {
            asset_id,
            asset_name: "test".to_string(),
            asset_version: "1.0.0".to_string(),
            asset_type: "model".to_string(),
        });
        assert!(!normal_event.is_critical());

        let critical_event = RegistryEvent::new(EventType::ChecksumFailed {
            asset_id,
            expected: "abc".to_string(),
            actual: "def".to_string(),
        });
        assert!(critical_event.is_critical());
    }

    #[test]
    fn test_registry_event_display() {
        let asset_id = AssetId::new();
        let event_type = EventType::AssetRegistered {
            asset_id,
            asset_name: "gpt-2".to_string(),
            asset_version: "1.0.0".to_string(),
            asset_type: "model".to_string(),
        };

        let event = RegistryEvent::builder(event_type)
            .actor("user@example.com")
            .build();

        let display = format!("{}", event);
        assert!(display.contains("asset_registered"));
        assert!(display.contains("actor=user@example.com"));
    }

    #[test]
    fn test_event_serialization() {
        let asset_id = AssetId::new();
        let event_type = EventType::AssetRegistered {
            asset_id,
            asset_name: "gpt-2".to_string(),
            asset_version: "1.0.0".to_string(),
            asset_type: "model".to_string(),
        };

        let event = RegistryEvent::new(event_type);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("asset_registered"));
        assert!(json.contains("gpt-2"));

        let deserialized: RegistryEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.event_name(), "asset_registered");
        assert_eq!(deserialized.asset_id(), Some(asset_id));
    }

    #[test]
    fn test_event_type_policy_validated() {
        let asset_id = AssetId::new();
        let event_type = EventType::PolicyValidated {
            asset_id,
            policy_name: "license_check".to_string(),
            passed: true,
            message: Some("All licenses are compliant".to_string()),
        };

        assert_eq!(event_type.event_name(), "policy_validated");
        assert!(!event_type.is_critical());
    }

    #[test]
    fn test_event_type_dependency_added() {
        let asset_id = AssetId::new();
        let dep_id = AssetId::new();
        let event_type = EventType::DependencyAdded {
            asset_id,
            dependency_id: Some(dep_id),
            dependency_name: None,
        };

        assert_eq!(event_type.event_name(), "dependency_added");
        assert_eq!(event_type.asset_id(), Some(asset_id));
    }
}
