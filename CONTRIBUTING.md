# Contributing to LLM Registry

Thank you for your interest in contributing to the LLM Registry! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Documentation](#documentation)
- [Submitting Changes](#submitting-changes)
- [Review Process](#review-process)
- [Community](#community)

---

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inspiring community for all. Please be respectful and constructive in all interactions.

### Standards

**Positive behaviors**:
- Using welcoming and inclusive language
- Being respectful of differing viewpoints
- Gracefully accepting constructive criticism
- Focusing on what is best for the community

**Unacceptable behaviors**:
- Trolling, insulting/derogatory comments, and personal attacks
- Public or private harassment
- Publishing others' private information
- Other conduct which could reasonably be considered inappropriate

### Enforcement

Instances of abusive, harassing, or otherwise unacceptable behavior may be reported to the project team at conduct@llm-registry.dev.

---

## Getting Started

### Prerequisites

Before you begin, ensure you have:

- **Rust 1.75+**: Install from [rustup.rs](https://rustup.rs/)
- **Docker & Docker Compose**: For running infrastructure
- **Git**: For version control
- **PostgreSQL client tools** (optional): For database management
- **A GitHub account**: For submitting changes

### Fork and Clone

1. **Fork the repository** on GitHub
2. **Clone your fork**:
   ```bash
   git clone https://github.com/YOUR_USERNAME/llm-registry.git
   cd llm-registry
   ```

3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/llm-devops/llm-registry.git
   ```

4. **Verify remotes**:
   ```bash
   git remote -v
   # origin    https://github.com/YOUR_USERNAME/llm-registry.git (fetch)
   # origin    https://github.com/YOUR_USERNAME/llm-registry.git (push)
   # upstream  https://github.com/llm-devops/llm-registry.git (fetch)
   # upstream  https://github.com/llm-devops/llm-registry.git (push)
   ```

### Development Setup

1. **Start infrastructure**:
   ```bash
   docker-compose up -d postgres redis nats
   ```

2. **Run database migrations**:
   ```bash
   cargo install sqlx-cli
   sqlx migrate run
   ```

3. **Build the project**:
   ```bash
   cargo build
   ```

4. **Run tests**:
   ```bash
   cargo test --workspace
   ```

5. **Run the server**:
   ```bash
   cargo run --bin llm-registry-server
   ```

### IDE Setup

**VS Code** (recommended):
```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy",
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

**IntelliJ IDEA**:
- Install Rust plugin
- Enable Cargo check on save
- Enable format on save

---

## Development Workflow

### Branching Strategy

We use the **Git Flow** branching model:

- `main` - Production-ready code
- `develop` - Integration branch for features
- `feature/*` - New features
- `bugfix/*` - Bug fixes
- `hotfix/*` - Urgent production fixes
- `release/*` - Release preparation

### Creating a Feature Branch

```bash
# Update your local main/develop
git checkout develop
git pull upstream develop

# Create feature branch
git checkout -b feature/your-feature-name

# Make your changes
# ...

# Commit your changes
git add .
git commit -m "feat: add amazing feature"

# Push to your fork
git push origin feature/your-feature-name
```

### Commit Message Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

**Format**:
```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types**:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Build process or auxiliary tool changes
- `ci`: CI/CD changes

**Examples**:
```
feat(api): add asset search endpoint

Implements full-text search for assets using name, description, and tags.
Includes pagination and filtering support.

Closes #123
```

```
fix(db): resolve connection pool exhaustion

Fixed connection leak in asset repository by properly closing
connections in error paths.

Fixes #456
```

```
docs(architecture): add deployment diagrams

Added Kubernetes deployment architecture diagrams to help
with production deployment planning.
```

### Keeping Your Fork Updated

```bash
# Fetch upstream changes
git fetch upstream

# Merge upstream/develop into your develop
git checkout develop
git merge upstream/develop

# Rebase your feature branch
git checkout feature/your-feature
git rebase develop
```

---

## Coding Standards

### Rust Style Guide

Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/):

**Naming Conventions**:
```rust
// Types: UpperCamelCase
struct AssetRepository;
enum AssetType;
trait AssetService;

// Functions and variables: snake_case
fn register_asset() {}
let asset_id = "123";

// Constants: SCREAMING_SNAKE_CASE
const MAX_ASSETS: usize = 1000;

// Lifetimes: short, lowercase
fn process<'a>(data: &'a str) {}
```

**Code Organization**:
```rust
// 1. Imports
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// 2. Type definitions
pub struct Asset {
    // fields
}

// 3. Trait implementations
impl Asset {
    pub fn new() -> Self {
        // ...
    }
}

// 4. Trait implementations
impl Display for Asset {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // ...
    }
}

// 5. Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_creation() {
        // ...
    }
}
```

**Error Handling**:
```rust
// ✅ Use Result for recoverable errors
pub fn register_asset(req: Request) -> Result<Asset, ServiceError> {
    let validated = validate_request(req)?;
    let asset = create_asset(validated)?;
    Ok(asset)
}

// ❌ Don't use unwrap() in production code
let value = map.get("key").unwrap(); // Bad!

// ✅ Use proper error handling
let value = map.get("key")
    .ok_or(Error::KeyNotFound)?;

// ✅ Or use expect() with meaningful message
let value = map.get("key")
    .expect("Key must exist after validation");
```

**Documentation**:
```rust
/// Registers a new asset in the registry.
///
/// # Arguments
///
/// * `request` - The asset registration request
///
/// # Returns
///
/// Returns the created `Asset` on success, or a `ServiceError` on failure.
///
/// # Errors
///
/// This function will return an error if:
/// * The asset name is invalid
/// * The version already exists
/// * Database operation fails
///
/// # Examples
///
/// ```
/// let request = RegisterAssetRequest::new("model-v1", "1.0.0");
/// let asset = service.register_asset(request).await?;
/// assert_eq!(asset.metadata.name, "model-v1");
/// ```
pub async fn register_asset(
    &self,
    request: RegisterAssetRequest,
) -> Result<Asset, ServiceError> {
    // implementation
}
```

### Code Quality Tools

**Format code**:
```bash
cargo fmt --all
```

**Lint code**:
```bash
# Run clippy
cargo clippy --workspace -- -D warnings

# Run clippy with all features
cargo clippy --workspace --all-features -- -D warnings
```

**Check for common mistakes**:
```bash
# Unused dependencies
cargo machete

# Security vulnerabilities
cargo audit

# Outdated dependencies
cargo outdated
```

### Performance Guidelines

1. **Use appropriate data structures**:
   ```rust
   // ✅ Use HashMap for O(1) lookups
   let mut cache: HashMap<AssetId, Asset> = HashMap::new();

   // ❌ Don't use Vec for frequent lookups
   let cache: Vec<Asset> = vec![]; // O(n) lookup
   ```

2. **Avoid unnecessary allocations**:
   ```rust
   // ✅ Use &str when you don't need ownership
   fn process(name: &str) { }

   // ❌ Don't use String unnecessarily
   fn process(name: String) { } // Requires allocation
   ```

3. **Use iterators efficiently**:
   ```rust
   // ✅ Chain iterator operations
   let result: Vec<_> = assets
       .iter()
       .filter(|a| a.status == Active)
       .map(|a| a.id.clone())
       .collect();

   // ❌ Don't create intermediate collections
   let active: Vec<_> = assets.iter()
       .filter(|a| a.status == Active)
       .collect();
   let ids: Vec<_> = active.iter()
       .map(|a| a.id.clone())
       .collect();
   ```

---

## Testing

### Test Organization

```
crates/
└── llm-registry-core/
    ├── src/
    │   ├── lib.rs
    │   └── asset.rs           # Implementation
    └── tests/
        └── asset_tests.rs     # Integration tests

tests/                          # Workspace integration tests
├── common/
│   ├── mod.rs
│   └── fixtures.rs
├── api_tests.rs
└── auth_tests.rs
```

### Writing Tests

**Unit Tests**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_creation() {
        let asset = Asset::new(
            AssetId::new(),
            AssetType::Model,
            metadata,
            storage,
            checksum,
        ).unwrap();

        assert_eq!(asset.asset_type, AssetType::Model);
        assert_eq!(asset.status, AssetStatus::Active);
    }

    #[test]
    fn test_invalid_asset() {
        let result = Asset::new(
            AssetId::new(),
            AssetType::Model,
            invalid_metadata,
            storage,
            checksum,
        );

        assert!(result.is_err());
    }
}
```

**Async Tests**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_asset_registration() {
        let service = create_test_service().await;
        let request = create_test_request();

        let result = service.register_asset(request).await;

        assert!(result.is_ok());
        let asset = result.unwrap();
        assert_eq!(asset.metadata.name, "test-model");
    }
}
```

**Integration Tests**:
```rust
// tests/api_tests.rs
use common::TestApp;

