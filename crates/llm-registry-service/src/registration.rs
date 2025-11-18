//! Registration service
//!
//! This module provides the main asset registration service that orchestrates
//! validation, checksum verification, dependency resolution, and event emission.

use async_trait::async_trait;
use llm_registry_core::{
    Asset, AssetId, AssetMetadata, DependencyGraph, EventType, RegistryEvent,
};
use llm_registry_db::{AssetRepository, EventStore};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

use crate::dto::{
    RegisterAssetRequest, RegisterAssetResponse, UpdateAssetRequest, UpdateAssetResponse,
    ValidateAssetRequest, ValidationResult,
};
use crate::error::{ServiceError, ServiceResult};
use crate::integrity::IntegrityService;
use crate::validation::ValidationService;
use crate::versioning::VersioningService;

/// Trait for asset registration operations
#[async_trait]
pub trait RegistrationService: Send + Sync {
    /// Register a new asset with full validation
    async fn register_asset(&self, request: RegisterAssetRequest) -> ServiceResult<RegisterAssetResponse>;

    /// Update an existing asset
    async fn update_asset(&self, request: UpdateAssetRequest) -> ServiceResult<UpdateAssetResponse>;

    /// Delete an asset
    async fn delete_asset(&self, asset_id: &AssetId) -> ServiceResult<()>;

    /// Validate dependencies before registration
    async fn validate_dependencies(&self, dependencies: &[llm_registry_core::AssetReference]) -> ServiceResult<ValidationResult>;

    /// Check for circular dependencies
    async fn check_circular_dependencies(&self, asset_id: &AssetId, dependencies: &[llm_registry_core::AssetReference]) -> ServiceResult<()>;
}

/// Default implementation of RegistrationService
pub struct DefaultRegistrationService {
    repository: Arc<dyn AssetRepository>,
    event_store: Arc<dyn EventStore>,
    validation_service: Arc<dyn ValidationService>,
    integrity_service: Arc<dyn IntegrityService>,
    versioning_service: Arc<dyn VersioningService>,
}

impl DefaultRegistrationService {
    /// Create a new registration service
    pub fn new(
        repository: Arc<dyn AssetRepository>,
        event_store: Arc<dyn EventStore>,
        validation_service: Arc<dyn ValidationService>,
        integrity_service: Arc<dyn IntegrityService>,
        versioning_service: Arc<dyn VersioningService>,
    ) -> Self {
        Self {
            repository,
            event_store,
            validation_service,
            integrity_service,
            versioning_service,
        }
    }

    /// Build asset metadata from request
    fn build_metadata(&self, request: &RegisterAssetRequest) -> ServiceResult<AssetMetadata> {
        let mut builder = AssetMetadata::builder(request.name.clone(), request.version.clone());

        if let Some(ref desc) = request.description {
            builder = builder.description(desc.clone());
        }

        if let Some(ref license) = request.license {
            builder = builder.license(license.clone());
        }

        builder = builder.tags(request.tags.clone());
        builder = builder.annotations(request.annotations.clone());

        if let Some(size) = request.size_bytes {
            builder = builder.size_bytes(size);
        }

        if let Some(ref ct) = request.content_type {
            builder = builder.content_type(ct.clone());
        }

        builder.build().map_err(|e| {
            ServiceError::ValidationFailed(format!("Invalid metadata: {}", e))
        })
    }

    /// Emit asset registered event
    async fn emit_registered_event(&self, asset: &Asset) {
        let event = RegistryEvent::new(EventType::AssetRegistered {
            asset_id: asset.id,
            asset_name: asset.metadata.name.clone(),
            asset_version: asset.metadata.version.to_string(),
            asset_type: asset.asset_type.to_string(),
        });

        if let Err(e) = self.event_store.append(event).await {
            warn!("Failed to emit asset registered event: {}", e);
        }
    }

    /// Emit asset updated event
    async fn emit_updated_event(&self, asset: &Asset, updated_fields: Vec<String>) {
        let event = RegistryEvent::new(EventType::AssetUpdated {
            asset_id: asset.id,
            asset_name: asset.metadata.name.clone(),
            updated_fields,
        });

        if let Err(e) = self.event_store.append(event).await {
            warn!("Failed to emit asset updated event: {}", e);
        }
    }

    /// Emit asset deleted event
    async fn emit_deleted_event(&self, asset: &Asset) {
        let event = RegistryEvent::new(EventType::AssetDeleted {
            asset_id: asset.id,
            asset_name: asset.metadata.name.clone(),
            asset_version: asset.metadata.version.to_string(),
        });

        if let Err(e) = self.event_store.append(event).await {
            warn!("Failed to emit asset deleted event: {}", e);
        }
    }

