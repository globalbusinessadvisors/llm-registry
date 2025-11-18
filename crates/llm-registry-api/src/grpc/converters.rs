// ! Type converters between protobuf and domain types
//!
//! This module provides conversion functions between the generated protobuf types
//! and the core domain types used throughout the registry.

use super::proto;
use crate::error::ApiError;
use llm_registry_core::{
    Asset, AssetId, AssetMetadata, AssetReference, AssetStatus, AssetType, Checksum,
    HashAlgorithm, Provenance, StorageBackend, StorageLocation,
};
use llm_registry_service::{DependencyNode, SortField, SortOrder};
use semver::Version;

// ============================================================================
// Enum Conversions
// ============================================================================

impl From<AssetType> for proto::AssetType {
    fn from(at: AssetType) -> Self {
        match at {
            AssetType::Model => proto::AssetType::Model,
            AssetType::Pipeline => proto::AssetType::Pipeline,
            AssetType::TestSuite => proto::AssetType::TestSuite,
            AssetType::Policy => proto::AssetType::Policy,
            AssetType::Dataset => proto::AssetType::Dataset,
            AssetType::Custom(_) => proto::AssetType::Model, // Default for custom
        }
    }
}

/// Convert i32 to AssetType (helper function to avoid orphan rule violations)
pub fn asset_type_from_i32(value: i32) -> Result<AssetType, ApiError> {
    match proto::AssetType::try_from(value) {
        Ok(proto::AssetType::Unspecified) => {
            Err(ApiError::bad_request("Asset type must be specified"))
        }
        Ok(proto::AssetType::Model) => Ok(AssetType::Model),
        Ok(proto::AssetType::Pipeline) => Ok(AssetType::Pipeline),
        Ok(proto::AssetType::TestSuite) => Ok(AssetType::TestSuite),
        Ok(proto::AssetType::Policy) => Ok(AssetType::Policy),
        Ok(proto::AssetType::Dataset) => Ok(AssetType::Dataset),
        Err(_) => Err(ApiError::bad_request("Invalid asset type")),
    }
}

impl From<AssetStatus> for proto::AssetStatus {
    fn from(status: AssetStatus) -> Self {
        match status {
            AssetStatus::Active => proto::AssetStatus::Active,
            AssetStatus::Deprecated => proto::AssetStatus::Deprecated,
            AssetStatus::Archived => proto::AssetStatus::Archived,
            AssetStatus::NonCompliant => proto::AssetStatus::NonCompliant,
        }
    }
}

/// Convert i32 to AssetStatus (helper function to avoid orphan rule violations)
pub fn asset_status_from_i32(value: i32) -> Result<AssetStatus, ApiError> {
    match proto::AssetStatus::try_from(value) {
        Ok(proto::AssetStatus::Unspecified) => Ok(AssetStatus::Active),
        Ok(proto::AssetStatus::Active) => Ok(AssetStatus::Active),
        Ok(proto::AssetStatus::Deprecated) => Ok(AssetStatus::Deprecated),
        Ok(proto::AssetStatus::Archived) => Ok(AssetStatus::Archived),
        Ok(proto::AssetStatus::NonCompliant) => Ok(AssetStatus::NonCompliant),
        Err(_) => Err(ApiError::bad_request("Invalid asset status")),
    }
}

impl From<HashAlgorithm> for proto::HashAlgorithm {
    fn from(alg: HashAlgorithm) -> Self {
        match alg {
            HashAlgorithm::SHA256 => proto::HashAlgorithm::Sha256,
            HashAlgorithm::SHA3_256 => proto::HashAlgorithm::Sha3256,
            HashAlgorithm::BLAKE3 => proto::HashAlgorithm::Blake3,
        }
    }
}

/// Convert i32 to HashAlgorithm (helper function to avoid orphan rule violations)
pub fn hash_algorithm_from_i32(value: i32) -> Result<HashAlgorithm, ApiError> {
    match proto::HashAlgorithm::try_from(value) {
        Ok(proto::HashAlgorithm::Unspecified) | Ok(proto::HashAlgorithm::Sha256) => {
            Ok(HashAlgorithm::SHA256)
        }
        Ok(proto::HashAlgorithm::Sha3256) => Ok(HashAlgorithm::SHA3_256),
        Ok(proto::HashAlgorithm::Blake3) => Ok(HashAlgorithm::BLAKE3),
        Err(_) => Err(ApiError::bad_request("Invalid hash algorithm")),
    }
}

impl From<SortField> for proto::SortField {
    fn from(field: SortField) -> Self {
        match field {
            SortField::CreatedAt => proto::SortField::CreatedAt,
            SortField::UpdatedAt => proto::SortField::UpdatedAt,
            SortField::Name => proto::SortField::Name,
            SortField::Version => proto::SortField::Version,
            SortField::SizeBytes => proto::SortField::SizeBytes,
        }
    }
}

