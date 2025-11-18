# LLM Registry Helm Chart

Enterprise-grade Helm chart for deploying LLM Registry on Kubernetes with high availability, security, and observability features.

## Features

- **High Availability**: Multi-replica deployment with Pod Disruption Budgets
- **Auto-scaling**: Horizontal Pod Autoscaler (HPA) based on CPU and memory
- **Dual APIs**: Both HTTP/REST and gRPC support
- **Security**: Non-root containers, Network Policies, RBAC, Pod Security Standards
- **Observability**: Prometheus metrics, Grafana dashboards, Jaeger tracing
- **Production-Ready**: Health checks, resource limits, graceful shutdown
- **Flexible**: Support for external or bundled databases (PostgreSQL, Redis, NATS)

## Prerequisites

- Kubernetes 1.24+
- Helm 3.8+
- kubectl configured to communicate with your cluster

### Optional

- cert-manager for automatic TLS certificate management
- Prometheus Operator for ServiceMonitor support
- Nginx Ingress Controller

## Installation

### Quick Start

```bash
# Add Helm repository dependencies
helm repo add bitnami https://charts.bitnami.com/bitnami
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm repo update

# Install with default values
helm install llm-registry ./helm/llm-registry --namespace llm-registry --create-namespace
```

### Using the Install Script

```bash
# Install with default values
./scripts/helm-install.sh

# Install in production with custom values
./scripts/helm-install.sh -n production -f values-prod.yaml

# Dry-run with debug output
./scripts/helm-install.sh --dry-run --debug
```

## Configuration

### Basic Configuration

Create a custom `values.yaml` file:

```yaml
# Custom image
image:
  registry: myregistry.com
  repository: llm-registry/server
  tag: "1.0.0"

# Replica count
replicaCount: 3

# Resources
resources:
  requests:
    cpu: 1000m
    memory: 1Gi
  limits:
    cpu: 2000m
    memory: 2Gi

# Ingress
ingress:
  enabled: true
  className: nginx
  hosts:
    - host: llm-registry.example.com
      paths:
        - path: /
          pathType: Prefix
          service:
            name: http
            port: 3000
```

Install with custom values:

```bash
helm install llm-registry ./helm/llm-registry -f custom-values.yaml
```

### Environment-Specific Values

The chart supports environment-specific value files:

- `values-dev.yaml` - Development environment
- `values-staging.yaml` - Staging environment
- `values-prod.yaml` - Production environment

```bash
./scripts/helm-install.sh -e prod
```

## Values

### Global Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `global.imageRegistry` | Global Docker image registry | `""` |
| `global.imagePullSecrets` | Global image pull secrets | `[]` |
| `global.storageClass` | Global storage class | `""` |

### Application Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `image.registry` | Image registry | `docker.io` |
| `image.repository` | Image repository | `llm-registry/llm-registry-server` |
| `image.tag` | Image tag | `latest` |
| `image.pullPolicy` | Image pull policy | `IfNotPresent` |
| `replicaCount` | Number of replicas | `2` |

### Service Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `service.type` | Service type | `ClusterIP` |
| `service.http.port` | HTTP service port | `3000` |
| `service.grpc.enabled` | Enable gRPC service | `true` |
| `service.grpc.port` | gRPC service port | `50051` |
| `service.sessionAffinity` | Session affinity | `ClientIP` |

### Ingress Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `ingress.enabled` | Enable ingress | `true` |
| `ingress.className` | Ingress class name | `nginx` |
| `ingress.annotations` | Ingress annotations | See values.yaml |
| `ingress.hosts` | Ingress hosts | See values.yaml |
| `ingress.tls` | TLS configuration | See values.yaml |
| `ingress.grpc.enabled` | Enable gRPC ingress | `true` |

### Autoscaling

| Parameter | Description | Default |
|-----------|-------------|---------|
| `autoscaling.enabled` | Enable HPA | `true` |
| `autoscaling.minReplicas` | Minimum replicas | `2` |
| `autoscaling.maxReplicas` | Maximum replicas | `10` |
| `autoscaling.targetCPUUtilizationPercentage` | Target CPU % | `70` |
| `autoscaling.targetMemoryUtilizationPercentage` | Target Memory % | `80` |

