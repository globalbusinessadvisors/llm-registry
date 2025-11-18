//! GraphQL mutation resolvers
//!
//! This module implements all GraphQL mutation operations.

use async_graphql::{Context, InputObject, Object, Result};
use llm_registry_core::{
    AssetId, Checksum, HashAlgorithm, StorageBackend, StorageLocation,
};
use llm_registry_service::{RegisterAssetRequest, ServiceRegistry, UpdateAssetRequest};
use semver::Version;
use std::sync::Arc;

use super::types::{
    GqlAsset, GqlAssetStatus, GqlAssetType, GqlDeleteResult, GqlRegisterResult, GqlUpdateResult,
};
use crate::auth::AuthUser;
use crate::error::ApiError;

/// Root Mutation type for GraphQL
pub struct Mutation;

/// Input for registering a new asset
#[derive(InputObject)]
pub struct RegisterAssetInput {
    /// Asset type
    pub asset_type: GqlAssetType,
    /// Asset name
    pub name: String,
    /// Asset version (semver)
    pub version: String,
    /// Optional description
    pub description: Option<String>,
    /// Optional license
    pub license: Option<String>,
    /// Tags for categorization
    #[graphql(default)]
    pub tags: Vec<String>,
    /// Key-value annotations
    #[graphql(default)]
    pub annotations: Vec<AnnotationInput>,
    /// Storage path
    pub storage_path: String,
    /// Storage backend type (s3, gcs, azure, local)
    #[graphql(default = "local")]
    pub storage_backend: String,
    /// Optional storage URI
    pub storage_uri: Option<String>,
    /// Checksum value (hex string)
    pub checksum: String,
    /// Checksum algorithm (SHA256, SHA3_256, BLAKE3)
    #[graphql(default = "SHA256")]
    pub checksum_algorithm: String,
    /// File size in bytes
    pub size_bytes: Option<u64>,
    /// Content type
    pub content_type: Option<String>,
}

/// Input for updating an asset
#[derive(InputObject)]
pub struct UpdateAssetInput {
    /// Asset ID to update
    pub asset_id: String,
    /// New status
    pub status: Option<GqlAssetStatus>,
    /// New description
    pub description: Option<String>,
    /// New license
    pub license: Option<String>,
    /// Tags to add
    #[graphql(default)]
    pub add_tags: Vec<String>,
    /// Tags to remove
    #[graphql(default)]
    pub remove_tags: Vec<String>,
    /// Annotations to add/update
    #[graphql(default)]
    pub add_annotations: Vec<AnnotationInput>,
    /// Annotation keys to remove
    #[graphql(default)]
    pub remove_annotations: Vec<String>,
}

/// Annotation key-value pair
#[derive(InputObject)]
pub struct AnnotationInput {
    /// Annotation key
    pub key: String,
    /// Annotation value
    pub value: String,
}

#[Object]
impl Mutation {
    /// Register a new asset
    async fn register_asset(
        &self,
        ctx: &Context<'_>,
        input: RegisterAssetInput,
    ) -> Result<GqlRegisterResult> {
        let services = ctx.data::<Arc<ServiceRegistry>>()?;

        // Check authentication (optional - can be made required)
        let _user = ctx.data_opt::<AuthUser>();

        // Parse version
        let version = Version::parse(&input.version)
            .map_err(|e| ApiError::bad_request(format!("Invalid version: {}", e)))?;

        // Parse hash algorithm
        let algorithm = match input.checksum_algorithm.to_uppercase().as_str() {
            "SHA256" => HashAlgorithm::SHA256,
            "SHA3_256" | "SHA3-256" => HashAlgorithm::SHA3_256,
            "BLAKE3" => HashAlgorithm::BLAKE3,
            _ => return Err(ApiError::bad_request("Invalid checksum algorithm"))?
        };

        // Create storage backend
        let backend = match input.storage_backend.to_lowercase().as_str() {
            "filesystem" | "local" => StorageBackend::FileSystem {
                base_path: "/var/lib/llm-registry".to_string(),
            },
            _ => StorageBackend::FileSystem {
                base_path: "/var/lib/llm-registry".to_string(),
            },
        };

        // Create storage location
        let storage = StorageLocation {
            backend,
            path: input.storage_path,
            uri: input.storage_uri,
        };

        // Create checksum
        let checksum = Checksum {
            algorithm,
            value: input.checksum,
        };

        // Build registration request
        let request = RegisterAssetRequest {
            asset_type: input.asset_type.to_core(),
            name: input.name,
            version,
            description: input.description,
            license: input.license,
            tags: input.tags,
            annotations: input
                .annotations
                .into_iter()
                .map(|a| (a.key, a.value))
                .collect(),
            storage,
            checksum,
            provenance: None,
            dependencies: vec![],
            size_bytes: input.size_bytes,
            content_type: input.content_type,
        };

        let response = services
            .registration()
            .register_asset(request)
            .await
            .map_err(|e| ApiError::from(e))?;

        Ok(GqlRegisterResult {
            asset: GqlAsset(response.asset),
            message: "Asset registered successfully".to_string(),
        })
    }

    /// Update an existing asset
    async fn update_asset(
        &self,
        ctx: &Context<'_>,
        input: UpdateAssetInput,
    ) -> Result<GqlUpdateResult> {
        let services = ctx.data::<Arc<ServiceRegistry>>()?;

        // Check authentication (optional - can be made required)
        let _user = ctx.data_opt::<AuthUser>();

        // Parse asset ID
        let asset_id = input
            .asset_id
            .parse::<AssetId>()
            .map_err(|e| ApiError::bad_request(format!("Invalid asset ID: {}", e)))?;

        // Build update request
        let request = UpdateAssetRequest {
            asset_id,
            status: input.status.map(|s| s.to_core()),
            description: input.description,
            license: input.license,
            add_tags: input.add_tags,
            remove_tags: input.remove_tags,
            add_annotations: input
                .add_annotations
                .into_iter()
                .map(|a| (a.key, a.value))
                .collect(),
            remove_annotations: input.remove_annotations,
        };

        let response = services
            .registration()
            .update_asset(request)
            .await
            .map_err(|e| ApiError::from(e))?;

        Ok(GqlUpdateResult {
            asset: GqlAsset(response.asset),
            message: "Asset updated successfully".to_string(),
        })
    }

    /// Delete an asset
    async fn delete_asset(&self, ctx: &Context<'_>, id: String) -> Result<GqlDeleteResult> {
        let services = ctx.data::<Arc<ServiceRegistry>>()?;

        // Check authentication (optional - can be made required)
        let _user = ctx.data_opt::<AuthUser>();

        // Parse asset ID
        let asset_id = id
            .parse::<AssetId>()
            .map_err(|e| ApiError::bad_request(format!("Invalid asset ID: {}", e)))?;

        services
            .registration()
            .delete_asset(&asset_id)
            .await
            .map_err(|e| ApiError::from(e))?;

        Ok(GqlDeleteResult {
            asset_id: id,
            message: "Asset deleted successfully".to_string(),
        })
    }
}
