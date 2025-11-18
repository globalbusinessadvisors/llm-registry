//! API Integration Tests
//!
//! Tests for HTTP REST API endpoints including health checks, versioning,
//! and basic CRUD operations.

mod common;

use common::{assert_status, assert_success, TestApp};
use reqwest::StatusCode;

#[tokio::test]
async fn test_health_endpoint() {
    let app = TestApp::new().await;
    let client = app.client();

    let response = client
        .get(&format!("{}/health", app.url()))
        .send()
        .await
        .expect("Failed to send request");

    assert_success(&response);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");

    assert_eq!(body["status"], "healthy");
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn test_version_endpoint() {
    let app = TestApp::new().await;
    let client = app.client();

    let response = client
        .get(&format!("{}/version", app.url()))
        .send()
        .await
        .expect("Failed to send request");

    assert_success(&response);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");

    assert!(body["version"].is_string());
    assert!(body["build_time"].is_string());
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let app = TestApp::new().await;
    let client = app.client();

    let response = client
        .get(&format!("{}/metrics", app.url()))
        .send()
        .await
        .expect("Failed to send request");

    // Metrics endpoint should return Prometheus format
    assert_success(&response);
    let text = response.text().await.expect("Failed to get response text");
    assert!(!text.is_empty());
}

#[tokio::test]
async fn test_not_found_endpoint() {
    let app = TestApp::new().await;
    let client = app.client();

    let response = client
        .get(&format!("{}/nonexistent", app.url()))
        .send()
        .await
        .expect("Failed to send request");

    assert_status(&response, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_assets_without_auth() {
    let app = TestApp::new().await;
    let client = app.client();

    let response = client
        .get(&format!("{}/api/assets", app.url()))
        .send()
        .await
        .expect("Failed to send request");

    // Should require authentication
    assert_status(&response, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_list_assets_with_auth() {
    let app = TestApp::new().await;
    let client = app.client();
    let token = app.generate_token("test-user");

    let response = client
        .get(&format!("{}/api/assets", app.url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to send request");

    assert_success(&response);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");

    assert!(body["data"].is_array());
    assert!(body["meta"].is_object());
}

#[tokio::test]
async fn test_cors_headers() {
    let app = TestApp::new().await;
    let client = app.client();

    let response = client
        .options(&format!("{}/health", app.url()))
        .header("Origin", "http://example.com")
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .expect("Failed to send request");

    // Should have CORS headers
    assert!(response.headers().contains_key("access-control-allow-origin"));
}

#[tokio::test]
async fn test_request_id_header() {
    let app = TestApp::new().await;
    let client = app.client();

    let response = client
        .get(&format!("{}/health", app.url()))
        .send()
        .await
        .expect("Failed to send request");

    // Should have request ID header
    assert!(response.headers().contains_key("x-request-id"));
}

#[tokio::test]
async fn test_malformed_json_request() {
    let app = TestApp::new().await;
    let client = app.client();
    let token = app.generate_token("test-user");

    let response = client
        .post(&format!("{}/api/assets", app.url()))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body("{invalid json}")
        .send()
        .await
        .expect("Failed to send request");

    assert_status(&response, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_content_type_validation() {
    let app = TestApp::new().await;
    let client = app.client();
    let token = app.generate_token("test-user");

    let response = client
        .post(&format!("{}/api/assets", app.url()))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "text/plain")
        .body("not json")
        .send()
        .await
        .expect("Failed to send request");

    // Should fail due to wrong content type
    assert!(response.status().is_client_error());
}