#[tokio::test]
async fn test_api_asset_creation() {
    let app = TestApp::new().await;
    let client = app.client();
    let token = app.generate_token("testuser");

    let response = client
        .post(&format!("{}/v1/assets", app.url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&test_asset_request())
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 201);
}
```

### Test Coverage

**Run tests with coverage**:
```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --workspace --out Html --output-dir coverage

# View report
open coverage/index.html
```

**Coverage targets**:
- **Overall**: 80%+
- **Core domain logic**: 90%+
- **API handlers**: 70%+

---

## Documentation

### Code Documentation

**Document all public APIs**:
```rust
/// The main service for asset registration.
///
/// This service coordinates validation, dependency resolution,
/// and persistence of assets.
pub struct RegistrationService {
    // fields
}
```

**Add examples for complex functions**:
```rust
/// Resolves dependencies for an asset.
///
/// # Examples
///
/// ```
/// let deps = service.resolve_dependencies(&asset).await?;
/// for dep in deps {
///     println!("Dependency: {}", dep.name);
/// }
/// ```
pub async fn resolve_dependencies(&self, asset: &Asset) -> Result<Vec<Asset>> {
    // implementation
}
```

### User Documentation

When adding features, update:
- **API Reference** (`docs/API_REFERENCE.md`)
- **Architecture** (`docs/ARCHITECTURE.md`)
- **README** (if adding major features)
- **Examples** (in `examples/` directory)

### Changelog

Add entries to `CHANGELOG.md`:

```markdown
## [Unreleased]

