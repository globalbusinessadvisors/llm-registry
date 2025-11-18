# Kubernetes Manifests for LLM Registry

This directory contains production-ready Kubernetes manifests for deploying the LLM Registry.

## Prerequisites

- Kubernetes cluster (v1.24+)
- kubectl configured to access your cluster
- nginx-ingress-controller installed
- cert-manager installed (for TLS)
- metrics-server installed (for HPA)

## Quick Start

### 1. Deploy in Order

```bash
# Create namespace
kubectl apply -f namespace.yaml

# Create secrets and configmaps
kubectl apply -f secret.yaml
kubectl apply -f configmap.yaml

# Deploy infrastructure components
kubectl apply -f postgres.yaml
kubectl apply -f redis.yaml
kubectl apply -f nats.yaml

# Wait for infrastructure to be ready
kubectl wait --for=condition=ready pod -l app.kubernetes.io/name=postgres -n llm-registry --timeout=300s
kubectl wait --for=condition=ready pod -l app.kubernetes.io/name=redis -n llm-registry --timeout=300s
kubectl wait --for=condition=ready pod -l app.kubernetes.io/name=nats -n llm-registry --timeout=300s

# Deploy application
kubectl apply -f serviceaccount.yaml
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
kubectl apply -f hpa.yaml
kubectl apply -f pdb.yaml
kubectl apply -f networkpolicy.yaml

# Deploy ingress (update host in ingress.yaml first)
kubectl apply -f ingress.yaml
```

### 2. All-in-One Deployment

```bash
kubectl apply -f .
```

## Configuration

### Secrets

**IMPORTANT**: Change the default secrets in `secret.yaml` before deploying to production!

```yaml
DATABASE_PASSWORD: "change-me-in-production"
JWT_SECRET: "change-me-in-production-use-strong-secret"
```

### Ingress

Update the host in `ingress.yaml`:

```yaml
spec:
  tls:
    - hosts:
        - your-domain.com  # Change this
  rules:
    - host: your-domain.com  # Change this
```

### Resource Limits

Adjust resource requests and limits in `deployment.yaml` based on your needs:

```yaml
resources:
  requests:
    memory: "512Mi"
    cpu: "500m"
  limits:
    memory: "1Gi"
    cpu: "1000m"
```

## Components

- **namespace.yaml**: Namespace for all resources
- **configmap.yaml**: Application configuration
- **secret.yaml**: Sensitive configuration (passwords, secrets)
- **postgres.yaml**: PostgreSQL StatefulSet
- **redis.yaml**: Redis Deployment
- **nats.yaml**: NATS Deployment
- **deployment.yaml**: Main application Deployment
- **service.yaml**: Kubernetes Services
- **serviceaccount.yaml**: RBAC configuration
- **ingress.yaml**: Ingress for external access
- **hpa.yaml**: Horizontal Pod Autoscaler
- **pdb.yaml**: Pod Disruption Budget
- **networkpolicy.yaml**: Network policies for security

## Scaling

### Manual Scaling

```bash
kubectl scale deployment llm-registry-server --replicas=5 -n llm-registry
```

### Auto Scaling

The HPA is configured to:
- Min replicas: 3
- Max replicas: 10
- CPU target: 70%
- Memory target: 80%

## Monitoring

Metrics are exposed at `/metrics` endpoint. Prometheus annotations are configured:

```yaml
annotations:
  prometheus.io/scrape: "true"
  prometheus.io/port: "8080"
  prometheus.io/path: "/metrics"
```

## Security

- All containers run as non-root (UID 1000)
- Read-only root filesystem
- Network policies restrict traffic
- Pod Security Standards enforced
- Service account with minimal permissions

## Troubleshooting

### Check Pod Status

```bash
kubectl get pods -n llm-registry
```

### View Logs

```bash
kubectl logs -f -l app.kubernetes.io/name=llm-registry -n llm-registry
```

### Check Events

```bash
kubectl get events -n llm-registry --sort-by='.lastTimestamp'
```

### Database Connection Issues

```bash
# Test database connectivity
kubectl exec -it postgres-0 -n llm-registry -- psql -U registry -d llm_registry -c "SELECT 1"
```

### Redis Connection Issues

```bash
# Test Redis connectivity
kubectl exec -it -n llm-registry deployment/redis -- redis-cli ping
```

## Cleanup

```bash
kubectl delete namespace llm-registry
```

Or delete individually:

```bash
kubectl delete -f .
```

## Production Checklist

- [ ] Update all secrets in `secret.yaml`
- [ ] Update ingress host in `ingress.yaml`
- [ ] Configure TLS certificates
- [ ] Set appropriate resource limits
- [ ] Enable backup for PostgreSQL
- [ ] Configure monitoring and alerting
- [ ] Set up log aggregation
- [ ] Review and adjust HPA settings
- [ ] Test disaster recovery procedures
- [ ] Configure external DNS
- [ ] Set up CI/CD pipeline
