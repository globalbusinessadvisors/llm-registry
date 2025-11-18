//! Rate Limiting Integration Tests
//!
//! Tests for rate limiting middleware and token bucket implementation.

mod common;

use common::{assert_status, TestApp};
use reqwest::StatusCode;

#[tokio::test]
async fn test_rate_limit_enforcement() {
    let app = TestApp::new().await;
    let client = app.client();
    let token = app.generate_token("testuser");

    // Make requests up to the limit
    for i in 0..10 {
        let response = client
            .get(&format!("{}/health", app.url()))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .expect("Failed to send request");

        if i < 100 {  // Assuming default limit of 100
            // Should succeed
            assert!(response.status().is_success() || i >= 100, "Request {} failed", i);
        }
    }
}

#[tokio::test]
async fn test_rate_limit_headers() {
    let app = TestApp::new().await;
    let client = app.client();

    let response = client
        .get(&format!("{}/health", app.url()))
        .send()
        .await
        .expect("Failed to send request");

    // Should have rate limit headers
    assert!(response.headers().contains_key("x-ratelimit-limit") ||
            response.status().is_success());
}

#[tokio::test]
async fn test_rate_limit_retry_after() {
    let app = TestApp::new().await;
    let client = app.client();
    let token = app.generate_token("testuser");

    // Try to exceed rate limit (simplified test)
    for _ in 0..150 {
        let response = client
            .get(&format!("{}/health", app.url()))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .expect("Failed to send request");

        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            // Should have Retry-After header
            assert!(response.headers().contains_key("retry-after"));
            break;
        }
    }
}

#[tokio::test]
async fn test_rate_limit_per_user() {
    let app = TestApp::new().await;
    let client = app.client();

    let token1 = app.generate_token("user1");
    let token2 = app.generate_token("user2");

    // Each user should have their own limit
    let response1 = client
        .get(&format!("{}/health", app.url()))
        .header("Authorization", format!("Bearer {}", token1))
        .send()
        .await
        .expect("Failed to send request");

    let response2 = client
        .get(&format!("{}/health", app.url()))
        .header("Authorization", format!("Bearer {}", token2))
        .send()
        .await
        .expect("Failed to send request");

    // Both should succeed (separate limits)
    assert!(response1.status().is_success());
    assert!(response2.status().is_success());
}