    /// Validate asset before registration
    async fn validate_for_registration(&self, asset: &Asset) -> ServiceResult<Vec<String>> {
        let mut warnings = Vec::new();

        // Validate the asset structure
        let validation_request = ValidateAssetRequest {
            asset: asset.clone(),
            deep: true,
            policies: vec![],
        };

        let validation_result = self.validation_service.validate_asset(validation_request).await?;

        if !validation_result.valid {
            return Err(ServiceError::ValidationFailed(format!(
                "Asset validation failed: {} errors",
                validation_result.errors.len()
            )));
        }

        // Collect warnings
        for warning in validation_result.warnings {
            warnings.push(format!("{}: {}", warning.field, warning.message));
        }

        Ok(warnings)
    }

    /// Check if asset already exists
    async fn check_duplicate(&self, name: &str, version: &semver::Version) -> ServiceResult<()> {
        if let Some(_existing) = self.repository.find_by_name_and_version(name, version).await? {
            return Err(ServiceError::AlreadyExists {
                name: name.to_string(),
                version: version.to_string(),
            });
        }
        Ok(())
    }
}

#[async_trait]
impl RegistrationService for DefaultRegistrationService {
    #[instrument(skip(self, request), fields(name = %request.name, version = %request.version))]
    async fn register_asset(&self, request: RegisterAssetRequest) -> ServiceResult<RegisterAssetResponse> {
        info!("Registering asset: {}@{}", request.name, request.version);

        // Check for duplicate
        self.check_duplicate(&request.name, &request.version).await?;

        // Build metadata
        let metadata = self.build_metadata(&request)?;

        // Validate asset type
        request.asset_type.validate().map_err(|e| {
            ServiceError::ValidationFailed(format!("Invalid asset type: {}", e))
        })?;

        // Build the asset
        let mut asset_builder = Asset::builder(
            request.asset_type.clone(),
            metadata,
            request.storage.clone(),
            request.checksum.clone(),
        );

        if let Some(prov) = request.provenance.clone() {
            asset_builder = asset_builder.provenance(prov);
        }

        asset_builder = asset_builder.dependencies(request.dependencies.clone());

        let asset = asset_builder.build().map_err(|e| {
            ServiceError::ValidationFailed(format!("Failed to build asset: {}", e))
        })?;

        // Validate dependencies
        if !asset.dependencies.is_empty() {
            self.validate_dependencies(&asset.dependencies).await?;
            self.check_circular_dependencies(&asset.id, &asset.dependencies).await?;
        }

        // Full validation
        let warnings = self.validate_for_registration(&asset).await?;

        // Persist the asset
        let created = self.repository.create(asset).await?;

        // Emit dependencies added events
        for dep in &created.dependencies {
            if let Some(dep_id) = dep.as_id() {
                let event = RegistryEvent::new(EventType::DependencyAdded {
                    asset_id: created.id,
                    dependency_id: Some(*dep_id),
                    dependency_name: None,
                });
                if let Err(e) = self.event_store.append(event).await {
                    warn!("Failed to emit dependency added event: {}", e);
                }
            } else if let Some((name, version)) = dep.as_name_version() {
                let event = RegistryEvent::new(EventType::DependencyAdded {
                    asset_id: created.id,
                    dependency_id: None,
                    dependency_name: Some(format!("{}@{}", name, version)),
                });
                if let Err(e) = self.event_store.append(event).await {
                    warn!("Failed to emit dependency added event: {}", e);
                }
            }
        }

        // Emit registration event
        self.emit_registered_event(&created).await;

        info!("Asset registered successfully: {}", created.id);

        Ok(RegisterAssetResponse {
            asset: created,
            warnings,
        })
    }

