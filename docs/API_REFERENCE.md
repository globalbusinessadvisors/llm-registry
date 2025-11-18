# LLM Registry API Reference

## Overview

The LLM Registry provides a RESTful HTTP API and a gRPC API for managing LLM assets, pipelines, datasets, policies, and test suites.

**Base URL (HTTP)**: `http://localhost:8080/v1`
**Base URL (gRPC)**: `localhost:50051`

**API Version**: v1.0
**Last Updated**: 2025-01-18

## Table of Contents

- [Authentication](#authentication)
- [Asset Management](#asset-management)
- [Dependency Management](#dependency-management)
- [Version Management](#version-management)
- [Health & Monitoring](#health--monitoring)
- [Error Handling](#error-handling)
- [Rate Limiting](#rate-limiting)
- [Pagination](#pagination)

---

## Authentication

### Overview

The API uses JWT (JSON Web Tokens) for authentication. All protected endpoints require a valid JWT token in the `Authorization` header.

### Endpoints

#### POST /auth/login

Authenticate and receive JWT tokens.

**Request:**
```json
{
  "username": "string",
  "password": "string"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "token_type": "Bearer",
    "expires_in": 3600,
    "user": {
      "id": "01HN9XWZP8XQYZVJ4KFQY6XQZV",
      "username": "admin",
      "email": "admin@example.com",
      "roles": ["admin"]
    }
  }
}
```

**Status Codes:**
- `200 OK` - Successfully authenticated
- `400 Bad Request` - Invalid request format
- `401 Unauthorized` - Invalid credentials

---

#### POST /auth/refresh

Refresh an access token using a refresh token.

**Request:**
```json
{
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "token_type": "Bearer",
    "expires_in": 3600
  }
}
```

**Status Codes:**
- `200 OK` - Token refreshed successfully
- `401 Unauthorized` - Invalid or expired refresh token

---

#### POST /auth/logout

Invalidate current session.

**Headers:**
```
Authorization: Bearer {access_token}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "message": "Logged out successfully"
  }
}
```

**Status Codes:**
- `200 OK` - Logged out successfully
- `401 Unauthorized` - Not authenticated

---

#### GET /auth/me

Get current authenticated user information.

**Headers:**
```
Authorization: Bearer {access_token}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "01HN9XWZP8XQYZVJ4KFQY6XQZV",
    "username": "admin",
    "email": "admin@example.com",
    "roles": ["admin"],
    "created_at": "2025-01-01T00:00:00Z",
    "last_login": "2025-01-18T10:30:00Z"
  }
}
```

**Status Codes:**
- `200 OK` - User information retrieved
- `401 Unauthorized` - Not authenticated

---

#### POST /auth/api-key

Generate an API key for programmatic access (requires developer or admin role).

**Headers:**
```
Authorization: Bearer {access_token}
```

**Request:**
```json
{
  "name": "CI/CD Pipeline Key",
  "expires_in_days": 90
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "api_key": "llmreg_1234567890abcdef",
    "name": "CI/CD Pipeline Key",
    "created_at": "2025-01-18T10:30:00Z",
    "expires_at": "2025-04-18T10:30:00Z"
  }
}
```

**Status Codes:**
- `200 OK` - API key created
- `401 Unauthorized` - Not authenticated
- `403 Forbidden` - Insufficient permissions

---

## Asset Management

### Endpoints

#### POST /assets

Register a new asset in the registry.

**Headers:**
```
Authorization: Bearer {access_token}
Content-Type: application/json
```

**Request:**
```json
{
  "asset_type": "model",
  "name": "gpt-custom-v1",
  "version": "1.0.0",
  "description": "Custom GPT model fine-tuned for domain",
  "license": "MIT",
  "tags": ["nlp", "transformer", "gpt"],
  "annotations": {
    "framework": "pytorch",
    "task": "text-generation",
    "architecture": "transformer"
  },
  "storage": {
    "backend": "s3",
    "bucket": "llm-models",
    "region": "us-east-1",
    "path": "models/gpt-custom-v1.bin"
  },
  "checksum": {
    "algorithm": "SHA256",
    "value": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  },
  "provenance": {
    "source_repo": "https://github.com/org/models",
    "commit_hash": "abc123def456",
    "author": "ML Team",
    "build_id": "build-123"
  },
  "dependencies": [
    {
      "name": "tokenizer-v1",
      "version": "1.0.0"
    }
  ],
  "size_bytes": 1073741824,
  "content_type": "application/octet-stream"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "01HN9XWZP8XQYZVJ4KFQY6XQZV",
    "asset_type": "model",
    "metadata": {
      "name": "gpt-custom-v1",
      "version": "1.0.0",
      "description": "Custom GPT model fine-tuned for domain",
      "license": "MIT",
      "tags": ["nlp", "transformer", "gpt"],
      "annotations": {
        "framework": "pytorch",
        "task": "text-generation",
        "architecture": "transformer"
      },
      "size_bytes": 1073741824,
      "content_type": "application/octet-stream"
    },
    "status": "active",
    "storage": {
      "backend": "s3",
      "bucket": "llm-models",
      "region": "us-east-1",
      "path": "models/gpt-custom-v1.bin"
    },
    "checksum": {
      "algorithm": "SHA256",
      "value": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    },
    "provenance": {
      "source_repo": "https://github.com/org/models",
      "commit_hash": "abc123def456",
      "author": "ML Team",
      "build_id": "build-123",
      "created_at": "2025-01-18T10:30:00Z"
    },
    "dependencies": ["01HN9XWZP8XQYZVJ4KFQY6XQZY"],
    "created_at": "2025-01-18T10:30:00Z",
    "updated_at": "2025-01-18T10:30:00Z"
  }
}
```

**Status Codes:**
- `201 Created` - Asset registered successfully
- `400 Bad Request` - Invalid request format or validation error
- `401 Unauthorized` - Not authenticated
- `403 Forbidden` - Insufficient permissions
- `409 Conflict` - Asset with same name and version already exists

---

#### GET /assets

List assets with filtering and pagination.

**Headers:**
```
Authorization: Bearer {access_token}
```

**Query Parameters:**
- `type` (string, optional) - Filter by asset type: `model`, `pipeline`, `dataset`, `policy`, `test_suite`
- `name` (string, optional) - Filter by asset name (partial match)
- `tag` (string, optional) - Filter by tag
- `status` (string, optional) - Filter by status: `active`, `deprecated`, `archived`
- `page` (integer, optional, default: 1) - Page number
- `per_page` (integer, optional, default: 20, max: 100) - Items per page
- `sort` (string, optional, default: `created_at`) - Sort field: `name`, `version`, `created_at`, `updated_at`
- `order` (string, optional, default: `desc`) - Sort order: `asc`, `desc`

**Example:**
```
GET /assets?type=model&tag=nlp&page=1&per_page=20&sort=created_at&order=desc
```

**Response:**
```json
{
  "success": true,
  "data": {
    "items": [
      {
        "id": "01HN9XWZP8XQYZVJ4KFQY6XQZV",
        "asset_type": "model",
        "metadata": {
          "name": "gpt-custom-v1",
          "version": "1.0.0",
          "description": "Custom GPT model",
          "tags": ["nlp", "transformer"]
        },
        "status": "active",
        "created_at": "2025-01-18T10:30:00Z",
        "updated_at": "2025-01-18T10:30:00Z"
      }
    ],
    "pagination": {
      "page": 1,
      "per_page": 20,
      "total_items": 150,
      "total_pages": 8
    }
  }
}
```

**Status Codes:**
- `200 OK` - Assets retrieved successfully
- `400 Bad Request` - Invalid query parameters
- `401 Unauthorized` - Not authenticated

---

#### GET /assets/{id}

Get detailed information about a specific asset.

**Headers:**
```
Authorization: Bearer {access_token}
```

**Path Parameters:**
- `id` (string, required) - Asset ID (ULID)

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "01HN9XWZP8XQYZVJ4KFQY6XQZV",
    "asset_type": "model",
    "metadata": {
      "name": "gpt-custom-v1",
      "version": "1.0.0",
      "description": "Custom GPT model fine-tuned for domain",
      "license": "MIT",
      "tags": ["nlp", "transformer", "gpt"],
      "annotations": {
        "framework": "pytorch",
        "task": "text-generation"
      },
      "size_bytes": 1073741824,
      "content_type": "application/octet-stream"
    },
    "status": "active",
    "storage": {
      "backend": "s3",
      "bucket": "llm-models",
      "path": "models/gpt-custom-v1.bin"
    },
    "checksum": {
      "algorithm": "SHA256",
      "value": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    },
    "provenance": {
      "source_repo": "https://github.com/org/models",
      "commit_hash": "abc123def456",
      "author": "ML Team",
      "created_at": "2025-01-18T10:30:00Z"
    },
    "dependencies": ["01HN9XWZP8XQYZVJ4KFQY6XQZY"],
    "created_at": "2025-01-18T10:30:00Z",
    "updated_at": "2025-01-18T10:30:00Z"
  }
}
```

**Status Codes:**
- `200 OK` - Asset found
- `401 Unauthorized` - Not authenticated
- `404 Not Found` - Asset not found

---

#### PATCH /assets/{id}

Update asset metadata (only metadata fields can be updated).

**Headers:**
```
Authorization: Bearer {access_token}
Content-Type: application/json
```

**Path Parameters:**
- `id` (string, required) - Asset ID

**Request:**
```json
{
  "description": "Updated description",
  "tags": ["nlp", "transformer", "production"],
  "annotations": {
    "environment": "production",
    "performance": "optimized"
  }
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "01HN9XWZP8XQYZVJ4KFQY6XQZV",
    "metadata": {
      "description": "Updated description",
      "tags": ["nlp", "transformer", "production"],
      "annotations": {
        "framework": "pytorch",
        "environment": "production",
        "performance": "optimized"
      }
    },
    "updated_at": "2025-01-18T11:00:00Z"
  }
}
```

**Status Codes:**
- `200 OK` - Asset updated successfully
- `400 Bad Request` - Invalid request format
- `401 Unauthorized` - Not authenticated
- `403 Forbidden` - Insufficient permissions
- `404 Not Found` - Asset not found

---

#### DELETE /assets/{id}

Delete an asset (soft delete - marks as archived).

**Headers:**
```
Authorization: Bearer {access_token}
```

**Path Parameters:**
- `id` (string, required) - Asset ID

**Response:**
```json
{
  "success": true,
  "data": {
    "message": "Asset archived successfully",
    "id": "01HN9XWZP8XQYZVJ4KFQY6XQZV"
  }
}
```

**Status Codes:**
- `200 OK` - Asset deleted successfully
- `401 Unauthorized` - Not authenticated
- `403 Forbidden` - Insufficient permissions
- `404 Not Found` - Asset not found
- `409 Conflict` - Asset has active dependencies

---

## Dependency Management

#### GET /assets/{id}/dependencies

Get the dependency graph for an asset.

**Headers:**
```
Authorization: Bearer {access_token}
```

**Query Parameters:**
- `depth` (integer, optional, default: 1, max: 10) - Depth of dependency tree

**Response:**
```json
{
  "success": true,
  "data": {
    "asset_id": "01HN9XWZP8XQYZVJ4KFQY6XQZV",
    "dependencies": [
      {
        "id": "01HN9XWZP8XQYZVJ4KFQY6XQZY",
        "name": "tokenizer-v1",
        "version": "1.0.0",
        "asset_type": "model",
        "depth": 1
      }
    ],
    "total_dependencies": 1
  }
}
```

**Status Codes:**
- `200 OK` - Dependencies retrieved
- `401 Unauthorized` - Not authenticated
- `404 Not Found` - Asset not found

---

#### GET /assets/{id}/dependents

Get assets that depend on this asset (reverse dependencies).

**Headers:**
```
Authorization: Bearer {access_token}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "asset_id": "01HN9XWZP8XQYZVJ4KFQY6XQZV",
    "dependents": [
      {
        "id": "01HN9XWZP8XQYZVJ4KFQY6XQZZ",
        "name": "pipeline-v1",
        "version": "2.0.0",
        "asset_type": "pipeline"
      }
    ],
    "total_dependents": 1
  }
}
```

**Status Codes:**
- `200 OK` - Dependents retrieved
- `401 Unauthorized` - Not authenticated
- `404 Not Found` - Asset not found

---

## Version Management

#### GET /assets/{name}/versions

List all versions of an asset by name.

**Headers:**
```
Authorization: Bearer {access_token}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "name": "gpt-custom",
    "versions": [
      {
        "version": "2.0.0",
        "id": "01HN9XWZP8XQYZVJ4KFQY6XQZZ",
        "status": "active",
        "created_at": "2025-01-18T10:30:00Z"
      },
      {
        "version": "1.0.0",
        "id": "01HN9XWZP8XQYZVJ4KFQY6XQZV",
        "status": "deprecated",
        "created_at": "2025-01-01T00:00:00Z",
        "deprecated_at": "2025-01-18T10:30:00Z"
      }
    ]
  }
}
```

**Status Codes:**
- `200 OK` - Versions retrieved
- `401 Unauthorized` - Not authenticated
- `404 Not Found` - No assets found with that name

---

## Health & Monitoring

#### GET /health

Health check endpoint for liveness probes.

**Response:**
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "components": {
    "database": "healthy",
    "cache": "healthy",
    "messaging": "healthy"
  },
  "uptime_seconds": 3600
}
```

**Status Codes:**
- `200 OK` - Service is healthy
- `503 Service Unavailable` - Service is unhealthy

---

#### GET /metrics

Prometheus metrics endpoint.

**Response:**
```
# HELP http_requests_total Total HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",path="/assets",status="200"} 1234

# HELP http_request_duration_seconds HTTP request duration
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{le="0.005"} 100
http_request_duration_seconds_bucket{le="0.01"} 200
...
```

**Status Codes:**
- `200 OK` - Metrics retrieved

---

#### GET /version

Get API version information.

**Response:**
```json
{
  "version": "1.0.0",
  "build": "abc123def456",
  "build_date": "2025-01-18T00:00:00Z",
  "api_version": "v1"
}
```

**Status Codes:**
- `200 OK` - Version information retrieved

---

## Error Handling

All error responses follow a consistent format:

```json
{
  "success": false,
  "error": {
    "code": "RESOURCE_NOT_FOUND",
    "message": "Asset not found",
    "details": {
      "asset_id": "01HN9XWZP8XQYZVJ4KFQY6XQZV"
    },
    "request_id": "req_1234567890"
  }
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `VALIDATION_ERROR` | 400 | Request validation failed |
| `UNAUTHORIZED` | 401 | Authentication required |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `RESOURCE_NOT_FOUND` | 404 | Resource not found |
| `CONFLICT` | 409 | Resource conflict (e.g., duplicate) |
| `RATE_LIMIT_EXCEEDED` | 429 | Rate limit exceeded |
| `INTERNAL_ERROR` | 500 | Internal server error |
| `SERVICE_UNAVAILABLE` | 503 | Service temporarily unavailable |

---

## Rate Limiting

The API implements rate limiting to prevent abuse.

### Default Limits

- **Authenticated requests**: 100 requests per minute
- **Unauthenticated requests**: 20 requests per minute

### Rate Limit Headers

All responses include rate limit information:

```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1705578600
```

### Rate Limit Exceeded Response

```json
{
  "success": false,
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Rate limit exceeded",
    "details": {
      "limit": 100,
      "reset_at": "2025-01-18T10:30:00Z"
    }
  }
}
```

**Headers:**
```
Retry-After: 30
```

---

## Pagination

List endpoints support pagination using the following query parameters:

- `page` (integer, default: 1) - Page number
- `per_page` (integer, default: 20, max: 100) - Items per page

### Pagination Response

```json
{
  "success": true,
  "data": {
    "items": [...],
    "pagination": {
      "page": 1,
      "per_page": 20,
      "total_items": 150,
      "total_pages": 8,
      "has_next": true,
      "has_previous": false
    }
  }
}
```

### Link Headers

Paginated responses include Link headers for navigation:

```
Link: </v1/assets?page=2&per_page=20>; rel="next",
      </v1/assets?page=8&per_page=20>; rel="last"
```

---

## Best Practices

### Authentication
- Store JWT tokens securely (not in localStorage)
- Implement token refresh logic before expiration
- Use API keys for server-to-server communication

### Error Handling
- Always check the `success` field in responses
- Log `request_id` for debugging
- Implement exponential backoff for retries

### Performance
- Use pagination for large result sets
- Implement client-side caching where appropriate
- Leverage ETags for conditional requests

### Security
- Always use HTTPS in production
- Validate all input data
- Follow the principle of least privilege for RBAC

---

## Support

For questions and issues:
- GitHub Issues: https://github.com/llm-devops/llm-registry/issues
- Documentation: https://llm-registry.dev/docs
- Email: support@llm-registry.dev

---

**Last Updated**: 2025-01-18
**API Version**: v1.0
