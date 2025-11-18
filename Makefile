.PHONY: help build test clean run dev docker-up docker-down docker-logs migrate fmt lint check

# Default target
.DEFAULT_GOAL := help

# Variables
RUST_VERSION := 1.75
PROJECT_NAME := llm-registry
DOCKER_COMPOSE := docker-compose
CARGO := cargo

# Colors for output
COLOR_RESET := \033[0m
COLOR_BOLD := \033[1m
COLOR_GREEN := \033[32m
COLOR_YELLOW := \033[33m
COLOR_BLUE := \033[34m

##@ General

help: ## Display this help message
	@awk 'BEGIN {FS = ":.*##"; printf "\n$(COLOR_BOLD)Usage:$(COLOR_RESET)\n  make $(COLOR_BLUE)<target>$(COLOR_RESET)\n"} /^[a-zA-Z_-]+:.*?##/ { printf "  $(COLOR_BLUE)%-15s$(COLOR_RESET) %s\n", $$1, $$2 } /^##@/ { printf "\n$(COLOR_BOLD)%s$(COLOR_RESET)\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

##@ Development

dev: ## Run the server in development mode with auto-reload
	$(CARGO) watch -x 'run --bin llm-registry-server'

run: ## Run the server
	$(CARGO) run --bin llm-registry-server

build: ## Build the project in debug mode
	$(CARGO) build --workspace

build-release: ## Build the project in release mode
	$(CARGO) build --workspace --release

test: ## Run all tests
	$(CARGO) test --workspace

test-verbose: ## Run tests with verbose output
	$(CARGO) test --workspace -- --nocapture

check: ## Check code for errors without building
	$(CARGO) check --workspace

fmt: ## Format code using rustfmt
	$(CARGO) fmt --all

fmt-check: ## Check code formatting
	$(CARGO) fmt --all -- --check

lint: ## Run clippy linter
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings

clean: ## Clean build artifacts
	$(CARGO) clean
	rm -rf target/

##@ Database

db-migrate: ## Run database migrations
	sqlx migrate run --database-url "$$DATABASE_URL"

db-migrate-revert: ## Revert last database migration
	sqlx migrate revert --database-url "$$DATABASE_URL"

db-create: ## Create database
	sqlx database create --database-url "$$DATABASE_URL"

db-drop: ## Drop database
	sqlx database drop --database-url "$$DATABASE_URL"

db-reset: db-drop db-create db-migrate ## Reset database (drop, create, migrate)

##@ Docker

docker-build: ## Build Docker image
	docker build -t $(PROJECT_NAME):latest .

docker-up: ## Start all Docker Compose services
	$(DOCKER_COMPOSE) up -d

docker-down: ## Stop all Docker Compose services
	$(DOCKER_COMPOSE) down

docker-restart: docker-down docker-up ## Restart all Docker Compose services

docker-logs: ## Show logs from all services
	$(DOCKER_COMPOSE) logs -f

docker-logs-api: ## Show logs from API service
	$(DOCKER_COMPOSE) logs -f api

docker-ps: ## Show status of Docker Compose services
	$(DOCKER_COMPOSE) ps

docker-clean: ## Remove all containers, volumes, and images
	$(DOCKER_COMPOSE) down -v --rmi all

##@ CI/CD

ci-test: ## Run CI tests locally
	$(CARGO) test --workspace --all-features
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings
	$(CARGO) fmt --all -- --check

ci-build: ## Build release binary for CI
	$(CARGO) build --workspace --release

##@ Utilities

install-tools: ## Install development tools
	$(CARGO) install cargo-watch
	$(CARGO) install cargo-edit
	$(CARGO) install sqlx-cli --no-default-features --features postgres

update-deps: ## Update dependencies
	$(CARGO) update

outdated: ## Check for outdated dependencies
	$(CARGO) outdated

audit: ## Audit dependencies for security vulnerabilities
	$(CARGO) audit

bloat: ## Analyze binary size
	$(CARGO) bloat --release --crates

bench: ## Run benchmarks
	$(CARGO) bench --workspace

doc: ## Generate and open documentation
	$(CARGO) doc --workspace --no-deps --open

coverage: ## Generate code coverage report
	$(CARGO) tarpaulin --workspace --out Html --output-dir coverage/

##@ Monitoring

metrics: ## View Prometheus metrics
	@echo "$(COLOR_GREEN)Opening Prometheus at http://localhost:9090$(COLOR_RESET)"
	@open http://localhost:9090 2>/dev/null || xdg-open http://localhost:9090 2>/dev/null || echo "Please open http://localhost:9090 in your browser"

grafana: ## Open Grafana dashboards
	@echo "$(COLOR_GREEN)Opening Grafana at http://localhost:3001 (admin/admin)$(COLOR_RESET)"
	@open http://localhost:3001 2>/dev/null || xdg-open http://localhost:3001 2>/dev/null || echo "Please open http://localhost:3001 in your browser"

jaeger: ## Open Jaeger tracing UI
	@echo "$(COLOR_GREEN)Opening Jaeger at http://localhost:16686$(COLOR_RESET)"
	@open http://localhost:16686 2>/dev/null || xdg-open http://localhost:16686 2>/dev/null || echo "Please open http://localhost:16686 in your browser"

##@ Quick Start

setup: install-tools docker-up ## Complete setup for new development environment
	@echo "$(COLOR_GREEN)✓ Development environment ready!$(COLOR_RESET)"
	@echo "$(COLOR_YELLOW)Run 'make dev' to start the development server$(COLOR_RESET)"

quick-start: docker-up ## Quick start all services
	@echo "$(COLOR_GREEN)✓ All services started!$(COLOR_RESET)"
	@echo "  - API Server: http://localhost:3000"
	@echo "  - Grafana: http://localhost:3001 (admin/admin)"
	@echo "  - Prometheus: http://localhost:9090"
	@echo "  - Jaeger: http://localhost:16686"
