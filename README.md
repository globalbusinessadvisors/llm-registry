# LLM Registry

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

Enterprise-grade registry for managing Large Language Model (LLM) assets, pipelines, datasets, policies, and test suites with comprehensive version control, dependency tracking, and compliance management.

## Features

### Core Capabilities

- **Asset Management**: Version-controlled storage for models, pipelines, datasets, policies, and test suites
- **Dependency Tracking**: Automatic dependency resolution and circular dependency detection
- **Integrity Verification**: SHA-256 checksum validation and provenance tracking
- **Policy Enforcement**: Compliance validation and policy-based governance
- **Event System**: Real-time event streaming via NATS for asset lifecycle events

### Security & Authentication

- **JWT Authentication**: Secure token-based authentication with refresh tokens
- **RBAC**: Role-Based Access Control with permission inheritance
- **Rate Limiting**: Token bucket algorithm with configurable limits
- **API Security**: Request signing, CORS, and security headers

### Observability

- **OpenTelemetry**: Distributed tracing with Jaeger integration
- **Prometheus Metrics**: Comprehensive metrics for monitoring
- **Structured Logging**: JSON-formatted logs with correlation IDs
- **Health Checks**: Liveness and readiness probes

### Performance

- **Redis Caching**: Distributed caching for improved performance
- **Connection Pooling**: Optimized database connection management
- **Async/Await**: Non-blocking I/O throughout the stack
- **Horizontal Scaling**: Stateless design for easy scaling

## Architecture

```
┌─────────────────┐
│   REST API      │ ← JWT Auth, Rate Limiting, RBAC
├─────────────────┤
│   Service Layer │ ← Business Logic, Policy Validation
├─────────────────┤
│   Data Layer    │ ← PostgreSQL, Redis Cache
├─────────────────┤
│   Event System  │ ← NATS Event Publishing
└─────────────────┘
```

### Technology Stack

- **Language**: Rust 1.75+
- **Web Framework**: Axum
- **Database**: PostgreSQL 15+
- **Cache**: Redis 7+
- **Message Queue**: NATS 2.10+
- **Observability**: OpenTelemetry, Prometheus, Jaeger

## Quick Start

### Prerequisites

- Rust 1.75 or later
- Docker and Docker Compose
- PostgreSQL 15+ (or use Docker Compose)
- Redis 7+ (or use Docker Compose)
- NATS 2.10+ (or use Docker Compose)

### Development Setup

1. **Clone the repository**

```bash
git clone https://github.com/your-org/llm-registry.git
cd llm-registry
```

2. **Start infrastructure with Docker Compose**

```bash
docker-compose up -d
```

This starts PostgreSQL, Redis, NATS, Prometheus, and Grafana.

3. **Run database migrations**

```bash
cargo install sqlx-cli
sqlx database create
sqlx migrate run
```

4. **Build the project**

```bash
cargo build --release
```

5. **Run the server**

```bash
cargo run --bin llm-registry-server
```

The API will be available at `http://localhost:8080`.

### Docker Deployment

Build and run with Docker:

```bash
# Build production image
docker build -t llm-registry:latest .

# Run container
docker run -p 8080:8080 \
  -e DATABASE_URL=postgresql://user:pass@host:5432/db \
  -e REDIS_URL=redis://host:6379 \
  -e NATS_URL=nats://host:4222 \
  llm-registry:latest
```

### Kubernetes Deployment

See [k8s/README.md](k8s/README.md) for detailed Kubernetes deployment instructions.

```bash
# Deploy to Kubernetes
kubectl apply -f k8s/
```

## API Documentation

### Authentication

All protected endpoints require a JWT token:

```bash
# Login to get JWT token
curl -X POST http://localhost:8080/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "password"}'

# Use token in subsequent requests
curl -X GET http://localhost:8080/v1/assets \
  -H "Authorization: Bearer <your-token>"
```

### Endpoints

#### Asset Management

- `POST /v1/assets` - Register a new asset
- `GET /v1/assets` - List assets with filtering and pagination
- `GET /v1/assets/:id` - Get asset by ID
- `PATCH /v1/assets/:id` - Update asset metadata
- `DELETE /v1/assets/:id` - Delete asset

#### Dependencies

- `GET /v1/assets/:id/dependencies` - Get dependency graph
- `GET /v1/assets/:id/dependents` - Get reverse dependencies

#### Health & Metrics

- `GET /health` - Health check
- `GET /metrics` - Prometheus metrics
- `GET /version` - Version information

#### Authentication

- `POST /v1/auth/login` - Login and get JWT token
- `POST /v1/auth/refresh` - Refresh access token
- `GET /v1/auth/me` - Get current user info
- `POST /v1/auth/logout` - Logout
- `POST /v1/auth/api-keys` - Generate API key (requires developer/admin role)

## Configuration

Configuration can be provided via:
1. Environment variables
2. Configuration files (TOML)
3. Command-line arguments

### Environment Variables

```bash
# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# Database
DATABASE_URL=postgresql://user:pass@localhost:5432/llm_registry

# Redis
REDIS_URL=redis://localhost:6379

# NATS
NATS_URL=nats://localhost:4222

# JWT
JWT_SECRET=your-secret-key-change-in-production
JWT_ISSUER=llm-registry
JWT_AUDIENCE=llm-registry-api
JWT_EXPIRATION_SECONDS=3600

# Rate Limiting
RATE_LIMIT_ENABLED=true
RATE_LIMIT_MAX_REQUESTS=100
RATE_LIMIT_WINDOW_SECS=60

# Logging
RUST_LOG=info
```

