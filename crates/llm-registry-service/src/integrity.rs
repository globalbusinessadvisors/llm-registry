//! Integrity verification service
//!
//! This module provides services for checksum computation, verification,
//! and signature validation to ensure asset integrity and authenticity.

use async_trait::async_trait;
use llm_registry_core::{Asset, AssetId, Checksum, EventType, HashAlgorithm, RegistryEvent};
use llm_registry_db::{AssetRepository, EventStore};
use std::sync::Arc;
use tracing::{debug, instrument, warn};

use crate::dto::{
    ComputeChecksumRequest, ComputeChecksumResponse, IntegrityVerificationResult,
    VerifyIntegrityRequest,
};
use crate::error::{ServiceError, ServiceResult};

/// Trait for integrity verification operations
#[async_trait]
pub trait IntegrityService: Send + Sync {
    /// Compute checksum for provided data
    async fn compute_checksum(&self, request: ComputeChecksumRequest) -> ServiceResult<ComputeChecksumResponse>;

    /// Verify asset integrity against stored checksum
    async fn verify_integrity(&self, request: VerifyIntegrityRequest) -> ServiceResult<IntegrityVerificationResult>;

    /// Verify checksum matches expected value
    async fn verify_checksum(&self, asset_id: &AssetId, computed: &Checksum) -> ServiceResult<bool>;

    /// Recompute and update asset checksum
    async fn update_checksum(&self, asset_id: &AssetId, new_checksum: Checksum) -> ServiceResult<Asset>;
}

/// Default implementation of IntegrityService
pub struct DefaultIntegrityService {
    repository: Arc<dyn AssetRepository>,
    event_store: Arc<dyn EventStore>,
}

impl DefaultIntegrityService {
    /// Create a new integrity service
    pub fn new(repository: Arc<dyn AssetRepository>, event_store: Arc<dyn EventStore>) -> Self {
        Self {
            repository,
            event_store,
        }
    }

    /// Hash data using the specified algorithm
    fn hash_data(data: &[u8], algorithm: HashAlgorithm) -> String {
        match algorithm {
            HashAlgorithm::SHA256 => {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(data);
                format!("{:x}", hasher.finalize())
            }
            HashAlgorithm::SHA3_256 => {
                use sha3::{Digest, Sha3_256};
                let mut hasher = Sha3_256::new();
                hasher.update(data);
                format!("{:x}", hasher.finalize())
            }
            HashAlgorithm::BLAKE3 => {
                let hash = blake3::hash(data);
                hash.to_hex().to_string()
            }
        }
    }
}

#[async_trait]
impl IntegrityService for DefaultIntegrityService {
    #[instrument(skip(self, request))]
    async fn compute_checksum(&self, request: ComputeChecksumRequest) -> ServiceResult<ComputeChecksumResponse> {
        debug!("Computing checksum with algorithm: {:?}", request.algorithm);

        // Decode base64 data
        use base64::Engine;
        let data = base64::engine::general_purpose::STANDARD.decode(&request.data)
            .map_err(|e| ServiceError::InvalidInput(format!("Invalid base64 data: {}", e)))?;

        // Compute hash
        let hash_value = Self::hash_data(&data, request.algorithm);

        // Create checksum
        let checksum = Checksum::new(request.algorithm, hash_value)
            .map_err(|e| ServiceError::Internal(format!("Failed to create checksum: {}", e)))?;

        Ok(ComputeChecksumResponse { checksum })
    }

    #[instrument(skip(self, request), fields(asset_id = %request.asset_id))]
    async fn verify_integrity(&self, request: VerifyIntegrityRequest) -> ServiceResult<IntegrityVerificationResult> {
        debug!("Verifying integrity for asset");

        // Fetch the asset
        let asset = self
            .repository
            .find_by_id(&request.asset_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(request.asset_id.to_string()))?;

        let expected_checksum = asset.checksum.clone();

        // If computed checksum provided, verify it
        if let Some(computed) = request.computed_checksum {
            let verified = expected_checksum.verify(&computed);

            // Emit verification event
            let event = RegistryEvent::new(EventType::ChecksumVerified {
                asset_id: request.asset_id,
                success: verified,
                algorithm: expected_checksum.algorithm().to_string(),
            });

            if let Err(e) = self.event_store.append(event).await {
                warn!("Failed to emit checksum verification event: {}", e);
            }

            if !verified {
                // Emit failure event with details
                let failure_event = RegistryEvent::new(EventType::ChecksumFailed {
                    asset_id: request.asset_id,
                    expected: expected_checksum.value().to_string(),
                    actual: computed.value().to_string(),
                });

                if let Err(e) = self.event_store.append(failure_event).await {
                    warn!("Failed to emit checksum failure event: {}", e);
                }

                return Ok(IntegrityVerificationResult {
                    verified: false,
                    expected_checksum: expected_checksum.clone(),
                    actual_checksum: Some(computed.clone()),
                    error: Some(format!(
                        "Checksum mismatch: expected {}, got {}",
                        expected_checksum.value(),
                        computed.value()
                    )),
                });
            }

            Ok(IntegrityVerificationResult {
                verified: true,
                expected_checksum,
                actual_checksum: Some(computed),
                error: None,
            })
        } else {
            // No computed checksum provided, just return expected
            Ok(IntegrityVerificationResult {
                verified: false,
                expected_checksum,
                actual_checksum: None,
                error: Some("No computed checksum provided for verification".to_string()),
            })
        }
    }

