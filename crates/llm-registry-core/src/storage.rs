//! Storage backend abstractions and location handling
//!
//! This module defines the types for representing different storage backends
//! and asset storage locations within those backends.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use url::Url;

use crate::error::{RegistryError, Result};

/// Supported storage backend types
///
/// Represents different cloud and local storage systems that can be used
/// to store LLM assets (models, datasets, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StorageBackend {
    /// Amazon S3 or S3-compatible storage
    S3 {
        /// S3 bucket name
        bucket: String,
        /// AWS region (e.g., "us-east-1")
        region: String,
        /// Optional endpoint URL for S3-compatible services
        #[serde(skip_serializing_if = "Option::is_none")]
        endpoint: Option<String>,
    },

    /// Google Cloud Storage
    GCS {
        /// GCS bucket name
        bucket: String,
        /// GCP project ID
        project_id: String,
    },

    /// Azure Blob Storage
    AzureBlob {
        /// Storage account name
        account_name: String,
        /// Container name
        container: String,
    },

    /// MinIO object storage
    MinIO {
        /// MinIO bucket name
        bucket: String,
        /// MinIO endpoint URL
        endpoint: String,
    },

    /// Local filesystem storage
    FileSystem {
        /// Base directory path
        base_path: String,
    },
}

impl StorageBackend {
    /// Validate the storage backend configuration
    pub fn validate(&self) -> Result<()> {
        match self {
            StorageBackend::S3 { bucket, region, endpoint } => {
                if bucket.is_empty() {
                    return Err(RegistryError::ValidationError(
                        "S3 bucket name cannot be empty".to_string(),
                    ));
                }
                if region.is_empty() {
                    return Err(RegistryError::ValidationError(
                        "S3 region cannot be empty".to_string(),
                    ));
                }
                if let Some(ep) = endpoint {
                    if ep.is_empty() {
                        return Err(RegistryError::ValidationError(
                            "S3 endpoint cannot be empty if specified".to_string(),
                        ));
                    }
                    // Validate endpoint URL format
                    Url::parse(ep).map_err(|e| {
                        RegistryError::ValidationError(format!("Invalid S3 endpoint URL: {}", e))
                    })?;
                }
                Ok(())
            }
            StorageBackend::GCS { bucket, project_id } => {
                if bucket.is_empty() {
                    return Err(RegistryError::ValidationError(
                        "GCS bucket name cannot be empty".to_string(),
                    ));
                }
                if project_id.is_empty() {
                    return Err(RegistryError::ValidationError(
                        "GCS project ID cannot be empty".to_string(),
                    ));
                }
                Ok(())
            }
            StorageBackend::AzureBlob { account_name, container } => {
                if account_name.is_empty() {
                    return Err(RegistryError::ValidationError(
                        "Azure account name cannot be empty".to_string(),
                    ));
                }
                if container.is_empty() {
                    return Err(RegistryError::ValidationError(
                        "Azure container name cannot be empty".to_string(),
                    ));
                }
                Ok(())
            }
            StorageBackend::MinIO { bucket, endpoint } => {
                if bucket.is_empty() {
                    return Err(RegistryError::ValidationError(
                        "MinIO bucket name cannot be empty".to_string(),
                    ));
                }
                if endpoint.is_empty() {
                    return Err(RegistryError::ValidationError(
                        "MinIO endpoint cannot be empty".to_string(),
                    ));
                }
                // Validate endpoint URL format
                Url::parse(endpoint).map_err(|e| {
                    RegistryError::ValidationError(format!("Invalid MinIO endpoint URL: {}", e))
                })?;
                Ok(())
            }
            StorageBackend::FileSystem { base_path } => {
                if base_path.is_empty() {
                    return Err(RegistryError::ValidationError(
                        "FileSystem base path cannot be empty".to_string(),
                    ));
                }
                Ok(())
            }
        }
    }

    /// Get a human-readable name for the storage backend type
    pub fn backend_type(&self) -> &str {
        match self {
            StorageBackend::S3 { .. } => "S3",
            StorageBackend::GCS { .. } => "GCS",
            StorageBackend::AzureBlob { .. } => "AzureBlob",
            StorageBackend::MinIO { .. } => "MinIO",
            StorageBackend::FileSystem { .. } => "FileSystem",
        }
    }
}

impl fmt::Display for StorageBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.backend_type())
    }
}

impl FromStr for StorageBackend {
    type Err = RegistryError;

    fn from_str(s: &str) -> Result<Self> {
        // Simple string-based parsing for basic backend type
        // For full deserialization with all fields, use serde_json
        match s {
            "FileSystem" | "filesystem" => Ok(StorageBackend::FileSystem {
                base_path: String::new(),
            }),
            _ => Err(RegistryError::ValidationError(format!(
                "Cannot parse StorageBackend from string '{}'. Use JSON deserialization for full configuration.",
                s
            ))),
        }
    }
}

/// Storage location information for an asset
///
/// Contains the storage backend configuration and the path/key to the asset
/// within that backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageLocation {
    /// The storage backend configuration
    pub backend: StorageBackend,
    /// The path or key to the asset within the backend
    pub path: String,
    /// Optional URI representation (e.g., s3://bucket/path)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

impl StorageLocation {
    /// Create a new storage location with validation
    ///
    /// # Arguments
    /// * `backend` - The storage backend configuration
    /// * `path` - The path or key to the asset
    /// * `uri` - Optional URI representation
    ///
    /// # Errors
    /// Returns an error if the backend or path is invalid
    pub fn new(backend: StorageBackend, path: String, uri: Option<String>) -> Result<Self> {
        // Validate backend
        backend.validate()?;

        // Validate path
        if path.is_empty() {
            return Err(RegistryError::ValidationError(
                "Storage path cannot be empty".to_string(),
            ));
        }

        // Validate URI if provided
        if let Some(ref uri_str) = uri {
            if uri_str.is_empty() {
                return Err(RegistryError::ValidationError(
                    "Storage URI cannot be empty if specified".to_string(),
                ));
            }
        }

        Ok(Self { backend, path, uri })
    }