### Security

| Parameter | Description | Default |
|-----------|-------------|---------|
| `securityContext.runAsNonRoot` | Run as non-root | `true` |
| `securityContext.runAsUser` | User ID | `10001` |
| `containerSecurityContext.readOnlyRootFilesystem` | Read-only root FS | `true` |
| `networkPolicy.enabled` | Enable network policy | `true` |
| `podDisruptionBudget.enabled` | Enable PDB | `true` |
| `podDisruptionBudget.minAvailable` | Minimum available pods | `1` |

### Database (PostgreSQL)

| Parameter | Description | Default |
|-----------|-------------|---------|
| `postgresql.enabled` | Use bundled PostgreSQL | `true` |
| `postgresql.auth.username` | Database username | `llmreg` |
| `postgresql.auth.database` | Database name | `llm_registry` |
| `externalDatabase.host` | External DB host | `""` |
| `externalDatabase.port` | External DB port | `5432` |

### Cache (Redis)

| Parameter | Description | Default |
|-----------|-------------|---------|
| `redis.enabled` | Use bundled Redis | `true` |
| `redis.architecture` | Redis architecture | `standalone` |
| `externalRedis.host` | External Redis host | `""` |
| `externalRedis.port` | External Redis port | `6379` |

### Monitoring

| Parameter | Description | Default |
|-----------|-------------|---------|
| `metrics.enabled` | Enable metrics | `true` |
| `metrics.serviceMonitor.enabled` | Enable ServiceMonitor | `true` |
| `metrics.serviceMonitor.interval` | Scrape interval | `30s` |
| `jaeger.enabled` | Enable Jaeger tracing | `true` |

For a complete list of values, see [values.yaml](values.yaml).

## Usage Examples

### Production Deployment

```bash
# Create production values file
cat > values-prod.yaml << EOF
replicaCount: 3

autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 20

resources:
  requests:
    cpu: 1000m
    memory: 2Gi
  limits:
    cpu: 2000m
    memory: 4Gi

postgresql:
  enabled: false

externalDatabase:
  host: postgres.example.com
  port: 5432
  database: llm_registry
  username: llmreg

ingress:
  hosts:
    - host: api.llm-registry.com
      paths:
        - path: /
          pathType: Prefix
          service:
            name: http
            port: 3000
  tls:
    - secretName: llm-registry-tls
      hosts:
        - api.llm-registry.com
EOF

# Install
./scripts/helm-install.sh -f values-prod.yaml
```

### Development Deployment

```bash
# Development with minimal resources
cat > values-dev.yaml << EOF
replicaCount: 1

autoscaling:
  enabled: false

resources:
  requests:
    cpu: 100m
    memory: 256Mi
  limits:
    cpu: 500m
    memory: 512Mi

postgresql:
  enabled: true
  primary:
    resources:
      requests:
        cpu: 100m
        memory: 256Mi

redis:
  enabled: true
  master:
    resources:
      requests:
        cpu: 50m
        memory: 128Mi

ingress:
  enabled: false
EOF

# Install
./scripts/helm-install.sh -e dev
```

### Disable gRPC

```bash
helm install llm-registry ./helm/llm-registry \
  --set service.grpc.enabled=false \
  --set ingress.grpc.enabled=false \
  --set config.features.grpc=false
```

### Use External Services

```bash
helm install llm-registry ./helm/llm-registry \
  --set postgresql.enabled=false \
  --set externalDatabase.host=my-postgres.example.com \
  --set externalDatabase.password=secret \
  --set redis.enabled=false \
  --set externalRedis.host=my-redis.example.com
```

## Managing the Deployment

### Using Management Script

```bash
# Show status
./scripts/helm-manage.sh status

# View logs
./scripts/helm-manage.sh logs

# Port forward to local
./scripts/helm-manage.sh port-forward

# Show history
./scripts/helm-manage.sh history

# Rollback
./scripts/helm-manage.sh rollback
```

### Manual Commands

