//! API request handlers
//!
//! This module implements HTTP request handlers for all API endpoints.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use llm_registry_core::AssetId;
use llm_registry_service::{
    GetDependencyGraphRequest, RegisterAssetRequest, SearchAssetsRequest, ServiceRegistry,
    UpdateAssetRequest,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, instrument};

use crate::{
    error::{ApiError, ApiResult},
    responses::{
        created, deleted, ok, ApiResponse, ComponentHealth, HealthResponse,
        PaginatedResponse,
    },
};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Service registry
    pub services: Arc<ServiceRegistry>,
}

impl AppState {
    /// Create new application state
    pub fn new(services: ServiceRegistry) -> Self {
        Self {
            services: Arc::new(services),
        }
    }
}

// ============================================================================
// Asset Management Handlers
// ============================================================================

/// Register a new asset
#[instrument(skip(state))]
pub async fn register_asset(
    State(state): State<AppState>,
    Json(request): Json<RegisterAssetRequest>,
) -> ApiResult<(StatusCode, Json<ApiResponse<llm_registry_service::RegisterAssetResponse>>)> {
    info!(
        "Registering asset: {}@{}",
        request.name, request.version
    );

    let response = state
        .services
        .registration()
        .register_asset(request)
        .await
        .map_err(ApiError::from)?;

    Ok(created(response))
}

/// Get asset by ID
#[instrument(skip(state))]
pub async fn get_asset(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiResponse<llm_registry_core::Asset>>> {
    debug!("Getting asset: {}", id);

    let asset_id = id.parse::<AssetId>().map_err(|e| {
        ApiError::bad_request(format!("Invalid asset ID: {}", e))
    })?;

    let asset = state
        .services
        .search()
        .get_asset(&asset_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("Asset not found: {}", id)))?;

    Ok(Json(ok(asset)))
}

/// List/search assets with pagination
#[instrument(skip(state))]
pub async fn list_assets(
    State(state): State<AppState>,
    Query(params): Query<SearchAssetsRequest>,
) -> ApiResult<Json<PaginatedResponse<llm_registry_core::Asset>>> {
    debug!("Searching assets with filters: {:?}", params);

    let response = state
        .services
        .search()
        .search_assets(params)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(PaginatedResponse::new(
        response.assets,
        response.total,
        response.offset,
        response.limit,
    )))
}

/// Update asset metadata
#[instrument(skip(state))]
pub async fn update_asset(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(mut request): Json<UpdateAssetRequest>,
) -> ApiResult<Json<ApiResponse<llm_registry_service::UpdateAssetResponse>>> {
    info!("Updating asset: {}", id);

    let asset_id = id.parse::<AssetId>().map_err(|e| {
        ApiError::bad_request(format!("Invalid asset ID: {}", e))
    })?;

    // Set asset ID from path
    request.asset_id = asset_id;

    let response = state
        .services
        .registration()
        .update_asset(request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ok(response)))
}

/// Delete asset
#[instrument(skip(state))]
pub async fn delete_asset(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<crate::responses::EmptyResponse>)> {
    info!("Deleting asset: {}", id);

    let asset_id = id.parse::<AssetId>().map_err(|e| {
        ApiError::bad_request(format!("Invalid asset ID: {}", e))
    })?;

    state
        .services
        .registration()
        .delete_asset(&asset_id)
        .await
        .map_err(ApiError::from)?;

    Ok(deleted())
}

// ============================================================================
// Dependency Handlers
// ============================================================================

/// Get dependency graph for an asset
#[instrument(skip(state))]
pub async fn get_dependencies(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<DependencyGraphParams>,
) -> ApiResult<Json<ApiResponse<llm_registry_service::DependencyGraphResponse>>> {
    debug!("Getting dependency graph for asset: {}", id);

    let asset_id = id.parse::<AssetId>().map_err(|e| {
        ApiError::bad_request(format!("Invalid asset ID: {}", e))
    })?;

    let request = GetDependencyGraphRequest {
        asset_id,
        max_depth: params.max_depth.unwrap_or(-1),
    };

    let response = state
        .services
        .search()
        .get_dependency_graph(request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ok(response)))
}

/// Query parameters for dependency graph
#[derive(Debug, Deserialize)]
pub struct DependencyGraphParams {
    /// Maximum depth to traverse (-1 for unlimited)
    pub max_depth: Option<i32>,
}

/// Get reverse dependencies (dependents)
#[instrument(skip(state))]
pub async fn get_dependents(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiResponse<Vec<llm_registry_core::Asset>>>> {
    debug!("Getting dependents for asset: {}", id);

    let asset_id = id.parse::<AssetId>().map_err(|e| {
        ApiError::bad_request(format!("Invalid asset ID: {}", e))
    })?;

    let dependents = state
        .services
        .search()
        .get_reverse_dependencies(&asset_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ok(dependents)))
}

// ============================================================================
// Health & Metrics Handlers
// ============================================================================

/// Health check endpoint
#[instrument(skip(state))]
pub async fn health_check(State(state): State<AppState>) -> ApiResult<HealthResponse> {
    debug!("Health check requested");

    // For now, simple health check
    // In production, you'd check database connectivity, etc.
    let mut response = HealthResponse::healthy()
        .with_version(env!("CARGO_PKG_VERSION"));

    // Add database health check
    // Try to perform a simple database operation
    let db_health = match state.services.search().list_all_tags().await {
        Ok(_) => ComponentHealth::healthy(),
        Err(e) => ComponentHealth::unhealthy(format!("Database error: {}", e)),
    };

    response = response
        .with_check("database", db_health)
        .with_check("service", ComponentHealth::healthy())
        .compute_status();

    Ok(response)
}

/// Metrics endpoint (Prometheus format)
///
/// This endpoint exposes Prometheus metrics for monitoring.
/// Metrics are collected throughout the application lifecycle.
#[instrument]
pub async fn metrics() -> ApiResult<String> {
    debug!("Metrics requested");

    // Return basic info - actual metrics are handled by the server binary
    // which has access to the prometheus registry
    let metrics = format!(
        "# HELP llm_registry_info Registry information\n\
         # TYPE llm_registry_info gauge\n\
         llm_registry_info{{version=\"{}\"}} 1\n",
        env!("CARGO_PKG_VERSION")
    );

    Ok(metrics)
}

// ============================================================================
// Version & Info Handlers
// ============================================================================

/// Get API version information
#[instrument]
pub async fn version_info() -> ApiResult<Json<ApiResponse<VersionInfo>>> {
    let info = VersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        api_version: "v1".to_string(),
        build_timestamp: option_env!("BUILD_TIMESTAMP")
            .unwrap_or("unknown")
            .to_string(),
    };

    Ok(Json(ok(info)))
}

/// Version information
#[derive(Debug, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Semantic version
    pub version: String,

    /// API version
    pub api_version: String,

    /// Build timestamp
    pub build_timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info_creation() {
        let info = VersionInfo {
            version: "0.1.0".to_string(),
            api_version: "v1".to_string(),
            build_timestamp: "2024-01-01".to_string(),
        };

        assert_eq!(info.version, "0.1.0");
        assert_eq!(info.api_version, "v1");
    }
}
