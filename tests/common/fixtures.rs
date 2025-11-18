//! Test fixtures
//!
//! This module provides test data fixtures for integration tests.

use llm_registry_core::{
    Asset, AssetMetadata, AssetStatus, AssetType, Checksum, HashAlgorithm, Provenance,
};
use std::collections::HashMap;

/// Create a test asset with default values
pub fn create_test_asset(name: &str) -> Asset {
    Asset {
        id: llm_registry_core::AssetId::new(),
        name: name.to_string(),
        version: semver::Version::new(1, 0, 0),
        asset_type: AssetType::Model,
        description: Some(format!("Test asset: {}", name)),
        location: format!("file:///tmp/assets/{}", name),
        checksum: Checksum {
            algorithm: HashAlgorithm::Sha256,
            value: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        },
        metadata: AssetMetadata {
            tags: vec!["test".to_string(), "integration".to_string()],
            properties: HashMap::new(),
            framework: Some("pytorch".to_string()),
            task: Some("classification".to_string()),
            architecture: Some("resnet50".to_string()),
        },
        provenance: Provenance {
            source_repo: Some("https://github.com/test/model".to_string()),
            commit_hash: Some("abc123def456".to_string()),
            created_at: chrono::Utc::now(),
            build_metadata: HashMap::new(),
        },
        status: AssetStatus::Active,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        dependencies: Vec::new(),
    }
}

/// Create a test asset with specific type
pub fn create_test_asset_with_type(name: &str, asset_type: AssetType) -> Asset {
    let mut asset = create_test_asset(name);
    asset.asset_type = asset_type;
    asset
}

/// Create a test asset with tags
pub fn create_test_asset_with_tags(name: &str, tags: Vec<String>) -> Asset {
    let mut asset = create_test_asset(name);
    asset.metadata.tags = tags;
    asset
}

/// Create a test user for authentication
pub struct TestUser {
    pub id: String,
    pub email: String,
    pub roles: Vec<String>,
}

impl TestUser {
    pub fn admin() -> Self {
        Self {
            id: "admin-user".to_string(),
            email: "admin@test.com".to_string(),
            roles: vec!["admin".to_string()],
        }
    }

    pub fn developer() -> Self {
        Self {
            id: "dev-user".to_string(),
            email: "dev@test.com".to_string(),
            roles: vec!["developer".to_string()],
        }
    }

    pub fn viewer() -> Self {
        Self {
            id: "viewer-user".to_string(),
            email: "viewer@test.com".to_string(),
            roles: vec!["viewer".to_string()],
        }
    }

    pub fn regular() -> Self {
        Self {
            id: "regular-user".to_string(),
            email: "user@test.com".to_string(),
            roles: vec!["user".to_string()],
        }
    }
}
