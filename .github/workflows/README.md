# CI/CD Pipeline Configuration

## GitHub Secrets

The CI/CD pipeline requires the following secrets to be configured in your GitHub repository settings:

### Authentication (Required for Integration Tests)

**`AUTH0_DOMAIN`**
- Description: Your Auth0 tenant domain
- Example: `dev-abc123.us.auth0.com`
- Used for: JWT token verification in integration tests
- How to find: Auth0 Dashboard → Applications → {Your App} → Settings → Domain

**`AUTH0_AUDIENCE`**
- Description: Your Auth0 audience identifier
- Example: `https://api.traveling-cook.example.com`
- Used for: JWT audience validation in integration tests
- How to find: Auth0 Dashboard → APIs → {Your API} → Identifier

### Optional Secrets

**`DOCKER_REGISTRY_TOKEN`** (if using private Docker registry)
- Currently uses `GITHUB_TOKEN` automatically for ghcr.io
- Only needed if pushing to alternative registries

## Environment Variables for Local Development

Create a `.env` file in the project root:

```bash
# Database
DATABASE_URL=postgresql://postgres:mysecretpassword@localhost:5432/postgres

# Auth0
AUTH0_DOMAIN=dev-abc123.us.auth0.com
AUTH0_AUDIENCE=https://api.traveling-cook.example.com

# Logging
RUST_LOG=info

# Server
ADDR=0.0.0.0:3000
```

## How to Set GitHub Secrets

1. Go to your GitHub repository
2. Settings → Secrets and variables → Actions
3. Click "New repository secret"
4. Add each secret with its value

## Workflow Branches

### Main Branch (`main`)
- ✅ Runs full pipeline: lint, tests, builds, docker
- ✅ Creates release with artifacts
- ✅ Tags docker image as `latest`
- ✅ Automatic versioning: `v0.{commit-count}.0`

### Feature Branches (`feature/*`)
- ✅ Runs full pipeline
- ✅ Creates pre-release with `prerelease: true` flag
- ✅ Tags docker image with branch name and commit SHA
- ✅ Version format: `v0.0.0-{branch-name}.{short-sha}`

### Pull Requests
- ✅ Runs tests and linting
- ❌ Does NOT create releases
- ❌ Does NOT build Docker images

## Artifacts & Releases

### Binary Artifacts (created for all branches)
- `tcc_api-{version}-linux-x86_64` - Linux x86_64
- `tcc_api-{version}-linux-arm64` - Linux ARM64
- `tcc_api-{version}-macos-x86_64` - macOS x86_64
- `tcc_api-{version}-macos-arm64` - macOS ARM64
- `tcc_api-{version}-windows-x86_64.exe` - Windows x86_64

### Docker Images

**Main branch (production):**
```bash
ghcr.io/{owner}/{repo}:{version}
ghcr.io/{owner}/{repo}:latest
```

**Feature branches (pre-release):**
```bash
ghcr.io/{owner}/{repo}:{version}-{branch}.{sha}
```

## Running Locally

### Build Docker Image Locally
```bash
docker build -t tcc-api:local .
```

### Run with Docker Compose (for testing)
```bash
docker-compose -f docker-compose.dev.yml up
```

### Build Binaries Locally

**Linux x86_64:**
```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

**macOS ARM64:**
```bash
cargo build --release --target aarch64-apple-darwin
```

**Windows x86_64:**
```bash
cargo build --release --target x86_64-pc-windows-msvc
```

## Troubleshooting

### Tests Fail on GitHub but Pass Locally

1. Check Auth0 credentials in secrets
2. Verify `AUTH0_DOMAIN` and `AUTH0_AUDIENCE` match your tenant
3. Ensure the test user has proper permissions

### Docker Build Fails

1. Check Dockerfile syntax: `docker build .`
2. Ensure migrations are in `./migrations/`
3. Verify Rust dependencies are available

### Release Not Created

1. Check that push is to `main` branch
2. Verify all previous jobs passed (lint, tests, builds)
3. Check GitHub Actions permissions allow releases

## Performance Optimization

### Cargo Caching
- The workflow caches `~/.cargo` and `target` directory
- Significantly speeds up builds (5-10x faster on second run)
- Cache key includes `Cargo.lock` hash

### Docker Layer Caching
- Uses buildx for multi-platform builds
- Caches intermediate layers in GitHub Container Registry
- Reduces build time on subsequent runs

## Manual Workflows

To manually trigger a workflow run:

```bash
gh workflow run ci-cd.yml
```

To manually trigger against a specific branch:

```bash
gh workflow run ci-cd.yml -r feature/my-feature
```

## Version Numbering Scheme

### Main Branch
Semantic versioning based on commit count:
- Format: `v0.{total-commits}.0`
- Example: `v0.1234.0`

### Feature Branches
Pre-release versioning:
- Format: `v0.0.0-{branch-name}.{commit-sha}`
- Example: `v0.0.0-feature-awesome.a1b2c3d`

### Git Tags
If you want to force a specific version, push a tag:

```bash
git tag v1.0.0
git push origin v1.0.0
```

The workflow will detect the tag and use it as the version.

## Security Considerations

1. **Secrets are masked** in workflow logs
2. **GITHUB_TOKEN** is automatically scoped and expires after the workflow
3. **Docker images** are pushed to GitHub Container Registry (private by default)
4. **Cross-compilation** uses official Rust toolchains (no custom tools)

## Next Steps

1. Add the required secrets to your GitHub repository
2. Push a commit to trigger the workflow
3. Monitor the workflow run in GitHub Actions
4. Check the "Releases" page for created artifacts

For more information, see:
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Rust GitHub Actions Setup](https://github.com/dtolnay/rust-toolchain)
- [Docker Buildx Documentation](https://docs.docker.com/build/architecture/)