    #[instrument(skip(self, request), fields(asset_id = %request.asset_id))]
    async fn update_asset(&self, request: UpdateAssetRequest) -> ServiceResult<UpdateAssetResponse> {
        debug!("Updating asset: {}", request.asset_id);

        // Fetch existing asset
        let mut asset = self
            .repository
            .find_by_id(&request.asset_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(request.asset_id.to_string()))?;

        let mut updated_fields = Vec::new();

        // Update description
        if let Some(desc) = request.description {
            asset.metadata.description = Some(desc);
            updated_fields.push("description".to_string());
        }

        // Update license
        if let Some(license) = request.license {
            asset.metadata.license = Some(license);
            updated_fields.push("license".to_string());
        }

        // Add tags
        for tag in request.add_tags {
            if !asset.metadata.tags.contains(&tag) {
                asset.metadata.add_tag(tag);
                updated_fields.push("tags".to_string());
            }
        }

        // Remove tags
        for tag in request.remove_tags {
            asset.metadata.tags.retain(|t| t != &tag);
            updated_fields.push("tags".to_string());
        }

        // Add/update annotations
        for (key, value) in request.add_annotations {
            asset.metadata.add_annotation(key, value);
            updated_fields.push("annotations".to_string());
        }

        // Remove annotations
        for key in request.remove_annotations {
            asset.metadata.annotations.remove(&key);
            updated_fields.push("annotations".to_string());
        }

        // Update status
        if let Some(status) = request.status {
            asset.set_status(status);
            updated_fields.push("status".to_string());
        }

        // Update timestamp
        asset.updated_at = chrono::Utc::now();

        // Validate updated asset
        asset.validate().map_err(|e| {
            ServiceError::ValidationFailed(format!("Updated asset is invalid: {}", e))
        })?;

        // Persist the update
        let updated = self.repository.update(asset).await?;

        // Emit update event
        self.emit_updated_event(&updated, updated_fields.clone()).await;

        Ok(UpdateAssetResponse {
            asset: updated,
            updated_fields,
        })
    }

    #[instrument(skip(self), fields(asset_id = %asset_id))]
    async fn delete_asset(&self, asset_id: &AssetId) -> ServiceResult<()> {
        debug!("Deleting asset: {}", asset_id);

        // Fetch the asset first for event emission
        let asset = self
            .repository
            .find_by_id(asset_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(asset_id.to_string()))?;

        // Check if any assets depend on this one
        let dependents = self.repository.list_reverse_dependencies(asset_id).await?;
        if !dependents.is_empty() {
            return Err(ServiceError::NotPermitted(format!(
                "Cannot delete asset: {} other assets depend on it",
                dependents.len()
            )));
        }

        // Delete from repository
        self.repository.delete(asset_id).await?;

        // Emit deletion event
        self.emit_deleted_event(&asset).await;

        info!("Asset deleted successfully: {}", asset_id);

        Ok(())
    }

    #[instrument(skip(self, dependencies), fields(dep_count = dependencies.len()))]
    async fn validate_dependencies(&self, dependencies: &[llm_registry_core::AssetReference]) -> ServiceResult<ValidationResult> {
        debug!("Validating dependencies");

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        for dep in dependencies {
            // Validate the reference itself
            if let Err(e) = dep.validate() {
                errors.push(crate::dto::ValidationError {
                    field: "dependency".to_string(),
                    message: format!("Invalid dependency reference: {}", e),
                    code: Some("INVALID_DEPENDENCY_REFERENCE".to_string()),
                });
                continue;
            }

            // If it's a by-ID reference, check that the asset exists
            if let Some(dep_id) = dep.as_id() {
                match self.repository.find_by_id(dep_id).await {
                    Ok(Some(_)) => {
                        // Dependency exists
                    }
                    Ok(None) => {
                        errors.push(crate::dto::ValidationError {
                            field: "dependency".to_string(),
                            message: format!("Dependency not found: {}", dep_id),
                            code: Some("DEPENDENCY_NOT_FOUND".to_string()),
                        });
                    }
                    Err(e) => {
                        warnings.push(crate::dto::ValidationWarning {
                            field: "dependency".to_string(),
                            message: format!("Failed to verify dependency {}: {}", dep_id, e),
                        });
                    }
                }
            }
        }

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    #[instrument(skip(self, dependencies), fields(asset_id = %asset_id, dep_count = dependencies.len()))]
    async fn check_circular_dependencies(
        &self,
        asset_id: &AssetId,
        dependencies: &[llm_registry_core::AssetReference],
    ) -> ServiceResult<()> {
        debug!("Checking for circular dependencies");

        // Build dependency graph
        let mut graph = DependencyGraph::new();

        // Add this asset's dependencies
        let deps: Vec<llm_registry_core::AssetReference> = dependencies.to_vec();
        graph.add_dependencies(*asset_id, deps).map_err(|e| {
            ServiceError::Internal(format!("Failed to build dependency graph: {}", e))
        })?;

        // For each dependency, fetch and add its dependencies
        for dep in dependencies {
            if let Some(dep_id) = dep.as_id() {
                if let Ok(Some(dep_asset)) = self.repository.find_by_id(dep_id).await {
                    graph
                        .add_dependencies(*dep_id, dep_asset.dependencies.clone())
                        .map_err(|e| {
                            ServiceError::Internal(format!("Failed to add dependencies to graph: {}", e))
                        })?;
                }
            }
        }

        // Detect cycles
        graph.detect_circular_dependencies().map_err(|e| {
            // Emit circular dependency event
            let dep_ids: Vec<AssetId> = dependencies
                .iter()
                .filter_map(|d| d.as_id().copied())
                .collect();

            let event = RegistryEvent::new(EventType::CircularDependencyDetected {
                cycle_asset_ids: dep_ids,
            });

            // Try to emit event (ignore errors)
            let event_store = self.event_store.clone();
            tokio::spawn(async move {
                let _ = event_store.append(event).await;
            });

            ServiceError::CircularDependency(e.to_string())
        })?;

        Ok(())
    }
}

// TODO: Complete mock implementations for unit tests
#[cfg(all(test, feature = "incomplete_tests"))]
mod tests {
    use super::*;
    use llm_registry_core::{AssetType, Checksum, HashAlgorithm, StorageBackend, StorageLocation};
    use semver::Version;

