//! Common test utilities and helpers
//!
//! This module provides shared utilities for integration tests including
//! test setup, fixtures, and helper functions.

use axum::Router;
use llm_registry_api::{build_api_server, AuthHandlerState, AuthState, JwtConfig, JwtManager};
use llm_registry_core::AssetId;
use llm_registry_db::{DbConfig, DbPool};
use llm_registry_service::ServiceRegistry;
use serde::de::DeserializeOwned;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

pub mod fixtures;

/// Test application state
pub struct TestApp {
    pub address: String,
    pub pool: DbPool,
    pub services: Arc<ServiceRegistry>,
    pub jwt_manager: Arc<JwtManager>,
}

impl TestApp {
    /// Create a new test application
    pub async fn new() -> Self {
        // Setup test database
        let pool = setup_test_database().await;

        // Create services
        let services = ServiceRegistry::new(pool.clone())
            .await
            .expect("Failed to create services");

        // Create JWT manager
        let jwt_config = JwtConfig::new("test-secret-key-for-integration-tests")
            .with_issuer("test")
            .with_audience("test")
            .with_expiration(3600);
        let jwt_manager = Arc::new(JwtManager::new(jwt_config).expect("Failed to create JWT manager"));

        // Build router
        let app = build_api_server(services.clone());

        // Start server on random port
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind random port");
        let address = listener.local_addr().expect("Failed to get local address");

        // Spawn server
        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("Failed to start test server");
        });

        // Wait a bit for server to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Self {
            address: format!("http://{}", address),
            pool,
            services: Arc::new(services),
            jwt_manager,
        }
    }

    /// Get base URL
    pub fn url(&self) -> &str {
        &self.address
    }

    /// Create HTTP client
    pub fn client(&self) -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to build client")
    }

    /// Generate test JWT token
    pub fn generate_token(&self, user_id: &str) -> String {
        self.jwt_manager
            .generate_token(user_id)
            .expect("Failed to generate token")
    }

    /// Generate token with roles
    pub fn generate_token_with_roles(&self, user_id: &str, roles: Vec<String>) -> String {
        let claims = llm_registry_api::Claims::new(
            user_id,
            "test",
            "test",
            3600,
        ).with_roles(roles);

        self.jwt_manager
            .generate_token_with_claims(claims)
            .expect("Failed to generate token")
    }

    /// Clean up test data
    pub async fn cleanup(&self) {
        // Clean up database
        sqlx::query("DELETE FROM assets")
            .execute(&self.pool)
            .await
            .ok();
    }
}

/// Setup test database
async fn setup_test_database() -> DbPool {
    // Use in-memory SQLite for tests
    let config = DbConfig {
        url: "sqlite::memory:".to_string(),
        max_connections: 5,
        min_connections: 1,
        connect_timeout: 30,
        idle_timeout: 600,
        max_lifetime: 1800,
    };

    let pool = DbPool::connect(&config)
        .await
        .expect("Failed to create database pool");

    // Run migrations
    sqlx::migrate!("../crates/llm-registry-db/migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

/// Helper to make authenticated GET request
pub async fn get_with_auth(
    client: &reqwest::Client,
    url: &str,
    token: &str,
) -> reqwest::Response {
    client
        .get(url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to send request")
}

/// Helper to make authenticated POST request
pub async fn post_with_auth<T: serde::Serialize>(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    body: &T,
) -> reqwest::Response {
    client
        .post(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .expect("Failed to send request")
}

/// Helper to make authenticated PUT request
pub async fn put_with_auth<T: serde::Serialize>(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    body: &T,
) -> reqwest::Response {
    client
        .put(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .expect("Failed to send request")
}

/// Helper to make authenticated DELETE request
pub async fn delete_with_auth(
    client: &reqwest::Client,
    url: &str,
    token: &str,
) -> reqwest::Response {
    client
        .delete(url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to send request")
}

/// Parse JSON response
pub async fn parse_json<T: DeserializeOwned>(response: reqwest::Response) -> T {
    response
        .json::<T>()
        .await
        .expect("Failed to parse JSON response")
}

/// Assert response status
pub fn assert_status(response: &reqwest::Response, expected: reqwest::StatusCode) {
    assert_eq!(
        response.status(),
        expected,
        "Expected status {}, got {}",
        expected,
        response.status()
    );
}

/// Assert response is successful (2xx)
pub fn assert_success(response: &reqwest::Response) {
    assert!(
        response.status().is_success(),
        "Expected success status, got {}",
        response.status()
    );
}

/// Assert response is client error (4xx)
pub fn assert_client_error(response: &reqwest::Response) {
    assert!(
        response.status().is_client_error(),
        "Expected client error status, got {}",
        response.status()
    );
}

/// Generate random asset ID
pub fn random_asset_id() -> AssetId {
    AssetId::new()
}

/// Generate random string
pub fn random_string(len: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
        .collect()
}
