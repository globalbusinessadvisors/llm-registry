#!/bin/bash
#
# LLM Registry Helm Uninstallation Script
#
# This script safely uninstalls the LLM Registry Helm chart
#
# Usage:
#   ./scripts/helm-uninstall.sh [options]
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
DELETE_NAMESPACE=false
DELETE_PVC=false

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
LLM Registry Helm Uninstallation Script

Usage:
  ./scripts/helm-uninstall.sh [options]

Options:
  -n, --namespace <namespace>     Kubernetes namespace (default: llm-registry)
  -r, --release <name>            Release name (default: llm-registry)
  --delete-namespace              Delete the namespace after uninstall
  --delete-pvc                    Delete PersistentVolumeClaims (WARNING: deletes data!)
  -h, --help                      Show this help message

Examples:
  # Uninstall the release
  ./scripts/helm-uninstall.sh

  # Uninstall and delete namespace
  ./scripts/helm-uninstall.sh --delete-namespace

  # Uninstall and delete all data
  ./scripts/helm-uninstall.sh --delete-namespace --delete-pvc

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
        --delete-namespace)
            DELETE_NAMESPACE=true
            shift
            ;;
        --delete-pvc)
            DELETE_PVC=true
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
    log_info "Starting LLM Registry Helm uninstallation"
    log_info "Release: ${RELEASE_NAME}, Namespace: ${NAMESPACE}"

    # Check if Helm is installed
    if ! command -v helm &> /dev/null; then
        log_error "Helm is not installed"
        exit 1
    fi

    # Check if release exists
    if ! helm list -n "${NAMESPACE}" | grep -q "${RELEASE_NAME}"; then
        log_warn "Release ${RELEASE_NAME} not found in namespace ${NAMESPACE}"
        exit 0
    fi

    # Show current status
    log_info "Current release status:"
    helm status "${RELEASE_NAME}" -n "${NAMESPACE}"

    # Confirm uninstallation
    echo ""
    log_warn "This will uninstall the LLM Registry release: ${RELEASE_NAME}"
    if [ "${DELETE_NAMESPACE}" = true ]; then
        log_warn "The namespace ${NAMESPACE} will be deleted!"
    fi
    if [ "${DELETE_PVC}" = true ]; then
        log_warn "All PersistentVolumeClaims will be deleted! This will permanently delete all data!"
    fi
    echo ""
    read -p "Are you sure you want to continue? (yes/no): " -r
    if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
        log_info "Uninstallation cancelled"
        exit 0
    fi

    # Uninstall the release
    log_info "Uninstalling Helm release..."
    if helm uninstall "${RELEASE_NAME}" -n "${NAMESPACE}" --wait; then
        log_success "Helm release uninstalled successfully"
    else
        log_error "Failed to uninstall Helm release"
        exit 1
    fi

    # Delete PVCs if requested
    if [ "${DELETE_PVC}" = true ]; then
        log_info "Deleting PersistentVolumeClaims..."
        kubectl delete pvc -n "${NAMESPACE}" -l app.kubernetes.io/name=llm-registry || true
        log_success "PersistentVolumeClaims deleted"
    fi

    # Delete namespace if requested
    if [ "${DELETE_NAMESPACE}" = true ]; then
        log_info "Deleting namespace: ${NAMESPACE}"
        kubectl delete namespace "${NAMESPACE}" --wait=true || true
        log_success "Namespace deleted"
    fi

    log_success "Uninstallation complete!"
}

# Run main function
main