/// Convert i32 to SortField (helper function to avoid orphan rule violations)
pub fn sort_field_from_i32(value: i32) -> Result<SortField, ApiError> {
    match proto::SortField::try_from(value) {
        Ok(proto::SortField::Unspecified) | Ok(proto::SortField::CreatedAt) => {
            Ok(SortField::CreatedAt)
        }
        Ok(proto::SortField::UpdatedAt) => Ok(SortField::UpdatedAt),
        Ok(proto::SortField::Name) => Ok(SortField::Name),
        Ok(proto::SortField::Version) => Ok(SortField::Version),
        Ok(proto::SortField::SizeBytes) => Ok(SortField::SizeBytes),
        Err(_) => Err(ApiError::bad_request("Invalid sort field")),
    }
}

impl From<SortOrder> for proto::SortOrder {
    fn from(order: SortOrder) -> Self {
        match order {
            SortOrder::Ascending => proto::SortOrder::Ascending,
            SortOrder::Descending => proto::SortOrder::Descending,
        }
    }
}

/// Convert i32 to SortOrder (helper function to avoid orphan rule violations)
pub fn sort_order_from_i32(value: i32) -> Result<SortOrder, ApiError> {
    match proto::SortOrder::try_from(value) {
        Ok(proto::SortOrder::Unspecified) | Ok(proto::SortOrder::Descending) => {
            Ok(SortOrder::Descending)
        }
        Ok(proto::SortOrder::Ascending) => Ok(SortOrder::Ascending),
        Err(_) => Err(ApiError::bad_request("Invalid sort order")),
    }
}

// ============================================================================
// Complex Type Conversions
// ============================================================================

