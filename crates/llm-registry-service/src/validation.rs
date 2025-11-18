//! Validation service
//!
//! This module provides validation services for assets, including schema validation,
//! policy validation, and business rule enforcement.

use async_trait::async_trait;
use llm_registry_core::{Asset, AssetId, AssetType, EventType, RegistryEvent};
use llm_registry_db::{AssetRepository, EventStore};
use std::sync::Arc;
use tracing::{debug, instrument, warn};

use crate::dto::{ValidateAssetRequest, ValidationError, ValidationResult, ValidationWarning};
use crate::error::{ServiceError, ServiceResult};

/// Trait for validation operations
#[async_trait]
pub trait ValidationService: Send + Sync {
    /// Validate an asset according to schema and business rules
    async fn validate_asset(&self, request: ValidateAssetRequest) -> ServiceResult<ValidationResult>;

    /// Validate asset metadata
    async fn validate_metadata(&self, asset: &Asset) -> ServiceResult<ValidationResult>;

    /// Validate asset dependencies
    async fn validate_dependencies(&self, asset: &Asset) -> ServiceResult<ValidationResult>;

    /// Apply policy validation
    async fn validate_policy(&self, asset: &Asset, policy_name: &str) -> ServiceResult<ValidationResult>;

    /// Validate all policies for an asset
    async fn validate_all_policies(&self, asset: &Asset) -> ServiceResult<ValidationResult>;
}

/// Default implementation of ValidationService
pub struct DefaultValidationService {
    repository: Arc<dyn AssetRepository>,
    event_store: Arc<dyn EventStore>,
}

impl DefaultValidationService {
    /// Create a new validation service
    pub fn new(repository: Arc<dyn AssetRepository>, event_store: Arc<dyn EventStore>) -> Self {
        Self {
            repository,
            event_store,
        }
    }

    /// Emit policy validation event
    async fn emit_policy_event(&self, asset_id: AssetId, policy_name: String, passed: bool, message: Option<String>) {
        let event = RegistryEvent::new(EventType::PolicyValidated {
            asset_id,
            policy_name,
            passed,
            message,
        });

        if let Err(e) = self.event_store.append(event).await {
            warn!("Failed to emit policy validation event: {}", e);
        }
    }

    /// Validate basic schema constraints
    fn validate_schema(&self, asset: &Asset) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate name
        if asset.metadata.name.is_empty() {
            errors.push(ValidationError {
                field: "metadata.name".to_string(),
                message: "Asset name cannot be empty".to_string(),
                code: Some("NAME_EMPTY".to_string()),
            });
        }

        if asset.metadata.name.len() > 255 {
            errors.push(ValidationError {
                field: "metadata.name".to_string(),
                message: "Asset name cannot exceed 255 characters".to_string(),
                code: Some("NAME_TOO_LONG".to_string()),
            });
        }

        // Validate version
        if asset.metadata.version.pre.is_empty() && asset.metadata.version.build.is_empty() {
            // Production version - no warnings
        } else {
            warnings.push(ValidationWarning {
                field: "metadata.version".to_string(),
                message: "Version contains pre-release or build metadata".to_string(),
            });
        }

        // Validate description if present
        if let Some(ref desc) = asset.metadata.description {
            if desc.len() > 5000 {
                warnings.push(ValidationWarning {
                    field: "metadata.description".to_string(),
                    message: "Description is very long (>5000 characters)".to_string(),
                });
            }
        }

        // Validate license if present
        if let Some(ref license) = asset.metadata.license {
            if license.is_empty() {
                errors.push(ValidationError {
                    field: "metadata.license".to_string(),
                    message: "License cannot be empty if specified".to_string(),
                    code: Some("LICENSE_EMPTY".to_string()),
                });
            }
        }

        // Validate content type if present
        if let Some(ref ct) = asset.metadata.content_type {
            if !ct.contains('/') {
                errors.push(ValidationError {
                    field: "metadata.content_type".to_string(),
                    message: format!("Invalid content type format: {}", ct),
                    code: Some("INVALID_CONTENT_TYPE".to_string()),
                });
            }
        }

        // Validate tags
        for (idx, tag) in asset.metadata.tags.iter().enumerate() {
            if tag.is_empty() {
                errors.push(ValidationError {
                    field: format!("metadata.tags[{}]", idx),
                    message: "Tag cannot be empty".to_string(),
                    code: Some("TAG_EMPTY".to_string()),
                });
            }
            if tag.len() > 100 {
                errors.push(ValidationError {
                    field: format!("metadata.tags[{}]", idx),
                    message: "Tag cannot exceed 100 characters".to_string(),
                    code: Some("TAG_TOO_LONG".to_string()),
                });
            }
        }

