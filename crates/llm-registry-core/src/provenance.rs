//! Provenance tracking for assets
//!
//! This module defines types for tracking the origin and build information
//! of assets, enabling reproducibility and auditability.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::error::{RegistryError, Result};

/// Provenance information for an asset
///
/// Tracks the origin, build process, and metadata associated with creating an asset.
/// This enables reproducibility, auditability, and trust in the asset's origin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Source repository URL (e.g., GitHub, GitLab)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_repo: Option<String>,

    /// Git commit hash that was used to build this asset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_hash: Option<String>,

    /// Build system identifier (e.g., Jenkins job ID, GitHub Actions run ID)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_id: Option<String>,

    /// Author or team responsible for creating this asset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Timestamp when the asset was created
    pub created_at: DateTime<Utc>,

    /// Additional build metadata (environment variables, tool versions, etc.)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub build_metadata: HashMap<String, String>,
}

impl Provenance {
    /// Create a new provenance record with the current timestamp
    pub fn new() -> Self {
        Self {
            source_repo: None,
            commit_hash: None,
            build_id: None,
            author: None,
            created_at: Utc::now(),
            build_metadata: HashMap::new(),
        }
    }

    /// Create a provenance record with a builder pattern
    pub fn builder() -> ProvenanceBuilder {
        ProvenanceBuilder::new()
    }

    /// Validate the provenance information
    pub fn validate(&self) -> Result<()> {
        // Validate source repo URL if present
        if let Some(ref repo) = self.source_repo {
            if repo.is_empty() {
                return Err(RegistryError::ValidationError(
                    "Source repository URL cannot be empty if specified".to_string(),
                ));
            }
            // Basic URL validation
            if !repo.starts_with("http://")
                && !repo.starts_with("https://")
                && !repo.starts_with("git@")
                && !repo.starts_with("ssh://") {
                return Err(RegistryError::ValidationError(
                    "Source repository must be a valid URL or SSH connection string".to_string(),
                ));
            }
        }

        // Validate commit hash format if present (SHA-1: 40 chars or SHA-256: 64 chars)
        if let Some(ref hash) = self.commit_hash {
            if hash.is_empty() {
                return Err(RegistryError::ValidationError(
                    "Commit hash cannot be empty if specified".to_string(),
                ));
            }
            let len = hash.len();
            if len != 40 && len != 64 {
                return Err(RegistryError::ValidationError(
                    format!("Commit hash must be 40 (SHA-1) or 64 (SHA-256) characters, got {}", len),
                ));
            }
            if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(RegistryError::ValidationError(
                    "Commit hash must contain only hexadecimal characters".to_string(),
                ));
            }
        }

        // Validate build ID if present
        if let Some(ref build_id) = self.build_id {
            if build_id.is_empty() {
                return Err(RegistryError::ValidationError(
                    "Build ID cannot be empty if specified".to_string(),
                ));
            }
        }

        // Validate author if present
        if let Some(ref author) = self.author {
            if author.is_empty() {
                return Err(RegistryError::ValidationError(
                    "Author cannot be empty if specified".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Add a build metadata entry
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.build_metadata.insert(key, value);
    }

    /// Get a build metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.build_metadata.get(key)
    }

    /// Check if the provenance has complete build information
    pub fn is_complete(&self) -> bool {
        self.source_repo.is_some() && self.commit_hash.is_some()
    }
}

impl Default for Provenance {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Provenance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Provenance(")?;

        let mut parts = Vec::new();

        if let Some(ref repo) = self.source_repo {
            parts.push(format!("repo={}", repo));
        }
        if let Some(ref hash) = self.commit_hash {
            parts.push(format!("commit={}", &hash[..8.min(hash.len())]));
        }
        if let Some(ref build_id) = self.build_id {
            parts.push(format!("build={}", build_id));
        }
        if let Some(ref author) = self.author {
            parts.push(format!("author={}", author));
        }

        write!(f, "{}", parts.join(", "))?;
        write!(f, ")")
    }
}

/// Builder for creating Provenance instances
pub struct ProvenanceBuilder {
    provenance: Provenance,
}

impl ProvenanceBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            provenance: Provenance::new(),
        }
    }

    /// Set the source repository URL
    pub fn source_repo(mut self, repo: impl Into<String>) -> Self {
        self.provenance.source_repo = Some(repo.into());
        self
    }

    /// Set the commit hash
    pub fn commit_hash(mut self, hash: impl Into<String>) -> Self {
        self.provenance.commit_hash = Some(hash.into());
        self
    }

    /// Set the build ID
    pub fn build_id(mut self, id: impl Into<String>) -> Self {
        self.provenance.build_id = Some(id.into());
        self
    }

    /// Set the author
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.provenance.author = Some(author.into());
        self
    }

    /// Set the created timestamp
    pub fn created_at(mut self, timestamp: DateTime<Utc>) -> Self {
        self.provenance.created_at = timestamp;
        self
    }

    /// Add build metadata
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.provenance.build_metadata.insert(key.into(), value.into());
        self
    }

    /// Add multiple build metadata entries
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.provenance.build_metadata.extend(metadata);
        self
    }

    /// Build the Provenance instance with validation
    pub fn build(self) -> Result<Provenance> {
        self.provenance.validate()?;
        Ok(self.provenance)
    }

    /// Build the Provenance instance without validation
    pub fn build_unchecked(self) -> Provenance {
        self.provenance
    }
}

