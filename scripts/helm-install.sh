#!/bin/bash
#
# LLM Registry Helm Installation Script
#
# This script installs the LLM Registry Helm chart with proper dependency management
# and validation.
#
# Usage:
#   ./scripts/helm-install.sh [options]
#
# Options:
#   -n, --namespace <namespace>     Kubernetes namespace (default: llm-registry)
#   -r, --release <name>            Release name (default: llm-registry)
#   -f, --values <file>             Values file (default: none)
#   -e, --environment <env>         Environment (dev, staging, prod)
#   --dry-run                       Perform a dry-run
#   --debug                         Enable debug output
#   -h, --help                      Show this help message
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
NAMESPACE="llm-registry"
RELEASE_NAME="llm-registry"
VALUES_FILE=""
ENVIRONMENT=""
DRY_RUN=""
DEBUG=""
CHART_DIR="helm/llm-registry"

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

show_help() {
    cat << EOF
LLM Registry Helm Installation Script

Usage:
  ./scripts/helm-install.sh [options]

Options:
  -n, --namespace <namespace>     Kubernetes namespace (default: llm-registry)
  -r, --release <name>            Release name (default: llm-registry)
  -f, --values <file>             Values file (default: none)
  -e, --environment <env>         Environment (dev, staging, prod)
  --dry-run                       Perform a dry-run
  --debug                         Enable debug output
  -h, --help                      Show this help message

Examples:
  # Install with default values
  ./scripts/helm-install.sh

  # Install in production namespace with custom values
  ./scripts/helm-install.sh -n production -f values-prod.yaml

  # Install for development environment
  ./scripts/helm-install.sh -e dev

  # Dry-run with debug output
  ./scripts/helm-install.sh --dry-run --debug

EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -n|--namespace)
            NAMESPACE="$2"
            shift 2
            ;;
        -r|--release)
            RELEASE_NAME="$2"
            shift 2
            ;;
        -f|--values)
            VALUES_FILE="$2"
            shift 2
            ;;
        -e|--environment)
            ENVIRONMENT="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN="--dry-run"
            shift
            ;;
        --debug)
            DEBUG="--debug"
            shift
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Main execution
main() {
    log_info "Starting LLM Registry Helm installation"
    log_info "Release: ${RELEASE_NAME}, Namespace: ${NAMESPACE}"

    # Change to project root
    cd "${PROJECT_ROOT}"

    # Check if Helm is installed
    if ! command -v helm &> /dev/null; then
        log_error "Helm is not installed. Please install Helm first."
        log_info "Visit: https://helm.sh/docs/intro/install/"
        exit 1
    fi

    # Check if kubectl is installed
    if ! command -v kubectl &> /dev/null; then
        log_error "kubectl is not installed. Please install kubectl first."
        exit 1
    fi

    # Check Kubernetes connection
    log_info "Checking Kubernetes connection..."
    if ! kubectl cluster-info &> /dev/null; then
        log_error "Cannot connect to Kubernetes cluster"
        exit 1
    fi
    log_success "Connected to Kubernetes cluster"

    # Create namespace if it doesn't exist
    if ! kubectl get namespace "${NAMESPACE}" &> /dev/null; then
        log_info "Creating namespace: ${NAMESPACE}"
        kubectl create namespace "${NAMESPACE}"
        log_success "Namespace created"
    else
        log_info "Namespace ${NAMESPACE} already exists"
    fi

    # Update Helm dependencies
    log_info "Updating Helm chart dependencies..."
    helm dependency update "${CHART_DIR}"
    log_success "Dependencies updated"

    # Build values file arguments
    VALUES_ARGS=""

    # Add environment-specific values file if specified
    if [ -n "${ENVIRONMENT}" ]; then
        ENV_VALUES_FILE="${CHART_DIR}/values-${ENVIRONMENT}.yaml"
        if [ -f "${ENV_VALUES_FILE}" ]; then
            log_info "Using environment values file: ${ENV_VALUES_FILE}"
            VALUES_ARGS="${VALUES_ARGS} -f ${ENV_VALUES_FILE}"
        else
            log_warn "Environment values file not found: ${ENV_VALUES_FILE}"
        fi
    fi

    # Add custom values file if specified
    if [ -n "${VALUES_FILE}" ]; then
        if [ -f "${VALUES_FILE}" ]; then
            log_info "Using custom values file: ${VALUES_FILE}"
            VALUES_ARGS="${VALUES_ARGS} -f ${VALUES_FILE}"
        else
            log_error "Values file not found: ${VALUES_FILE}"
            exit 1
        fi
    fi

    # Lint the chart
    log_info "Linting Helm chart..."
    if helm lint "${CHART_DIR}" ${VALUES_ARGS}; then
        log_success "Helm chart validation passed"
    else
        log_error "Helm chart validation failed"
        exit 1
    fi

    # Template rendering test (only if not dry-run)
    if [ -z "${DRY_RUN}" ]; then
        log_info "Testing template rendering..."
        helm template "${RELEASE_NAME}" "${CHART_DIR}" \
            --namespace "${NAMESPACE}" \
            ${VALUES_ARGS} > /dev/null
        log_success "Template rendering successful"
    fi

    # Install or upgrade the chart
    log_info "Installing/upgrading Helm chart..."

    # Build Helm command
    HELM_CMD="helm upgrade --install ${RELEASE_NAME} ${CHART_DIR} \
        --namespace ${NAMESPACE} \
        --create-namespace \
        --timeout 10m \
        --wait \
        ${VALUES_ARGS} \
        ${DRY_RUN} \
        ${DEBUG}"

    if [ -n "${DRY_RUN}" ]; then
        log_info "Running in dry-run mode..."
    fi

    # Execute Helm command
    if eval "${HELM_CMD}"; then
        log_success "Helm chart installed successfully!"
    else
        log_error "Helm installation failed"
        exit 1
    fi

    # Show status (skip if dry-run)
    if [ -z "${DRY_RUN}" ]; then
        echo ""
        log_info "Deployment Status:"
        helm status "${RELEASE_NAME}" -n "${NAMESPACE}"

        echo ""
        log_info "Waiting for pods to be ready..."
        kubectl wait --for=condition=ready pod \
            -l app.kubernetes.io/name=llm-registry \
            -n "${NAMESPACE}" \
            --timeout=300s || true

        echo ""
        log_info "Pod Status:"
        kubectl get pods -n "${NAMESPACE}" -l app.kubernetes.io/name=llm-registry

        echo ""
        log_info "Service Status:"
        kubectl get svc -n "${NAMESPACE}" -l app.kubernetes.io/name=llm-registry

        echo ""
        log_success "Installation complete!"
        log_info "Run 'helm status ${RELEASE_NAME} -n ${NAMESPACE}' for more information"
    fi
}

# Run main function
main