        // Validate annotations
        for (key, value) in &asset.metadata.annotations {
            if key.is_empty() {
                errors.push(ValidationError {
                    field: "metadata.annotations".to_string(),
                    message: "Annotation key cannot be empty".to_string(),
                    code: Some("ANNOTATION_KEY_EMPTY".to_string()),
                });
            }
            if key.len() > 255 {
                errors.push(ValidationError {
                    field: format!("metadata.annotations.{}", key),
                    message: "Annotation key cannot exceed 255 characters".to_string(),
                    code: Some("ANNOTATION_KEY_TOO_LONG".to_string()),
                });
            }
            if value.len() > 10000 {
                warnings.push(ValidationWarning {
                    field: format!("metadata.annotations.{}", key),
                    message: "Annotation value is very long (>10000 characters)".to_string(),
                });
            }
        }

        // Validate asset type
        if let AssetType::Custom(ref name) = asset.asset_type {
            if name.is_empty() {
                errors.push(ValidationError {
                    field: "asset_type".to_string(),
                    message: "Custom asset type name cannot be empty".to_string(),
                    code: Some("ASSET_TYPE_EMPTY".to_string()),
                });
            }
        }

        ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Validate license compliance (example policy)
    fn validate_license_policy(&self, asset: &Asset) -> ValidationResult {
        let errors = Vec::new();
        let mut warnings = Vec::new();

        // Check if license is specified
        if asset.metadata.license.is_none() {
            warnings.push(ValidationWarning {
                field: "metadata.license".to_string(),
                message: "No license specified. Consider adding a license.".to_string(),
            });
        } else if let Some(ref license) = asset.metadata.license {
            // List of approved licenses (example)
            let approved_licenses = vec![
                "MIT", "Apache-2.0", "GPL-3.0", "BSD-3-Clause", "ISC", "CC0-1.0",
            ];

            if !approved_licenses.iter().any(|&l| license.contains(l)) {
                warnings.push(ValidationWarning {
                    field: "metadata.license".to_string(),
                    message: format!(
                        "License '{}' is not in the standard approved list. Please review.",
                        license
                    ),
                });
            }
        }

        ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Validate size constraints (example policy)
    fn validate_size_policy(&self, asset: &Asset) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if let Some(size) = asset.metadata.size_bytes {
            const MAX_SIZE: u64 = 10 * 1024 * 1024 * 1024; // 10 GB
            const WARN_SIZE: u64 = 1 * 1024 * 1024 * 1024; // 1 GB

            if size > MAX_SIZE {
                errors.push(ValidationError {
                    field: "metadata.size_bytes".to_string(),
                    message: format!(
                        "Asset size {} exceeds maximum allowed size of {}",
                        size, MAX_SIZE
                    ),
                    code: Some("SIZE_EXCEEDS_LIMIT".to_string()),
                });
            } else if size > WARN_SIZE {
                warnings.push(ValidationWarning {
                    field: "metadata.size_bytes".to_string(),
                    message: format!("Asset size {} is very large (>1 GB)", size),
                });
            }
        }

        ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Merge multiple validation results
    fn merge_results(&self, results: Vec<ValidationResult>) -> ValidationResult {
        let mut all_errors = Vec::new();
        let mut all_warnings = Vec::new();

        for result in results {
            all_errors.extend(result.errors);
            all_warnings.extend(result.warnings);
        }

        ValidationResult {
            valid: all_errors.is_empty(),
            errors: all_errors,
            warnings: all_warnings,
        }
    }
}

#[async_trait]
impl ValidationService for DefaultValidationService {
    #[instrument(skip(self, request))]
    async fn validate_asset(&self, request: ValidateAssetRequest) -> ServiceResult<ValidationResult> {
        debug!("Validating asset: {}", request.asset.id);

        let mut results = Vec::new();

        // Schema validation
        results.push(self.validate_schema(&request.asset));

        // Metadata validation
        results.push(self.validate_metadata(&request.asset).await?);

        // Dependency validation if deep validation requested
        if request.deep {
            results.push(self.validate_dependencies(&request.asset).await?);
        }

        // Policy validation
        if request.policies.is_empty() {
            // Validate all default policies
            results.push(self.validate_all_policies(&request.asset).await?);
        } else {
            // Validate specific policies
            for policy in &request.policies {
                results.push(self.validate_policy(&request.asset, policy).await?);
            }
        }

        Ok(self.merge_results(results))
    }