/// Convert domain Asset to proto Asset
impl From<Asset> for proto::Asset {
    fn from(asset: Asset) -> Self {
        proto::Asset {
            id: asset.id.to_string(),
            asset_type: proto::AssetType::from(asset.asset_type) as i32,
            metadata: Some(proto::AssetMetadata::from(asset.metadata)),
            storage: Some(proto::StorageLocation::from(asset.storage)),
            checksum: Some(proto::Checksum::from(asset.checksum)),
            status: proto::AssetStatus::from(asset.status) as i32,
            provenance: asset.provenance.map(proto::Provenance::from),
            dependencies: asset
                .dependencies
                .into_iter()
                .map(proto::AssetReference::from)
                .collect(),
            created_at: asset.created_at.to_rfc3339(),
            updated_at: asset.updated_at.to_rfc3339(),
            deprecated_at: asset.deprecated_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// Convert domain AssetMetadata to proto
impl From<AssetMetadata> for proto::AssetMetadata {
    fn from(meta: AssetMetadata) -> Self {
        proto::AssetMetadata {
            name: meta.name,
            version: meta.version.to_string(),
            description: meta.description,
            license: meta.license,
            tags: meta.tags,
            annotations: meta.annotations,
            size_bytes: meta.size_bytes,
            content_type: meta.content_type,
        }
    }
}

/// Convert domain StorageLocation to proto
impl From<StorageLocation> for proto::StorageLocation {
    fn from(storage: StorageLocation) -> Self {
        let (backend_type, config) = match storage.backend {
            StorageBackend::S3 { bucket, region, endpoint } => (
                proto::StorageBackend::S3 as i32,
                Some(proto::storage_config::Config::S3(proto::S3Config {
                    bucket,
                    region,
                    endpoint,
                })),
            ),
            StorageBackend::GCS { bucket, project_id } => (
                proto::StorageBackend::Gcs as i32,
                Some(proto::storage_config::Config::Gcs(proto::GcsConfig {
                    bucket,
                    project_id,
                })),
            ),
            StorageBackend::AzureBlob { account_name, container } => (
                proto::StorageBackend::AzureBlob as i32,
                Some(proto::storage_config::Config::Azure(proto::AzureBlobConfig {
                    account_name,
                    container,
                })),
            ),
            StorageBackend::MinIO { bucket, endpoint } => (
                proto::StorageBackend::Minio as i32,
                Some(proto::storage_config::Config::Minio(proto::MinIoConfig {
                    bucket,
                    endpoint,
                })),
            ),
            StorageBackend::FileSystem { base_path } => (
                proto::StorageBackend::Filesystem as i32,
                Some(proto::storage_config::Config::Filesystem(
                    proto::FileSystemConfig { base_path },
                )),
            ),
        };

        proto::StorageLocation {
            backend: backend_type,
            path: storage.path,
            uri: storage.uri,
            config: config.map(|c| proto::StorageConfig { config: Some(c) }),
        }
    }
}

/// Convert proto StorageLocation to domain
impl TryFrom<proto::StorageLocation> for StorageLocation {
    type Error = ApiError;

    fn try_from(proto: proto::StorageLocation) -> Result<Self, Self::Error> {
        let backend = if let Some(config) = proto.config.and_then(|c| c.config) {
            match config {
                proto::storage_config::Config::S3(s3) => StorageBackend::S3 {
                    bucket: s3.bucket,
                    region: s3.region,
                    endpoint: s3.endpoint,
                },
                proto::storage_config::Config::Gcs(gcs) => StorageBackend::GCS {
                    bucket: gcs.bucket,
                    project_id: gcs.project_id,
                },
                proto::storage_config::Config::Azure(azure) => StorageBackend::AzureBlob {
                    account_name: azure.account_name,
                    container: azure.container,
                },
                proto::storage_config::Config::Minio(minio) => StorageBackend::MinIO {
                    bucket: minio.bucket,
                    endpoint: minio.endpoint,
                },
                proto::storage_config::Config::Filesystem(fs) => StorageBackend::FileSystem {
                    base_path: fs.base_path,
                },
            }
        } else {
            // Default to FileSystem if no config provided
            StorageBackend::FileSystem {
                base_path: "/var/lib/llm-registry".to_string(),
            }
        };

        Ok(StorageLocation {
            backend,
            path: proto.path,
            uri: proto.uri,
        })
    }
}

/// Convert domain Checksum to proto
impl From<Checksum> for proto::Checksum {
    fn from(checksum: Checksum) -> Self {
        proto::Checksum {
            algorithm: proto::HashAlgorithm::from(checksum.algorithm) as i32,
            value: checksum.value,
        }
    }
}

/// Convert proto Checksum to domain
impl TryFrom<proto::Checksum> for Checksum {
    type Error = ApiError;

    fn try_from(proto: proto::Checksum) -> Result<Self, Self::Error> {
        Ok(Checksum {
            algorithm: hash_algorithm_from_i32(proto.algorithm)?,
            value: proto.value,
        })
    }
}

/// Convert domain Provenance to proto
impl From<Provenance> for proto::Provenance {
    fn from(prov: Provenance) -> Self {
        proto::Provenance {
            source: prov.source_repo,
            author: prov.author,
            created: Some(prov.created_at.to_rfc3339()),
            metadata: prov.build_metadata,
        }
    }
}

/// Convert proto Provenance to domain
impl TryFrom<proto::Provenance> for Provenance {
    type Error = ApiError;

    fn try_from(proto: proto::Provenance) -> Result<Self, Self::Error> {
        use chrono::DateTime;

        let created_at = if let Some(created_str) = proto.created {
            DateTime::parse_from_rfc3339(&created_str)
                .map_err(|e| ApiError::bad_request(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&chrono::Utc)
        } else {
            chrono::Utc::now()
        };

        Ok(Provenance {
            source_repo: proto.source,
            commit_hash: None,
            build_id: None,
            author: proto.author,
            created_at,
            build_metadata: proto.metadata,
        })
    }
}

/// Convert domain AssetReference to proto
impl From<AssetReference> for proto::AssetReference {
    fn from(ref_: AssetReference) -> Self {
        let reference = match ref_ {
            AssetReference::ById { id } => proto::asset_reference::Reference::Id(id.to_string()),
            AssetReference::ByNameVersion { name, version } => {
                proto::asset_reference::Reference::NameVersion(proto::NameVersion {
                    name,
                    version: version.to_string(),
                })
            }
        };

        proto::AssetReference {
            reference: Some(reference),
        }
    }
}

/// Convert proto AssetReference to domain
impl TryFrom<proto::AssetReference> for AssetReference {
    type Error = ApiError;

    fn try_from(proto: proto::AssetReference) -> Result<Self, Self::Error> {
        match proto.reference {
            Some(proto::asset_reference::Reference::Id(id)) => {
                let asset_id = id
                    .parse::<AssetId>()
                    .map_err(|e| ApiError::bad_request(format!("Invalid asset ID: {}", e)))?;
                Ok(AssetReference::ById { id: asset_id })
            }
            Some(proto::asset_reference::Reference::NameVersion(nv)) => {
                // Validate version format
                Version::parse(&nv.version)
                    .map_err(|e| ApiError::bad_request(format!("Invalid version: {}", e)))?;
                Ok(AssetReference::ByNameVersion {
                    name: nv.name,
                    version: nv.version,
                })
            }
            None => Err(ApiError::bad_request("Asset reference must be specified")),
        }
    }
}

/// Convert domain DependencyNode to proto
impl From<DependencyNode> for proto::DependencyNode {
    fn from(node: DependencyNode) -> Self {
        proto::DependencyNode {
            asset_id: node.asset_id.to_string(),
            name: node.name,
            version: node.version.to_string(),
            depth: node.depth,
            dependency_count: node.dependencies.len() as u32,
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert RFC3339 string to chrono DateTime
pub fn parse_timestamp(s: &str) -> Result<chrono::DateTime<chrono::Utc>, ApiError> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| ApiError::bad_request(format!("Invalid timestamp: {}", e)))
}

/// Convert Version string to semver::Version
pub fn parse_version(s: &str) -> Result<Version, ApiError> {
    Version::parse(s).map_err(|e| ApiError::bad_request(format!("Invalid version: {}", e)))
}
