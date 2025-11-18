#!/usr/bin/env bash
# ============================================================================
# Docker Build Script for LLM Registry
#
# This script builds production-ready Docker images with all optimizations
# ============================================================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
IMAGE_NAME="${IMAGE_NAME:-llm-registry-server}"
IMAGE_TAG="${IMAGE_TAG:-latest}"
DOCKER_REGISTRY="${DOCKER_REGISTRY:-}"
BUILD_TARGET="${BUILD_TARGET:-runtime}"
PLATFORM="${PLATFORM:-linux/amd64,linux/arm64}"
CACHE_FROM="${CACHE_FROM:-type=registry,ref=${IMAGE_NAME}:buildcache}"
CACHE_TO="${CACHE_TO:-type=registry,ref=${IMAGE_NAME}:buildcache,mode=max}"

# Functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_dependencies() {
    log_info "Checking dependencies..."

    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        exit 1
    fi

    if ! docker buildx version &> /dev/null; then
        log_error "Docker Buildx is not available"
        exit 1
    fi

    log_info "All dependencies are satisfied"
}

create_builder() {
    log_info "Setting up Docker Buildx builder..."

    if ! docker buildx inspect llm-registry-builder &> /dev/null; then
        docker buildx create \
            --name llm-registry-builder \
            --driver docker-container \
            --use
        log_info "Created new buildx builder: llm-registry-builder"
    else
        docker buildx use llm-registry-builder
        log_info "Using existing builder: llm-registry-builder"
    fi

    docker buildx inspect --bootstrap
}

build_image() {
    log_info "Building Docker image..."
    log_info "Target: ${BUILD_TARGET}"
    log_info "Platform: ${PLATFORM}"
    log_info "Image: ${IMAGE_NAME}:${IMAGE_TAG}"

    local full_image_name="${IMAGE_NAME}:${IMAGE_TAG}"
    if [ -n "${DOCKER_REGISTRY}" ]; then
        full_image_name="${DOCKER_REGISTRY}/${full_image_name}"
    fi

    # Build arguments
    local build_args=(
        --target "${BUILD_TARGET}"
        --platform "${PLATFORM}"
        --tag "${full_image_name}"
        --tag "${IMAGE_NAME}:${IMAGE_TAG}"
        --build-arg BUILDKIT_INLINE_CACHE=1
        --build-arg RUST_VERSION=1.75
    )

    # Add cache configuration if not disabled
    if [ "${NO_CACHE:-false}" != "true" ]; then
        build_args+=(
            --cache-from "${CACHE_FROM}"
            --cache-to "${CACHE_TO}"
        )
    fi

    # Add push flag if requested
    if [ "${PUSH:-false}" == "true" ]; then
        build_args+=(--push)
        log_info "Images will be pushed to registry"
    else
        build_args+=(--load)
        log_info "Images will be loaded locally"
    fi

    # Add labels
    build_args+=(
        --label "org.opencontainers.image.created=$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
        --label "org.opencontainers.image.version=${IMAGE_TAG}"
        --label "org.opencontainers.image.revision=$(git rev-parse HEAD 2>/dev/null || echo 'unknown')"
    )

    # Execute build
    docker buildx build "${build_args[@]}" .

    log_info "Build completed successfully"
}

display_image_info() {
    log_info "Image information:"
    docker images "${IMAGE_NAME}:${IMAGE_TAG}" --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedAt}}"
}

# Main execution
main() {
    log_info "Starting Docker build process..."
    log_info "Working directory: $(pwd)"

    check_dependencies
    create_builder
    build_image

    if [ "${PUSH:-false}" != "true" ]; then
        display_image_info
    fi

    log_info "Build process completed successfully!"
    log_info "Run './scripts/deploy.sh' to deploy the application"
}

# Run main function
main "$@"
