//! RBAC Integration Tests
//!
//! Tests for role-based access control and authorization.

mod common;

use common::{assert_status, fixtures::TestUser, TestApp};
use reqwest::StatusCode;

#[tokio::test]
async fn test_admin_full_access() {
    let app = TestApp::new().await;
    let client = app.client();
    let user = TestUser::admin();
    let token = app.generate_token_with_roles(&user.id, user.roles);

    // Admin should access all endpoints
    let response = client
        .get(&format!("{}/api/assets", app.url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_success());
}

#[tokio::test]
async fn test_viewer_read_only() {
    let app = TestApp::new().await;
    let client = app.client();
    let user = TestUser::viewer();
    let token = app.generate_token_with_roles(&user.id, user.roles);

    // Viewer should read
    let response = client
        .get(&format!("{}/api/assets", app.url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_success());
}

#[tokio::test]
async fn test_role_based_api_key_generation() {
    let app = TestApp::new().await;
    let client = app.client();

    // Admin can generate
    let admin = TestUser::admin();
    let admin_token = app.generate_token_with_roles(&admin.id, admin.roles);

    let response = client
        .post(&format!("{}/auth/api-key", app.url()))
        .header("Authorization", format!("Bearer {}", admin_token))
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_success());

    // Viewer cannot generate
    let viewer = TestUser::viewer();
    let viewer_token = app.generate_token_with_roles(&viewer.id, viewer.roles);

    let response = client
        .post(&format!("{}/auth/api-key", app.url()))
        .header("Authorization", format!("Bearer {}", viewer_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_status(&response, StatusCode::FORBIDDEN);
}
