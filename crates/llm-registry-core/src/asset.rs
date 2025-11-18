//! Core asset types and metadata structures
//!
//! This module defines the main Asset type and related structures for representing
//! LLM artifacts in the registry.

use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::checksum::Checksum;
use crate::dependency::AssetReference;
use crate::error::{RegistryError, Result};
use crate::provenance::Provenance;
use crate::storage::StorageLocation;
use crate::types::{Annotations, AssetId, AssetStatus, Tags};

/// Types of assets that can be stored in the registry
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    /// Language model (GPT, BERT, etc.)
    Model,
    /// ML pipeline or workflow
    Pipeline,
    /// Test suite for validation
    TestSuite,
    /// Policy or governance rule
    Policy,
    /// Training or evaluation dataset
    Dataset,
    /// Custom user-defined type
    Custom(String),
}

impl AssetType {
    /// Create a custom asset type
    pub fn custom(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        if name.is_empty() {
            return Err(RegistryError::ValidationError(
                "Custom asset type name cannot be empty".to_string(),
            ));
        }
        Ok(AssetType::Custom(name))
    }

    /// Get the string representation of the asset type
    pub fn as_str(&self) -> &str {
        match self {
            AssetType::Model => "model",
            AssetType::Pipeline => "pipeline",
            AssetType::TestSuite => "test_suite",
            AssetType::Policy => "policy",
            AssetType::Dataset => "dataset",
            AssetType::Custom(name) => name.as_str(),
        }
    }

    /// Validate the asset type
    pub fn validate(&self) -> Result<()> {
        match self {
            AssetType::Custom(name) => {
                if name.is_empty() {
                    return Err(RegistryError::ValidationError(
                        "Custom asset type name cannot be empty".to_string(),
                    ));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

impl fmt::Display for AssetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Default for AssetType {
    fn default() -> Self {
        AssetType::Model
    }
}

/// Metadata associated with an asset
///
/// Contains descriptive and technical information about the asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetMetadata {
    /// Human-readable name of the asset
    pub name: String,

    /// Semantic version of the asset
    pub version: Version,

    /// Optional description of the asset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// License identifier (SPDX format recommended)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// User-defined tags for categorization
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Tags,

    /// Key-value annotations for flexible metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub annotations: Annotations,

    /// File size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,

    /// Content type / MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

impl AssetMetadata {
    /// Create new asset metadata with required fields
    pub fn new(name: impl Into<String>, version: Version) -> Self {
        Self {
            name: name.into(),
            version,
            description: None,
            license: None,
            tags: Vec::new(),
            annotations: HashMap::new(),
            size_bytes: None,
            content_type: None,
        }
    }

    /// Create a builder for constructing asset metadata
    pub fn builder(name: impl Into<String>, version: Version) -> AssetMetadataBuilder {
        AssetMetadataBuilder::new(name, version)
    }

    /// Validate the metadata
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(RegistryError::ValidationError(
                "Asset name cannot be empty".to_string(),
            ));
        }

        // Validate license format if present (basic check)
        if let Some(ref license) = self.license {
            if license.is_empty() {
                return Err(RegistryError::ValidationError(
                    "License cannot be empty if specified".to_string(),
                ));
            }
        }

        // Validate content type format if present
        if let Some(ref ct) = self.content_type {
            if ct.is_empty() {
                return Err(RegistryError::ValidationError(
                    "Content type cannot be empty if specified".to_string(),
                ));
            }
            // Basic MIME type validation
            if !ct.contains('/') {
                return Err(RegistryError::ValidationError(
                    format!("Invalid content type format: {}", ct),
                ));
            }
        }

        Ok(())
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        self.tags.push(tag.into());
    }

    /// Add an annotation
    pub fn add_annotation(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.annotations.insert(key.into(), value.into());
    }

    /// Check if metadata has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Get an annotation value
    pub fn get_annotation(&self, key: &str) -> Option<&String> {
        self.annotations.get(key)
    }
}

/// Builder for AssetMetadata
pub struct AssetMetadataBuilder {
    metadata: AssetMetadata,
}

impl AssetMetadataBuilder {
    /// Create a new builder
    pub fn new(name: impl Into<String>, version: Version) -> Self {
        Self {
            metadata: AssetMetadata::new(name, version),
        }
    }

    /// Set the description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.metadata.description = Some(desc.into());
        self
    }

    /// Set the license
    pub fn license(mut self, license: impl Into<String>) -> Self {
        self.metadata.license = Some(license.into());
        self
    }

    /// Add a tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.metadata.tags.push(tag.into());
        self
    }

    /// Add multiple tags
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.metadata.tags.extend(tags);
        self
    }

