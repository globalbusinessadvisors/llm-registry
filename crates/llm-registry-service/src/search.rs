//! Search service
//!
//! This module provides search and query operations for assets,
//! including tag filtering, text search, and dependency graph queries.

use async_trait::async_trait;
use llm_registry_core::{Asset, AssetId, AssetType};
use llm_registry_db::{AssetRepository, SearchQuery, SortField as DbSortField, SortOrder as DbSortOrder};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, instrument};

use crate::dto::{
    DependencyGraphResponse, DependencyNode, GetDependencyGraphRequest, SearchAssetsRequest,
    SearchAssetsResponse, SortField, SortOrder,
};
use crate::error::{ServiceError, ServiceResult};

/// Trait for search and query operations
#[async_trait]
pub trait SearchService: Send + Sync {
    /// Search for assets with filters
    async fn search_assets(&self, request: SearchAssetsRequest) -> ServiceResult<SearchAssetsResponse>;

    /// Get asset by ID
    async fn get_asset(&self, asset_id: &AssetId) -> ServiceResult<Option<Asset>>;

    /// Get asset by name and version
    async fn get_asset_by_name_version(&self, name: &str, version: &str) -> ServiceResult<Option<Asset>>;

    /// Get dependency graph for an asset
    async fn get_dependency_graph(&self, request: GetDependencyGraphRequest) -> ServiceResult<DependencyGraphResponse>;

    /// Get all tags in the registry
    async fn list_all_tags(&self) -> ServiceResult<Vec<String>>;

    /// Search assets by tags (assets must have all specified tags)
    async fn search_by_tags(&self, tags: Vec<String>) -> ServiceResult<Vec<Asset>>;

    /// Get assets of a specific type
    async fn get_assets_by_type(&self, asset_type: AssetType) -> ServiceResult<Vec<Asset>>;

    /// Get reverse dependencies (assets that depend on this asset)
    async fn get_reverse_dependencies(&self, asset_id: &AssetId) -> ServiceResult<Vec<Asset>>;
}

/// Default implementation of SearchService
pub struct DefaultSearchService {
    repository: Arc<dyn AssetRepository>,
}

impl DefaultSearchService {
    /// Create a new search service
    pub fn new(repository: Arc<dyn AssetRepository>) -> Self {
        Self { repository }
    }

    /// Convert DTO sort field to DB sort field
    fn convert_sort_field(&self, field: SortField) -> DbSortField {
        match field {
            SortField::CreatedAt => DbSortField::CreatedAt,
            SortField::UpdatedAt => DbSortField::UpdatedAt,
            SortField::Name => DbSortField::Name,
            SortField::Version => DbSortField::Version,
            SortField::SizeBytes => DbSortField::SizeBytes,
        }
    }

    /// Convert DTO sort order to DB sort order
    fn convert_sort_order(&self, order: SortOrder) -> DbSortOrder {
        match order {
            SortOrder::Ascending => DbSortOrder::Ascending,
            SortOrder::Descending => DbSortOrder::Descending,
        }
    }

    /// Build dependency graph recursively
    fn build_dependency_graph_recursive<'a>(
        &'a self,
        asset_id: &'a AssetId,
        max_depth: i32,
        current_depth: i32,
        visited: &'a mut HashSet<AssetId>,
        nodes: &'a mut HashMap<AssetId, DependencyNode>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ServiceResult<()>> + 'a + Send>> {
        Box::pin(async move {
        // Check depth limit
        if max_depth >= 0 && current_depth >= max_depth {
            return Ok(());
        }

        // Avoid cycles
        if visited.contains(asset_id) {
            return Ok(());
        }
        visited.insert(*asset_id);

        // Get the asset
        let asset = match self.repository.find_by_id(asset_id).await? {
            Some(a) => a,
            None => return Ok(()), // Skip if asset not found
        };

        // Get dependencies
        let deps = self.repository.list_dependencies(asset_id).await?;
        let dep_ids: Vec<AssetId> = deps.iter().map(|d| d.id).collect();

        // Create node
        let node = DependencyNode {
            asset_id: *asset_id,
            name: asset.metadata.name.clone(),
            version: asset.metadata.version.clone(),
            depth: current_depth,
            dependencies: dep_ids.clone(),
        };
        nodes.insert(*asset_id, node);

        // Recursively process dependencies
        for dep in deps {
            self.build_dependency_graph_recursive(
                &dep.id,
                max_depth,
                current_depth + 1,
                visited,
                nodes,
            )
            .await?;
        }

        Ok(())
        })
    }
}

