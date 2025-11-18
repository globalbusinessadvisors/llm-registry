# LLM Registry - Docker Deployment Guide

## Overview

This document provides comprehensive instructions for deploying the LLM Registry using Docker in production environments.

## Features

The Docker implementation includes:

✅ **Multi-stage builds** for minimal image size
✅ **Security hardening** with non-root users
✅ **gRPC and HTTP/REST APIs** support
✅ **Layer caching** optimization with cargo-chef
✅ **Health checks** for container orchestration
✅ **Multi-architecture** support (amd64/arm64)
✅ **Production-grade** reverse proxy with Nginx
✅ **Observability stack** (Prometheus, Grafana, Jaeger)
✅ **High availability** configuration
✅ **Resource limits** and reservations

## Quick Start

### Development

```bash
# Start all services for development
docker compose up -d

# View logs
docker compose logs -f api

# Stop services
docker compose down
```

### Production

```bash
# 1. Copy and configure environment file
cp .env.production .env.production.local
# Edit .env.production.local and replace all CHANGE_ME values

# 2. Build production image
./scripts/build.sh

# 3. Deploy to production
ENV_FILE=.env.production.local ./scripts/deploy.sh

# 4. Check status
./scripts/deploy.sh status
```

## Architecture

### Services

| Service | Port | Description |
|---------|------|-------------|
| **api** | 3000 | HTTP/REST API |
| **api** | 50051 | gRPC API |
| **postgres** | 5432 | PostgreSQL database |
| **redis** | 6379 | Cache and sessions |
| **nats** | 4222 | Event streaming |
| **prometheus** | 9090 | Metrics collection |
| **grafana** | 3001 | Dashboards |
| **jaeger** | 16686 | Distributed tracing UI |
| **nginx** | 80/443 | Reverse proxy |

### Network Architecture

```
                    ┌─────────────────┐
                    │   Nginx Proxy   │
                    │   (Port 80/443) │
                    └────────┬────────┘
                             │
                ┌────────────┴────────────┐
                │                         │
        ┌───────▼────────┐       ┌───────▼────────┐
        │  HTTP API      │       │   gRPC API     │
        │  (Port 3000)   │       │  (Port 50051)  │
        └───────┬────────┘       └───────┬────────┘
                │                        │
                └───────────┬────────────┘
                            │
          ┌─────────────────┼─────────────────┐
          │                 │                 │
    ┌─────▼─────┐    ┌─────▼─────┐    ┌─────▼─────┐
    │ PostgreSQL│    │   Redis   │    │   NATS    │
    │  Database │    │   Cache   │    │ Messaging │
    └───────────┘    └───────────┘    └───────────┘
```

## Docker Image Stages

### 1. Planner Stage
Analyzes dependencies using cargo-chef for optimal caching.

### 2. Builder Stage
- Compiles the application with all optimizations
- Includes protobuf compiler for gRPC support
- Strips binary for minimal size

### 3. Runtime Stage (Production)
- Based on Debian Bookworm Slim
- Non-root user (UID 10001)
- Only runtime dependencies
- Health check script included
- ~50MB final image size

### 4. Development Stage
- Full development environment
- Hot reload with cargo-watch
- Development tools included
- PostgreSQL and Redis clients

### 5. Testing Stage
- Optimized for running tests
- Includes cargo-nextest and tarpaulin
- Test coverage support

## Building Images

### Build Production Image

```bash
# Build for local platform
./scripts/build.sh

# Build for multiple architectures
PLATFORM="linux/amd64,linux/arm64" ./scripts/build.sh

# Build and push to registry
PUSH=true DOCKER_REGISTRY=myregistry.com ./scripts/build.sh

# Build without cache
NO_CACHE=true ./scripts/build.sh

# Build specific target
BUILD_TARGET=development ./scripts/build.sh
```

### Build with Docker Compose

```bash
# Development build
docker compose build

# Production build
docker compose -f docker-compose.prod.yml build
```

## Configuration

### Environment Variables

Create `.env.production.local` from the template:

```bash
cp .env.production .env.production.local
```

**Required variables:**

- `POSTGRES_PASSWORD` - Database password
- `GRAFANA_ADMIN_PASSWORD` - Grafana admin password
- `JWT_SECRET` - JWT signing secret (min 32 chars)

**Optional variables:**

- `CORS_ALLOWED_ORIGINS` - Comma-separated allowed origins
- `API_REPLICAS` - Number of API replicas (default: 2)
- `DB_MAX_CONNECTIONS` - Max database connections (default: 20)
- `ENABLE_GRPC` - Enable gRPC server (default: true)

### Application Configuration

The application reads configuration in this order:

1. Default values in code
2. `config/default.toml`
3. `config/{ENVIRONMENT}.toml`
4. Environment variables (prefix: `LLM_REGISTRY__`)
5. Command-line arguments

Example configuration structure:

```toml
[server]
host = "0.0.0.0"
port = 3000
graceful_shutdown = true
shutdown_timeout_seconds = 30

[grpc]
enabled = true
host = "0.0.0.0"
port = 50051

[database]
url = "postgresql://user:pass@postgres:5432/llm_registry"
max_connections = 20
min_connections = 5

[logging]
level = "info"
json_format = false
```

