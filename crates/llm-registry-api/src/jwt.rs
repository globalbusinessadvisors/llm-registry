//! JWT token management
//!
//! This module provides JWT token generation, validation, and refresh functionality
//! for API authentication.

use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, errors::Error as JwtError, Algorithm, DecodingKey, EncodingKey, Header,
    Validation,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

/// JWT configuration
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Secret key for signing tokens
    pub secret: String,

    /// Token expiration in seconds
    pub expiration_seconds: i64,

    /// Refresh token expiration in seconds
    pub refresh_expiration_seconds: i64,

    /// Token issuer
    pub issuer: String,

    /// Token audience
    pub audience: String,

    /// Algorithm for signing
    pub algorithm: Algorithm,
}

// Make config accessible through getters
impl JwtConfig {
    /// Get the issuer
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    /// Get the audience
    pub fn audience(&self) -> &str {
        &self.audience
    }

    /// Get expiration seconds
    pub fn expiration_seconds(&self) -> i64 {
        self.expiration_seconds
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "change-me-in-production".to_string(),
            expiration_seconds: 3600,          // 1 hour
            refresh_expiration_seconds: 86400 * 7, // 7 days
            issuer: "llm-registry".to_string(),
            audience: "llm-registry-api".to_string(),
            algorithm: Algorithm::HS256,
        }
    }
}

impl JwtConfig {
    /// Create new JWT configuration
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            ..Default::default()
        }
    }

    /// Set token expiration in seconds
    pub fn with_expiration(mut self, seconds: i64) -> Self {
        self.expiration_seconds = seconds;
        self
    }

    /// Set refresh token expiration in seconds
    pub fn with_refresh_expiration(mut self, seconds: i64) -> Self {
        self.refresh_expiration_seconds = seconds;
        self
    }

    /// Set issuer
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = issuer.into();
        self
    }

    /// Set audience
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = audience.into();
        self
    }

    /// Set signing algorithm
    pub fn with_algorithm(mut self, algorithm: Algorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), JwtConfigError> {
        if self.secret.is_empty() {
            return Err(JwtConfigError::EmptySecret);
        }

        if self.secret == "change-me-in-production" {
            tracing::warn!("Using default JWT secret - change this in production!");
        }

        if self.expiration_seconds <= 0 {
            return Err(JwtConfigError::InvalidExpiration);
        }

        if self.refresh_expiration_seconds <= 0 {
            return Err(JwtConfigError::InvalidExpiration);
        }

        if self.issuer.is_empty() {
            return Err(JwtConfigError::EmptyIssuer);
        }

        if self.audience.is_empty() {
            return Err(JwtConfigError::EmptyAudience);
        }

        Ok(())
    }
}

/// JWT configuration errors
#[derive(Debug, Error)]
pub enum JwtConfigError {
    #[error("JWT secret cannot be empty")]
    EmptySecret,

    #[error("JWT expiration must be positive")]
    InvalidExpiration,

    #[error("JWT issuer cannot be empty")]
    EmptyIssuer,

    #[error("JWT audience cannot be empty")]
    EmptyAudience,
}

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,

    /// Issuer
    pub iss: String,

    /// Audience
    pub aud: String,

    /// Expiration time (Unix timestamp)
    pub exp: i64,

    /// Issued at (Unix timestamp)
    pub iat: i64,

    /// Not before (Unix timestamp)
    pub nbf: i64,

    /// JWT ID (unique token identifier)
    pub jti: String,

    /// User email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// User roles
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,

    /// Custom claims
    #[serde(flatten)]
    pub custom: serde_json::Value,
}

impl Claims {
    /// Create new claims with default values
    pub fn new(
        user_id: impl Into<String>,
        issuer: impl Into<String>,
        audience: impl Into<String>,
        expiration_seconds: i64,
    ) -> Self {
        let now = Utc::now();
        let exp = now + Duration::seconds(expiration_seconds);

        Self {
            sub: user_id.into(),
            iss: issuer.into(),
            aud: audience.into(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
            email: None,
            roles: Vec::new(),
            custom: serde_json::json!({}),
        }
    }

    /// Add email to claims
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Add roles to claims
    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }

    /// Add a single role
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }

    /// Add custom claims
    pub fn with_custom(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        if let Some(obj) = self.custom.as_object_mut() {
            obj.insert(key.into(), value);
        }
        self
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp();
        self.exp < now
    }

    /// Check if token is not yet valid
    pub fn is_not_yet_valid(&self) -> bool {
        let now = Utc::now().timestamp();
        self.nbf > now
    }

    /// Check if claims are valid
    pub fn validate(&self) -> Result<(), TokenError> {
        if self.is_expired() {
            return Err(TokenError::Expired);
        }

        if self.is_not_yet_valid() {
            return Err(TokenError::NotYetValid);
        }

        if self.sub.is_empty() {
            return Err(TokenError::InvalidClaims("Subject cannot be empty".to_string()));
        }

        Ok(())
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    /// Check if user has all of the specified roles
    pub fn has_all_roles(&self, roles: &[&str]) -> bool {
        roles.iter().all(|role| self.has_role(role))
    }
}

impl fmt::Display for Claims {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Claims(sub={}, jti={})", self.sub, self.jti)
    }
}

/// Token errors
#[derive(Debug, Error)]
pub enum TokenError {
    #[error("Token has expired")]
    Expired,

    #[error("Token is not yet valid")]
    NotYetValid,

    #[error("Invalid token claims: {0}")]
    InvalidClaims(String),

    #[error("JWT error: {0}")]
    JwtError(#[from] JwtError),

    #[error("Invalid token format")]
    InvalidFormat,
}

/// JWT token pair (access + refresh)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    /// Access token
    pub access_token: String,

    /// Refresh token
    pub refresh_token: String,

    /// Token type (always "Bearer")
    pub token_type: String,

    /// Expiration in seconds
    pub expires_in: i64,
}

impl TokenPair {
    /// Create a new token pair
    pub fn new(access_token: String, refresh_token: String, expires_in: i64) -> Self {
        Self {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in,
        }
    }
}

/// JWT token manager
pub struct JwtManager {
    pub config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtManager {
    /// Create a new JWT manager
    pub fn new(config: JwtConfig) -> Result<Self, JwtConfigError> {
        config.validate()?;

        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());

        let mut validation = Validation::new(config.algorithm);
        validation.set_issuer(&[&config.issuer]);
        validation.set_audience(&[&config.audience]);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        Ok(Self {
            config,
            encoding_key,
            decoding_key,
            validation,
        })
    }

    /// Generate a new access token
    pub fn generate_token(&self, user_id: impl Into<String>) -> Result<String, TokenError> {
        let claims = Claims::new(
            user_id,
            &self.config.issuer,
            &self.config.audience,
            self.config.expiration_seconds,
        );

        let header = Header::new(self.config.algorithm);
        encode(&header, &claims, &self.encoding_key).map_err(TokenError::from)
    }

    /// Generate a new access token with custom claims
    pub fn generate_token_with_claims(&self, claims: Claims) -> Result<String, TokenError> {
        let header = Header::new(self.config.algorithm);
        encode(&header, &claims, &self.encoding_key).map_err(TokenError::from)
    }

    /// Generate a new refresh token
    pub fn generate_refresh_token(&self, user_id: impl Into<String>) -> Result<String, TokenError> {
        let claims = Claims::new(
            user_id,
            &self.config.issuer,
            &self.config.audience,
            self.config.refresh_expiration_seconds,
        )
        .with_role("refresh");

        let header = Header::new(self.config.algorithm);
        encode(&header, &claims, &self.encoding_key).map_err(TokenError::from)
    }

    /// Generate a token pair (access + refresh)
    pub fn generate_token_pair(&self, user_id: impl Into<String>) -> Result<TokenPair, TokenError> {
        let user_id = user_id.into();
        let access_token = self.generate_token(&user_id)?;
        let refresh_token = self.generate_refresh_token(&user_id)?;

        Ok(TokenPair::new(
            access_token,
            refresh_token,
            self.config.expiration_seconds,
        ))
    }

    /// Validate and decode a token
    pub fn validate_token(&self, token: &str) -> Result<Claims, TokenError> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &self.validation)?;
        let claims = token_data.claims;
        claims.validate()?;
        Ok(claims)
    }

    /// Refresh an access token using a refresh token
    pub fn refresh_access_token(&self, refresh_token: &str) -> Result<TokenPair, TokenError> {
        let claims = self.validate_token(refresh_token)?;

        // Verify it's a refresh token
        if !claims.has_role("refresh") {
            return Err(TokenError::InvalidClaims(
                "Not a refresh token".to_string(),
            ));
        }

        // Generate new token pair
        self.generate_token_pair(&claims.sub)
    }

    /// Decode token without validation (use with caution)
    pub fn decode_unverified(&self, token: &str) -> Result<Claims, TokenError> {
        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &Validation::new(self.config.algorithm),
        )?;
        Ok(token_data.claims)
    }

    /// Extract token from Authorization header value
    pub fn extract_token_from_header(header_value: &str) -> Result<&str, TokenError> {
        let parts: Vec<&str> = header_value.split_whitespace().collect();

        if parts.len() != 2 {
            return Err(TokenError::InvalidFormat);
        }

        if parts[0].to_lowercase() != "bearer" {
            return Err(TokenError::InvalidFormat);
        }

        Ok(parts[1])
    }
}