    /// Add an annotation
    pub fn annotation(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.annotations.insert(key.into(), value.into());
        self
    }

    /// Add multiple annotations
    pub fn annotations(mut self, annotations: HashMap<String, String>) -> Self {
        self.metadata.annotations.extend(annotations);
        self
    }

    /// Set the size in bytes
    pub fn size_bytes(mut self, size: u64) -> Self {
        self.metadata.size_bytes = Some(size);
        self
    }

    /// Set the content type
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.metadata.content_type = Some(content_type.into());
        self
    }

    /// Build the metadata with validation
    pub fn build(self) -> Result<AssetMetadata> {
        self.metadata.validate()?;
        Ok(self.metadata)
    }

    /// Build without validation
    pub fn build_unchecked(self) -> AssetMetadata {
        self.metadata
    }
}

/// Main asset structure representing a versioned artifact in the registry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    /// Unique identifier for this asset
    pub id: AssetId,

    /// Asset type
    pub asset_type: AssetType,

    /// Metadata about the asset
    pub metadata: AssetMetadata,

    /// Current status of the asset
    pub status: AssetStatus,

    /// Storage location information
    pub storage: StorageLocation,

    /// Checksum for integrity verification
    pub checksum: Checksum,

    /// Provenance information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,

    /// List of dependencies
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<AssetReference>,

    /// Timestamp when the asset was created
    pub created_at: DateTime<Utc>,

    /// Timestamp when the asset was last updated
    pub updated_at: DateTime<Utc>,

    /// Optional timestamp when the asset was deprecated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated_at: Option<DateTime<Utc>>,
}

impl Asset {
    /// Create a new asset with required fields
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: AssetId,
        asset_type: AssetType,
        metadata: AssetMetadata,
        storage: StorageLocation,
        checksum: Checksum,
    ) -> Result<Self> {
        // Validate inputs
        asset_type.validate()?;
        metadata.validate()?;

        let now = Utc::now();

        Ok(Self {
            id,
            asset_type,
            metadata,
            status: AssetStatus::default(),
            storage,
            checksum,
            provenance: None,
            dependencies: Vec::new(),
            created_at: now,
            updated_at: now,
            deprecated_at: None,
        })
    }

    /// Create a builder for constructing assets
    pub fn builder(
        asset_type: AssetType,
        metadata: AssetMetadata,
        storage: StorageLocation,
        checksum: Checksum,
    ) -> AssetBuilder {
        AssetBuilder::new(asset_type, metadata, storage, checksum)
    }

    /// Validate the asset
    pub fn validate(&self) -> Result<()> {
        self.asset_type.validate()?;
        self.metadata.validate()?;

        if let Some(ref prov) = self.provenance {
            prov.validate()?;
        }

        for dep in &self.dependencies {
            dep.validate()?;
        }

        Ok(())
    }

    /// Update the asset status
    pub fn set_status(&mut self, status: AssetStatus) {
        self.status = status;
        self.updated_at = Utc::now();

        if status == AssetStatus::Deprecated && self.deprecated_at.is_none() {
            self.deprecated_at = Some(Utc::now());
        }
    }

    /// Add a dependency to the asset
    pub fn add_dependency(&mut self, dependency: AssetReference) -> Result<()> {
        dependency.validate()?;
        self.dependencies.push(dependency);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Set the provenance information
    pub fn set_provenance(&mut self, provenance: Provenance) -> Result<()> {
        provenance.validate()?;
        self.provenance = Some(provenance);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Check if the asset is active
    pub fn is_active(&self) -> bool {
        self.status == AssetStatus::Active
    }

    /// Check if the asset is deprecated
    pub fn is_deprecated(&self) -> bool {
        self.status == AssetStatus::Deprecated
    }

    /// Check if the asset is compliant
    pub fn is_compliant(&self) -> bool {
        self.status != AssetStatus::NonCompliant
    }

    /// Get the full name with version
    pub fn full_name(&self) -> String {
        format!("{}@{}", self.metadata.name, self.metadata.version)
    }
}

impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Asset({}, {}, {})",
            self.id,
            self.full_name(),
            self.asset_type
        )
    }
}