### Added
- Asset search endpoint with full-text search (#123)

### Changed
- Improved error messages for validation failures (#124)

### Fixed
- Connection pool exhaustion under high load (#125)
```

---

## Submitting Changes

### Pre-Submission Checklist

Before submitting a pull request, ensure:

- [ ] Code builds successfully (`cargo build`)
- [ ] All tests pass (`cargo test --workspace`)
- [ ] Code is formatted (`cargo fmt --all`)
- [ ] Lints pass (`cargo clippy --workspace -- -D warnings`)
- [ ] Documentation is updated
- [ ] CHANGELOG.md is updated
- [ ] Commit messages follow conventions
- [ ] Branch is up-to-date with develop

### Creating a Pull Request

1. **Push your changes**:
   ```bash
   git push origin feature/your-feature
   ```

2. **Create PR on GitHub**:
   - Go to your fork on GitHub
   - Click "New Pull Request"
   - Select `llm-devops/llm-registry:develop` as base
   - Select `YOUR_USERNAME/llm-registry:feature/your-feature` as compare

3. **Fill in PR template**:
   ```markdown
   ## Description
   Brief description of your changes

   ## Type of Change
   - [ ] Bug fix
   - [ ] New feature
   - [ ] Breaking change
   - [ ] Documentation update

   ## Testing
   - [ ] Unit tests added/updated
   - [ ] Integration tests added/updated
   - [ ] Manual testing performed

   ## Checklist
   - [ ] Code builds and tests pass
   - [ ] Documentation updated
   - [ ] CHANGELOG updated
   - [ ] Commits follow convention

   Closes #issue_number
   ```

---

## Review Process

### What to Expect

1. **Automated Checks**: CI/CD pipeline runs tests, lints, and builds
2. **Code Review**: Maintainers review your code
3. **Feedback**: You may be asked to make changes
4. **Approval**: Once approved, your PR will be merged

### Review Timeline

- **Initial Response**: Within 2 business days
- **Full Review**: Within 5 business days
- **Merge**: After approval and passing all checks

### Addressing Feedback

```bash
# Make requested changes
git add .
git commit -m "refactor: address review feedback"

# Push updates
git push origin feature/your-feature

# PR will automatically update
```

---

## Community

### Communication Channels

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: Questions and general discussion
- **Slack** (coming soon): Real-time chat
- **Monthly Community Call**: Last Friday of each month

### Getting Help

**Before asking for help**:
1. Check the documentation
2. Search existing issues
3. Read the FAQ

**When asking for help**:
- Provide context and details
- Include error messages and logs
- Share minimal reproducible example
- Be patient and respectful

### Recognition

Contributors will be:
- Listed in `CONTRIBUTORS.md`
- Mentioned in release notes
- Invited to contributor events

---

## Additional Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Tokio Documentation](https://tokio.rs/tokio/tutorial)
- [Axum Documentation](https://docs.rs/axum/)

---

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (Apache-2.0 OR MIT).

---

Thank you for contributing to LLM Registry! 🚀

Questions? Email: contrib@llm-registry.dev