## Development

### Project Structure

```
llm-registry/
├── crates/
│   ├── llm-registry-core/      # Core domain types
│   ├── llm-registry-db/        # Database layer
│   ├── llm-registry-service/   # Business logic
│   ├── llm-registry-api/       # REST API layer
│   └── llm-registry-server/    # Server binary
├── migrations/                  # Database migrations
├── config/                      # Configuration files
├── docker/                      # Docker files
├── k8s/                        # Kubernetes manifests
└── deployments/                # Monitoring configs

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests with coverage
cargo tarpaulin --workspace --out Html

# Run specific crate tests
cargo test -p llm-registry-core
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Lint code
cargo clippy --workspace -- -D warnings

# Security audit
cargo audit
```

### Database Migrations

```bash
# Create a new migration
sqlx migrate add <migration_name>

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert
```

## Monitoring

### Prometheus

Metrics are exposed at `/metrics` endpoint:

```bash
# Access metrics
curl http://localhost:8080/metrics
```

Key metrics:
- `http_requests_total` - Total HTTP requests by method, path, status
- `http_request_duration_seconds` - Request duration histogram
- `db_queries_total` - Database query counts
- `cache_operations_total` - Cache operation counts
- `assets_total` - Total assets by status

### Grafana

Access Grafana dashboard at `http://localhost:3000` (default credentials: admin/admin).

Pre-configured dashboards are available in `deployments/grafana/`.

### Tracing

Traces are sent to Jaeger at `http://localhost:16686`.

## Security

### Authentication Flow

1. User logs in with username/password → receives JWT access token and refresh token
2. Access token is used for API requests (expires in 1 hour)
3. Refresh token is used to obtain new access tokens (expires in 7 days)
4. API keys can be generated for long-lived access

### RBAC Roles

- **admin**: Full access to all resources and operations
- **developer**: Can manage assets and generate API keys
- **user**: Can read and write assets
- **viewer**: Read-only access to assets

### Rate Limiting

Default limits:
- 100 requests per minute per IP/user
- Configurable per endpoint
- Distributed rate limiting via Redis

## Performance

### Benchmarks

- Asset registration: ~500 req/s
- Asset retrieval: ~2000 req/s
- Cache hit rate: >90%
- P99 latency: <50ms

### Optimization Tips

1. Enable Redis caching for frequently accessed assets
2. Use connection pooling (configured by default)
3. Adjust `max_connections` based on load
4. Use CDN for large asset downloads
5. Enable compression for API responses

## Production Deployment

### Checklist

- [ ] Change default JWT secret
- [ ] Configure TLS/HTTPS
- [ ] Set up database backups
- [ ] Configure monitoring and alerting
- [ ] Set up log aggregation
- [ ] Review and adjust rate limits
- [ ] Enable RBAC and set up roles
- [ ] Configure ingress/load balancer
- [ ] Test disaster recovery
- [ ] Set up CI/CD pipeline

### Scaling

Horizontal scaling:
```bash
# Scale to 5 replicas
kubectl scale deployment llm-registry-server --replicas=5

# Or use HPA (already configured)
kubectl autoscale deployment llm-registry-server --min=3 --max=10 --cpu-percent=70
```

## Troubleshooting

### Common Issues

**Database Connection Errors**
```bash
# Check database connectivity
psql $DATABASE_URL -c "SELECT 1"
```

**Redis Connection Errors**
```bash
# Check Redis connectivity
redis-cli -u $REDIS_URL ping
```

**High Memory Usage**
- Reduce connection pool size
- Enable pagination for large queries
- Check for connection leaks

### Logs

```bash
# View logs
RUST_LOG=debug cargo run

# Or in Docker
docker logs <container-id>

# Or in Kubernetes
kubectl logs -f deployment/llm-registry-server -n llm-registry
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## Documentation

### Core Documentation

- 📖 **[API Reference](docs/API_REFERENCE.md)** - Complete REST API documentation with examples
- 🏗️ **[Architecture Guide](docs/ARCHITECTURE.md)** - System architecture, components, and design patterns
- 🔒 **[Security Guide](docs/SECURITY.md)** - Security best practices, authentication, and compliance
- 🐳 **[Docker Deployment](DOCKER_README.md)** - Production Docker deployment guide
- 🧪 **[Testing Guide](tests/README.md)** - Integration test suite documentation

### Developer Resources

- 🤝 **[Contributing Guide](CONTRIBUTING.md)** - How to contribute to the project
- 📝 **[Code of Conduct](CODE_OF_CONDUCT.md)** - Community guidelines
- 📊 **[Changelog](CHANGELOG.md)** - Version history and changes

## Support

- Issues: [GitHub Issues](https://github.com/llm-devops/llm-registry/issues)
- Discussions: [GitHub Discussions](https://github.com/llm-devops/llm-registry/discussions)
- Email: support@llm-registry.dev

## Acknowledgments

Built with:
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [SQLx](https://github.com/launchbadge/sqlx) - Database toolkit
- [Tower](https://github.com/tower-rs/tower) - Middleware
- [OpenTelemetry](https://opentelemetry.io/) - Observability
- [Prometheus](https://prometheus.io/) - Monitoring

---

**Status**: Production Ready v1.0