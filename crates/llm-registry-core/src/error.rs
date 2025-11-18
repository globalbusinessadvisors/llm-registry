//! Error types for the LLM Registry

use thiserror::Error;

/// Result type alias for Registry operations
pub type Result<T> = std::result::Result<T, RegistryError>;

/// Main error type for Registry operations
#[derive(Error, Debug)]
pub enum RegistryError {
    /// Asset not found
    #[error("Asset not found: {0}")]
    AssetNotFound(String),

    /// Duplicate asset (same name and version)
    #[error("Duplicate asset: {name} version {version}")]
    DuplicateAsset { name: String, version: String },

    /// Checksum mismatch
    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    /// Invalid dependency
    #[error("Invalid dependency: {0}")]
    InvalidDependency(String),

    /// Circular dependency detected
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    /// Policy validation failed
    #[error("Policy validation failed: {0}")]
    PolicyValidationFailed(String),

    /// Invalid version format
    #[error("Invalid version: {0}")]
    InvalidVersion(String),

    /// Invalid asset type
    #[error("Invalid asset type: {0}")]
    InvalidAssetType(String),

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Database error (generic)
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Storage error (generic)
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Serialization/Deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Authentication error
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    /// Authorization error
    #[error("Authorization failed: {0}")]
    AuthorizationError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(String),

    /// Generic internal error
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<serde_json::Error> for RegistryError {
    fn from(err: serde_json::Error) -> Self {
        RegistryError::SerializationError(err.to_string())
    }
}

impl From<semver::Error> for RegistryError {
    fn from(err: semver::Error) -> Self {
        RegistryError::InvalidVersion(err.to_string())
    }
}