    #[instrument(skip(self, asset), fields(asset_id = %asset.id))]
    async fn validate_metadata(&self, asset: &Asset) -> ServiceResult<ValidationResult> {
        debug!("Validating asset metadata");

        // Use the core asset validation
        if let Err(e) = asset.validate() {
            return Ok(ValidationResult {
                valid: false,
                errors: vec![ValidationError {
                    field: "asset".to_string(),
                    message: e.to_string(),
                    code: Some("VALIDATION_FAILED".to_string()),
                }],
                warnings: vec![],
            });
        }

        Ok(ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
        })
    }

    #[instrument(skip(self, asset), fields(asset_id = %asset.id))]
    async fn validate_dependencies(&self, asset: &Asset) -> ServiceResult<ValidationResult> {
        debug!("Validating asset dependencies");

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check each dependency exists
        for dep in &asset.dependencies {
            if let Some(dep_id) = dep.as_id() {
                match self.repository.find_by_id(dep_id).await {
                    Ok(Some(_)) => {
                        // Dependency exists
                    }
                    Ok(None) => {
                        errors.push(ValidationError {
                            field: "dependencies".to_string(),
                            message: format!("Dependency not found: {}", dep_id),
                            code: Some("DEPENDENCY_NOT_FOUND".to_string()),
                        });
                    }
                    Err(e) => {
                        warnings.push(ValidationWarning {
                            field: "dependencies".to_string(),
                            message: format!("Failed to check dependency {}: {}", dep_id, e),
                        });
                    }
                }
            }
        }

        // Check for too many dependencies
        if asset.dependencies.len() > 100 {
            warnings.push(ValidationWarning {
                field: "dependencies".to_string(),
                message: format!(
                    "Asset has {} dependencies, which is quite high",
                    asset.dependencies.len()
                ),
            });
        }

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    #[instrument(skip(self, asset), fields(asset_id = %asset.id, policy = %policy_name))]
    async fn validate_policy(&self, asset: &Asset, policy_name: &str) -> ServiceResult<ValidationResult> {
        debug!("Validating policy: {}", policy_name);

        let result = match policy_name {
            "license" => self.validate_license_policy(asset),
            "size" => self.validate_size_policy(asset),
            "schema" => self.validate_schema(asset),
            _ => {
                return Err(ServiceError::InvalidInput(format!(
                    "Unknown policy: {}",
                    policy_name
                )));
            }
        };

        // Emit policy validation event
        self.emit_policy_event(
            asset.id,
            policy_name.to_string(),
            result.valid,
            if result.valid {
                Some("Policy validation passed".to_string())
            } else {
                Some(format!("{} errors found", result.errors.len()))
            },
        )
        .await;

        Ok(result)
    }

    #[instrument(skip(self, asset), fields(asset_id = %asset.id))]
    async fn validate_all_policies(&self, asset: &Asset) -> ServiceResult<ValidationResult> {
        debug!("Validating all policies");

        let policies = vec!["license", "size", "schema"];
        let mut results = Vec::new();

        for policy in policies {
            results.push(self.validate_policy(asset, policy).await?);
        }

        Ok(self.merge_results(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_registry_core::{AssetMetadata, Checksum, HashAlgorithm, StorageBackend, StorageLocation};
    use semver::Version;

    fn create_test_asset() -> Asset {
        let metadata = AssetMetadata::new("test-asset", Version::parse("1.0.0").unwrap());
        let storage = StorageLocation::new(
            StorageBackend::S3 {
                bucket: "test".to_string(),
                region: "us-east-1".to_string(),
                endpoint: None,
            },
            "test.bin".to_string(),
            None,
        )
        .unwrap();
        let checksum = Checksum::new(
            HashAlgorithm::SHA256,
            "a".repeat(64),
        )
        .unwrap();

        Asset::new(AssetId::new(), AssetType::Model, metadata, storage, checksum).unwrap()
    }

    #[test]
    fn test_validate_schema_valid_asset() {
        let service = DefaultValidationService {
            repository: Arc::new(MockRepository),
            event_store: Arc::new(MockEventStore),
        };

        let asset = create_test_asset();
        let result = service.validate_schema(&asset);
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_schema_empty_name() {
        let service = DefaultValidationService {
            repository: Arc::new(MockRepository),
            event_store: Arc::new(MockEventStore),
        };

        let mut asset = create_test_asset();
        asset.metadata.name = String::new();

        let result = service.validate_schema(&asset);
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
        assert!(result.errors[0].code.as_ref().unwrap() == "NAME_EMPTY");
    }

    #[test]
    fn test_validate_license_policy() {
        let service = DefaultValidationService {
            repository: Arc::new(MockRepository),
            event_store: Arc::new(MockEventStore),
        };

        let mut asset = create_test_asset();
        asset.metadata.license = Some("MIT".to_string());

        let result = service.validate_license_policy(&asset);
        assert!(result.valid);
    }

    // Mock implementations for testing
    struct MockRepository;
    struct MockEventStore;

    #[async_trait]
    impl AssetRepository for MockRepository {
        async fn create(&self, _: Asset) -> llm_registry_db::DbResult<Asset> {
            unimplemented!()
        }
        async fn find_by_id(&self, _: &AssetId) -> llm_registry_db::DbResult<Option<Asset>> {
            Ok(None)
        }
        async fn find_by_name_and_version(&self, _: &str, _: &semver::Version) -> llm_registry_db::DbResult<Option<Asset>> {
            Ok(None)
        }
        async fn find_by_ids(&self, _: &[AssetId]) -> llm_registry_db::DbResult<Vec<Asset>> {
            Ok(vec![])
        }
        async fn search(&self, _: &llm_registry_db::SearchQuery) -> llm_registry_db::DbResult<llm_registry_db::SearchResults> {
            unimplemented!()
        }
        async fn update(&self, asset: Asset) -> llm_registry_db::DbResult<Asset> {
            Ok(asset)
        }
        async fn delete(&self, _: &AssetId) -> llm_registry_db::DbResult<()> {
            Ok(())
        }
        async fn list_versions(&self, _: &str) -> llm_registry_db::DbResult<Vec<Asset>> {
            Ok(vec![])
        }
        async fn list_dependencies(&self, _: &AssetId) -> llm_registry_db::DbResult<Vec<Asset>> {
            Ok(vec![])
        }
        async fn list_reverse_dependencies(&self, _: &AssetId) -> llm_registry_db::DbResult<Vec<Asset>> {
            Ok(vec![])
        }
        async fn add_tag(&self, _: &AssetId, _: &str) -> llm_registry_db::DbResult<()> {
            Ok(())
        }
        async fn remove_tag(&self, _: &AssetId, _: &str) -> llm_registry_db::DbResult<()> {
            Ok(())
        }
        async fn get_tags(&self, _: &AssetId) -> llm_registry_db::DbResult<Vec<String>> {
            Ok(vec![])
        }
        async fn list_all_tags(&self) -> llm_registry_db::DbResult<Vec<String>> {
            Ok(vec![])
        }
        async fn add_dependency(&self, _: &AssetId, _: &AssetId, _: Option<&str>) -> llm_registry_db::DbResult<()> {
            Ok(())
        }
        async fn remove_dependency(&self, _: &AssetId, _: &AssetId) -> llm_registry_db::DbResult<()> {
            Ok(())
        }
        async fn count_assets(&self) -> llm_registry_db::DbResult<i64> {
            Ok(0)
        }
        async fn count_by_type(&self, _: &AssetType) -> llm_registry_db::DbResult<i64> {
            Ok(0)
        }
        async fn health_check(&self) -> llm_registry_db::DbResult<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl EventStore for MockEventStore {
        async fn append(&self, event: RegistryEvent) -> llm_registry_db::DbResult<RegistryEvent> {
            Ok(event)
        }
        async fn append_batch(&self, events: Vec<RegistryEvent>) -> llm_registry_db::DbResult<Vec<RegistryEvent>> {
            Ok(events)
        }
        async fn query(&self, _: &llm_registry_db::EventQuery) -> llm_registry_db::DbResult<llm_registry_db::EventQueryResults> {
            unimplemented!()
        }
        async fn get_asset_events(&self, _: &AssetId, _: i64) -> llm_registry_db::DbResult<Vec<RegistryEvent>> {
            Ok(vec![])
        }
        async fn get_latest_event(&self, _: &AssetId) -> llm_registry_db::DbResult<Option<RegistryEvent>> {
            Ok(None)
        }
        async fn count_events(&self) -> llm_registry_db::DbResult<i64> {
            Ok(0)
        }
        async fn count_by_type(&self, _: &str) -> llm_registry_db::DbResult<i64> {
            Ok(0)
        }
        async fn health_check(&self) -> llm_registry_db::DbResult<()> {
            Ok(())
        }
    }
}
