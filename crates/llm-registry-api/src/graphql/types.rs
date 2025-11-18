//! GraphQL schema types
//!
//! This module defines GraphQL types that wrap the core domain types.

use async_graphql::{Enum, Object, SimpleObject};
use chrono::{DateTime, Utc};
use llm_registry_core::{Asset, AssetStatus, AssetType};
use llm_registry_service::DependencyNode;

/// GraphQL representation of an Asset
#[derive(Clone)]
pub struct GqlAsset(pub Asset);

#[Object]
impl GqlAsset {
    /// Unique identifier for this asset
    async fn id(&self) -> String {
        self.0.id.to_string()
    }

    /// Asset type
    async fn asset_type(&self) -> GqlAssetType {
        GqlAssetType::from_core(&self.0.asset_type)
    }

    /// Asset name
    async fn name(&self) -> &str {
        &self.0.metadata.name
    }

    /// Asset version
    async fn version(&self) -> String {
        self.0.metadata.version.to_string()
    }

    /// Asset description
    async fn description(&self) -> Option<&str> {
        self.0.metadata.description.as_deref()
    }

    /// License identifier
    async fn license(&self) -> Option<&str> {
        self.0.metadata.license.as_deref()
    }

    /// Tags for categorization
    async fn tags(&self) -> &[String] {
        &self.0.metadata.tags
    }

    /// Key-value annotations
    async fn annotations(&self) -> Vec<GqlAnnotation> {
        self.0
            .metadata
            .annotations
            .iter()
            .map(|(k, v)| GqlAnnotation {
                key: k.clone(),
                value: v.clone(),
            })
            .collect()
    }

    /// File size in bytes
    async fn size_bytes(&self) -> Option<u64> {
        self.0.metadata.size_bytes
    }

    /// Content type / MIME type
    async fn content_type(&self) -> Option<&str> {
        self.0.metadata.content_type.as_deref()
    }

    /// Current status of the asset
    async fn status(&self) -> GqlAssetStatus {
        GqlAssetStatus::from_core(&self.0.status)
    }

    /// Storage path
    async fn storage_path(&self) -> &str {
        &self.0.storage.path
    }

    /// Storage URI
    async fn storage_uri(&self) -> Option<&str> {
        self.0.storage.uri.as_deref()
    }

    /// Checksum algorithm
    async fn checksum_algorithm(&self) -> String {
        self.0.checksum.algorithm.to_string()
    }

    /// Checksum value
    async fn checksum_value(&self) -> &str {
        &self.0.checksum.value
    }

    /// Number of dependencies
    async fn dependency_count(&self) -> usize {
        self.0.dependencies.len()
    }

    /// Creation timestamp
    async fn created_at(&self) -> DateTime<Utc> {
        self.0.created_at
    }

    /// Last update timestamp
    async fn updated_at(&self) -> DateTime<Utc> {
        self.0.updated_at
    }

    /// Deprecation timestamp
    async fn deprecated_at(&self) -> Option<DateTime<Utc>> {
        self.0.deprecated_at
    }
}

/// GraphQL representation of a dependency node
#[derive(Clone)]
pub struct GqlDependencyNode {
    node: DependencyNode,
}

impl From<DependencyNode> for GqlDependencyNode {
    fn from(node: DependencyNode) -> Self {
        GqlDependencyNode { node }
    }
}

#[Object]
impl GqlDependencyNode {
    /// Asset ID
    async fn asset_id(&self) -> String {
        self.node.asset_id.to_string()
    }

    /// Asset name
    async fn name(&self) -> &str {
        &self.node.name
    }

    /// Asset version
    async fn version(&self) -> String {
        self.node.version.to_string()
    }

    /// Depth from root (0 = direct dependency)
    async fn depth(&self) -> i32 {
        self.node.depth
    }

    /// Number of dependencies this node has
    async fn dependency_count(&self) -> usize {
        self.node.dependencies.len()
    }
}

/// GraphQL representation of asset type
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum GqlAssetType {
    /// Language model
    Model,
    /// ML pipeline
    Pipeline,
    /// Test suite
    TestSuite,
    /// Policy
    Policy,
    /// Dataset
    Dataset,
}

impl GqlAssetType {
    pub fn from_core(asset_type: &AssetType) -> Self {
        match asset_type {
            AssetType::Model => GqlAssetType::Model,
            AssetType::Pipeline => GqlAssetType::Pipeline,
            AssetType::TestSuite => GqlAssetType::TestSuite,
            AssetType::Policy => GqlAssetType::Policy,
            AssetType::Dataset => GqlAssetType::Dataset,
            AssetType::Custom(_) => GqlAssetType::Model, // Default for custom types
        }
    }

    pub fn to_core(&self) -> AssetType {
        match self {
            GqlAssetType::Model => AssetType::Model,
            GqlAssetType::Pipeline => AssetType::Pipeline,
            GqlAssetType::TestSuite => AssetType::TestSuite,
            GqlAssetType::Policy => AssetType::Policy,
            GqlAssetType::Dataset => AssetType::Dataset,
        }
    }
}

/// GraphQL representation of asset status
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum GqlAssetStatus {
    /// Asset is active and available
    Active,
    /// Asset is deprecated
    Deprecated,
    /// Asset is archived
    Archived,
    /// Asset is non-compliant
    NonCompliant,
}

impl GqlAssetStatus {
    pub fn from_core(status: &AssetStatus) -> Self {
        match status {
            AssetStatus::Active => GqlAssetStatus::Active,
            AssetStatus::Deprecated => GqlAssetStatus::Deprecated,
            AssetStatus::Archived => GqlAssetStatus::Archived,
            AssetStatus::NonCompliant => GqlAssetStatus::NonCompliant,
        }
    }

    pub fn to_core(&self) -> AssetStatus {
        match self {
            GqlAssetStatus::Active => AssetStatus::Active,
            GqlAssetStatus::Deprecated => AssetStatus::Deprecated,
            GqlAssetStatus::Archived => AssetStatus::Archived,
            GqlAssetStatus::NonCompliant => AssetStatus::NonCompliant,
        }
    }
}

/// Key-value annotation
#[derive(SimpleObject, Clone)]
pub struct GqlAnnotation {
    /// Annotation key
    pub key: String,
    /// Annotation value
    pub value: String,
}

/// Paginated assets response
#[derive(SimpleObject)]
pub struct GqlAssetConnection {
    /// List of assets
    pub nodes: Vec<GqlAsset>,
    /// Total count
    pub total_count: i64,
    /// Whether there are more results
    pub has_next_page: bool,
}

/// Asset search filters
#[derive(async_graphql::InputObject)]
pub struct GqlAssetFilter {
    /// Filter by asset type
    pub asset_type: Option<GqlAssetType>,
    /// Filter by status
    pub status: Option<GqlAssetStatus>,
    /// Filter by tag (must have all specified tags)
    pub tags: Option<Vec<String>>,
    /// Filter by name (partial match)
    pub name: Option<String>,
}

/// Registration result
#[derive(SimpleObject)]
pub struct GqlRegisterResult {
    /// The registered asset
    pub asset: GqlAsset,
    /// Success message
    pub message: String,
}

/// Update result
#[derive(SimpleObject)]
pub struct GqlUpdateResult {
    /// The updated asset
    pub asset: GqlAsset,
    /// Success message
    pub message: String,
}

/// Delete result
#[derive(SimpleObject)]
pub struct GqlDeleteResult {
    /// ID of deleted asset
    pub asset_id: String,
    /// Success message
    pub message: String,
}