    /// Generate a URI representation if not already set
    ///
    /// Creates a URI based on the backend type and path.
    pub fn generate_uri(&self) -> String {
        match &self.backend {
            StorageBackend::S3 { bucket, .. } => {
                format!("s3://{}/{}", bucket, self.path)
            }
            StorageBackend::GCS { bucket, .. } => {
                format!("gs://{}/{}", bucket, self.path)
            }
            StorageBackend::AzureBlob { account_name, container } => {
                format!("https://{}.blob.core.windows.net/{}/{}", account_name, container, self.path)
            }
            StorageBackend::MinIO { bucket, endpoint } => {
                format!("{}/{}/{}", endpoint, bucket, self.path)
            }
            StorageBackend::FileSystem { base_path } => {
                format!("file://{}/{}", base_path, self.path)
            }
        }
    }

    /// Get the URI, generating one if not already set
    pub fn get_uri(&self) -> String {
        self.uri.clone().unwrap_or_else(|| self.generate_uri())
    }
}

impl fmt::Display for StorageLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_uri())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s3_backend_validation() {
        let backend = StorageBackend::S3 {
            bucket: "my-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: None,
        };
        assert!(backend.validate().is_ok());
    }

    #[test]
    fn test_s3_backend_validation_empty_bucket() {
        let backend = StorageBackend::S3 {
            bucket: "".to_string(),
            region: "us-east-1".to_string(),
            endpoint: None,
        };
        assert!(backend.validate().is_err());
    }

    #[test]
    fn test_s3_backend_with_endpoint() {
        let backend = StorageBackend::S3 {
            bucket: "my-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: Some("https://s3.example.com".to_string()),
        };
        assert!(backend.validate().is_ok());
    }

    #[test]
    fn test_gcs_backend_validation() {
        let backend = StorageBackend::GCS {
            bucket: "my-bucket".to_string(),
            project_id: "my-project".to_string(),
        };
        assert!(backend.validate().is_ok());
    }

    #[test]
    fn test_azure_backend_validation() {
        let backend = StorageBackend::AzureBlob {
            account_name: "myaccount".to_string(),
            container: "mycontainer".to_string(),
        };
        assert!(backend.validate().is_ok());
    }

    #[test]
    fn test_minio_backend_validation() {
        let backend = StorageBackend::MinIO {
            bucket: "my-bucket".to_string(),
            endpoint: "https://minio.example.com".to_string(),
        };
        assert!(backend.validate().is_ok());
    }

    #[test]
    fn test_filesystem_backend_validation() {
        let backend = StorageBackend::FileSystem {
            base_path: "/var/lib/registry".to_string(),
        };
        assert!(backend.validate().is_ok());
    }

    #[test]
    fn test_storage_location_creation() {
        let backend = StorageBackend::S3 {
            bucket: "my-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: None,
        };
        let location = StorageLocation::new(
            backend,
            "models/gpt-2/model.bin".to_string(),
            None,
        ).unwrap();
        assert_eq!(location.path, "models/gpt-2/model.bin");
    }

    #[test]
    fn test_storage_location_empty_path() {
        let backend = StorageBackend::S3 {
            bucket: "my-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: None,
        };
        assert!(StorageLocation::new(backend, "".to_string(), None).is_err());
    }

    #[test]
    fn test_s3_uri_generation() {
        let backend = StorageBackend::S3 {
            bucket: "my-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: None,
        };
        let location = StorageLocation::new(
            backend,
            "models/gpt-2/model.bin".to_string(),
            None,
        ).unwrap();
        assert_eq!(location.generate_uri(), "s3://my-bucket/models/gpt-2/model.bin");
    }

    #[test]
    fn test_gcs_uri_generation() {
        let backend = StorageBackend::GCS {
            bucket: "my-bucket".to_string(),
            project_id: "my-project".to_string(),
        };
        let location = StorageLocation::new(
            backend,
            "models/bert/model.bin".to_string(),
            None,
        ).unwrap();
        assert_eq!(location.generate_uri(), "gs://my-bucket/models/bert/model.bin");
    }

    #[test]
    fn test_filesystem_uri_generation() {
        let backend = StorageBackend::FileSystem {
            base_path: "/var/lib/registry".to_string(),
        };
        let location = StorageLocation::new(
            backend,
            "models/model.bin".to_string(),
            None,
        ).unwrap();
        assert_eq!(location.generate_uri(), "file:///var/lib/registry/models/model.bin");
    }

    #[test]
    fn test_storage_location_with_custom_uri() {
        let backend = StorageBackend::S3 {
            bucket: "my-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: None,
        };
        let custom_uri = "s3://my-bucket/custom/path".to_string();
        let location = StorageLocation::new(
            backend,
            "models/model.bin".to_string(),
            Some(custom_uri.clone()),
        ).unwrap();
        assert_eq!(location.get_uri(), custom_uri);
    }

    #[test]
    fn test_backend_type() {
        let s3 = StorageBackend::S3 {
            bucket: "bucket".to_string(),
            region: "region".to_string(),
            endpoint: None,
        };
        assert_eq!(s3.backend_type(), "S3");

        let gcs = StorageBackend::GCS {
            bucket: "bucket".to_string(),
            project_id: "project".to_string(),
        };
        assert_eq!(gcs.backend_type(), "GCS");
    }
}