/// Builder for constructing Asset instances
pub struct AssetBuilder {
    id: AssetId,
    asset_type: AssetType,
    metadata: AssetMetadata,
    status: AssetStatus,
    storage: StorageLocation,
    checksum: Checksum,
    provenance: Option<Provenance>,
    dependencies: Vec<AssetReference>,
    created_at: DateTime<Utc>,
}

impl AssetBuilder {
    /// Create a new asset builder
    pub fn new(
        asset_type: AssetType,
        metadata: AssetMetadata,
        storage: StorageLocation,
        checksum: Checksum,
    ) -> Self {
        Self {
            id: AssetId::new(),
            asset_type,
            metadata,
            status: AssetStatus::default(),
            storage,
            checksum,
            provenance: None,
            dependencies: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// Set the asset ID
    pub fn id(mut self, id: AssetId) -> Self {
        self.id = id;
        self
    }

    /// Set the status
    pub fn status(mut self, status: AssetStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the provenance
    pub fn provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// Add a dependency
    pub fn dependency(mut self, dependency: AssetReference) -> Self {
        self.dependencies.push(dependency);
        self
    }

    /// Add multiple dependencies
    pub fn dependencies(mut self, dependencies: Vec<AssetReference>) -> Self {
        self.dependencies.extend(dependencies);
        self
    }

    /// Set the created timestamp
    pub fn created_at(mut self, timestamp: DateTime<Utc>) -> Self {
        self.created_at = timestamp;
        self
    }

    /// Build the asset with validation
    pub fn build(self) -> Result<Asset> {
        self.asset_type.validate()?;
        self.metadata.validate()?;

        if let Some(ref prov) = self.provenance {
            prov.validate()?;
        }

        for dep in &self.dependencies {
            dep.validate()?;
        }

        let deprecated_at = if self.status == AssetStatus::Deprecated {
            Some(self.created_at)
        } else {
            None
        };

        Ok(Asset {
            id: self.id,
            asset_type: self.asset_type,
            metadata: self.metadata,
            status: self.status,
            storage: self.storage,
            checksum: self.checksum,
            provenance: self.provenance,
            dependencies: self.dependencies,
            created_at: self.created_at,
            updated_at: self.created_at,
            deprecated_at,
        })
    }

    /// Build without validation
    pub fn build_unchecked(self) -> Asset {
        let deprecated_at = if self.status == AssetStatus::Deprecated {
            Some(self.created_at)
        } else {
            None
        };

        Asset {
            id: self.id,
            asset_type: self.asset_type,
            metadata: self.metadata,
            status: self.status,
            storage: self.storage,
            checksum: self.checksum,
            provenance: self.provenance,
            dependencies: self.dependencies,
            created_at: self.created_at,
            updated_at: self.created_at,
            deprecated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checksum::HashAlgorithm;
    use crate::storage::StorageBackend;

    fn create_test_storage() -> StorageLocation {
        StorageLocation::new(
            StorageBackend::S3 {
                bucket: "test-bucket".to_string(),
                region: "us-east-1".to_string(),
                endpoint: None,
            },
            "models/test.bin".to_string(),
            None,
        )
        .unwrap()
    }

    fn create_test_checksum() -> Checksum {
        Checksum::new(
            HashAlgorithm::SHA256,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        )
        .unwrap()
    }

    #[test]
    fn test_asset_type_default() {
        assert_eq!(AssetType::default(), AssetType::Model);
    }

    #[test]
    fn test_asset_type_custom() {
        let custom = AssetType::custom("tokenizer").unwrap();
        assert_eq!(custom.as_str(), "tokenizer");
    }

    #[test]
    fn test_asset_type_custom_empty() {
        assert!(AssetType::custom("").is_err());
    }

    #[test]
    fn test_asset_metadata_creation() {
        let version = Version::parse("1.0.0").unwrap();
        let metadata = AssetMetadata::new("gpt-2", version.clone());

        assert_eq!(metadata.name, "gpt-2");
        assert_eq!(metadata.version, version);
        assert!(metadata.description.is_none());
        assert!(metadata.tags.is_empty());
    }

    #[test]
    fn test_asset_metadata_builder() {
        let version = Version::parse("1.0.0").unwrap();
        let metadata = AssetMetadata::builder("gpt-2", version.clone())
            .description("A test model")
            .license("MIT")
            .tag("nlp")
            .tag("transformer")
            .annotation("framework", "pytorch")
            .size_bytes(1024)
            .content_type("application/octet-stream")
            .build()
            .unwrap();

        assert_eq!(metadata.name, "gpt-2");
        assert_eq!(metadata.description.as_deref(), Some("A test model"));
        assert_eq!(metadata.license.as_deref(), Some("MIT"));
        assert_eq!(metadata.tags.len(), 2);
        assert!(metadata.has_tag("nlp"));
        assert_eq!(metadata.get_annotation("framework"), Some(&"pytorch".to_string()));
        assert_eq!(metadata.size_bytes, Some(1024));
        assert_eq!(metadata.content_type.as_deref(), Some("application/octet-stream"));
    }

    #[test]
    fn test_asset_metadata_validation_empty_name() {
        let version = Version::parse("1.0.0").unwrap();
        let result = AssetMetadata::builder("", version).build();
        assert!(result.is_err());
    }

    #[test]
    fn test_asset_metadata_validation_invalid_content_type() {
        let version = Version::parse("1.0.0").unwrap();
        let result = AssetMetadata::builder("test", version)
            .content_type("invalid")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_asset_creation() {
        let version = Version::parse("1.0.0").unwrap();
        let metadata = AssetMetadata::new("gpt-2", version);
        let storage = create_test_storage();
        let checksum = create_test_checksum();

        let asset = Asset::new(
            AssetId::new(),
            AssetType::Model,
            metadata,
            storage,
            checksum,
        )
        .unwrap();

        assert_eq!(asset.status, AssetStatus::Active);
        assert_eq!(asset.asset_type, AssetType::Model);
        assert!(asset.provenance.is_none());
        assert!(asset.dependencies.is_empty());
    }

    #[test]
    fn test_asset_builder() {
        let version = Version::parse("1.0.0").unwrap();
        let metadata = AssetMetadata::new("gpt-2", version);
        let storage = create_test_storage();
        let checksum = create_test_checksum();

        let asset = Asset::builder(AssetType::Model, metadata, storage, checksum)
            .status(AssetStatus::Active)
            .build()
            .unwrap();

        assert_eq!(asset.status, AssetStatus::Active);
        assert_eq!(asset.asset_type, AssetType::Model);
    }

    #[test]
    fn test_asset_set_status() {
        let version = Version::parse("1.0.0").unwrap();
        let metadata = AssetMetadata::new("gpt-2", version);
        let storage = create_test_storage();
        let checksum = create_test_checksum();

        let mut asset = Asset::new(
            AssetId::new(),
            AssetType::Model,
            metadata,
            storage,
            checksum,
        )
        .unwrap();

        assert!(asset.is_active());
        assert!(!asset.is_deprecated());

        asset.set_status(AssetStatus::Deprecated);
        assert!(!asset.is_active());
        assert!(asset.is_deprecated());
        assert!(asset.deprecated_at.is_some());
    }

    #[test]
    fn test_asset_add_dependency() {
        let version = Version::parse("1.0.0").unwrap();
        let metadata = AssetMetadata::new("gpt-2", version);
        let storage = create_test_storage();
        let checksum = create_test_checksum();

        let mut asset = Asset::new(
            AssetId::new(),
            AssetType::Model,
            metadata,
            storage,
            checksum,
        )
        .unwrap();

        let dep = AssetReference::by_id(AssetId::new());
        asset.add_dependency(dep).unwrap();

        assert_eq!(asset.dependencies.len(), 1);
    }

    #[test]
    fn test_asset_full_name() {
        let version = Version::parse("1.0.0").unwrap();
        let metadata = AssetMetadata::new("gpt-2", version);
        let storage = create_test_storage();
        let checksum = create_test_checksum();

        let asset = Asset::new(
            AssetId::new(),
            AssetType::Model,
            metadata,
            storage,
            checksum,
        )
        .unwrap();

        assert_eq!(asset.full_name(), "gpt-2@1.0.0");
    }

    #[test]
    fn test_asset_is_compliant() {
        let version = Version::parse("1.0.0").unwrap();
        let metadata = AssetMetadata::new("gpt-2", version);
        let storage = create_test_storage();
        let checksum = create_test_checksum();

        let mut asset = Asset::new(
            AssetId::new(),
            AssetType::Model,
            metadata,
            storage,
            checksum,
        )
        .unwrap();

        assert!(asset.is_compliant());

        asset.set_status(AssetStatus::NonCompliant);
        assert!(!asset.is_compliant());
    }
}
