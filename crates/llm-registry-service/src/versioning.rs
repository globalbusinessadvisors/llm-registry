//! Versioning service
//!
//! This module provides version management services including SemVer validation,
//! version conflict detection, and deprecation management.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use llm_registry_core::{Asset, AssetId, AssetStatus, EventType, RegistryEvent};
use llm_registry_db::{AssetRepository, EventStore};
use semver::{Version, VersionReq};
use std::sync::Arc;
use tracing::{debug, instrument, warn};

use crate::dto::{
    CheckVersionConflictRequest, ListVersionsRequest, ListVersionsResponse, VersionConflictResult,
    VersionInfo,
};
use crate::error::{ServiceError, ServiceResult};

/// Trait for version management operations
#[async_trait]
pub trait VersioningService: Send + Sync {
    /// List all versions of an asset
    async fn list_versions(&self, request: ListVersionsRequest) -> ServiceResult<ListVersionsResponse>;

    /// Check for version conflicts
    async fn check_version_conflict(&self, request: CheckVersionConflictRequest) -> ServiceResult<VersionConflictResult>;

    /// Get the latest version of an asset
    async fn get_latest_version(&self, name: &str) -> ServiceResult<Option<Asset>>;

    /// Find assets matching a version requirement
    async fn find_by_version_req(&self, name: &str, req: &VersionReq) -> ServiceResult<Vec<Asset>>;

    /// Deprecate a specific version
    async fn deprecate_version(&self, asset_id: &AssetId, reason: Option<String>) -> ServiceResult<Asset>;

    /// Check if a version is deprecated
    async fn is_deprecated(&self, asset_id: &AssetId) -> ServiceResult<bool>;

    /// Get deprecation info
    async fn get_deprecation_info(&self, asset_id: &AssetId) -> ServiceResult<Option<DeprecationInfo>>;
}

/// Information about a deprecated version
#[derive(Debug, Clone)]
pub struct DeprecationInfo {
    /// When it was deprecated
    pub deprecated_at: DateTime<Utc>,
    /// Reason for deprecation
    pub reason: Option<String>,
    /// Suggested alternative version
    pub alternative: Option<Version>,
}

/// Default implementation of VersioningService
pub struct DefaultVersioningService {
    repository: Arc<dyn AssetRepository>,
    event_store: Arc<dyn EventStore>,
}

impl DefaultVersioningService {
    /// Create a new versioning service
    pub fn new(repository: Arc<dyn AssetRepository>, event_store: Arc<dyn EventStore>) -> Self {
        Self {
            repository,
            event_store,
        }
    }

    /// Convert Asset to VersionInfo
    fn asset_to_version_info(&self, asset: &Asset) -> VersionInfo {
        VersionInfo {
            version: asset.metadata.version.clone(),
            asset_id: asset.id,
            status: asset.status,
            created_at: asset.created_at,
            deprecated_at: asset.deprecated_at,
        }
    }

    /// Sort versions in descending order (newest first)
    fn sort_versions_desc(&self, mut assets: Vec<Asset>) -> Vec<Asset> {
        assets.sort_by(|a, b| b.metadata.version.cmp(&a.metadata.version));
        assets
    }

    /// Find the latest non-deprecated version
    fn find_latest_active<'a>(&self, assets: &'a [Asset]) -> Option<&'a Asset> {
        assets
            .iter()
            .filter(|a| a.status == AssetStatus::Active)
            .max_by(|a, b| a.metadata.version.cmp(&b.metadata.version))
    }
}

#[async_trait]
impl VersioningService for DefaultVersioningService {
    #[instrument(skip(self, request), fields(name = %request.name))]
    async fn list_versions(&self, request: ListVersionsRequest) -> ServiceResult<ListVersionsResponse> {
        debug!("Listing versions for asset: {}", request.name);

        // Get all versions from repository
        let mut assets = self.repository.list_versions(&request.name).await?;

        // Filter deprecated if requested
        if !request.include_deprecated {
            assets.retain(|a| a.status != AssetStatus::Deprecated);
        }

        // Sort by version descending
        assets = self.sort_versions_desc(assets);

        // Convert to VersionInfo
        let versions: Vec<VersionInfo> = assets.iter().map(|a| self.asset_to_version_info(a)).collect();

        // Find latest active version
        let latest = self
            .find_latest_active(&assets)
            .map(|a| a.metadata.version.clone());

        Ok(ListVersionsResponse {
            name: request.name.clone(),
            versions,
            latest,
        })
    }

