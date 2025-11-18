#!/bin/bash
#
# LLM Registry Helm Management Script
#
# This script provides various management operations for the LLM Registry Helm chart
#
# Usage:
#   ./scripts/helm-manage.sh <command> [options]
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
LLM Registry Helm Management Script

Usage:
  ./scripts/helm-manage.sh <command> [options]

Commands:
  status          Show release status
  list            List all releases
  logs            Show application logs
  describe        Describe pods
  port-forward    Set up port forwarding
  rollback        Rollback to previous version
  history         Show release history
  test            Run Helm tests
  lint            Lint the Helm chart
  template        Render templates
  diff            Show diff between current and new release

Options:
  -n, --namespace <namespace>     Kubernetes namespace (default: llm-registry)
  -r, --release <name>            Release name (default: llm-registry)
  -h, --help                      Show this help message

Examples:
  # Show status
  ./scripts/helm-manage.sh status

  # View logs
  ./scripts/helm-manage.sh logs

  # Port forward to local
  ./scripts/helm-manage.sh port-forward

  # Rollback to previous version
  ./scripts/helm-manage.sh rollback

EOF
}

# Parse common options
parse_options() {
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
            -h|--help)
                show_help
                exit 0
                ;;
            *)
                shift
                ;;
        esac
    done
}

# Command functions
cmd_status() {
    log_info "Getting release status..."
    helm status "${RELEASE_NAME}" -n "${NAMESPACE}"
}

cmd_list() {
    log_info "Listing all releases in namespace ${NAMESPACE}..."
    helm list -n "${NAMESPACE}"
}

cmd_logs() {
    log_info "Fetching logs..."
    kubectl logs -n "${NAMESPACE}" \
        -l app.kubernetes.io/name=llm-registry \
        --tail=100 \
        --follow
}

cmd_describe() {
    log_info "Describing pods..."
    kubectl describe pod -n "${NAMESPACE}" \
        -l app.kubernetes.io/name=llm-registry
}

cmd_port_forward() {
    log_info "Setting up port forwarding..."
    log_info "HTTP API will be available at http://localhost:3000"
    log_info "gRPC API will be available at grpc://localhost:50051"
    log_info "Press Ctrl+C to stop"

    kubectl port-forward -n "${NAMESPACE}" \
        svc/"${RELEASE_NAME}" \
        3000:3000 \
        50051:50051
}

cmd_rollback() {
    log_warn "Rolling back to previous version..."
    read -p "Are you sure you want to rollback? (yes/no): " -r
    if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
        log_info "Rollback cancelled"
        exit 0
    fi

    helm rollback "${RELEASE_NAME}" -n "${NAMESPACE}" --wait
    log_success "Rollback complete"
}

cmd_history() {
    log_info "Showing release history..."
    helm history "${RELEASE_NAME}" -n "${NAMESPACE}"
}

cmd_test() {
    cd "${PROJECT_ROOT}"
    log_info "Running Helm tests..."
    helm test "${RELEASE_NAME}" -n "${NAMESPACE}"
}

cmd_lint() {
    cd "${PROJECT_ROOT}"
    log_info "Linting Helm chart..."
    helm lint "${CHART_DIR}"
    log_success "Lint successful"
}

cmd_template() {
    cd "${PROJECT_ROOT}"
    log_info "Rendering templates..."
    helm template "${RELEASE_NAME}" "${CHART_DIR}" \
        --namespace "${NAMESPACE}"
}

cmd_diff() {
    cd "${PROJECT_ROOT}"

    if ! command -v helm-diff &> /dev/null; then
        log_error "helm-diff plugin not installed"
        log_info "Install with: helm plugin install https://github.com/databus23/helm-diff"
        exit 1
    fi

    log_info "Showing diff..."
    helm diff upgrade "${RELEASE_NAME}" "${CHART_DIR}" \
        --namespace "${NAMESPACE}"
}

# Main execution
main() {
    if [ $# -eq 0 ]; then
        show_help
        exit 1
    fi

    COMMAND=$1
    shift

    # Parse common options
    parse_options "$@"

    # Execute command
    case $COMMAND in
        status)
            cmd_status
            ;;
        list)
            cmd_list
            ;;
        logs)
            cmd_logs
            ;;
        describe)
            cmd_describe
            ;;
        port-forward|pf)
            cmd_port_forward
            ;;
        rollback)
            cmd_rollback
            ;;
        history)
            cmd_history
            ;;
        test)
            cmd_test
            ;;
        lint)
            cmd_lint
            ;;
        template)
            cmd_template
            ;;
        diff)
            cmd_diff
            ;;
        help|-h|--help)
            show_help
            ;;
        *)
            log_error "Unknown command: $COMMAND"
            show_help
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