impl fmt::Debug for JwtManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JwtManager")
            .field("issuer", &self.config.issuer)
            .field("audience", &self.config.audience)
            .field("algorithm", &self.config.algorithm)
            .field("expiration_seconds", &self.config.expiration_seconds)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> JwtConfig {
        JwtConfig::new("test-secret-key-for-testing")
            .with_issuer("test-issuer")
            .with_audience("test-audience")
            .with_expiration(3600)
    }

    #[test]
    fn test_jwt_config_validation() {
        let config = create_test_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_jwt_config_empty_secret() {
        let config = JwtConfig {
            secret: String::new(),
            ..create_test_config()
        };
        assert!(matches!(config.validate(), Err(JwtConfigError::EmptySecret)));
    }

    #[test]
    fn test_claims_creation() {
        let claims = Claims::new("user123", "test-issuer", "test-audience", 3600);

        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.iss, "test-issuer");
        assert_eq!(claims.aud, "test-audience");
        assert!(!claims.is_expired());
    }

    #[test]
    fn test_claims_with_roles() {
        let claims = Claims::new("user123", "test", "test", 3600)
            .with_role("admin")
            .with_role("user");

        assert!(claims.has_role("admin"));
        assert!(claims.has_role("user"));
        assert!(!claims.has_role("moderator"));
        assert!(claims.has_any_role(&["admin", "moderator"]));
        assert!(claims.has_all_roles(&["admin", "user"]));
        assert!(!claims.has_all_roles(&["admin", "moderator"]));
    }

    #[test]
    fn test_jwt_manager_creation() {
        let config = create_test_config();
        let manager = JwtManager::new(config);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_generate_and_validate_token() {
        let config = create_test_config();
        let manager = JwtManager::new(config).unwrap();

        let token = manager.generate_token("user123").unwrap();
        let claims = manager.validate_token(&token).unwrap();

        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.iss, "test-issuer");
        assert_eq!(claims.aud, "test-audience");
    }

    #[test]
    fn test_generate_token_pair() {
        let config = create_test_config();
        let manager = JwtManager::new(config).unwrap();

        let pair = manager.generate_token_pair("user123").unwrap();

        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
        assert_eq!(pair.token_type, "Bearer");
        assert_eq!(pair.expires_in, 3600);

        // Validate access token
        let access_claims = manager.validate_token(&pair.access_token).unwrap();
        assert_eq!(access_claims.sub, "user123");

        // Validate refresh token
        let refresh_claims = manager.validate_token(&pair.refresh_token).unwrap();
        assert_eq!(refresh_claims.sub, "user123");
        assert!(refresh_claims.has_role("refresh"));
    }

    #[test]
    fn test_refresh_access_token() {
        let config = create_test_config();
        let manager = JwtManager::new(config).unwrap();

        let pair = manager.generate_token_pair("user123").unwrap();
        let new_pair = manager.refresh_access_token(&pair.refresh_token).unwrap();

        assert!(!new_pair.access_token.is_empty());
        assert_ne!(pair.access_token, new_pair.access_token);
    }

    #[test]
    fn test_extract_token_from_header() {
        let header = "Bearer abc123xyz";
        let token = JwtManager::extract_token_from_header(header).unwrap();
        assert_eq!(token, "abc123xyz");
    }

    #[test]
    fn test_extract_token_invalid_format() {
        let header = "InvalidFormat";
        assert!(JwtManager::extract_token_from_header(header).is_err());
    }

    #[test]
    fn test_validate_invalid_token() {
        let config = create_test_config();
        let manager = JwtManager::new(config).unwrap();

        let result = manager.validate_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_claims_with_email_and_custom() {
        let claims = Claims::new("user123", "test", "test", 3600)
            .with_email("user@example.com")
            .with_custom("org_id", serde_json::json!("org-456"));

        assert_eq!(claims.email, Some("user@example.com".to_string()));
        assert_eq!(claims.custom["org_id"], "org-456");
    }
}
