//! Authentication Integration Tests
//!
//! Tests for JWT authentication, token refresh, and user authentication flows.

mod common;

use common::{assert_status, assert_success, TestApp, fixtures::TestUser};
use reqwest::StatusCode;
use serde_json::json;

#[tokio::test]
async fn test_login_success() {
    let app = TestApp::new().await;
    let client = app.client();

    let response = client
        .post(&format!("{}/auth/login", app.url()))
        .json(&json!({
            "username": "testuser",
            "password": "password123"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_success(&response);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");

    assert!(body["data"]["access_token"].is_string());
    assert!(body["data"]["refresh_token"].is_string());
    assert_eq!(body["data"]["token_type"], "Bearer");
    assert!(body["data"]["user"]["id"].is_string());
}

#[tokio::test]
async fn test_login_empty_credentials() {
    let app = TestApp::new().await;
    let client = app.client();

    let response = client
        .post(&format!("{}/auth/login", app.url()))
        .json(&json!({
            "username": "",
            "password": ""
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_status(&response, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_token_validation() {
    let app = TestApp::new().await;
    let client = app.client();
    let token = app.generate_token("testuser");

    // Use token to access protected endpoint
    let response = client
        .get(&format!("{}/auth/me", app.url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to send request");

    assert_success(&response);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["data"]["id"], "testuser");
}

#[tokio::test]
async fn test_invalid_token() {
    let app = TestApp::new().await;
    let client = app.client();

    let response = client
        .get(&format!("{}/auth/me", app.url()))
        .header("Authorization", "Bearer invalid.token.here")
        .send()
        .await
        .expect("Failed to send request");

    assert_status(&response, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_missing_authorization_header() {
    let app = TestApp::new().await;
    let client = app.client();

    let response = client
        .get(&format!("{}/auth/me", app.url()))
        .send()
        .await
        .expect("Failed to send request");

    assert_status(&response, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_malformed_authorization_header() {
    let app = TestApp::new().await;
    let client = app.client();

    let response = client
        .get(&format!("{}/auth/me", app.url()))
        .header("Authorization", "InvalidFormat token123")
        .send()
        .await
        .expect("Failed to send request");

    assert_status(&response, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_refresh_token() {
    let app = TestApp::new().await;
    let client = app.client();

    // Login first
    let login_response = client
        .post(&format!("{}/auth/login", app.url()))
        .json(&json!({
            "username": "testuser",
            "password": "password123"
        }))
        .send()
        .await
        .expect("Failed to send request");

    let login_body: serde_json::Value = login_response
        .json()
        .await
        .expect("Failed to parse JSON");

    let refresh_token = login_body["data"]["refresh_token"]
        .as_str()
        .expect("No refresh token");

    // Use refresh token
    let response = client
        .post(&format!("{}/auth/refresh", app.url()))
        .json(&json!({
            "refresh_token": refresh_token
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_success(&response);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body["data"]["access_token"].is_string());
    assert!(body["data"]["refresh_token"].is_string());
}

#[tokio::test]
async fn test_refresh_with_invalid_token() {
    let app = TestApp::new().await;
    let client = app.client();

    let response = client
        .post(&format!("{}/auth/refresh", app.url()))
        .json(&json!({
            "refresh_token": "invalid.refresh.token"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_status(&response, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_logout() {
    let app = TestApp::new().await;
    let client = app.client();
    let token = app.generate_token("testuser");

    let response = client
        .post(&format!("{}/auth/logout", app.url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to send request");

    assert_success(&response);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body["data"]["message"].is_string());
}

#[tokio::test]
async fn test_logout_without_auth() {
    let app = TestApp::new().await;
    let client = app.client();

    let response = client
        .post(&format!("{}/auth/logout", app.url()))
        .send()
        .await
        .expect("Failed to send request");

    assert_status(&response, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_me_endpoint() {
    let app = TestApp::new().await;
    let client = app.client();
    let user = TestUser::admin();
    let token = app.generate_token_with_roles(&user.id, user.roles.clone());

    let response = client
        .get(&format!("{}/auth/me", app.url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to send request");

    assert_success(&response);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["data"]["id"], user.id);
    assert!(body["data"]["roles"].as_array().unwrap().contains(&json!("admin")));
}

#[tokio::test]
async fn test_generate_api_key_as_admin() {
    let app = TestApp::new().await;
    let client = app.client();
    let user = TestUser::admin();
    let token = app.generate_token_with_roles(&user.id, user.roles.clone());

    let response = client
        .post(&format!("{}/auth/api-key", app.url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to send request");

    assert_success(&response);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body["data"]["api_key"].is_string());
}

#[tokio::test]
async fn test_generate_api_key_as_regular_user() {
    let app = TestApp::new().await;
    let client = app.client();
    let user = TestUser::regular();
    let token = app.generate_token_with_roles(&user.id, user.roles.clone());

    let response = client
        .post(&format!("{}/auth/api-key", app.url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to send request");

    // Should be forbidden
    assert_status(&response, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_token_with_roles() {
    let app = TestApp::new().await;
    let roles = vec!["admin".to_string(), "developer".to_string()];
    let token = app.generate_token_with_roles("testuser", roles.clone());

    let client = app.client();
    let response = client
        .get(&format!("{}/auth/me", app.url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to send request");

    assert_success(&response);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    let returned_roles: Vec<String> = body["data"]["roles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    assert!(returned_roles.contains(&"admin".to_string()));
    assert!(returned_roles.contains(&"developer".to_string()));
}