    #[instrument(skip(self, request), fields(name = %request.name, version = %request.version))]
    async fn check_version_conflict(&self, request: CheckVersionConflictRequest) -> ServiceResult<VersionConflictResult> {
        debug!("Checking version conflict for {}@{}", request.name, request.version);

        // Check if this exact version already exists
        match self
            .repository
            .find_by_name_and_version(&request.name, &request.version)
            .await?
        {
            Some(existing) => {
                let version_info = self.asset_to_version_info(&existing);
                Ok(VersionConflictResult {
                    has_conflict: true,
                    existing_version: Some(version_info),
                    message: Some(format!(
                        "Version {} already exists for asset {}",
                        request.version, request.name
                    )),
                })
            }
            None => Ok(VersionConflictResult {
                has_conflict: false,
                existing_version: None,
                message: None,
            }),
        }
    }

    #[instrument(skip(self), fields(name = %name))]
    async fn get_latest_version(&self, name: &str) -> ServiceResult<Option<Asset>> {
        debug!("Getting latest version for: {}", name);

        let assets = self.repository.list_versions(name).await?;

        Ok(self.find_latest_active(&assets).cloned())
    }

    #[instrument(skip(self, req), fields(name = %name, requirement = %req))]
    async fn find_by_version_req(&self, name: &str, req: &VersionReq) -> ServiceResult<Vec<Asset>> {
        debug!("Finding versions matching requirement: {}", req);

        let assets = self.repository.list_versions(name).await?;

        // Filter by version requirement
        let matching: Vec<Asset> = assets
            .into_iter()
            .filter(|a| req.matches(&a.metadata.version))
            .collect();

        Ok(self.sort_versions_desc(matching))
    }

    #[instrument(skip(self), fields(asset_id = %asset_id))]
    async fn deprecate_version(&self, asset_id: &AssetId, reason: Option<String>) -> ServiceResult<Asset> {
        debug!("Deprecating version");

        // Fetch the asset
        let mut asset = self
            .repository
            .find_by_id(asset_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(asset_id.to_string()))?;

        // Check if already deprecated
        if asset.status == AssetStatus::Deprecated {
            return Err(ServiceError::InvalidInput(format!(
                "Asset {} is already deprecated",
                asset_id
            )));
        }

        let old_status = asset.status;

        // Set status to deprecated
        asset.set_status(AssetStatus::Deprecated);

        // Update in repository
        let updated = self.repository.update(asset).await?;

        // Emit status change event
        let event = RegistryEvent::new(EventType::AssetStatusChanged {
            asset_id: *asset_id,
            asset_name: updated.metadata.name.clone(),
            old_status,
            new_status: AssetStatus::Deprecated,
        });

        if let Err(e) = self.event_store.append(event).await {
            warn!("Failed to emit status change event: {}", e);
        }

        // Store deprecation reason in annotations if provided
        if let Some(reason_text) = reason {
            let mut updated_copy = updated.clone();
            updated_copy
                .metadata
                .add_annotation("deprecation_reason", reason_text);
            return self.repository.update(updated_copy).await.map_err(Into::into);
        }

        Ok(updated)
    }

    #[instrument(skip(self), fields(asset_id = %asset_id))]
    async fn is_deprecated(&self, asset_id: &AssetId) -> ServiceResult<bool> {
        debug!("Checking if version is deprecated");

        let asset = self
            .repository
            .find_by_id(asset_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(asset_id.to_string()))?;

        Ok(asset.status == AssetStatus::Deprecated)
    }

    #[instrument(skip(self), fields(asset_id = %asset_id))]
    async fn get_deprecation_info(&self, asset_id: &AssetId) -> ServiceResult<Option<DeprecationInfo>> {
        debug!("Getting deprecation info");

        let asset = self
            .repository
            .find_by_id(asset_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(asset_id.to_string()))?;

        if asset.status != AssetStatus::Deprecated {
            return Ok(None);
        }

        let deprecated_at = asset
            .deprecated_at
            .unwrap_or_else(|| asset.updated_at);

        let reason = asset
            .metadata
            .get_annotation("deprecation_reason")
            .cloned();

        let alternative = asset
            .metadata
            .get_annotation("alternative_version")
            .and_then(|v| Version::parse(v).ok());

        Ok(Some(DeprecationInfo {
            deprecated_at,
            reason,
            alternative,
        }))
    }
}

