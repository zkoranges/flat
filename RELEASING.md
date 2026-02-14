# Release Guide

This guide explains how to create a new release of `flat` with automated binary builds.

## Prerequisites

- Push access to the repository
- All tests passing locally
- Changes merged to `main` branch

## Release Process

### 1. Update Version

Update version in `Cargo.toml`:

```toml
[package]
name = "flat"
version = "0.2.0"  # Update this
```

### 2. Test Everything

```bash
# Run all tests
cargo test --all

# Run clippy with strict warnings
cargo clippy --all-targets -- -D warnings

# Build release binary
cargo build --release

# Test the binary
./target/release/flat --version
./target/release/flat --help
```

### 3. Commit Version Bump

```bash
git add Cargo.toml Cargo.lock
git commit -m "Bump version to X.Y.Z"
git push
```

### 4. Create and Push Tag

```bash
# Create annotated tag
git tag -a vX.Y.Z -m "Release vX.Y.Z"

# Push the tag
git push origin vX.Y.Z
```

### 5. GitHub Actions Takes Over

Once you push the tag, GitHub Actions will automatically:

1. ✅ Run all tests
2. ✅ Run clippy checks
3. ✅ Build binaries for:
   - macOS Intel (x86_64-apple-darwin)
   - macOS Apple Silicon (aarch64-apple-darwin)
4. ✅ Create a GitHub Release
5. ✅ Upload binaries to the release

The process takes about 5-10 minutes.

### 6. Verify Release

1. Go to https://github.com/zkoranges/flat/releases
2. Check that the release was created
3. Verify both binary files are attached:
   - `flat-x86_64-apple-darwin.tar.gz` (Intel Mac)
   - `flat-aarch64-apple-darwin.tar.gz` (Apple Silicon)

### 7. Test Installation

Test the quick install script:

```bash
# On a different machine or clean environment
curl -sSL https://raw.githubusercontent.com/zkoranges/flat/main/install.sh | bash

# Verify it works
flat --version
```

## Troubleshooting

### Build Failed

Check the Actions tab: https://github.com/zkoranges/flat/actions

Common issues:
- Tests failing → Fix tests and create new tag
- Clippy warnings → Fix warnings and create new tag
- Build errors → Fix code and create new tag

### Release Not Created

- Ensure tag starts with `v` (e.g., `v0.1.0`, not `0.1.0`)
- Check Actions tab for errors
- Verify `GITHUB_TOKEN` has permissions

### Binary Download Fails

- Wait 5-10 minutes for build to complete
- Check release page for binaries
- Verify binaries are attached

## Version Numbering

Follow [Semantic Versioning](https://semver.org/):

- **Major** (1.0.0): Breaking changes
- **Minor** (0.1.0): New features, backwards compatible
- **Patch** (0.0.1): Bug fixes

Examples:
- `v0.2.0` - New feature (compression, token budgets)
- `v0.2.1` - Bug fix
- `v0.3.0` - New feature
- `v1.0.0` - Stable release

## Checklist

Before releasing:

- [ ] All tests pass locally
- [ ] Clippy passes with `-D warnings`
- [ ] Version updated in `Cargo.toml`
- [ ] Changes documented (if applicable)
- [ ] Tag follows `vX.Y.Z` format
- [ ] Tag pushed to GitHub

After releasing:

- [ ] GitHub Actions completed successfully
- [ ] Release created on GitHub
- [ ] Both macOS binaries attached
- [ ] Install script tested
- [ ] Release announcement (optional)

## Notes

- The install script falls back to building from source if binaries aren't available
- Users don't need to wait for releases - they can always build from source
- Consider adding Linux and Windows builds in the future