    #[instrument(skip(self), fields(asset_id = %asset_id))]
    async fn verify_checksum(&self, asset_id: &AssetId, computed: &Checksum) -> ServiceResult<bool> {
        debug!("Verifying checksum");

        let asset = self
            .repository
            .find_by_id(asset_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(asset_id.to_string()))?;

        let verified = asset.checksum.verify(computed);

        // Emit event
        let event = RegistryEvent::new(if verified {
            EventType::ChecksumVerified {
                asset_id: *asset_id,
                success: true,
                algorithm: computed.algorithm().to_string(),
            }
        } else {
            EventType::ChecksumFailed {
                asset_id: *asset_id,
                expected: asset.checksum.value().to_string(),
                actual: computed.value().to_string(),
            }
        });

        if let Err(e) = self.event_store.append(event).await {
            warn!("Failed to emit checksum event: {}", e);
        }

        Ok(verified)
    }

    #[instrument(skip(self), fields(asset_id = %asset_id))]
    async fn update_checksum(&self, asset_id: &AssetId, new_checksum: Checksum) -> ServiceResult<Asset> {
        debug!("Updating asset checksum");

        // Fetch the asset
        let mut asset = self
            .repository
            .find_by_id(asset_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(asset_id.to_string()))?;

        // Update checksum
        asset.checksum = new_checksum;
        asset.updated_at = chrono::Utc::now();

        // Persist the update
        let updated = self.repository.update(asset).await?;

        // Emit update event
        let event = RegistryEvent::new(EventType::AssetUpdated {
            asset_id: *asset_id,
            asset_name: updated.metadata.name.clone(),
            updated_fields: vec!["checksum".to_string()],
        });

        if let Err(e) = self.event_store.append(event).await {
            warn!("Failed to emit asset update event: {}", e);
        }

        Ok(updated)
    }
}

/// Utility functions for computing checksums
pub mod utils {
    use super::*;

    /// Compute SHA256 checksum from bytes
    pub fn compute_sha256(data: &[u8]) -> ServiceResult<Checksum> {
        let hash_value = DefaultIntegrityService::hash_data(data, HashAlgorithm::SHA256);
        Checksum::new(HashAlgorithm::SHA256, hash_value)
            .map_err(|e| ServiceError::Internal(format!("Failed to create checksum: {}", e)))
    }

    /// Compute SHA3-256 checksum from bytes
    pub fn compute_sha3_256(data: &[u8]) -> ServiceResult<Checksum> {
        let hash_value = DefaultIntegrityService::hash_data(data, HashAlgorithm::SHA3_256);
        Checksum::new(HashAlgorithm::SHA3_256, hash_value)
            .map_err(|e| ServiceError::Internal(format!("Failed to create checksum: {}", e)))
    }

    /// Compute BLAKE3 checksum from bytes
    pub fn compute_blake3(data: &[u8]) -> ServiceResult<Checksum> {
        let hash_value = DefaultIntegrityService::hash_data(data, HashAlgorithm::BLAKE3);
        Checksum::new(HashAlgorithm::BLAKE3, hash_value)
            .map_err(|e| ServiceError::Internal(format!("Failed to create checksum: {}", e)))
    }

    /// Verify data against checksum
    pub fn verify_data(data: &[u8], expected: &Checksum) -> bool {
        let computed_hash = DefaultIntegrityService::hash_data(data, expected.algorithm());
        expected.verify_hash(&computed_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_sha256() {
        let data = b"hello world";
        let hash = DefaultIntegrityService::hash_data(data, HashAlgorithm::SHA256);
        // SHA256 of "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_hash_blake3() {
        let data = b"hello world";
        let hash = DefaultIntegrityService::hash_data(data, HashAlgorithm::BLAKE3);
        // BLAKE3 of "hello world"
        assert_eq!(
            hash,
            "d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24"
        );
    }

    #[test]
    fn test_compute_sha256_util() {
        let data = b"test data";
        let checksum = utils::compute_sha256(data).unwrap();
        assert_eq!(checksum.algorithm(), HashAlgorithm::SHA256);
        assert_eq!(checksum.value().len(), 64); // SHA256 is 64 hex chars
    }

    #[test]
    fn test_verify_data_util() {
        let data = b"test data";
        let checksum = utils::compute_sha256(data).unwrap();
        assert!(utils::verify_data(data, &checksum));

        let wrong_data = b"wrong data";
        assert!(!utils::verify_data(wrong_data, &checksum));
    }
}