/// Utility functions for version management
pub mod utils {
    use super::*;

    /// Parse a version requirement string
    pub fn parse_version_req(req_str: &str) -> ServiceResult<VersionReq> {
        VersionReq::parse(req_str).map_err(|e| {
            ServiceError::ValidationFailed(format!("Invalid version requirement '{}': {}", req_str, e))
        })
    }

    /// Check if a version satisfies a requirement
    pub fn version_matches(version: &Version, req: &VersionReq) -> bool {
        req.matches(version)
    }

    /// Compare two versions
    pub fn compare_versions(v1: &Version, v2: &Version) -> std::cmp::Ordering {
        v1.cmp(v2)
    }

    /// Check if a version is a pre-release
    pub fn is_prerelease(version: &Version) -> bool {
        !version.pre.is_empty()
    }

    /// Check if a version has build metadata
    pub fn has_build_metadata(version: &Version) -> bool {
        !version.build.is_empty()
    }

    /// Get the next major version
    pub fn next_major(version: &Version) -> Version {
        Version::new(version.major + 1, 0, 0)
    }

    /// Get the next minor version
    pub fn next_minor(version: &Version) -> Version {
        Version::new(version.major, version.minor + 1, 0)
    }

    /// Get the next patch version
    pub fn next_patch(version: &Version) -> Version {
        Version::new(version.major, version.minor, version.patch + 1)
    }

    /// Check if upgrading from v1 to v2 is a breaking change
    pub fn is_breaking_change(v1: &Version, v2: &Version) -> bool {
        v2.major > v1.major
    }

    /// Check if upgrading from v1 to v2 is a feature addition
    pub fn is_feature_addition(v1: &Version, v2: &Version) -> bool {
        v2.major == v1.major && v2.minor > v1.minor
    }

    /// Check if upgrading from v1 to v2 is a patch/bugfix
    pub fn is_patch_update(v1: &Version, v2: &Version) -> bool {
        v2.major == v1.major && v2.minor == v1.minor && v2.patch > v1.patch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_req() {
        let req = utils::parse_version_req("^1.0.0").unwrap();
        assert!(utils::version_matches(&Version::parse("1.2.3").unwrap(), &req));
        assert!(!utils::version_matches(&Version::parse("2.0.0").unwrap(), &req));
    }

    #[test]
    fn test_compare_versions() {
        let v1 = Version::parse("1.0.0").unwrap();
        let v2 = Version::parse("2.0.0").unwrap();
        assert_eq!(utils::compare_versions(&v1, &v2), std::cmp::Ordering::Less);
        assert_eq!(utils::compare_versions(&v2, &v1), std::cmp::Ordering::Greater);
        assert_eq!(utils::compare_versions(&v1, &v1), std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_is_prerelease() {
        let stable = Version::parse("1.0.0").unwrap();
        let prerelease = Version::parse("1.0.0-alpha.1").unwrap();
        assert!(!utils::is_prerelease(&stable));
        assert!(utils::is_prerelease(&prerelease));
    }

    #[test]
    fn test_next_versions() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(utils::next_major(&v), Version::parse("2.0.0").unwrap());
        assert_eq!(utils::next_minor(&v), Version::parse("1.3.0").unwrap());
        assert_eq!(utils::next_patch(&v), Version::parse("1.2.4").unwrap());
    }

    #[test]
    fn test_breaking_change_detection() {
        let v1 = Version::parse("1.0.0").unwrap();
        let v2_breaking = Version::parse("2.0.0").unwrap();
        let v2_feature = Version::parse("1.1.0").unwrap();
        let v2_patch = Version::parse("1.0.1").unwrap();

        assert!(utils::is_breaking_change(&v1, &v2_breaking));
        assert!(!utils::is_breaking_change(&v1, &v2_feature));
        assert!(!utils::is_breaking_change(&v1, &v2_patch));

        assert!(!utils::is_feature_addition(&v1, &v2_breaking));
        assert!(utils::is_feature_addition(&v1, &v2_feature));
        assert!(!utils::is_feature_addition(&v1, &v2_patch));

        assert!(!utils::is_patch_update(&v1, &v2_breaking));
        assert!(!utils::is_patch_update(&v1, &v2_feature));
        assert!(utils::is_patch_update(&v1, &v2_patch));
    }
}
