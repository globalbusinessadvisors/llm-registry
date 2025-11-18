# LLM Registry - Architecture Documentation

## Table of Contents

- [System Overview](#system-overview)
- [Architecture Principles](#architecture-principles)
- [Layered Architecture](#layered-architecture)
- [Component Details](#component-details)
- [Data Flow](#data-flow)
- [Technology Stack](#technology-stack)
- [Deployment Architecture](#deployment-architecture)
- [Security Architecture](#security-architecture)
- [Scalability & Performance](#scalability--performance)
- [Observability](#observability)

---

## System Overview

The LLM Registry is an enterprise-grade asset management system designed for storing, versioning, and managing Large Language Model (LLM) assets, pipelines, datasets, policies, and test suites.

### Key Characteristics

- **Multi-layered**: Clear separation of concerns across presentation, business logic, and data layers
- **Event-driven**: Async event processing for audit trails and integrations
- **Highly scalable**: Horizontal scaling with stateless design
- **Observable**: Comprehensive metrics, logging, and distributed tracing
- **Secure**: JWT authentication, RBAC, rate limiting, and encryption

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Clients                               │
│  (CLI, Web UI, SDKs, CI/CD Pipelines, External Systems)    │
└───────────┬─────────────────────────────────────────────────┘
            │
            ▼
┌───────────────────────────────────────────────────────────────┐
│                   API Gateway / Load Balancer                  │
│              (Nginx, AWS ALB, Kubernetes Ingress)             │
└───────────┬───────────────────────────────────────────────────┘
            │
     ┌──────┴──────┐
     │             │
     ▼             ▼
┌─────────┐   ┌─────────┐
│ REST API│   │ gRPC API│
│ :8080   │   │ :50051  │
└────┬────┘   └────┬────┘
     │             │
     └──────┬──────┘
            │
            ▼
┌───────────────────────────────────────────────────────────────┐
│                    Application Layer                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ Auth/RBAC    │  │ Rate Limiting│  │  Middleware  │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└───────────┬───────────────────────────────────────────────────┘
            │
            ▼
┌───────────────────────────────────────────────────────────────┐
│                     Service Layer                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ Registration │  │ Validation   │  │ Dependency   │       │
│  │   Service    │  │   Service    │  │   Service    │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │  Versioning  │  │   Integrity  │  │   Policy     │       │
│  │   Service    │  │   Service    │  │   Service    │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└───────────┬───────────────────────────────────────────────────┘
            │
            ▼
┌───────────────────────────────────────────────────────────────┐
│                       Data Layer                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ PostgreSQL   │  │  Redis Cache │  │ Event Store  │       │
│  │  (Primary)   │  │  (Metadata)  │  │    (NATS)    │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└───────────────────────────────────────────────────────────────┘
```

---

## Architecture Principles

### 1. Separation of Concerns
Each layer has a well-defined responsibility:
- **API Layer**: Request/response handling, protocol translation
- **Service Layer**: Business logic, orchestration
- **Data Layer**: Persistence, caching, messaging

### 2. Dependency Inversion
- High-level modules don't depend on low-level modules
- Both depend on abstractions (traits)
- Enables testability and flexibility

### 3. Asynchronous by Default
- Non-blocking I/O throughout
- Tokio async runtime
- Efficient resource utilization

### 4. Fail-Fast Philosophy
- Input validation at boundaries
- Type-safe error handling with Result<T, E>
- Explicit error types

### 5. Observability First
- Structured logging from the start
- Metrics for all operations
- Distributed tracing for request flows

---

## Layered Architecture

### 1. Presentation Layer

**Crate**: `llm-registry-api`

Responsible for:
- HTTP/REST API endpoints
- gRPC service implementations
- GraphQL schema (future)
- Request validation
- Response formatting
- API documentation (OpenAPI/Swagger)

**Key Components**:
```
llm-registry-api/
├── src/
│   ├── lib.rs                # API module exports
│   ├── routes.rs             # Route definitions
│   ├── handlers/             # Request handlers
│   │   ├── assets.rs         # Asset endpoints
│   │   ├── auth.rs           # Auth endpoints
│   │   └── health.rs         # Health checks
│   ├── middleware/           # HTTP middleware
│   │   ├── auth.rs           # JWT middleware
│   │   ├── rbac.rs           # Permission checks
│   │   └── rate_limit.rs     # Rate limiting
│   ├── grpc/                 # gRPC services
│   │   └── proto/            # Protobuf definitions
│   └── graphql/              # GraphQL (future)
```

---

### 2. Business Logic Layer

**Crate**: `llm-registry-service`

Responsible for:
- Domain logic implementation
- Service orchestration
- Business rule enforcement
- Transaction management

**Key Services**:

#### Registration Service
- Asset registration workflow
- Validation orchestration
- Metadata construction
- Event publishing

#### Validation Service
- Schema validation
- Policy enforcement
- Constraint checking
- Custom validators

#### Dependency Service
- Dependency graph management
- Circular dependency detection
- Dependency resolution
- Impact analysis

#### Versioning Service
- Semantic versioning rules
- Version compatibility checks
- Deprecation management
- Version lifecycle

#### Integrity Service
- Checksum verification
- Signature validation
- Provenance tracking
- Attestation

**Service Layer Structure**:
```
llm-registry-service/
├── src/
│   ├── lib.rs                # Service registry
│   ├── registration.rs       # Registration service
│   ├── validation.rs         # Validation service
│   ├── dependency.rs         # Dependency service
│   ├── versioning.rs         # Versioning service
│   ├── integrity.rs          # Integrity service
│   ├── policy.rs             # Policy service
│   └── error.rs              # Service errors
```

---

### 3. Data Access Layer

**Crate**: `llm-registry-db`

Responsible for:
- Database operations
- Caching logic
- Event publishing
- Query optimization

**Key Repositories**:

#### Asset Repository
```rust
#[async_trait]
pub trait AssetRepository {
    async fn create(&self, asset: &Asset) -> DbResult<()>;
    async fn find_by_id(&self, id: &AssetId) -> DbResult<Option<Asset>>;
    async fn find_by_name_version(&self, name: &str, version: &Version)
        -> DbResult<Option<Asset>>;
    async fn update(&self, asset: &Asset) -> DbResult<()>;
    async fn delete(&self, id: &AssetId) -> DbResult<()>;
    async fn list(&self, filter: AssetFilter) -> DbResult<Vec<Asset>>;
}
```

#### Event Store
```rust
#[async_trait]
pub trait EventStore {
    async fn append(&self, event: &Event) -> DbResult<()>;
    async fn get_by_asset_id(&self, asset_id: &AssetId)
        -> DbResult<Vec<Event>>;
    async fn subscribe(&self, handler: EventHandler) -> DbResult<()>;
}
```

**Data Layer Structure**:
```
llm-registry-db/
├── src/
│   ├── lib.rs                # Database config
│   ├── repositories/         # Repository implementations
│   │   ├── asset.rs          # Asset repository
│   │   ├── dependency.rs     # Dependency repository
│   │   └── event.rs          # Event repository
│   ├── cache.rs              # Redis cache layer
│   ├── migrations/           # SQL migrations
│   ├── models.rs             # Database models
│   └── schema.sql            # Database schema
```

---

### 4. Core Domain Layer

**Crate**: `llm-registry-core`

Responsible for:
- Domain types and entities
- Business rules
- Value objects
- Domain errors

**Key Entities**:

#### Asset
```rust
pub struct Asset {
    pub id: AssetId,
    pub asset_type: AssetType,
    pub metadata: AssetMetadata,
    pub status: AssetStatus,
    pub storage: StorageLocation,
    pub checksum: Checksum,
    pub provenance: Option<Provenance>,
    pub dependencies: Vec<AssetReference>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

#### AssetMetadata
```rust
pub struct AssetMetadata {
    pub name: String,
    pub version: Version,
    pub description: Option<String>,
    pub license: Option<String>,
    pub tags: Tags,
    pub annotations: Annotations,
    pub size_bytes: Option<u64>,
    pub content_type: Option<String>,
}
```

**Core Domain Structure**:
```
llm-registry-core/
├── src/
│   ├── lib.rs                # Core exports
│   ├── asset.rs              # Asset entity
│   ├── types.rs              # Value objects
│   ├── checksum.rs           # Integrity types
│   ├── dependency.rs         # Dependency types
│   ├── provenance.rs         # Provenance types
│   ├── storage.rs            # Storage abstractions
│   └── error.rs              # Domain errors
```

---

## Component Details

### Authentication & Authorization

#### JWT Authentication Flow

```
┌────────┐                                    ┌────────┐
│ Client │                                    │ Server │
└───┬────┘                                    └───┬────┘
    │                                             │
    │  1. POST /auth/login                        │
    │  {username, password}                       │
    │────────────────────────────────────────────>│
    │                                             │
    │                                2. Verify    │
    │                             credentials     │
    │                                  ┌──────────┤
    │                                  │          │
    │                                  └─────────>│
    │                                             │
    │  3. {access_token, refresh_token}           │
    │<────────────────────────────────────────────│
    │                                             │
    │  4. GET /assets                             │
    │  Authorization: Bearer {access_token}       │
    │────────────────────────────────────────────>│
    │                                             │
    │                          5. Validate token  │
    │                                  ┌──────────┤
    │                                  │          │
    │                                  └─────────>│
    │                                             │
    │  6. {assets}                                │
    │<────────────────────────────────────────────│
```

#### RBAC Model

**Roles** (hierarchical):
```
admin
  ├── developer
  │     └── user
  │           └── viewer
```

**Permissions** (resource:action format):
```
assets:create
assets:read
assets:update
assets:delete
policies:manage
users:manage
```

**Role Definitions**:
```rust
Admin:
  - *:*  (all permissions)

Developer:
  - assets:*
  - policies:read
  - api-keys:create

User:
  - assets:create
  - assets:read
  - assets:update (own)

Viewer:
  - assets:read
```

---

### Rate Limiting

#### Token Bucket Algorithm

```
┌─────────────────────────────────────────────────────────┐
│                    Token Bucket                          │
│                                                          │
│  Capacity: 100 tokens                                   │
│  Refill rate: 100 tokens / minute                       │
│  Current tokens: 75                                      │
│                                                          │
│  ┌────────────────────────────────────────────┐        │
│  │ [████████████████████████████████░░░░░░░░] │ 75/100  │
│  └────────────────────────────────────────────┘        │
│                                                          │
│  Refill: +1.67 tokens/second                            │
│                                                          │
└─────────────────────────────────────────────────────────┘

Request → Check bucket → Has tokens?
                            ├─ Yes: Allow & consume 1 token
                            └─ No: Return 429 Too Many Requests
```

**Implementation**:
```rust
pub struct RateLimiter {
    buckets: DashMap<String, TokenBucket>,
    config: RateLimitConfig,
}

impl RateLimiter {
    pub async fn check_rate_limit(
        &self,
        identifier: &str,
    ) -> Result<(), RateLimitError> {
        let mut bucket = self.buckets.entry(identifier.to_string())
            .or_insert_with(|| TokenBucket::new(self.config));

        bucket.refill();

        if bucket.consume(1) {
            Ok(())
        } else {
            Err(RateLimitError::Exceeded {
                retry_after: bucket.time_until_available(),
            })
        }
    }
}
```

---

### Caching Strategy

#### Multi-Level Caching

```
┌──────────────────────────────────────────────────────────┐
│                   Request Flow                            │
└──────────────────────────────────────────────────────────┘

1. In-Memory Cache (moka)                → Hit (fastest)
   ├─ Asset metadata
   └─ User permissions

2. Redis Cache (distributed)             → Hit (fast)
   ├─ Asset lookup by name/version
   ├─ Dependency graphs
   └─ Policy evaluations

3. PostgreSQL (primary source)           → Miss (authoritative)
   └─ All data
```

**Cache Invalidation**:
- **Write-through**: Update cache on write
- **TTL-based**: Expire after configured time
- **Event-based**: Invalidate on entity changes
- **Pattern-based**: Invalidate related keys

```rust
pub async fn invalidate_asset_cache(&self, asset_id: &AssetId) {
    // Invalidate direct lookup
    self.cache.delete(&format!("asset:{}", asset_id)).await;

    // Invalidate name-version lookup
    let asset = self.get_asset(asset_id).await?;
    self.cache.delete(&format!(
        "asset:{}:{}",
        asset.metadata.name,
        asset.metadata.version
    )).await;

    // Invalidate dependency cache
    self.cache.delete_pattern(&format!("deps:*:{}*", asset_id)).await;
}
```

---

## Data Flow

### Asset Registration Flow

```
Client → API → Service Layer → Data Layer → Event Stream

1. Client sends POST /assets request
   ↓
2. API validates request format
   ↓
3. Authentication middleware verifies JWT
   ↓
4. RBAC middleware checks permissions
   ↓
5. Rate limiter checks request quota
   ↓
6. Registration Service receives request
   ├─→ 7. Validation Service validates schema
   ├─→ 8. Dependency Service resolves dependencies
   ├─→ 9. Integrity Service verifies checksum
   └─→ 10. Policy Service enforces policies
       ↓
11. Asset Repository persists to PostgreSQL
    ↓
12. Event Store publishes AssetRegistered event to NATS
    ↓
13. Cache layer updates Redis
    ↓
14. Response returned to client
```

### Event Processing Flow

```
Asset Change → Event Store → NATS → Event Handlers

1. Asset registered/updated/deleted
   ↓
2. Event created with metadata:
   - event_id (ULID)
   - event_type (asset_registered)
   - asset_id
   - actor (user_id)
   - timestamp
   - payload
   ↓
3. Event stored in PostgreSQL events table
   ↓
4. Event published to NATS topic:
   Subject: assets.registered.{asset_type}
   ↓
5. Subscribers receive event:
   ├─→ Audit log consumer
   ├─→ Webhook dispatcher
   ├─→ Search indexer
   ├─→ Analytics collector
   └─→ Cache invalidator
```

---

## Technology Stack

### Backend

| Component | Technology | Version | Purpose |
|-----------|-----------|---------|---------|
| Language | Rust | 1.75+ | System programming language |
| Web Framework | Axum | 0.7 | HTTP server and routing |
| Async Runtime | Tokio | 1.35 | Async task execution |
| Database | PostgreSQL | 15+ | Primary data store |
| Cache | Redis | 7+ | Distributed caching |
| Messaging | NATS | 2.10+ | Event streaming |
| ORM | SQLx | 0.7 | Database toolkit |
| Serialization | Serde | 1.0 | JSON/data serialization |

### Observability

| Component | Technology | Purpose |
|-----------|-----------|---------|
| Metrics | Prometheus | Time-series metrics |
| Visualization | Grafana | Dashboards and alerts |
| Tracing | Jaeger | Distributed tracing |
| Logging | tracing | Structured logging |
| OpenTelemetry | opentelemetry-rust | Unified observability |

---

## Deployment Architecture

### Kubernetes Deployment

```
┌────────────────────────────────────────────────────────────┐
│                     Kubernetes Cluster                       │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Ingress Controller                       │  │
│  │        (cert-manager for TLS termination)            │  │
│  └─────────────────┬────────────────────────────────────┘  │
│                    │                                        │
│  ┌─────────────────┴────────────────────────────────────┐  │
│  │         API Service (ClusterIP)                      │  │
│  │    LoadBalancer across multiple pods                 │  │
│  └─────────────────┬────────────────────────────────────┘  │
│                    │                                        │
│  ┌─────────────────┴────────────────────────────────────┐  │
│  │           API Deployment (3 replicas)                │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │  │
│  │  │ Pod 1    │  │ Pod 2    │  │ Pod 3    │          │  │
│  │  │ API:8080 │  │ API:8080 │  │ API:8080 │          │  │
│  │  │gRPC:50051│  │gRPC:50051│  │gRPC:50051│          │  │
│  │  └──────────┘  └──────────┘  └──────────┘          │  │
│  └──────────────────────────────────────────────────────┘  │
│                    │                                        │
│  ┌─────────────────┼────────────────────────────────────┐  │
│  │                 │                                     │  │
│  │  ┌──────────────┴──────┐  ┌───────────────────────┐ │  │
│  │  │ PostgreSQL StatefulSet│  │ Redis StatefulSet     │ │  │
│  │  │  (Primary + Replica)  │  │ (Master + Replicas)   │ │  │
│  │  └───────────────────────┘  └───────────────────────┘ │  │
│  │                                                        │  │
│  │  ┌─────────────────────────────────────────────────┐ │  │
│  │  │           NATS StatefulSet                       │ │  │
│  │  │        (Clustered mode for HA)                  │ │  │
│  │  └─────────────────────────────────────────────────┘ │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │        Observability Stack                           │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │  │
│  │  │Prometheus│  │ Grafana  │  │  Jaeger  │          │  │
│  │  └──────────┘  └──────────┘  └──────────┘          │  │
│  └──────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────┘
```

### Resource Requirements

**API Pods**:
```yaml
resources:
  requests:
    cpu: 500m
    memory: 512Mi
  limits:
    cpu: 2000m
    memory: 2Gi
```

**PostgreSQL**:
```yaml
resources:
  requests:
    cpu: 1000m
    memory: 2Gi
  limits:
    cpu: 4000m
    memory: 8Gi
storage:
  size: 100Gi
  class: fast-ssd
```

**Redis**:
```yaml
resources:
  requests:
    cpu: 250m
    memory: 256Mi
  limits:
    cpu: 1000m
    memory: 1Gi
```

---

## Security Architecture

### Defense in Depth

```
┌──────────────────────────────────────────────────────┐
│ Layer 1: Network Security                            │
│  - TLS/HTTPS encryption                              │
│  - Network policies                                  │
│  - Firewall rules                                    │
└──────────────────────────────────────────────────────┘
                       ▼
┌──────────────────────────────────────────────────────┐
│ Layer 2: API Gateway                                 │
│  - Rate limiting                                     │
│  - Request validation                                │
│  - DDoS protection                                   │
└──────────────────────────────────────────────────────┘
                       ▼
┌──────────────────────────────────────────────────────┐
│ Layer 3: Authentication                              │
│  - JWT token validation                              │
│  - Multi-factor authentication                       │
│  - API key management                                │
└──────────────────────────────────────────────────────┘
                       ▼
┌──────────────────────────────────────────────────────┐
│ Layer 4: Authorization                               │
│  - RBAC permission checks                            │
│  - Resource-level access control                     │
│  - Audit logging                                     │
└──────────────────────────────────────────────────────┘
                       ▼
┌──────────────────────────────────────────────────────┐
│ Layer 5: Data Security                               │
│  - Encryption at rest                                │
│  - Encrypted backups                                 │
│  - Secure credential storage                         │
└──────────────────────────────────────────────────────┘
```

---

## Scalability & Performance

### Horizontal Scaling

**Stateless Design**:
- No server-side session state
- All state in PostgreSQL/Redis
- Scale by adding more pods/containers

**Auto-scaling Configuration**:
```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
spec:
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

### Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| API Latency (P50) | < 20ms | Cached requests |
| API Latency (P95) | < 50ms | Database queries |
| API Latency (P99) | < 100ms | Complex operations |
| Throughput | 2000 req/s | Per instance |
| Database Connections | 20/instance | Connection pooling |
| Cache Hit Rate | > 90% | For metadata |

---

## Observability

### Metrics Collection

**Application Metrics**:
- Request rate, latency, errors (RED method)
- Database query performance
- Cache hit/miss rates
- Event processing metrics

**Infrastructure Metrics**:
- CPU, memory, disk usage
- Network I/O
- Pod restart counts
- Resource quotas

**Business Metrics**:
- Asset registration rate
- Active users
- API usage by endpoint
- Storage utilization

### Distributed Tracing

**Trace Context Propagation**:
```
Client Request
  └─> HTTP Handler (span)
      └─> Auth Middleware (span)
          └─> RBAC Middleware (span)
              └─> Service Layer (span)
                  ├─> Database Query (span)
                  ├─> Cache Lookup (span)
                  └─> Event Publishing (span)
```

### Logging Strategy

**Log Levels**:
- **ERROR**: System errors requiring immediate attention
- **WARN**: Warnings about potential issues
- **INFO**: Important business events
- **DEBUG**: Detailed diagnostic information
- **TRACE**: Very detailed execution traces

**Structured Logging Format**:
```json
{
  "timestamp": "2025-01-18T10:30:00.123Z",
  "level": "INFO",
  "message": "Asset registered successfully",
  "request_id": "req_abc123",
  "user_id": "user_xyz789",
  "asset_id": "01HN9XWZP8XQYZVJ4KFQY6XQZV",
  "duration_ms": 45,
  "span_id": "span_def456",
  "trace_id": "trace_ghi789"
}
```

---

## Conclusion

The LLM Registry architecture is designed for:
- **Enterprise scalability**: Horizontal scaling with stateless services
- **High availability**: Multi-replica deployments with health checks
- **Security**: Defense-in-depth with authentication, authorization, and encryption
- **Observability**: Comprehensive metrics, logging, and tracing
- **Performance**: Sub-100ms P99 latency with caching and optimization
- **Maintainability**: Clean architecture with separation of concerns

For implementation details, see:
- [API Reference](API_REFERENCE.md)
- [Security Guide](SECURITY.md)
- [Deployment Guide](../DOCKER_README.md)
- [Contributing Guidelines](CONTRIBUTING.md)

---

**Last Updated**: 2025-01-18
**Version**: 1.0.0
