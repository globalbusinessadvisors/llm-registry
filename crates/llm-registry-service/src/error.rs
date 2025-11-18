//! Service-layer error types
//!
//! This module defines error types specific to the service layer,
//! mapping domain and database errors to service-level errors.

use llm_registry_core::RegistryError;
use llm_registry_db::DbError;
use thiserror::Error;

/// Result type alias for service operations
pub type ServiceResult<T> = std::result::Result<T, ServiceError>;

/// Service-layer error types
#[derive(Error, Debug)]
pub enum ServiceError {
    /// Asset not found
    #[error("Asset not found: {0}")]
    NotFound(String),

    /// Asset already exists (duplicate)
    #[error("Asset already exists: {name}@{version}")]
    AlreadyExists { name: String, version: String },

    /// Validation failed
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    /// Checksum verification failed
    #[error("Checksum verification failed: {0}")]
    ChecksumVerificationFailed(String),

    /// Circular dependency detected
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    /// Dependency not found
    #[error("Dependency not found: {0}")]
    DependencyNotFound(String),

    /// Version conflict
    #[error("Version conflict: {0}")]
    VersionConflict(String),

    /// Policy validation failed
    #[error("Policy validation failed: {policy_name}: {message}")]
    PolicyValidationFailed {
        policy_name: String,
        message: String,
    },

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Operation not permitted
    #[error("Operation not permitted: {0}")]
    NotPermitted(String),

    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// Internal service error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<RegistryError> for ServiceError {
    fn from(err: RegistryError) -> Self {
        match err {
            RegistryError::AssetNotFound(msg) => ServiceError::NotFound(msg),
            RegistryError::DuplicateAsset { name, version } => {
                ServiceError::AlreadyExists { name, version }
            }
            RegistryError::ChecksumMismatch { expected, actual } => {
                ServiceError::ChecksumVerificationFailed(format!(
                    "expected {}, got {}",
                    expected, actual
                ))
            }
            RegistryError::CircularDependency(msg) => ServiceError::CircularDependency(msg),
            RegistryError::InvalidDependency(msg) => ServiceError::DependencyNotFound(msg),
            RegistryError::PolicyValidationFailed(msg) => ServiceError::PolicyValidationFailed {
                policy_name: "unknown".to_string(),
                message: msg,
            },
            RegistryError::InvalidVersion(msg) => ServiceError::ValidationFailed(msg),
            RegistryError::ValidationError(msg) => ServiceError::ValidationFailed(msg),
            RegistryError::DatabaseError(msg) => ServiceError::Database(msg),
            _ => ServiceError::Internal(err.to_string()),
        }
    }
}

impl From<DbError> for ServiceError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFound(msg) => ServiceError::NotFound(msg),
            DbError::AlreadyExists(msg) => {
                // Try to parse name and version from message
                let parts: Vec<&str> = msg.split('@').collect();
                if parts.len() == 2 {
                    ServiceError::AlreadyExists {
                        name: parts[0].to_string(),
                        version: parts[1].to_string(),
                    }
                } else {
                    ServiceError::AlreadyExists {
                        name: msg.clone(),
                        version: "unknown".to_string(),
                    }
                }
            }
            DbError::ConstraintViolation(msg) => ServiceError::ValidationFailed(msg),
            DbError::ForeignKeyViolation(msg) => ServiceError::ValidationFailed(msg),
            DbError::UniqueViolation(msg) => ServiceError::ValidationFailed(msg),
            DbError::Connection(msg) => ServiceError::Database(msg),
            DbError::Pool(msg) => ServiceError::Database(msg),
            DbError::Query(msg) => ServiceError::Database(msg),
            DbError::Transaction(msg) => ServiceError::Database(msg),
            DbError::InvalidData(msg) => ServiceError::ValidationFailed(msg),
            DbError::Serialization(msg) => ServiceError::Internal(msg),
            DbError::CircularDependency(msg) => ServiceError::CircularDependency(msg),
            DbError::InvalidQuery(msg) => ServiceError::InvalidInput(msg),
            DbError::Configuration(msg) => ServiceError::Internal(msg),
            DbError::Cache(msg) => ServiceError::Internal(msg),
            DbError::Migration(msg) => ServiceError::Internal(msg),
            DbError::Internal(msg) => ServiceError::Internal(msg),
            DbError::Domain(err) => ServiceError::from(err),
            DbError::Other(msg) => ServiceError::Internal(msg),
        }
    }
}

impl From<semver::Error> for ServiceError {
    fn from(err: semver::Error) -> Self {
        ServiceError::ValidationFailed(format!("Invalid version: {}", err))
    }
}

impl From<serde_json::Error> for ServiceError {
    fn from(err: serde_json::Error) -> Self {
        ServiceError::Internal(format!("Serialization error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_error_from_registry_error() {
        let registry_err = RegistryError::AssetNotFound("test-asset".to_string());
        let service_err: ServiceError = registry_err.into();
        assert!(matches!(service_err, ServiceError::NotFound(_)));
    }

    #[test]
    fn test_service_error_from_db_error() {
        let db_err = DbError::NotFound("asset not found".to_string());
        let service_err: ServiceError = db_err.into();
        assert!(matches!(service_err, ServiceError::NotFound(_)));
    }

    #[test]
    fn test_service_error_display() {
        let err = ServiceError::ValidationFailed("Invalid name".to_string());
        assert_eq!(err.to_string(), "Validation failed: Invalid name");
    }
}
