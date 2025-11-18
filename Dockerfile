# ============================================================================
# Enterprise-Grade Multi-Stage Dockerfile for LLM Registry
#
# Features:
# - Multi-stage builds for minimal image size
# - Security hardening with non-root user
# - Layer caching optimization
# - gRPC/Protobuf support
# - Health checks
# - Multi-architecture support (amd64/arm64)
# ============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Planner - Analyze dependencies
# -----------------------------------------------------------------------------
FROM rust:1.75-bookworm AS planner

WORKDIR /app

# Install cargo-chef for dependency caching
RUN cargo install cargo-chef

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Generate recipe file
RUN cargo chef prepare --recipe-path recipe.json

# -----------------------------------------------------------------------------
# Stage 2: Builder - Build dependencies and application
# -----------------------------------------------------------------------------
FROM rust:1.75-bookworm AS builder

WORKDIR /app

# Install system dependencies including protobuf compiler for gRPC
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    libprotobuf-dev \
    musl-tools \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-chef for caching
RUN cargo install cargo-chef

# Copy recipe from planner
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies (this layer will be cached unless dependencies change)
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY migrations ./migrations

# Build application with all optimizations
RUN cargo build --release --bin llm-registry-server && \
    strip /app/target/release/llm-registry-server

# -----------------------------------------------------------------------------
# Stage 3: Runtime - Minimal secure runtime image
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies and security updates
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && apt-get upgrade -y \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user with specific UID/GID for security
RUN groupadd -g 10001 appuser && \
    useradd -m -u 10001 -g appuser -s /bin/bash appuser

# Create necessary directories
RUN mkdir -p /app/config /app/data /var/log/llm-registry && \
    chown -R appuser:appuser /app /var/log/llm-registry

WORKDIR /app

# Copy binary from builder
COPY --from=builder --chown=appuser:appuser /app/target/release/llm-registry-server /usr/local/bin/llm-registry-server

# Copy configuration files
COPY --chown=appuser:appuser config ./config

# Switch to non-root user
USER appuser

# Expose ports for HTTP, gRPC, and metrics
EXPOSE 3000 50051 9090

# Set environment variables with secure defaults
ENV RUST_LOG=info \
    RUST_BACKTRACE=1 \
    SERVER_HOST=0.0.0.0 \
    SERVER_PORT=3000 \
    GRPC_HOST=0.0.0.0 \
    GRPC_PORT=50051 \
    ENVIRONMENT=production

# Add health check script
COPY --chown=appuser:appuser <<'EOF' /usr/local/bin/healthcheck.sh
#!/bin/bash
set -eo pipefail

# Check HTTP health endpoint
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:${SERVER_PORT:-3000}/health || echo "000")

if [ "$HTTP_STATUS" = "200" ]; then
    echo "Health check passed"
    exit 0
else
    echo "Health check failed with status: $HTTP_STATUS"
    exit 1
fi
EOF

RUN chmod +x /usr/local/bin/healthcheck.sh

# Health check configuration
HEALTHCHECK --interval=30s \
            --timeout=10s \
            --start-period=40s \
            --retries=3 \
    CMD ["/usr/local/bin/healthcheck.sh"]

# Metadata labels
LABEL org.opencontainers.image.title="LLM Registry Server" \
      org.opencontainers.image.description="Enterprise-grade LLM asset registry with gRPC and REST APIs" \
      org.opencontainers.image.vendor="LLM DevOps Team" \
      org.opencontainers.image.version="0.1.0" \
      org.opencontainers.image.licenses="Apache-2.0 OR MIT"

# Run the application
CMD ["llm-registry-server"]

# -----------------------------------------------------------------------------
# Stage 4: Development - Full development environment
# -----------------------------------------------------------------------------
FROM rust:1.75-bookworm AS development

WORKDIR /app

# Install development dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    libprotobuf-dev \
    postgresql-client \
    redis-tools \
    curl \
    vim \
    git \
    && rm -rf /var/lib/apt/lists/*

# Install development tools
RUN cargo install cargo-watch cargo-edit sqlx-cli cargo-audit cargo-outdated

# Copy all files
COPY . .

# Expose ports for HTTP, gRPC, and debugger
EXPOSE 3000 50051 9090

# Set development environment variables
ENV RUST_LOG=debug \
    RUST_BACKTRACE=full \
    SERVER_HOST=0.0.0.0 \
    SERVER_PORT=3000

# Default command for development
CMD ["cargo", "watch", "-x", "run --bin llm-registry-server"]

# -----------------------------------------------------------------------------
# Stage 5: Testing - Optimized for running tests
# -----------------------------------------------------------------------------
FROM rust:1.75-bookworm AS testing

WORKDIR /app

# Install test dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    libprotobuf-dev \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# Install test tools
RUN cargo install cargo-tarpaulin cargo-nextest

# Copy source
COPY . .

# Default command runs all tests
CMD ["cargo", "nextest", "run", "--all-features"]
