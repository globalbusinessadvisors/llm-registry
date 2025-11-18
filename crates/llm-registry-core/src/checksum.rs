//! Checksum verification and hashing algorithm support
//!
//! This module provides types for representing and validating checksums of assets.
//! It supports multiple hashing algorithms to ensure data integrity and security.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::{RegistryError, Result};

/// Supported hashing algorithms for checksum verification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HashAlgorithm {
    /// SHA-256 (most widely supported)
    SHA256,
    /// SHA3-256 (newer, more secure)
    SHA3_256,
    /// BLAKE3 (fastest, most modern)
    BLAKE3,
}

impl HashAlgorithm {
    /// Get the expected length of the hash in bytes
    pub fn hash_length(&self) -> usize {
        match self {
            HashAlgorithm::SHA256 => 32,
            HashAlgorithm::SHA3_256 => 32,
            HashAlgorithm::BLAKE3 => 32,
        }
    }

    /// Get the expected length of the hash in hexadecimal characters
    pub fn hex_length(&self) -> usize {
        self.hash_length() * 2
    }

    /// Validate that a hash string has the correct length for this algorithm
    pub fn validate_hash_format(&self, hash: &str) -> Result<()> {
        let expected_len = self.hex_length();
        let actual_len = hash.len();

        if actual_len != expected_len {
            return Err(RegistryError::ValidationError(format!(
                "Invalid hash length for {:?}: expected {} characters, got {}",
                self, expected_len, actual_len
            )));
        }

        // Validate hexadecimal format
        if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(RegistryError::ValidationError(format!(
                "Invalid hash format: must be hexadecimal string"
            )));
        }

        Ok(())
    }
}

impl fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HashAlgorithm::SHA256 => write!(f, "SHA256"),
            HashAlgorithm::SHA3_256 => write!(f, "SHA3-256"),
            HashAlgorithm::BLAKE3 => write!(f, "BLAKE3"),
        }
    }
}

impl Default for HashAlgorithm {
    fn default() -> Self {
        HashAlgorithm::SHA256
    }
}

impl FromStr for HashAlgorithm {
    type Err = RegistryError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "SHA256" => Ok(HashAlgorithm::SHA256),
            "SHA3-256" | "SHA3_256" => Ok(HashAlgorithm::SHA3_256),
            "BLAKE3" => Ok(HashAlgorithm::BLAKE3),
            _ => Err(RegistryError::ValidationError(format!(
                "Invalid hash algorithm: {}",
                s
            ))),
        }
    }
}

/// Checksum for verifying asset integrity
///
/// Stores a hash value along with the algorithm used to compute it.
/// This allows for verification of asset contents and detection of tampering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checksum {
    /// The hashing algorithm used
    pub algorithm: HashAlgorithm,
    /// The hash value as a hexadecimal string
    pub value: String,
}

impl Checksum {
    /// Create a new checksum with validation
    ///
    /// # Arguments
    /// * `algorithm` - The hashing algorithm used
    /// * `value` - The hash value as a hexadecimal string
    ///
    /// # Errors
    /// Returns an error if the hash value format is invalid for the algorithm
    pub fn new(algorithm: HashAlgorithm, value: String) -> Result<Self> {
        // Normalize to lowercase for consistency
        let normalized_value = value.to_lowercase();

        // Validate the hash format
        algorithm.validate_hash_format(&normalized_value)?;

        Ok(Self {
            algorithm,
            value: normalized_value,
        })
    }

    /// Verify if this checksum matches another checksum
    ///
    /// Returns true if both the algorithm and value match exactly.
    pub fn verify(&self, other: &Checksum) -> bool {
        self.algorithm == other.algorithm && self.value == other.value
    }

    /// Verify if this checksum matches a raw hash value
    ///
    /// The provided hash value will be normalized to lowercase before comparison.
    ///
    /// # Arguments
    /// * `hash_value` - The hash value to compare against
    pub fn verify_hash(&self, hash_value: &str) -> bool {
        self.value == hash_value.to_lowercase()
    }

    /// Get a reference to the hash value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Get the algorithm used
    pub fn algorithm(&self) -> HashAlgorithm {
        self.algorithm
    }
}

impl fmt::Display for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.algorithm, self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_algorithm_lengths() {
        assert_eq!(HashAlgorithm::SHA256.hash_length(), 32);
        assert_eq!(HashAlgorithm::SHA256.hex_length(), 64);
        assert_eq!(HashAlgorithm::SHA3_256.hash_length(), 32);
        assert_eq!(HashAlgorithm::BLAKE3.hash_length(), 32);
    }

    #[test]
    fn test_hash_algorithm_validation() {
        let valid_sha256 = "a".repeat(64);
        assert!(HashAlgorithm::SHA256.validate_hash_format(&valid_sha256).is_ok());

        let invalid_length = "a".repeat(63);
        assert!(HashAlgorithm::SHA256.validate_hash_format(&invalid_length).is_err());

        let invalid_chars = "g".repeat(64);
        assert!(HashAlgorithm::SHA256.validate_hash_format(&invalid_chars).is_err());
    }

    #[test]
    fn test_checksum_creation() {
        let hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let checksum = Checksum::new(HashAlgorithm::SHA256, hash.to_string()).unwrap();
        assert_eq!(checksum.algorithm, HashAlgorithm::SHA256);
        assert_eq!(checksum.value, hash);
    }

    #[test]
    fn test_checksum_normalization() {
        let hash_upper = "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855";
        let checksum = Checksum::new(HashAlgorithm::SHA256, hash_upper.to_string()).unwrap();
        assert_eq!(checksum.value, hash_upper.to_lowercase());
    }

    #[test]
    fn test_checksum_verification() {
        let hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let checksum1 = Checksum::new(HashAlgorithm::SHA256, hash.to_string()).unwrap();
        let checksum2 = Checksum::new(HashAlgorithm::SHA256, hash.to_string()).unwrap();
        assert!(checksum1.verify(&checksum2));
    }

    #[test]
    fn test_checksum_verify_hash() {
        let hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let checksum = Checksum::new(HashAlgorithm::SHA256, hash.to_string()).unwrap();
        assert!(checksum.verify_hash(hash));
        assert!(checksum.verify_hash(&hash.to_uppercase()));
    }

    #[test]
    fn test_checksum_invalid() {
        let invalid = "not_a_valid_hash";
        assert!(Checksum::new(HashAlgorithm::SHA256, invalid.to_string()).is_err());
    }

    #[test]
    fn test_checksum_display() {
        let hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let checksum = Checksum::new(HashAlgorithm::SHA256, hash.to_string()).unwrap();
        assert_eq!(
            checksum.to_string(),
            format!("SHA256:{}", hash)
        );
    }
}