impl Default for ProvenanceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provenance_new() {
        let prov = Provenance::new();
        assert!(prov.source_repo.is_none());
        assert!(prov.commit_hash.is_none());
        assert!(prov.build_id.is_none());
        assert!(prov.author.is_none());
        assert!(prov.build_metadata.is_empty());
    }

    #[test]
    fn test_provenance_builder() {
        let prov = Provenance::builder()
            .source_repo("https://github.com/example/repo")
            .commit_hash("a94a8fe5ccb19ba61c4c0873d391e987982fbbd3")
            .build_id("build-123")
            .author("Alice")
            .build()
            .unwrap();

        assert_eq!(prov.source_repo.as_deref(), Some("https://github.com/example/repo"));
        assert_eq!(prov.commit_hash.as_deref(), Some("a94a8fe5ccb19ba61c4c0873d391e987982fbbd3"));
        assert_eq!(prov.build_id.as_deref(), Some("build-123"));
        assert_eq!(prov.author.as_deref(), Some("Alice"));
    }

    #[test]
    fn test_provenance_validation_valid_urls() {
        let valid_urls = vec![
            "https://github.com/example/repo",
            "http://gitlab.com/example/repo",
            "git@github.com:example/repo.git",
            "ssh://git@github.com/example/repo.git",
        ];

        for url in valid_urls {
            let prov = Provenance::builder()
                .source_repo(url)
                .build()
                .unwrap();
            assert_eq!(prov.source_repo.as_deref(), Some(url));
        }
    }

    #[test]
    fn test_provenance_validation_invalid_url() {
        let result = Provenance::builder()
            .source_repo("not-a-url")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_provenance_validation_empty_repo() {
        let result = Provenance::builder()
            .source_repo("")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_provenance_validation_valid_commit_sha1() {
        let prov = Provenance::builder()
            .commit_hash("a94a8fe5ccb19ba61c4c0873d391e987982fbbd3")
            .build()
            .unwrap();
        assert_eq!(prov.commit_hash.as_deref(), Some("a94a8fe5ccb19ba61c4c0873d391e987982fbbd3"));
    }

    #[test]
    fn test_provenance_validation_valid_commit_sha256() {
        let prov = Provenance::builder()
            .commit_hash("a94a8fe5ccb19ba61c4c0873d391e987982fbbd3a94a8fe5ccb19ba61c4c0873")
            .build()
            .unwrap();
        assert_eq!(prov.commit_hash.as_deref(), Some("a94a8fe5ccb19ba61c4c0873d391e987982fbbd3a94a8fe5ccb19ba61c4c0873"));
    }

    #[test]
    fn test_provenance_validation_invalid_commit_length() {
        let result = Provenance::builder()
            .commit_hash("abc123")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_provenance_validation_invalid_commit_chars() {
        let result = Provenance::builder()
            .commit_hash("g94a8fe5ccb19ba61c4c0873d391e987982fbbd3")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_provenance_metadata() {
        let mut prov = Provenance::new();
        prov.add_metadata("python_version".to_string(), "3.11".to_string());
        prov.add_metadata("torch_version".to_string(), "2.0.0".to_string());

        assert_eq!(prov.get_metadata("python_version"), Some(&"3.11".to_string()));
        assert_eq!(prov.get_metadata("torch_version"), Some(&"2.0.0".to_string()));
        assert_eq!(prov.build_metadata.len(), 2);
    }

    #[test]
    fn test_provenance_builder_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("key1".to_string(), "value1".to_string());
        metadata.insert("key2".to_string(), "value2".to_string());

        let prov = Provenance::builder()
            .metadata("key3", "value3")
            .with_metadata(metadata)
            .build()
            .unwrap();

        assert_eq!(prov.build_metadata.len(), 3);
        assert_eq!(prov.get_metadata("key1"), Some(&"value1".to_string()));
        assert_eq!(prov.get_metadata("key3"), Some(&"value3".to_string()));
    }

    #[test]
    fn test_provenance_is_complete() {
        let incomplete = Provenance::new();
        assert!(!incomplete.is_complete());

        let complete = Provenance::builder()
            .source_repo("https://github.com/example/repo")
            .commit_hash("a94a8fe5ccb19ba61c4c0873d391e987982fbbd3")
            .build()
            .unwrap();
        assert!(complete.is_complete());
    }

    #[test]
    fn test_provenance_display() {
        let prov = Provenance::builder()
            .source_repo("https://github.com/example/repo")
            .commit_hash("a94a8fe5ccb19ba61c4c0873d391e987982fbbd3")
            .build_id("build-123")
            .author("Alice")
            .build_unchecked();

        let display = format!("{}", prov);
        assert!(display.contains("repo="));
        assert!(display.contains("commit="));
        assert!(display.contains("build="));
        assert!(display.contains("author="));
    }
}
