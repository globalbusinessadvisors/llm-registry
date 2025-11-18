//! Core type definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use ulid::Ulid;

/// Asset identifier using ULID (Universally Unique Lexicographically Sortable Identifier)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(Ulid);

impl AssetId {
    /// Generate a new AssetId
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Create AssetId from a ULID
    pub fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    /// Get the underlying ULID
    pub fn as_ulid(&self) -> &Ulid {
        &self.0
    }

    /// Convert to string representation
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }

    /// Parse from string
    pub fn from_string(s: &str) -> Result<Self, String> {
        Ulid::from_string(s)
            .map(Self)
            .map_err(|e| format!("Invalid AssetId: {}", e))
    }
}

impl Default for AssetId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Asset status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetStatus {
    /// Asset is active and usable
    Active,
    /// Asset is deprecated but still available
    Deprecated,
    /// Asset is archived and not recommended for use
    Archived,
    /// Asset violates compliance policies
    NonCompliant,
}

impl Default for AssetStatus {
    fn default() -> Self {
        Self::Active
    }
}

impl fmt::Display for AssetStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Deprecated => write!(f, "deprecated"),
            Self::Archived => write!(f, "archived"),
            Self::NonCompliant => write!(f, "non_compliant"),
        }
    }
}

impl FromStr for AssetStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "deprecated" => Ok(Self::Deprecated),
            "archived" => Ok(Self::Archived),
            "non_compliant" => Ok(Self::NonCompliant),
            _ => Err(format!("Invalid asset status: {}", s)),
        }
    }
}

impl FromStr for AssetId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        AssetId::from_string(s)
    }
}

/// Type alias for tags (user-defined labels)
pub type Tags = Vec<String>;

/// Type alias for annotations (key-value metadata)
pub type Annotations = HashMap<String, String>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_id_generation() {
        let id1 = AssetId::new();
        let id2 = AssetId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_asset_id_string_conversion() {
        let id = AssetId::new();
        let id_str = id.to_string();
        let parsed = AssetId::from_string(&id_str).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_asset_status_default() {
        let status = AssetStatus::default();
        assert_eq!(status, AssetStatus::Active);
    }
}