#[async_trait]
impl SearchService for DefaultSearchService {
    #[instrument(skip(self, request))]
    async fn search_assets(&self, request: SearchAssetsRequest) -> ServiceResult<SearchAssetsResponse> {
        debug!("Searching assets with query");

        // Convert DTO request to DB query
        let mut query = SearchQuery::new()
            .limit(request.limit)
            .offset(request.offset)
            .sort_by(self.convert_sort_field(request.sort_by))
            .sort_order(self.convert_sort_order(request.sort_order))
            .exclude_deprecated(request.exclude_deprecated);

        if let Some(text) = request.text {
            query = query.text(text);
        }

        for asset_type in request.asset_types {
            query = query.asset_type(asset_type);
        }

        for tag in request.tags {
            query = query.tag(tag);
        }

        if let Some(author) = request.author {
            query = query.author(author);
        }

        if let Some(backend) = request.storage_backend {
            query = query.storage_backend(backend);
        }

        // Execute search
        let results = self.repository.search(&query).await?;
        let has_more = results.has_more();

        Ok(SearchAssetsResponse {
            assets: results.assets,
            total: results.total,
            offset: results.offset,
            limit: results.limit,
            has_more,
        })
    }

    #[instrument(skip(self), fields(asset_id = %asset_id))]
    async fn get_asset(&self, asset_id: &AssetId) -> ServiceResult<Option<Asset>> {
        debug!("Getting asset by ID");
        self.repository
            .find_by_id(asset_id)
            .await
            .map_err(Into::into)
    }

    #[instrument(skip(self), fields(name = %name, version = %version))]
    async fn get_asset_by_name_version(&self, name: &str, version: &str) -> ServiceResult<Option<Asset>> {
        debug!("Getting asset by name and version");

        let semver = semver::Version::parse(version)
            .map_err(|e| ServiceError::ValidationFailed(format!("Invalid version: {}", e)))?;

        self.repository
            .find_by_name_and_version(name, &semver)
            .await
            .map_err(Into::into)
    }

    #[instrument(skip(self, request), fields(asset_id = %request.asset_id, max_depth = request.max_depth))]
    async fn get_dependency_graph(&self, request: GetDependencyGraphRequest) -> ServiceResult<DependencyGraphResponse> {
        debug!("Building dependency graph");

        let mut visited = HashSet::new();
        let mut nodes = HashMap::new();

        self.build_dependency_graph_recursive(
            &request.asset_id,
            request.max_depth,
            0,
            &mut visited,
            &mut nodes,
        )
        .await?;

        // Check if truncated
        let truncated = if request.max_depth >= 0 {
            // If max_depth is set, we might have truncated
            nodes.values().any(|n| n.depth == request.max_depth - 1 && !n.dependencies.is_empty())
        } else {
            false
        };

        let dependencies: Vec<DependencyNode> = nodes.into_values().collect();

        Ok(DependencyGraphResponse {
            root: request.asset_id,
            dependencies,
            truncated,
        })
    }

    #[instrument(skip(self))]
    async fn list_all_tags(&self) -> ServiceResult<Vec<String>> {
        debug!("Listing all tags");
        self.repository.list_all_tags().await.map_err(Into::into)
    }

    #[instrument(skip(self, tags), fields(tag_count = tags.len()))]
    async fn search_by_tags(&self, tags: Vec<String>) -> ServiceResult<Vec<Asset>> {
        debug!("Searching by tags");

        if tags.is_empty() {
            return Ok(Vec::new());
        }

        let mut query = SearchQuery::new();
        for tag in tags {
            query = query.tag(tag);
        }

        let results = self.repository.search(&query).await?;
        Ok(results.assets)
    }

