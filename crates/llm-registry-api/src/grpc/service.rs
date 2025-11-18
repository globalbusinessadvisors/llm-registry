//! gRPC service implementation
//!
//! This module implements the RegistryService gRPC service defined in the proto file.

use super::converters::*;
use super::proto::{self, registry_service_server::RegistryService};
use crate::error::ApiError;
use llm_registry_core::{AssetId, AssetReference};
use llm_registry_service::{
    GetDependencyGraphRequest, RegisterAssetRequest, SearchAssetsRequest, ServiceRegistry,
    UpdateAssetRequest,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

/// gRPC service implementation
#[derive(Clone)]
pub struct RegistryServiceImpl {
    services: Arc<ServiceRegistry>,
}

impl RegistryServiceImpl {
    /// Create a new gRPC service instance
    pub fn new(services: Arc<ServiceRegistry>) -> Self {
        Self { services }
    }
}

#[tonic::async_trait]
impl RegistryService for RegistryServiceImpl {
    /// Type alias for streaming response
    type WatchAssetsStream =
        std::pin::Pin<Box<dyn futures::Stream<Item = Result<proto::AssetEvent, Status>> + Send>>;

    /// Register a new asset
    async fn register_asset(
        &self,
        request: Request<proto::RegisterAssetRequest>,
    ) -> Result<Response<proto::RegisterAssetResponse>, Status> {
        let req = request.into_inner();

        // Convert proto request to domain request
        let asset_type = asset_type_from_i32(req.asset_type)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let version = parse_version(&req.version)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let storage = req
            .storage
            .ok_or_else(|| Status::invalid_argument("Storage location is required"))?
            .try_into()
            .map_err(|e: ApiError| Status::invalid_argument(e.to_string()))?;

        let checksum = req
            .checksum
            .ok_or_else(|| Status::invalid_argument("Checksum is required"))?
            .try_into()
            .map_err(|e: ApiError| Status::invalid_argument(e.to_string()))?;

        let provenance = req
            .provenance
            .map(|p| p.try_into())
            .transpose()
            .map_err(|e: ApiError| Status::invalid_argument(e.to_string()))?;

        let dependencies: Result<Vec<AssetReference>, ApiError> = req
            .dependencies
            .into_iter()
            .map(|d| d.try_into())
            .collect();
        let dependencies = dependencies.map_err(|e| Status::invalid_argument(e.to_string()))?;

        let domain_request = RegisterAssetRequest {
            asset_type,
            name: req.name,
            version,
            description: req.description,
            license: req.license,
            tags: req.tags,
            annotations: req.annotations,
            storage,
            checksum,
            provenance,
            dependencies,
            size_bytes: req.size_bytes,
            content_type: req.content_type,
        };

        // Execute registration
        let response = self
            .services
            .registration()
            .register_asset(domain_request)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(proto::RegisterAssetResponse {
            asset: Some(response.asset.into()),
            warnings: response.warnings,
        }))
    }

    /// Get an asset by ID
    async fn get_asset(
        &self,
        request: Request<proto::GetAssetRequest>,
    ) -> Result<Response<proto::GetAssetResponse>, Status> {
        let req = request.into_inner();

        let asset_id = req
            .id
            .parse::<AssetId>()
            .map_err(|e| Status::invalid_argument(format!("Invalid asset ID: {}", e)))?;

        let asset = self
            .services
            .search()
            .get_asset(&asset_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(proto::GetAssetResponse {
            asset: asset.map(|a| a.into()),
        }))
    }

    /// Search and list assets
    async fn search_assets(
        &self,
        request: Request<proto::SearchAssetsRequest>,
    ) -> Result<Response<proto::SearchAssetsResponse>, Status> {
        let req = request.into_inner();

        let asset_types: Result<Vec<_>, ApiError> = req
            .asset_types
            .into_iter()
            .map(asset_type_from_i32)
            .collect();
        let asset_types = asset_types.map_err(|e| Status::invalid_argument(e.to_string()))?;

        let sort_by = sort_field_from_i32(req.sort_by)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let sort_order = sort_order_from_i32(req.sort_order)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let search_request = SearchAssetsRequest {
            text: req.text,
            asset_types,
            tags: req.tags,
            author: req.author,
            storage_backend: req.storage_backend,
            exclude_deprecated: req.exclude_deprecated,
            limit: req.limit,
            offset: req.offset,
            sort_by,
            sort_order,
        };

        let response = self
            .services
            .search()
            .search_assets(search_request)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(proto::SearchAssetsResponse {
            assets: response.assets.into_iter().map(|a| a.into()).collect(),
            total: response.total,
            offset: response.offset,
            limit: response.limit,
            has_more: (response.offset + response.limit) < response.total,
        }))
    }

    /// Update an existing asset
    async fn update_asset(
        &self,
        request: Request<proto::UpdateAssetRequest>,
    ) -> Result<Response<proto::UpdateAssetResponse>, Status> {
        let req = request.into_inner();

        let asset_id = req
            .asset_id
            .parse::<AssetId>()
            .map_err(|e| Status::invalid_argument(format!("Invalid asset ID: {}", e)))?;

        let status = req
            .status
            .map(asset_status_from_i32)
            .transpose()
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let update_request = UpdateAssetRequest {
            asset_id,
            status,
            description: req.description,
            license: req.license,
            add_tags: req.add_tags,
            remove_tags: req.remove_tags,
            add_annotations: req.add_annotations,
            remove_annotations: req.remove_annotations,
        };

        let response = self
            .services
            .registration()
            .update_asset(update_request)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(proto::UpdateAssetResponse {
            asset: Some(response.asset.into()),
            updated_fields: response.updated_fields,
        }))
    }

    /// Delete an asset
    async fn delete_asset(
        &self,
        request: Request<proto::DeleteAssetRequest>,
    ) -> Result<Response<proto::DeleteAssetResponse>, Status> {
        let req = request.into_inner();

        let asset_id = req
            .asset_id
            .parse::<AssetId>()
            .map_err(|e| Status::invalid_argument(format!("Invalid asset ID: {}", e)))?;

        self.services
            .registration()
            .delete_asset(&asset_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(proto::DeleteAssetResponse {
            asset_id: req.asset_id,
            message: "Asset deleted successfully".to_string(),
        }))
    }

    /// Get dependency graph for an asset
    async fn get_dependencies(
        &self,
        request: Request<proto::GetDependenciesRequest>,
    ) -> Result<Response<proto::GetDependenciesResponse>, Status> {
        let req = request.into_inner();

        let asset_id = req
            .asset_id
            .parse::<AssetId>()
            .map_err(|e| Status::invalid_argument(format!("Invalid asset ID: {}", e)))?;

        let graph_request = GetDependencyGraphRequest {
            asset_id,
            max_depth: req.max_depth,
        };

        let response = self
            .services
            .search()
            .get_dependency_graph(graph_request)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(proto::GetDependenciesResponse {
            dependencies: response
                .dependencies
                .into_iter()
                .map(|d| d.into())
                .collect(),
        }))
    }

    /// Get reverse dependencies (dependents) for an asset
    async fn get_dependents(
        &self,
        request: Request<proto::GetDependentsRequest>,
    ) -> Result<Response<proto::GetDependentsResponse>, Status> {
        let req = request.into_inner();

        let asset_id = req
            .asset_id
            .parse::<AssetId>()
            .map_err(|e| Status::invalid_argument(format!("Invalid asset ID: {}", e)))?;

        let dependents = self
            .services
            .search()
            .get_reverse_dependencies(&asset_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(proto::GetDependentsResponse {
            dependents: dependents.into_iter().map(|a| a.into()).collect(),
        }))
    }

    /// List all unique tags
    async fn list_tags(
        &self,
        _request: Request<proto::ListTagsRequest>,
    ) -> Result<Response<proto::ListTagsResponse>, Status> {
        let tags = self
            .services
            .search()
            .list_all_tags()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(proto::ListTagsResponse { tags }))
    }

    /// Health check
    async fn get_health(
        &self,
        _request: Request<proto::HealthRequest>,
    ) -> Result<Response<proto::HealthResponse>, Status> {
        // Simple health check - try to query tags
        let healthy = match self.services.search().list_all_tags().await {
            Ok(_) => true,
            Err(_) => false,
        };

        Ok(Response::new(proto::HealthResponse {
            healthy,
            version: env!("CARGO_PKG_VERSION").to_string(),
            message: if healthy {
                Some("Service is healthy".to_string())
            } else {
                Some("Service is unhealthy".to_string())
            },
        }))
    }

    /// Get version information
    async fn get_version(
        &self,
        _request: Request<proto::VersionRequest>,
    ) -> Result<Response<proto::VersionResponse>, Status> {
        Ok(Response::new(proto::VersionResponse {
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_date: option_env!("VERGEN_BUILD_TIMESTAMP")
                .unwrap_or("unknown")
                .to_string(),
            git_commit: option_env!("VERGEN_GIT_SHA")
                .unwrap_or("unknown")
                .to_string(),
        }))
    }

    /// Watch assets (server streaming)
    async fn watch_assets(
        &self,
        _request: Request<proto::WatchAssetsRequest>,
    ) -> Result<Response<Self::WatchAssetsStream>, Status> {
        // This would be implemented with actual event streaming
        // For now, return unimplemented
        Err(Status::unimplemented(
            "Asset watching not yet implemented - requires event streaming infrastructure",
        ))
    }

    /// Batch register assets (client streaming)
    async fn batch_register(
        &self,
        _request: Request<tonic::Streaming<proto::RegisterAssetRequest>>,
    ) -> Result<Response<proto::BatchRegisterResponse>, Status> {
        // This would be implemented with actual batch processing
        // For now, return unimplemented
        Err(Status::unimplemented(
            "Batch registration not yet implemented - use individual registration",
        ))
    }
}