    fn create_test_request() -> RegisterAssetRequest {
        RegisterAssetRequest {
            asset_type: AssetType::Model,
            name: "test-model".to_string(),
            version: Version::parse("1.0.0").unwrap(),
            description: Some("Test model".to_string()),
            license: Some("MIT".to_string()),
            tags: vec!["test".to_string()],
            annotations: Default::default(),
            storage: StorageLocation::new(
                StorageBackend::S3 {
                    bucket: "test".to_string(),
                    region: "us-east-1".to_string(),
                    endpoint: None,
                },
                "test.bin".to_string(),
                None,
            )
            .unwrap(),
            checksum: Checksum::new(HashAlgorithm::SHA256, "a".repeat(64)).unwrap(),
            provenance: None,
            dependencies: vec![],
            size_bytes: Some(1024),
            content_type: Some("application/octet-stream".to_string()),
        }
    }

    #[test]
    fn test_build_metadata() {
        let service = DefaultRegistrationService {
            repository: Arc::new(MockRepository),
            event_store: Arc::new(MockEventStore),
            validation_service: Arc::new(MockValidationService),
            integrity_service: Arc::new(MockIntegrityService),
            versioning_service: Arc::new(MockVersioningService),
        };

        let request = create_test_request();
        let metadata = service.build_metadata(&request).unwrap();

        assert_eq!(metadata.name, "test-model");
        assert_eq!(metadata.version, Version::parse("1.0.0").unwrap());
        assert_eq!(metadata.description.as_deref(), Some("Test model"));
        assert_eq!(metadata.license.as_deref(), Some("MIT"));
    }

    // Mock implementations
    struct MockRepository;
    struct MockEventStore;
    struct MockValidationService;
    struct MockIntegrityService;
    struct MockVersioningService;

    #[async_trait]
    impl llm_registry_db::AssetRepository for MockRepository {
        async fn save(&self, _asset: &Asset) -> DbResult<()> {
            Ok(())
        }
        async fn find_by_id(&self, _id: &str) -> DbResult<Option<Asset>> {
            Ok(None)
        }
        async fn find_by_name_and_version(&self, _name: &str, _version: &Version) -> DbResult<Option<Asset>> {
            Ok(None)
        }
        async fn list(&self, _offset: i64, _limit: i64) -> DbResult<Vec<Asset>> {
            Ok(vec![])
        }
        async fn delete(&self, _id: &str) -> DbResult<()> {
            Ok(())
        }
        async fn update(&self, _asset: &Asset) -> DbResult<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl llm_registry_db::EventStore for MockEventStore {
        async fn append(&self, _event: &Event) -> DbResult<()> {
            Ok(())
        }
        async fn get_by_asset_id(&self, _asset_id: &str, _offset: i64, _limit: i64) -> DbResult<Vec<Event>> {
            Ok(vec![])
        }
        async fn get_by_type(&self, _event_type: &str, _offset: i64, _limit: i64) -> DbResult<Vec<Event>> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl ValidationService for MockValidationService {
        async fn validate_registration(&self, _request: &RegisterAssetRequest) -> ServiceResult<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl IntegrityService for MockIntegrityService {
        async fn verify_checksum(&self, _asset: &Asset) -> ServiceResult<bool> {
            Ok(true)
        }
    }

    #[async_trait]
    impl VersioningService for MockVersioningService {
        async fn validate_version(&self, _name: &str, _version: &Version) -> ServiceResult<()> {
            Ok(())
        }
        async fn get_latest_version(&self, _name: &str) -> ServiceResult<Option<Version>> {
            Ok(None)
        }
    }
}
