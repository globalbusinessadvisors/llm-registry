# NPM Publishing Guide

This guide covers how to publish the LLM Registry packages to npm under the `@llm-dev-ops` organization.

## Packages

The LLM Registry includes two npm packages:

1. **`@llm-dev-ops/llm-registry-sdk`** - TypeScript SDK for the LLM Registry API
2. **`@llm-dev-ops/llm-registry`** - CLI for interacting with the LLM Registry

## Prerequisites

1. **npm account**: You need an npm account with access to the `@llm-dev-ops` organization
2. **npm token**: Generate an automation token from npm for CI/CD
3. **GitHub secret**: Add `NPM_TOKEN` to your GitHub repository secrets

### Setting up npm Organization Access

```bash
# Login to npm
npm login

# Verify you have access to the organization
npm org ls llm-dev-ops
```

## Manual Publishing

### 1. Install Dependencies

```bash
npm install
```

### 2. Build Packages

```bash
npm run build
```

This will build both the SDK and CLI packages.

### 3. Publish SDK

```bash
cd packages/sdk
npm publish --access public
```

### 4. Publish CLI

```bash
cd packages/cli
npm publish --access public
```

### Or Publish Both at Once

From the root directory:

```bash
npm run publish:all
```

## Automated Publishing (GitHub Actions)

A GitHub Actions workflow is configured to automatically publish packages when you push a tag.

### 1. Add NPM Token to GitHub Secrets

1. Go to your GitHub repository settings
2. Navigate to **Secrets and variables** → **Actions**
3. Click **New repository secret**
4. Name: `NPM_TOKEN`
5. Value: Your npm automation token

### 2. Create and Push a Tag

```bash
# Create a tag for npm release
git tag npm-v0.1.0

# Push the tag
git push origin npm-v0.1.0
```

The workflow will automatically:
- Install dependencies
- Build both packages
- Publish to npm
- Create a GitHub release

### Manual Workflow Trigger

You can also manually trigger the workflow from the GitHub Actions UI.

## Version Management

Both packages use the same version number defined in their respective `package.json` files.

### Updating Versions

1. Update version in `packages/sdk/package.json`
2. Update version in `packages/cli/package.json`
3. Ensure CLI's dependency on SDK uses the correct version range
4. Commit changes
5. Create and push a tag

```bash
# Update versions
cd packages/sdk
npm version patch  # or minor, or major

cd ../cli
npm version patch

# Commit
git add .
git commit -m "chore: bump version to 0.1.1"

# Tag and push
git tag npm-v0.1.1
git push origin main --tags
```

## Testing Before Publishing

### Test SDK Locally

```bash
cd packages/sdk
npm run build
npm link

# In a test project
npm link @llm-dev-ops/llm-registry-sdk
```

### Test CLI Locally

```bash
cd packages/cli
npm run build
npm link

# Test the CLI
llm-registry --version
llm-registry health
```

### Test Package Contents

```bash
# SDK
cd packages/sdk
npm pack
tar -xvzf llm-dev-ops-llm-registry-sdk-0.1.0.tgz
cd package && ls -la

# CLI
cd packages/cli
npm pack
tar -xvzf llm-dev-ops-llm-registry-0.1.0.tgz
cd package && ls -la
```

## Package Structure

### SDK Package (`@llm-dev-ops/llm-registry-sdk`)

```
packages/sdk/
├── src/
│   └── index.ts          # Main SDK source
├── dist/                 # Built JavaScript (generated)
│   ├── index.js
│   └── index.d.ts
├── package.json
├── tsconfig.json
└── README.md
```

### CLI Package (`@llm-dev-ops/llm-registry`)

```
packages/cli/
├── src/
│   └── cli.ts            # CLI source
├── dist/                 # Built JavaScript (generated)
│   └── cli.js
├── package.json
├── tsconfig.json
└── README.md
```

## Publishing Checklist

Before publishing, ensure:

- [ ] All tests pass
- [ ] Version numbers are updated
- [ ] CHANGELOG is updated (if maintained)
- [ ] README files are up to date
- [ ] Build succeeds: `npm run build`
- [ ] No sensitive data in package
- [ ] Dependencies are correct
- [ ] `publishConfig.access` is set to `public`

## Troubleshooting

### Permission Denied

If you get a permission error:

```bash
npm ERR! code E403
npm ERR! 403 403 Forbidden - PUT https://registry.npmjs.org/@llm-dev-ops/...
```

**Solution**: Ensure you're logged in and have access to the organization:

```bash
npm login
npm org ls llm-dev-ops
```

### Package Already Exists

If the version already exists:

```bash
npm ERR! code E409
npm ERR! 409 Conflict - PUT https://registry.npmjs.org/@llm-dev-ops/...
```

**Solution**: Bump the version number:

```bash
npm version patch
```

### Build Errors

If the build fails:

```bash
# Clean and rebuild
npm run clean
npm install
npm run build
```

## Post-Publishing

After publishing:

1. Verify packages are available:
   ```bash
   npm view @llm-dev-ops/llm-registry-sdk
   npm view @llm-dev-ops/llm-registry
   ```

2. Test installation:
   ```bash
   npm install @llm-dev-ops/llm-registry-sdk
   npm install -g @llm-dev-ops/llm-registry
   ```

3. Update documentation with new version numbers

4. Announce the release (optional)

## Resources

- [npm Organizations](https://docs.npmjs.com/orgs/)
- [Publishing scoped packages](https://docs.npmjs.com/creating-and-publishing-scoped-public-packages)
- [npm version](https://docs.npmjs.com/cli/v8/commands/npm-version)
- [Semantic Versioning](https://semver.org/)
