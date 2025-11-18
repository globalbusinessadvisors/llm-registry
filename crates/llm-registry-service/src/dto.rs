//! Data Transfer Objects (DTOs) for service layer
//!
//! This module defines request and response types used at service boundaries,
//! separating internal domain models from external interfaces.

use chrono::{DateTime, Utc};
use llm_registry_core::{
    Asset, AssetId, AssetReference, AssetStatus, AssetType, Checksum,
    HashAlgorithm, Provenance, StorageLocation,
};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Registration DTOs
// ============================================================================

/// Request to register a new asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterAssetRequest {
    /// Asset type
    pub asset_type: AssetType,

    /// Asset name
    pub name: String,

    /// Semantic version
    pub version: Version,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Optional license identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Tags for categorization
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Key-value annotations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub annotations: HashMap<String, String>,

    /// Storage location
    pub storage: StorageLocation,

    /// Checksum for verification
    pub checksum: Checksum,

    /// Optional provenance information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,

    /// List of dependencies
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<AssetReference>,

    /// File size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,

    /// Content type / MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

/// Response from registering an asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterAssetResponse {
    /// The registered asset
    pub asset: Asset,

    /// Any warnings generated during registration
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

// ============================================================================
// Search DTOs
// ============================================================================

/// Search query parameters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchAssetsRequest {
    /// Text search across name, description, and annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Filter by asset types
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub asset_types: Vec<AssetType>,

    /// Filter by tags (asset must have all tags)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Filter by author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Filter by storage backend
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_backend: Option<String>,

    /// Only include non-deprecated assets
    #[serde(default = "default_exclude_deprecated")]
    pub exclude_deprecated: bool,

    /// Maximum number of results
    #[serde(default = "default_limit")]
    pub limit: i64,

    /// Number of results to skip
    #[serde(default)]
    pub offset: i64,

    /// Sort field
    #[serde(default)]
    pub sort_by: SortField,

    /// Sort order
    #[serde(default)]
    pub sort_order: SortOrder,
}

fn default_exclude_deprecated() -> bool {
    true
}

fn default_limit() -> i64 {
    50
}

/// Fields to sort by
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    CreatedAt,
    UpdatedAt,
    Name,
    Version,
    SizeBytes,
}

impl Default for SortField {
    fn default() -> Self {
        SortField::CreatedAt
    }
}

/// Sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl Default for SortOrder {
    fn default() -> Self {
        SortOrder::Descending
    }
}

/// Search results response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAssetsResponse {
    /// Matching assets
    pub assets: Vec<Asset>,

    /// Total number of results (without pagination)
    pub total: i64,

    /// Current offset
    pub offset: i64,

    /// Current limit
    pub limit: i64,

    /// Whether there are more results
    pub has_more: bool,
}

// ============================================================================
// Validation DTOs
// ============================================================================

/// Request to validate an asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateAssetRequest {
    /// The asset to validate
    pub asset: Asset,

    /// Whether to perform deep validation (including dependencies)
    #[serde(default)]
    pub deep: bool,

    /// Custom policies to apply
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policies: Vec<String>,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,

    /// List of validation errors
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ValidationError>,

    /// List of validation warnings
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<ValidationWarning>,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Field or context where error occurred
    pub field: String,

    /// Error message
    pub message: String,

    /// Error code for programmatic handling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Field or context where warning occurred
    pub field: String,

    /// Warning message
    pub message: String,
}

// ============================================================================
// Integrity DTOs
// ============================================================================

/// Request to verify asset integrity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyIntegrityRequest {
    /// Asset ID to verify
    pub asset_id: AssetId,

    /// Optional computed checksum to verify against
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computed_checksum: Option<Checksum>,
}

/// Integrity verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityVerificationResult {
    /// Whether integrity check passed
    pub verified: bool,

    /// Expected checksum
    pub expected_checksum: Checksum,

    /// Actual checksum (if computed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_checksum: Option<Checksum>,

    /// Error message if verification failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Request to compute checksum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeChecksumRequest {
    /// Data to hash (base64 encoded)
    pub data: String,

    /// Hash algorithm to use
    #[serde(default)]
    pub algorithm: HashAlgorithm,
}

/// Checksum computation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeChecksumResponse {
    /// Computed checksum
    pub checksum: Checksum,
}

// ============================================================================
// Versioning DTOs
// ============================================================================

/// Request to list versions of an asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListVersionsRequest {
    /// Asset name
    pub name: String,

    /// Whether to include deprecated versions
    #[serde(default)]
    pub include_deprecated: bool,
}

/// Response with asset versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListVersionsResponse {
    /// Asset name
    pub name: String,

    /// All versions
    pub versions: Vec<VersionInfo>,

    /// Latest active version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest: Option<Version>,
}

/// Information about a specific version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Version number
    pub version: Version,

    /// Asset ID
    pub asset_id: AssetId,

    /// Current status
    pub status: AssetStatus,

    /// When it was created
    pub created_at: DateTime<Utc>,

    /// When it was deprecated (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated_at: Option<DateTime<Utc>>,
}

/// Request to check for version conflicts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckVersionConflictRequest {
    /// Asset name
    pub name: String,

    /// Version to check
    pub version: Version,
}

/// Version conflict check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionConflictResult {
    /// Whether there's a conflict
    pub has_conflict: bool,

    /// Existing version (if conflict exists)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existing_version: Option<VersionInfo>,

    /// Message describing the conflict
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// ============================================================================
// Dependency DTOs
// ============================================================================

/// Request to get dependency graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDependencyGraphRequest {
    /// Asset ID
    pub asset_id: AssetId,

    /// Maximum depth to traverse (-1 for unlimited)
    #[serde(default = "default_max_depth")]
    pub max_depth: i32,
}

fn default_max_depth() -> i32 {
    -1 // unlimited
}

/// Dependency graph response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraphResponse {
    /// Root asset ID
    pub root: AssetId,

    /// All dependencies (direct and transitive)
    pub dependencies: Vec<DependencyNode>,

    /// Whether the graph was truncated due to max_depth
    pub truncated: bool,
}

/// Node in dependency graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyNode {
    /// Asset ID
    pub asset_id: AssetId,

    /// Asset name
    pub name: String,

    /// Asset version
    pub version: Version,

    /// Depth from root (0 = direct dependency)
    pub depth: i32,

    /// Direct dependencies of this node
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<AssetId>,
}

// ============================================================================
// Update DTOs
// ============================================================================

/// Request to update asset metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAssetRequest {
    /// Asset ID
    pub asset_id: AssetId,

    /// New description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// New license
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Tags to add
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub add_tags: Vec<String>,

    /// Tags to remove
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_tags: Vec<String>,

    /// Annotations to add/update
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub add_annotations: HashMap<String, String>,

    /// Annotation keys to remove
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_annotations: Vec<String>,

    /// New status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<AssetStatus>,
}

/// Response from updating an asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAssetResponse {
    /// Updated asset
    pub asset: Asset,

    /// Fields that were updated
    pub updated_fields: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_request_defaults() {
        let req = SearchAssetsRequest::default();
        assert_eq!(req.limit, 50);
        assert_eq!(req.offset, 0);
        assert!(req.exclude_deprecated);
        assert_eq!(req.sort_by, SortField::CreatedAt);
        assert_eq!(req.sort_order, SortOrder::Descending);
    }

    #[test]
    fn test_validation_result_is_valid() {
        let result = ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
        };
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_dependency_graph_request_default() {
        let req = GetDependencyGraphRequest {
            asset_id: AssetId::new(),
            max_depth: default_max_depth(),
        };
        assert_eq!(req.max_depth, -1);
    }
}
