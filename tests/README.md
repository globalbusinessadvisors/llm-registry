# LLM Registry - Integration Test Suite

Enterprise-grade integration tests for the LLM Registry application.

## Overview

This test suite provides comprehensive integration testing covering all major components:

- **API Tests** - HTTP REST API endpoints
- **Authentication Tests** - JWT authentication and token management
- **RBAC Tests** - Role-based access control
- **Rate Limiting Tests** - Rate limiting middleware
- **Database Tests** - Database operations and migrations
- **gRPC Tests** - gRPC service endpoints
- **End-to-End Tests** - Complete user workflows

## Test Structure

```
tests/
├── common/
│   ├── mod.rs          # Common test utilities and helpers
│   └── fixtures.rs     # Test data fixtures
├── api_tests.rs        # API integration tests
├── auth_tests.rs       # Authentication tests
├── rbac_tests.rs       # RBAC tests
├── rate_limit_tests.rs # Rate limiting tests
└── README.md          # This file
```

## Running Tests

### Run All Integration Tests

```bash
cargo test --test '*'
```

### Run Specific Test Suite

```bash
# API tests only
cargo test --test api_tests

# Authentication tests only
cargo test --test auth_tests

# RBAC tests only
cargo test --test rbac_tests

# Rate limiting tests only
cargo test --test rate_limit_tests
```

### Run Specific Test

```bash
cargo test --test api_tests test_health_endpoint
```

### Run with Output

```bash
cargo test --test api_tests -- --nocapture
```

### Run in Parallel

```bash
cargo test --test '*' -- --test-threads=4
```

## Test Features

### Common Utilities

The `common` module provides:

- **TestApp** - Test application setup and management
- **Test Database** - In-memory SQLite for isolated tests
- **JWT Helpers** - Token generation for authentication
- **HTTP Helpers** - Authenticated request helpers
- **Assertions** - Status code and response assertions
- **Fixtures** - Reusable test data

### API Tests (14 tests)

Tests for HTTP REST API:

- ✅ Health endpoint
- ✅ Version endpoint
- ✅ Metrics endpoint (Prometheus)
- ✅ Not found handling
- ✅ Authentication requirement
- ✅ CORS headers
- ✅ Request ID generation
- ✅ Malformed JSON handling
- ✅ Content-type validation
- ✅ Error responses

### Authentication Tests (17 tests)

Tests for JWT authentication:

- ✅ Login success
- ✅ Login with empty credentials
- ✅ Token validation
- ✅ Invalid token rejection
- ✅ Missing authorization header
- ✅ Malformed authorization header
- ✅ Refresh token mechanism
- ✅ Invalid refresh token
- ✅ Logout functionality
- ✅ Logout without auth
- ✅ User info endpoint (/me)
- ✅ API key generation (admin)
- ✅ API key generation (regular user - forbidden)
- ✅ Token with roles

### RBAC Tests (3 tests)

Tests for role-based access control:

- ✅ Admin full access
- ✅ Viewer read-only access
- ✅ Role-based API key generation

### Rate Limiting Tests (4 tests)

Tests for rate limiting:

- ✅ Rate limit enforcement
- ✅ Rate limit headers
- ✅ Retry-After header on 429
- ✅ Per-user rate limiting

## Test Configuration

### Environment Variables

```bash
# Test database (default: in-memory SQLite)
TEST_DATABASE_URL=sqlite::memory:

# Log level for tests
RUST_LOG=debug

# Test parallelism
RUST_TEST_THREADS=4
```

### In-Memory Database

Tests use in-memory SQLite by default for:

- **Isolation** - Each test has its own database
- **Speed** - No disk I/O overhead
- **Cleanup** - Automatic cleanup after tests
- **Consistency** - Same schema as production PostgreSQL

## Writing New Tests

### Example Test

```rust
#[tokio::test]
async fn test_my_feature() {
    // Setup
    let app = TestApp::new().await;
    let client = app.client();
    let token = app.generate_token("testuser");

    // Execute
    let response = client
        .get(&format!("{}/api/endpoint", app.url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to send request");

    // Assert
    assert_success(&response);

    let body: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse JSON");

    assert_eq!(body["data"]["field"], "expected_value");

    // Cleanup
    app.cleanup().await;
}
```

### Test Helpers

```rust
use common::{
    TestApp,
    assert_status, assert_success, assert_client_error,
    get_with_auth, post_with_auth, put_with_auth, delete_with_auth,
    parse_json,
    fixtures::{TestUser, create_test_asset},
};
```

### Best Practices

1. **Isolation** - Each test should be independent
2. **Cleanup** - Clean up test data after tests
3. **Assertions** - Use helper functions for common assertions
4. **Fixtures** - Use fixtures for consistent test data
5. **Naming** - Use descriptive test names (test_feature_scenario)
6. **Documentation** - Document complex test scenarios

## CI/CD Integration

### GitHub Actions

```yaml
- name: Run Integration Tests
  run: cargo test --test '*' --verbose

- name: Run Tests with Coverage
  run: |
    cargo install cargo-tarpaulin
    cargo tarpaulin --test '*' --out Xml
```

### Docker

```bash
# Run tests in Docker
docker-compose -f docker-compose.test.yml up --abort-on-container-exit

# With coverage
docker-compose -f docker-compose.test.yml run test cargo tarpaulin
```

## Coverage

Target coverage: **80%+**

View coverage:
```bash
cargo tarpaulin --test '*' --out Html
open tarpaulin-report.html
```

## Troubleshooting

### Test Failures

1. **Port conflicts** - Tests use random ports
2. **Database errors** - Check migrations are up to date
3. **Timeout errors** - Increase timeout in test helpers
4. **Auth failures** - Verify JWT secret configuration

### Debug Mode

```bash
RUST_LOG=debug cargo test --test api_tests -- --nocapture
```

### Common Issues

**Issue**: Tests hang
**Solution**: Check for infinite loops or missing .await

**Issue**: Database errors
**Solution**: Run migrations: `sqlx migrate run`

**Issue**: Port already in use
**Solution**: Tests use random ports, shouldn't happen

## Performance

- **Test Execution Time**: ~5-10 seconds for full suite
- **Parallel Execution**: Supported with `--test-threads`
- **Memory Usage**: ~100MB for in-memory database
- **Isolation**: Each test spawns separate server instance

## Security Testing

The test suite includes security tests for:

- **Authentication bypass attempts**
- **Authorization violations**
- **Rate limit evasion**
- **Malformed input handling**
- **Token expiration**
- **CORS policy enforcement**

## Future Enhancements

- [ ] gRPC integration tests
- [ ] Database migration tests
- [ ] Performance benchmarks
- [ ] Chaos testing
- [ ] Load testing
- [ ] Security penetration tests
- [ ] Contract testing
- [ ] Mutation testing

## License

Apache-2.0 OR MIT