```bash
# Upgrade release
helm upgrade llm-registry ./helm/llm-registry -f values.yaml

# Check status
helm status llm-registry

# View history
helm history llm-registry

# Rollback to previous version
helm rollback llm-registry

# Rollback to specific revision
helm rollback llm-registry 3
```

## Uninstallation

### Using Uninstall Script

```bash
# Uninstall release
./scripts/helm-uninstall.sh

# Uninstall and delete namespace
./scripts/helm-uninstall.sh --delete-namespace

# Uninstall and delete all data
./scripts/helm-uninstall.sh --delete-namespace --delete-pvc
```

### Manual Uninstall

```bash
# Uninstall release
helm uninstall llm-registry -n llm-registry

# Delete namespace
kubectl delete namespace llm-registry
```

## Troubleshooting

### Check Pod Status

```bash
kubectl get pods -n llm-registry
kubectl describe pod -n llm-registry <pod-name>
```

### View Logs

```bash
# All pods
kubectl logs -n llm-registry -l app.kubernetes.io/name=llm-registry --tail=100

# Specific pod
kubectl logs -n llm-registry <pod-name> -f
```

### Check Events

```bash
kubectl get events -n llm-registry --sort-by='.lastTimestamp'
```

### Debug Pod

```bash
# Get shell access
kubectl exec -it -n llm-registry <pod-name> -- /bin/sh

# Run health check
kubectl exec -n llm-registry <pod-name> -- curl localhost:3000/health
```

### Common Issues

#### Pods Not Starting

```bash
# Check pod events
kubectl describe pod -n llm-registry <pod-name>

# Check resource constraints
kubectl top nodes
kubectl top pods -n llm-registry
```

#### Database Connection Issues

```bash
# Check database pod
kubectl get pods -n llm-registry -l app.kubernetes.io/name=postgresql

# Test database connection
kubectl exec -it -n llm-registry <api-pod> -- sh -c 'nc -zv postgresql 5432'
```

#### Image Pull Errors

```bash
# Check image pull secrets
kubectl get secrets -n llm-registry

# Verify image exists
docker pull <image-name>
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Ingress                              │
│                    (HTTP + gRPC)                             │
└────────────┬────────────────────────────┬───────────────────┘
             │                            │
    ┌────────▼────────┐          ┌───────▼────────┐
    │  HTTP Service   │          │  gRPC Service  │
    │  (Port 3000)    │          │  (Port 50051)  │
    └────────┬────────┘          └───────┬────────┘
             │                            │
             └────────────┬───────────────┘
                          │
                ┌─────────▼─────────┐
                │   LLM Registry    │
                │   Deployment      │
                │   (2-10 Pods)     │
                └─────────┬─────────┘
                          │
         ┌────────────────┼────────────────┐
         │                │                │
    ┌────▼────┐     ┌────▼────┐     ┌────▼────┐
    │  Postgres│     │  Redis  │     │  NATS   │
    │ Database │     │  Cache  │     │  Queue  │
    └──────────┘     └─────────┘     └─────────┘
```

## Security Considerations

1. **Non-root containers**: All containers run as user ID 10001
2. **Read-only filesystem**: Container root filesystem is read-only
3. **Network policies**: Restrict ingress/egress traffic
4. **RBAC**: Minimal permissions via Role-based access control
5. **Pod Security Standards**: Enforces restricted policy
6. **Secrets management**: Use Kubernetes secrets or external secret managers
7. **TLS/SSL**: Configure ingress TLS for encrypted communication

## Performance Tuning

### Resource Allocation

Adjust based on your workload:

```yaml
resources:
  requests:
    cpu: 2000m
    memory: 4Gi
  limits:
    cpu: 4000m
    memory: 8Gi
```

### Database Connection Pool

```yaml
config:
  database:
    maxConnections: 50
    minConnections: 10
```

### Autoscaling Behavior

Fine-tune HPA behavior:

```yaml
autoscaling:
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
    scaleUp:
      stabilizationWindowSeconds: 0
```

## License

Apache-2.0 OR MIT

## Support

- Documentation: https://llm-registry.dev/docs
- GitHub: https://github.com/llm-devops/llm-registry
- Issues: https://github.com/llm-devops/llm-registry/issues
