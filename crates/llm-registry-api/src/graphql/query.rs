//! GraphQL query resolvers
//!
//! This module implements all GraphQL query operations.

use async_graphql::{Context, Object, Result};
use llm_registry_core::AssetId;
use llm_registry_service::{SearchAssetsRequest, ServiceRegistry, SortField, SortOrder};
use std::sync::Arc;

use super::types::{GqlAsset, GqlAssetConnection, GqlAssetFilter, GqlDependencyNode};
use crate::error::ApiError;

/// Root Query type for GraphQL
pub struct Query;

#[Object]
impl Query {
    /// Get an asset by ID
    async fn asset(&self, ctx: &Context<'_>, id: String) -> Result<Option<GqlAsset>> {
        let services = ctx.data::<Arc<ServiceRegistry>>()?;

        let asset_id = id
            .parse::<AssetId>()
            .map_err(|e| ApiError::bad_request(format!("Invalid asset ID: {}", e)))?;

        let asset = services
            .search()
            .get_asset(&asset_id)
            .await
            .map_err(|e| ApiError::from(e))?;

        Ok(asset.map(GqlAsset))
    }

    /// Search and list assets with optional filters
    async fn assets(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Filter criteria", default)] filter: Option<GqlAssetFilter>,
        #[graphql(desc = "Number of items to return", default = 20)] limit: i64,
        #[graphql(desc = "Number of items to skip", default = 0)] offset: i64,
    ) -> Result<GqlAssetConnection> {
        let services = ctx.data::<Arc<ServiceRegistry>>()?;

        // Build search request
        let mut search_request = SearchAssetsRequest {
            text: None,
            asset_types: vec![],
            tags: vec![],
            author: None,
            storage_backend: None,
            exclude_deprecated: true,
            limit,
            offset,
            sort_by: SortField::CreatedAt,
            sort_order: SortOrder::Descending,
        };

        // Apply filters if provided
        if let Some(f) = filter {
            if let Some(asset_type) = f.asset_type {
                search_request.asset_types = vec![asset_type.to_core()];
            }
            if let Some(tags) = f.tags {
                search_request.tags = tags;
            }
            search_request.text = f.name;
            // Note: GqlAssetFilter has a status field but SearchAssetsRequest doesn't have one directly
            // We can use exclude_deprecated based on status if needed
            if let Some(status) = f.status {
                use super::types::GqlAssetStatus;
                search_request.exclude_deprecated = status != GqlAssetStatus::Deprecated;
            }
        }

        let response = services
            .search()
            .search_assets(search_request)
            .await
            .map_err(|e| ApiError::from(e))?;

        Ok(GqlAssetConnection {
            nodes: response.assets.into_iter().map(GqlAsset).collect(),
            total_count: response.total,
            has_next_page: (response.offset + response.limit) < response.total,
        })
    }

    /// Get all dependencies for an asset
    async fn dependencies(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Asset ID")] id: String,
        #[graphql(desc = "Maximum depth to traverse (-1 for unlimited)", default = -1)]
        max_depth: i32,
    ) -> Result<Vec<GqlDependencyNode>> {
        let services = ctx.data::<Arc<ServiceRegistry>>()?;

        let asset_id = id
            .parse::<AssetId>()
            .map_err(|e| ApiError::bad_request(format!("Invalid asset ID: {}", e)))?;

        let request = llm_registry_service::GetDependencyGraphRequest {
            asset_id,
            max_depth,
        };

        let response = services
            .search()
            .get_dependency_graph(request)
            .await
            .map_err(|e| ApiError::from(e))?;

        Ok(response
            .dependencies
            .into_iter()
            .map(GqlDependencyNode::from)
            .collect())
    }

    /// Get all assets that depend on this asset (reverse dependencies)
    async fn dependents(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Asset ID")] id: String,
    ) -> Result<Vec<GqlAsset>> {
        let services = ctx.data::<Arc<ServiceRegistry>>()?;

        let asset_id = id
            .parse::<AssetId>()
            .map_err(|e| ApiError::bad_request(format!("Invalid asset ID: {}", e)))?;

        let dependents = services
            .search()
            .get_reverse_dependencies(&asset_id)
            .await
            .map_err(|e| ApiError::from(e))?;

        Ok(dependents.into_iter().map(GqlAsset).collect())
    }

    /// Get all unique tags across all assets
    async fn all_tags(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
        let services = ctx.data::<Arc<ServiceRegistry>>()?;

        let tags = services
            .search()
            .list_all_tags()
            .await
            .map_err(|e| ApiError::from(e))?;

        Ok(tags)
    }

    /// Health check - returns true if service is healthy
    async fn health(&self, ctx: &Context<'_>) -> Result<bool> {
        let services = ctx.data::<Arc<ServiceRegistry>>()?;

        // Perform a simple database check
        match services.search().list_all_tags().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Get API version information
    async fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}
