# LLM Registry - Security Guide

## Table of Contents

- [Security Overview](#security-overview)
- [Authentication](#authentication)
- [Authorization (RBAC)](#authorization-rbac)
- [API Security](#api-security)
- [Data Security](#data-security)
- [Network Security](#network-security)
- [Secrets Management](#secrets-management)
- [Audit Logging](#audit-logging)
- [Security Best Practices](#security-best-practices)
- [Vulnerability Disclosure](#vulnerability-disclosure)
- [Security Checklist](#security-checklist)

---

## Security Overview

The LLM Registry implements a defense-in-depth security strategy with multiple layers of protection:

1. **Network Layer**: TLS encryption, firewalls, network policies
2. **Application Layer**: Authentication, authorization, rate limiting
3. **Data Layer**: Encryption at rest, encrypted backups
4. **Operational Layer**: Audit logging, monitoring, alerting

### Security Principles

- **Least Privilege**: Users and services have minimum required permissions
- **Defense in Depth**: Multiple security layers
- **Fail Secure**: System fails to a secure state
- **Zero Trust**: Verify every request, never assume trust
- **Audit Everything**: Comprehensive logging of security events

---

## Authentication

### JWT (JSON Web Tokens)

The API uses JWT for stateless authentication with the following characteristics:

**Token Structure**:
```
Header.Payload.Signature
```

**Header**:
```json
{
  "alg": "HS256",
  "typ": "JWT"
}
```

**Payload**:
```json
{
  "sub": "user_id",
  "exp": 1705578600,
  "iat": 1705575000,
  "iss": "llm-registry",
  "aud": "llm-registry-api",
  "roles": ["admin"],
  "permissions": ["assets:*"]
}
```

**Signature**:
```
HMACSHA256(
  base64UrlEncode(header) + "." + base64UrlEncode(payload),
  secret
)
```

### Token Lifecycle

#### 1. Access Tokens
- **Lifetime**: 1 hour (configurable)
- **Purpose**: API authentication
- **Storage**: Memory only (never localStorage)
- **Refresh**: Use refresh token before expiration

#### 2. Refresh Tokens
- **Lifetime**: 7 days (configurable)
- **Purpose**: Obtain new access tokens
- **Storage**: Secure HTTP-only cookies
- **Rotation**: New refresh token issued on each refresh

#### 3. API Keys
- **Lifetime**: 90 days (configurable)
- **Purpose**: Machine-to-machine authentication
- **Format**: `llmreg_<32_random_chars>`
- **Scope**: Limited permissions per key

### Authentication Flow

```
┌──────┐                 ┌────────┐                 ┌──────┐
│Client│                 │  API   │                 │  DB  │
└──┬───┘                 └───┬────┘                 └──┬───┘
   │                         │                         │
   │  1. POST /auth/login    │                         │
   │  {username, password}   │                         │
   │─────────────────────────>│                         │
   │                         │                         │
   │                         │  2. Verify credentials  │
   │                         │────────────────────────>│
   │                         │<────────────────────────│
   │                         │                         │
   │                         │  3. Generate JWT        │
   │                         │  (sign with secret)     │
   │                         │                         │
   │  4. {access_token,      │                         │
   │      refresh_token}     │                         │
   │<─────────────────────────│                         │
   │                         │                         │
   │  5. API Request         │                         │
   │  Authorization: Bearer  │                         │
   │─────────────────────────>│                         │
   │                         │                         │
   │                         │  6. Verify JWT signature│
   │                         │  7. Check expiration    │
   │                         │  8. Extract claims      │
   │                         │                         │
   │  9. Response            │                         │
   │<─────────────────────────│                         │
```

### Security Recommendations

**JWT Secret Management**:
```bash
# Generate secure secret (minimum 256 bits)
openssl rand -base64 32

# Store in environment variable (NEVER in code)
export JWT_SECRET="your-generated-secret"
```

**Token Validation**:
- ✅ Verify signature
- ✅ Check expiration (exp claim)
- ✅ Validate issuer (iss claim)
- ✅ Validate audience (aud claim)
- ✅ Check not-before (nbf claim) if present

**Token Storage**:
- ❌ Never store in localStorage (vulnerable to XSS)
- ✅ Use secure HTTP-only cookies for refresh tokens
- ✅ Store access tokens in memory only
- ✅ Clear tokens on logout

---

## Authorization (RBAC)

### Role Hierarchy

```
admin (Full system access)
  │
  ├─→ developer (Can manage assets, generate API keys)
  │     │
  │     └─→ user (Can create and manage own assets)
  │           │
  │           └─→ viewer (Read-only access)
```

### Permission Model

Permissions follow the format: `resource:action`

**Resources**:
- `assets` - Asset management
- `policies` - Policy management
- `users` - User management
- `api-keys` - API key management
- `audit` - Audit log access

**Actions**:
- `create` - Create new resources
- `read` - View resources
- `update` - Modify resources
- `delete` - Remove resources
- `*` - All actions

**Examples**:
- `assets:read` - Can view assets
- `assets:*` - Can perform all asset operations
- `*:*` - Full system access (admin only)

### Role Definitions

#### Admin Role
```rust
Role {
    name: "admin",
    permissions: ["*:*"],  // All permissions
    inherits_from: []
}
```

#### Developer Role
```rust
Role {
    name: "developer",
    permissions: [
        "assets:*",
        "policies:read",
        "api-keys:create",
        "api-keys:read",
        "api-keys:delete"
    ],
    inherits_from: []
}
```

#### User Role
```rust
Role {
    name: "user",
    permissions: [
        "assets:create",
        "assets:read",
        "assets:update",  // Own assets only
    ],
    inherits_from: []
}
```

#### Viewer Role
```rust
Role {
    name: "viewer",
    permissions: [
        "assets:read",
        "policies:read"
    ],
    inherits_from: []
}
```

### Permission Checking

**Code Example**:
```rust
// Check single permission
if rbac.has_permission(&user.roles, &Permission::new("assets", "create"))? {
    // Allow asset creation
} else {
    return Err(Error::Forbidden);
}

// Check multiple permissions
let required = vec![
    Permission::new("assets", "delete"),
    Permission::new("audit", "read"),
];

if rbac.has_all_permissions(&user.roles, &required)? {
    // Allow operation
}
```

**HTTP Middleware**:
```rust
// Require authentication
.layer(middleware::from_fn_with_state(auth_state, require_auth))

// Require specific role
.layer(middleware::from_fn_with_state(
    (auth_state, vec!["admin".to_string()]),
    require_role
))
```

---

## API Security

### Rate Limiting

**Default Limits**:
- Authenticated: 100 requests/minute
- Unauthenticated: 20 requests/minute
- Burst: 10 requests

**Configuration**:
```toml
[rate_limit]
enabled = true
max_requests = 100
window_secs = 60
by_ip = true
by_user = true
```

**Rate Limit Headers**:
```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1705578600
```

**429 Response**:
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

### Input Validation

**Validation Layers**:
1. **Schema validation** - JSON schema enforcement
2. **Type validation** - Rust type system
3. **Business rules** - Domain-specific constraints
4. **Sanitization** - XSS/injection prevention

**Example**:
```rust
#[derive(Deserialize, Validate)]
pub struct RegisterAssetRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    #[validate(custom = "validate_version")]
    pub version: Version,

    #[validate(url)]
    pub source_repo: Option<String>,

    #[validate(length(max = 1000))]
    pub description: Option<String>,
}
```

### CORS Configuration

**Production Settings**:
```toml
[cors]
allowed_origins = ["https://app.example.com"]
allowed_methods = ["GET", "POST", "PUT", "PATCH", "DELETE"]
allowed_headers = ["Authorization", "Content-Type"]
allow_credentials = true
max_age = 3600
```

**Security Headers**:
```
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Strict-Transport-Security: max-age=31536000; includeSubDomains
Content-Security-Policy: default-src 'self'
```

### Request Signing (Optional)

For high-security environments, implement request signing:

```
Authorization: LLMREG-HMAC-SHA256
  Credential=<access_key_id>,
  SignedHeaders=host;x-llmreg-date,
  Signature=<signature>
```

**Signature Calculation**:
```rust
fn calculate_signature(
    secret_key: &str,
    method: &str,
    path: &str,
    headers: &HashMap<String, String>,
    payload: &str,
) -> String {
    let canonical_request = format!(
        "{}\n{}\n{}\n{}",
        method,
        path,
        canonicalize_headers(headers),
        payload
    );

    let mac = Hmac::<Sha256>::new_from_slice(secret_key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(canonical_request.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}
```

---

## Data Security

### Encryption at Rest

**Database Encryption**:
```yaml
# PostgreSQL with encryption
postgresql:
  ssl_mode: require
  ssl_cert: /certs/postgres-client.crt
  ssl_key: /certs/postgres-client.key
  ssl_ca: /certs/postgres-ca.crt

  # Transparent Data Encryption (TDE)
  data_encryption:
    enabled: true
    algorithm: AES-256-GCM
```

**Backup Encryption**:
```bash
# Encrypt backups with GPG
pg_dump llm_registry | gzip | gpg --encrypt --recipient backup@example.com > backup.sql.gz.gpg

# Decrypt and restore
gpg --decrypt backup.sql.gz.gpg | gunzip | psql llm_registry
```

### Encryption in Transit

**TLS Configuration**:
```nginx
server {
    listen 443 ssl http2;
    server_name api.llm-registry.com;

    # TLS 1.3 only
    ssl_protocols TLSv1.3;

    # Strong ciphers
    ssl_ciphers 'TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256';
    ssl_prefer_server_ciphers off;

    # Certificates
    ssl_certificate /etc/ssl/certs/llm-registry.crt;
    ssl_certificate_key /etc/ssl/private/llm-registry.key;

    # OCSP Stapling
    ssl_stapling on;
    ssl_stapling_verify on;

    # HSTS
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload" always;
}
```

### Sensitive Data Handling

**PII Protection**:
- Never log passwords, tokens, or API keys
- Redact sensitive fields in logs
- Use secure deletion for sensitive data

**Example Redaction**:
```rust
#[derive(Debug, Serialize)]
pub struct User {
    pub id: String,
    pub email: String,
    #[serde(skip_serializing)]  // Never serialize password
    pub password_hash: String,
}

// Logging with redaction
info!(
    user_id = %user.id,
    email = redact_email(&user.email),  // user@example.com -> u***@example.com
    "User logged in"
);
```

---

## Network Security

### Firewall Rules

**Ingress Rules** (Kubernetes Network Policy):
```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: api-ingress
spec:
  podSelector:
    matchLabels:
      app: llm-registry-api
  policyTypes:
  - Ingress
  ingress:
  - from:
    - podSelector:
        matchLabels:
          app: nginx-ingress
    ports:
    - protocol: TCP
      port: 8080
```

**Egress Rules**:
```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: api-egress
spec:
  podSelector:
    matchLabels:
      app: llm-registry-api
  policyTypes:
  - Egress
  egress:
  # Allow DNS
  - to:
    - namespaceSelector:
        matchLabels:
          name: kube-system
    ports:
    - protocol: UDP
      port: 53
  # Allow PostgreSQL
  - to:
    - podSelector:
        matchLabels:
          app: postgresql
    ports:
    - protocol: TCP
      port: 5432
  # Allow Redis
  - to:
    - podSelector:
        matchLabels:
          app: redis
    ports:
    - protocol: TCP
      port: 6379
```

### Container Security

**Security Context**:
```yaml
apiVersion: v1
kind: Pod
spec:
  securityContext:
    runAsNonRoot: true
    runAsUser: 10001
    fsGroup: 10001
    seccompProfile:
      type: RuntimeDefault

  containers:
  - name: api
    image: llm-registry:latest
    securityContext:
      allowPrivilegeEscalation: false
      readOnlyRootFilesystem: true
      capabilities:
        drop:
        - ALL
```

**Image Scanning**:
```bash
# Scan with Trivy
trivy image llm-registry:latest

# Scan with Snyk
snyk container test llm-registry:latest
```

---

## Secrets Management

### Environment Variables

**Never commit secrets**:
```bash
# ❌ Bad - hardcoded
JWT_SECRET="my-secret-key"

# ✅ Good - from secure source
JWT_SECRET=$(vault kv get -field=jwt_secret secret/llm-registry)
```

### Kubernetes Secrets

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: llm-registry-secrets
type: Opaque
data:
  jwt-secret: <base64-encoded-secret>
  database-password: <base64-encoded-password>

---
apiVersion: v1
kind: Pod
spec:
  containers:
  - name: api
    env:
    - name: JWT_SECRET
      valueFrom:
        secretKeyRef:
          name: llm-registry-secrets
          key: jwt-secret
```

### HashiCorp Vault Integration

```rust
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};

async fn get_secret_from_vault(path: &str) -> Result<String> {
    let client = VaultClient::new(
        VaultClientSettingsBuilder::default()
            .address("https://vault.example.com")
            .token(std::env::var("VAULT_TOKEN")?)
            .build()?
    )?;

    let secret: HashMap<String, String> = vaultrs::kv2::read(
        &client,
        "secret",
        path
    ).await?;

    Ok(secret.get("value")
        .ok_or("Secret not found")?
        .clone())
}
```

### Secret Rotation

**JWT Secret Rotation**:
```bash
# 1. Generate new secret
NEW_SECRET=$(openssl rand -base64 32)

# 2. Update vault
vault kv put secret/llm-registry/jwt-secret value=$NEW_SECRET

# 3. Rolling update (both secrets valid during rollout)
kubectl set env deployment/llm-registry-api JWT_SECRET_NEW=$NEW_SECRET

# 4. After rollout, make new secret primary
kubectl set env deployment/llm-registry-api JWT_SECRET=$NEW_SECRET

# 5. Remove old secret
kubectl set env deployment/llm-registry-api JWT_SECRET_OLD-
```

---

## Audit Logging

### Event Types

**Authentication Events**:
- User login success/failure
- Token refresh
- API key creation/deletion
- Session termination

**Authorization Events**:
- Permission denied
- Role changes
- Access violations

**Data Events**:
- Asset creation/update/deletion
- Policy changes
- Configuration updates

### Audit Log Format

```json
{
  "timestamp": "2025-01-18T10:30:00.123Z",
  "event_type": "asset.created",
  "actor": {
    "type": "user",
    "id": "user_abc123",
    "ip_address": "192.168.1.100",
    "user_agent": "Mozilla/5.0..."
  },
  "resource": {
    "type": "asset",
    "id": "01HN9XWZP8XQYZVJ4KFQY6XQZV",
    "name": "gpt-custom-v1"
  },
  "action": "create",
  "result": "success",
  "metadata": {
    "asset_type": "model",
    "size_bytes": 1073741824
  },
  "request_id": "req_xyz789"
}
```

### Audit Log Storage

**Requirements**:
- Tamper-proof (append-only)
- Long-term retention (7 years for compliance)
- Searchable
- Encrypted

**Implementation Options**:
1. **PostgreSQL** (with append-only tables)
2. **Elasticsearch** (with Watcher for alerts)
3. **AWS CloudWatch Logs** (for cloud deployments)
4. **Splunk/DataDog** (for enterprise)

---

## Security Best Practices

### Development

1. **Dependency Scanning**
   ```bash
   # Check for vulnerabilities
   cargo audit

   # Update dependencies
   cargo update

   # Automated in CI/CD
   ```

2. **Code Analysis**
   ```bash
   # Static analysis
   cargo clippy -- -D warnings

   # Security lints
   cargo clippy -- -W clippy::unwrap_used

   # Format check
   cargo fmt -- --check
   ```

3. **Secret Scanning**
   ```bash
   # Pre-commit hook
   git secrets --scan

   # CI/CD check
   trufflehog --regex --entropy=False .
   ```

### Deployment

1. **Principle of Least Privilege**
   - Use service accounts with minimal permissions
   - Restrict network access
   - Run containers as non-root

2. **Security Updates**
   - Automated security patches
   - Regular base image updates
   - Dependency updates

3. **Monitoring & Alerting**
   - Failed authentication attempts
   - Permission denied events
   - Unusual API patterns
   - High error rates

### Operations

1. **Incident Response Plan**
   - Security incident procedures
   - Communication plan
   - Escalation matrix

2. **Regular Security Audits**
   - Penetration testing
   - Code reviews
   - Configuration reviews

3. **Disaster Recovery**
   - Encrypted backups
   - Tested restore procedures
   - Off-site backup storage

---

## Vulnerability Disclosure

### Reporting Security Issues

**DO NOT** create public GitHub issues for security vulnerabilities.

**Instead**:
1. Email: security@llm-registry.dev
2. Include:
   - Vulnerability description
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

**Response Timeline**:
- Acknowledgment: Within 24 hours
- Initial assessment: Within 72 hours
- Fix deployment: Based on severity
  - Critical: Within 7 days
  - High: Within 30 days
  - Medium: Within 90 days

### Security Advisory Process

1. **Triage**: Assess severity and impact
2. **Fix Development**: Create patch in private branch
3. **Testing**: Thorough testing of fix
4. **Disclosure**: Coordinate with reporter
5. **Release**: Deploy fix and publish advisory
6. **Post-Mortem**: Document lessons learned

---

## Security Checklist

### Pre-Production

- [ ] Change all default credentials
- [ ] Generate strong JWT secret (>= 256 bits)
- [ ] Enable TLS/HTTPS
- [ ] Configure CORS properly
- [ ] Set up rate limiting
- [ ] Configure audit logging
- [ ] Enable encryption at rest
- [ ] Set up secrets management
- [ ] Configure backup encryption
- [ ] Review RBAC permissions
- [ ] Set up monitoring and alerting
- [ ] Scan containers for vulnerabilities
- [ ] Review network policies
- [ ] Enable security headers
- [ ] Test disaster recovery

### Post-Production

- [ ] Monitor security events
- [ ] Review audit logs regularly
- [ ] Update dependencies monthly
- [ ] Rotate secrets quarterly
- [ ] Conduct security reviews quarterly
- [ ] Test incident response annually
- [ ] Update security documentation
- [ ] Train team on security practices

---

## Resources

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [NIST Cybersecurity Framework](https://www.nist.gov/cyberframework)
- [CIS Benchmarks](https://www.cisecurity.org/cis-benchmarks/)
- [Rust Security Working Group](https://www.rust-lang.org/governance/wgs/wg-security-response)

---

**Last Updated**: 2025-01-18
**Security Version**: 1.0.0

For security issues, contact: security@llm-registry.dev
