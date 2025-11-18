//! Authentication API handlers
//!
//! This module provides HTTP handlers for authentication endpoints including
//! login, token refresh, and user information.

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, instrument};

use crate::{
    auth::AuthUser,
    error::{ApiError, ApiResult},
    jwt::{Claims, JwtManager, TokenPair},
    responses::{ok, ApiResponse},
};

/// Authentication state for handlers
#[derive(Clone, Debug)]
pub struct AuthHandlerState {
    jwt_manager: Arc<JwtManager>,
}

impl AuthHandlerState {
    /// Create new auth handler state
    pub fn new(jwt_manager: JwtManager) -> Self {
        Self {
            jwt_manager: Arc::new(jwt_manager),
        }
    }

    /// Get JWT manager reference
    pub fn jwt_manager(&self) -> &JwtManager {
        &self.jwt_manager
    }
}

/// Login request
#[derive(Debug, Deserialize, Serialize)]
pub struct LoginRequest {
    /// Username or email
    pub username: String,

    /// Password
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    /// Token pair (access + refresh)
    #[serde(flatten)]
    pub token_pair: TokenPair,

    /// User information
    pub user: UserInfo,
}

/// User information
#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    /// User ID
    pub id: String,

    /// Email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Roles
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
}

impl UserInfo {
    /// Create from claims
    pub fn from_claims(claims: &Claims) -> Self {
        Self {
            id: claims.sub.clone(),
            email: claims.email.clone(),
            roles: claims.roles.clone(),
        }
    }
}

/// Token refresh request
#[derive(Debug, Deserialize, Serialize)]
pub struct RefreshTokenRequest {
    /// Refresh token
    pub refresh_token: String,
}

/// Token refresh response
#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshTokenResponse {
    /// New token pair
    #[serde(flatten)]
    pub token_pair: TokenPair,
}

/// Login handler
///
/// NOTE: This is a simplified login handler for demonstration.
/// In production, you would:
/// 1. Validate credentials against a database
/// 2. Use proper password hashing (bcrypt, argon2)
/// 3. Implement rate limiting
/// 4. Add audit logging
/// 5. Handle MFA/2FA if required
#[instrument(skip(state, request))]
pub async fn login(
    State(state): State<AuthHandlerState>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<(StatusCode, Json<ApiResponse<LoginResponse>>)> {
    info!("Login attempt for user: {}", request.username);

    // TODO: In production, validate against database
    // For now, this is a stub implementation
    if request.username.is_empty() || request.password.is_empty() {
        return Err(ApiError::bad_request("Username and password are required"));
    }

    // Create claims for the user
    let claims = Claims::new(
        &request.username,
        state.jwt_manager().config.issuer.clone(),
        state.jwt_manager().config.audience.clone(),
        state.jwt_manager().config.expiration_seconds,
    )
    .with_email(format!("{}@example.com", request.username))
    .with_role("user");

    // Generate token pair
    let token_pair = state
        .jwt_manager()
        .generate_token_pair(&request.username)
        .map_err(|e| ApiError::internal_server_error(format!("Failed to generate token: {}", e)))?;

    let response = LoginResponse {
        token_pair,
        user: UserInfo::from_claims(&claims),
    };

    info!("User logged in successfully: {}", request.username);
    Ok((StatusCode::OK, Json(ok(response))))
}

/// Refresh token handler
#[instrument(skip(state, request))]
pub async fn refresh_token(
    State(state): State<AuthHandlerState>,
    Json(request): Json<RefreshTokenRequest>,
) -> ApiResult<Json<ApiResponse<RefreshTokenResponse>>> {
    debug!("Token refresh requested");

    // Refresh the token
    let token_pair = state
        .jwt_manager()
        .refresh_access_token(&request.refresh_token)
        .map_err(|e| match e {
            crate::jwt::TokenError::Expired => {
                ApiError::unauthorized("Refresh token has expired")
            }
            crate::jwt::TokenError::InvalidClaims(_) => {
                ApiError::bad_request("Invalid refresh token")
            }
            _ => ApiError::unauthorized("Invalid refresh token"),
        })?;

    let response = RefreshTokenResponse { token_pair };

    debug!("Token refreshed successfully");
    Ok(Json(ok(response)))
}

/// Get current user information
#[instrument(skip(user))]
pub async fn me(
    Extension(user): Extension<AuthUser>,
) -> ApiResult<Json<ApiResponse<UserInfo>>> {
    debug!("Current user info requested");

    let user_info = UserInfo::from_claims(&user.claims);

    Ok(Json(ok(user_info)))
}

/// Logout handler
///
/// NOTE: JWT tokens are stateless, so logout is primarily client-side.
/// In production, you might:
/// 1. Maintain a token blacklist in Redis
/// 2. Use short-lived tokens with refresh tokens
/// 3. Implement token revocation
#[instrument(skip(user))]
pub async fn logout(
    Extension(user): Extension<AuthUser>,
) -> ApiResult<Json<ApiResponse<LogoutResponse>>> {
    info!("User logout: {}", user.user_id());

    // TODO: In production, add token to blacklist
    // For now, just acknowledge the logout

    let response = LogoutResponse {
        message: "Logged out successfully".to_string(),
    };

    Ok(Json(ok(response)))
}

/// Logout response
#[derive(Debug, Serialize, Deserialize)]
pub struct LogoutResponse {
    /// Success message
    pub message: String,
}

/// Generate API key handler (example of protected endpoint)
#[instrument(skip(user))]
pub async fn generate_api_key(
    State(state): State<AuthHandlerState>,
    Extension(user): Extension<AuthUser>,
) -> ApiResult<Json<ApiResponse<ApiKeyResponse>>> {
    info!("Generating API key for user: {}", user.user_id());

    // Check if user has permission
    if !user.has_role("admin") && !user.has_role("developer") {
        return Err(ApiError::forbidden(
            "Only admin or developer roles can generate API keys",
        ));
    }

    // Create a long-lived token for API access
    let claims = Claims::new(
        user.user_id(),
        state.jwt_manager().config.issuer.clone(),
        state.jwt_manager().config.audience.clone(),
        86400 * 30, // 30 days
    )
    .with_roles(user.claims.roles.clone())
    .with_custom("api_key", serde_json::json!(true));

    let api_key = state
        .jwt_manager()
        .generate_token_with_claims(claims)
        .map_err(|e| ApiError::internal_server_error(format!("Failed to generate API key: {}", e)))?;

    let response = ApiKeyResponse { api_key };

    info!("API key generated for user: {}", user.user_id());
    Ok(Json(ok(response)))
}

/// API key response
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyResponse {
    /// Generated API key
    pub api_key: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jwt::JwtConfig;

    fn create_test_state() -> AuthHandlerState {
        let config = JwtConfig::new("test-secret")
            .with_issuer("test")
            .with_audience("test");
        let jwt_manager = JwtManager::new(config).unwrap();
        AuthHandlerState::new(jwt_manager)
    }

    #[test]
    fn test_user_info_from_claims() {
        let claims = Claims::new("user123", "test", "test", 3600)
            .with_email("user@example.com")
            .with_role("admin");

        let user_info = UserInfo::from_claims(&claims);

        assert_eq!(user_info.id, "user123");
        assert_eq!(user_info.email, Some("user@example.com".to_string()));
        assert_eq!(user_info.roles, vec!["admin"]);
    }

    #[tokio::test]
    async fn test_login_request_validation() {
        let state = create_test_state();

        let request = LoginRequest {
            username: "".to_string(),
            password: "password".to_string(),
        };

        let result = login(State(state), Json(request)).await;
        assert!(result.is_err());
    }
}