    #[instrument(skip(self), fields(asset_type = %asset_type))]
    async fn get_assets_by_type(&self, asset_type: AssetType) -> ServiceResult<Vec<Asset>> {
        debug!("Getting assets by type");

        let query = SearchQuery::new().asset_type(asset_type);
        let results = self.repository.search(&query).await?;
        Ok(results.assets)
    }

    #[instrument(skip(self), fields(asset_id = %asset_id))]
    async fn get_reverse_dependencies(&self, asset_id: &AssetId) -> ServiceResult<Vec<Asset>> {
        debug!("Getting reverse dependencies");
        self.repository
            .list_reverse_dependencies(asset_id)
            .await
            .map_err(Into::into)
    }
}

/// Utility functions for search operations
pub mod utils {
    use super::*;

    /// Build a full text search query
    pub fn build_text_query(terms: Vec<&str>) -> String {
        terms.join(" ")
    }

    /// Parse and validate a search query string
    pub fn parse_search_query(query: &str) -> ServiceResult<Vec<String>> {
        if query.trim().is_empty() {
            return Err(ServiceError::InvalidInput("Empty search query".to_string()));
        }

        let terms: Vec<String> = query
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();

        Ok(terms)
    }

    /// Build a tag filter from comma-separated string
    pub fn parse_tag_filter(tag_str: &str) -> Vec<String> {
        tag_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Create a default search request with common defaults
    pub fn default_search_request() -> SearchAssetsRequest {
        SearchAssetsRequest {
            text: None,
            asset_types: vec![],
            tags: vec![],
            author: None,
            storage_backend: None,
            exclude_deprecated: true,
            limit: 50,
            offset: 0,
            sort_by: SortField::CreatedAt,
            sort_order: SortOrder::Descending,
        }
    }

    /// Validate pagination parameters
    pub fn validate_pagination(limit: i64, offset: i64) -> ServiceResult<()> {
        if limit <= 0 {
            return Err(ServiceError::InvalidInput(
                "Limit must be positive".to_string(),
            ));
        }
        if limit > 1000 {
            return Err(ServiceError::InvalidInput(
                "Limit cannot exceed 1000".to_string(),
            ));
        }
        if offset < 0 {
            return Err(ServiceError::InvalidInput(
                "Offset cannot be negative".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_text_query() {
        let query = utils::build_text_query(vec!["hello", "world"]);
        assert_eq!(query, "hello world");
    }

    #[test]
    fn test_parse_search_query() {
        let terms = utils::parse_search_query("  Hello  World  ").unwrap();
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn test_parse_search_query_empty() {
        assert!(utils::parse_search_query("   ").is_err());
    }

    #[test]
    fn test_parse_tag_filter() {
        let tags = utils::parse_tag_filter("production, ml, nlp");
        assert_eq!(tags, vec!["production", "ml", "nlp"]);
    }

    #[test]
    fn test_parse_tag_filter_with_spaces() {
        let tags = utils::parse_tag_filter(" tag1 , tag2 , tag3 ");
        assert_eq!(tags, vec!["tag1", "tag2", "tag3"]);
    }

    #[test]
    fn test_validate_pagination_valid() {
        assert!(utils::validate_pagination(50, 0).is_ok());
        assert!(utils::validate_pagination(100, 50).is_ok());
    }

    #[test]
    fn test_validate_pagination_invalid_limit() {
        assert!(utils::validate_pagination(0, 0).is_err());
        assert!(utils::validate_pagination(-10, 0).is_err());
        assert!(utils::validate_pagination(1001, 0).is_err());
    }

    #[test]
    fn test_validate_pagination_invalid_offset() {
        assert!(utils::validate_pagination(50, -1).is_err());
    }

    #[test]
    fn test_default_search_request() {
        let req = utils::default_search_request();
        assert_eq!(req.limit, 50);
        assert_eq!(req.offset, 0);
        assert!(req.exclude_deprecated);
    }
}
