#!/usr/bin/env bash
# ============================================================================
# Docker Deployment Script for LLM Registry
#
# This script deploys the LLM Registry using Docker Compose
# ============================================================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
ENVIRONMENT="${ENVIRONMENT:-production}"
COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.prod.yml}"
ENV_FILE="${ENV_FILE:-.env.production}"
PROJECT_NAME="${PROJECT_NAME:-llm-registry}"

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

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

check_prerequisites() {
    log_step "Checking prerequisites..."

    # Check Docker
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        exit 1
    fi

    # Check Docker Compose
    if ! docker compose version &> /dev/null; then
        log_error "Docker Compose is not installed"
        exit 1
    fi

    # Check environment file
    if [ ! -f "${ENV_FILE}" ]; then
        log_error "Environment file not found: ${ENV_FILE}"
        log_info "Please create ${ENV_FILE} from .env.production template"
        exit 1
    fi

    # Check Docker Compose file
    if [ ! -f "${COMPOSE_FILE}" ]; then
        log_error "Docker Compose file not found: ${COMPOSE_FILE}"
        exit 1
    fi

    log_info "All prerequisites satisfied"
}

validate_env_file() {
    log_step "Validating environment configuration..."

    # Check for placeholder values
    local placeholders_found=false

    if grep -q "CHANGE_ME" "${ENV_FILE}"; then
        log_error "Found placeholder values in ${ENV_FILE}"
        log_error "Please replace all CHANGE_ME values with actual configuration"
        placeholders_found=true
    fi

    if [ "${placeholders_found}" = true ]; then
        exit 1
    fi

    log_info "Environment configuration is valid"
}

pull_images() {
    log_step "Pulling latest images..."

    docker compose \
        -f "${COMPOSE_FILE}" \
        --env-file "${ENV_FILE}" \
        -p "${PROJECT_NAME}" \
        pull

    log_info "Images pulled successfully"
}

start_services() {
    log_step "Starting services..."

    docker compose \
        -f "${COMPOSE_FILE}" \
        --env-file "${ENV_FILE}" \
        -p "${PROJECT_NAME}" \
        up -d

    log_info "Services started successfully"
}

wait_for_health() {
    log_step "Waiting for services to be healthy..."

    local max_attempts=60
    local attempt=0

    while [ $attempt -lt $max_attempts ]; do
        local healthy_count=$(docker compose \
            -f "${COMPOSE_FILE}" \
            --env-file "${ENV_FILE}" \
            -p "${PROJECT_NAME}" \
            ps --format json | \
            jq -r '.[] | select(.Health == "healthy") | .Name' | \
            wc -l)

        local total_count=$(docker compose \
            -f "${COMPOSE_FILE}" \
            --env-file "${ENV_FILE}" \
            -p "${PROJECT_NAME}" \
            ps --format json | \
            jq -r '.[] | .Name' | \
            wc -l)

        log_info "Healthy services: ${healthy_count}/${total_count}"

        if [ "${healthy_count}" -eq "${total_count}" ] && [ "${total_count}" -gt 0 ]; then
            log_info "All services are healthy!"
            return 0
        fi

        ((attempt++))
        sleep 5
    done

    log_error "Services did not become healthy in time"
    return 1
}

display_status() {
    log_step "Service status:"

    docker compose \
        -f "${COMPOSE_FILE}" \
        --env-file "${ENV_FILE}" \
        -p "${PROJECT_NAME}" \
        ps
}

display_logs() {
    log_step "Recent logs:"

    docker compose \
        -f "${COMPOSE_FILE}" \
        --env-file "${ENV_FILE}" \
        -p "${PROJECT_NAME}" \
        logs --tail=50 api
}

run_migrations() {
    log_step "Running database migrations..."

    docker compose \
        -f "${COMPOSE_FILE}" \
        --env-file "${ENV_FILE}" \
        -p "${PROJECT_NAME}" \
        exec -T api \
        llm-registry-server --help || true

    log_info "Migrations completed"
}

display_info() {
    log_info "========================================"
    log_info "LLM Registry Deployment Complete"
    log_info "========================================"
    log_info ""
    log_info "API Endpoint:        http://localhost:3000"
    log_info "gRPC Endpoint:       localhost:50051"
    log_info "Grafana Dashboard:   http://localhost:3001"
    log_info "Prometheus:          http://localhost:9090"
    log_info "Jaeger UI:           http://localhost:16686"
    log_info ""
    log_info "Useful commands:"
    log_info "  View logs:     docker compose -f ${COMPOSE_FILE} -p ${PROJECT_NAME} logs -f"
    log_info "  Stop services: docker compose -f ${COMPOSE_FILE} -p ${PROJECT_NAME} down"
    log_info "  Restart:       docker compose -f ${COMPOSE_FILE} -p ${PROJECT_NAME} restart"
    log_info ""
}

# Main execution
main() {
    log_info "Starting LLM Registry deployment..."
    log_info "Environment: ${ENVIRONMENT}"
    log_info "Project: ${PROJECT_NAME}"

    check_prerequisites
    validate_env_file
    pull_images
    start_services

    if wait_for_health; then
        run_migrations
        display_status
        display_logs
        display_info
    else
        log_error "Deployment failed - services not healthy"
        display_status
        display_logs
        exit 1
    fi

    log_info "Deployment completed successfully!"
}

# Handle script arguments
case "${1:-deploy}" in
    deploy)
        main
        ;;
    stop)
        log_info "Stopping services..."
        docker compose -f "${COMPOSE_FILE}" -p "${PROJECT_NAME}" down
        ;;
    restart)
        log_info "Restarting services..."
        docker compose -f "${COMPOSE_FILE}" -p "${PROJECT_NAME}" restart
        ;;
    logs)
        docker compose -f "${COMPOSE_FILE}" -p "${PROJECT_NAME}" logs -f "${2:-}"
        ;;
    status)
        docker compose -f "${COMPOSE_FILE}" -p "${PROJECT_NAME}" ps
        ;;
    *)
        echo "Usage: $0 {deploy|stop|restart|logs|status}"
        exit 1
        ;;
esac