## Health Checks

### HTTP Health Check

```bash
curl http://localhost:3000/health
```

Response:
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "components": {
    "database": "healthy",
    "cache": "healthy"
  }
}
```

### Container Health

```bash
docker inspect llm-registry-api | jq '.[0].State.Health'
```

## Deployment Strategies

### Single Host Deployment

```bash
# Using Docker Compose
./scripts/deploy.sh
```

### Docker Swarm

```bash
# Initialize swarm
docker swarm init

# Deploy stack
docker stack deploy -c docker-compose.prod.yml llm-registry

# Scale API service
docker service scale llm-registry_api=4

# View services
docker stack services llm-registry
```

### Kubernetes

See `k8s/` directory for Kubernetes manifests.

```bash
kubectl apply -f k8s/
```

## Monitoring

### Prometheus Metrics

Access Prometheus at `http://localhost:9090`

Key metrics:
- `http_requests_total` - Total HTTP requests
- `http_request_duration_seconds` - Request latency
- `db_connections_active` - Active database connections
- `cache_hits_total` - Cache hit rate

### Grafana Dashboards

Access Grafana at `http://localhost:3001`

Default credentials:
- Username: `admin`
- Password: (see `GRAFANA_ADMIN_PASSWORD`)

### Distributed Tracing

Access Jaeger UI at `http://localhost:16686`

## Backup and Recovery

### Database Backup

```bash
# Create backup
docker exec llm-registry-postgres pg_dump -U llmreg llm_registry > backup.sql

# Restore from backup
docker exec -i llm-registry-postgres psql -U llmreg llm_registry < backup.sql
```

### Volume Backup

```bash
# Backup all volumes
docker run --rm \
  -v llm-registry_postgres_data:/data \
  -v $(pwd):/backup \
  alpine tar czf /backup/postgres-data-$(date +%Y%m%d).tar.gz -C /data .
```

## Security

### Security Features

1. **Non-root containers** - All containers run as non-root users
2. **Read-only filesystems** - Where possible
3. **Resource limits** - CPU and memory limits set
4. **Network isolation** - Internal Docker network
5. **Security headers** - Configured in Nginx
6. **TLS/SSL** - Ready for HTTPS configuration

### SSL/TLS Setup

1. Generate or obtain SSL certificates
2. Place certificates in `./certs/` directory:
   - `cert.pem` - Certificate
   - `key.pem` - Private key
3. Uncomment HTTPS server blocks in `deployments/nginx/conf.d/api.conf`
4. Restart nginx

```bash
docker compose restart nginx
```

## Performance Tuning

### PostgreSQL Tuning

The production configuration includes optimized PostgreSQL settings:

- `shared_buffers=256MB`
- `effective_cache_size=1GB`
- `work_mem=16MB`
- `maintenance_work_mem=64MB`

Adjust based on your hardware in `docker-compose.prod.yml`.

### API Scaling

```bash
# Scale API replicas
API_REPLICAS=4 ./scripts/deploy.sh
```

### Resource Limits

Modify resource limits in `docker-compose.prod.yml`:

```yaml
deploy:
  resources:
    limits:
      cpus: '2'
      memory: 2G
    reservations:
      cpus: '0.5'
      memory: 512M
```

## Troubleshooting

### View Logs

```bash
# All services
docker compose logs -f

# Specific service
docker compose logs -f api

# Last 100 lines
docker compose logs --tail=100 api
```

### Container Shell Access

```bash
# API container
docker exec -it llm-registry-api /bin/bash

# Database container
docker exec -it llm-registry-postgres psql -U llmreg llm_registry
```

### Common Issues

#### Port Already in Use

```bash
# Find process using port
lsof -i :3000

# Stop conflicting service
docker compose down
```

#### Database Connection Failed

```bash
# Check database health
docker compose exec postgres pg_isready -U llmreg

# View database logs
docker compose logs postgres
```

#### Out of Memory

Increase memory limits in `docker-compose.prod.yml` or add swap space.

## Maintenance

### Update Images

```bash
# Pull latest images
docker compose pull

# Rebuild and restart
docker compose up -d --build
```

### Clean Up

```bash
# Remove unused images
docker image prune -a

# Remove unused volumes
docker volume prune

# Complete cleanup (WARNING: removes all data)
docker compose down -v
```

## Production Checklist

Before deploying to production:

- [ ] Set strong passwords in `.env.production.local`
- [ ] Configure SSL/TLS certificates
- [ ] Set appropriate CORS origins
- [ ] Configure backup strategy
- [ ] Set up monitoring alerts
- [ ] Review resource limits
- [ ] Enable log aggregation
- [ ] Configure firewall rules
- [ ] Set up health check monitoring
- [ ] Test disaster recovery procedures

## Support

For issues and questions:

- GitHub Issues: https://github.com/llm-devops/llm-registry/issues
- Documentation: https://llm-registry.dev/docs

## License

Apache-2.0 OR MIT
