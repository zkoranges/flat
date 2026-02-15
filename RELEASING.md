# Release Process

This document describes how to release a new version of flat.

## Prerequisites

- Maintainer access to the GitHub repository
- crates.io publish token (already configured)
- Homebrew tap token (already configured in GitHub Secrets)
- All tests passing locally

## Quick Release (Recommended)

The simplest way to release:

```bash
./release 0.5.0
```

The script will:
1. ✅ Run tests (validation)
2. ✅ Commit all changes with changelog
3. ✅ Create git tag
4. ✅ Push to GitHub
5. ✅ Trigger GitHub Actions

Then GitHub Actions automatically:
1. ✅ Builds binaries (macOS Intel, macOS ARM, Linux, Windows)
2. ✅ Runs test suite
3. ✅ Creates GitHub release with binaries
4. ✅ Updates Homebrew formula

**Total time:** ~2 minutes for script + 10 minutes for CI/CD

## Manual Release Process

If you prefer step-by-step control:

### 1. Prepare the release

```bash
# Ensure working directory is clean
git status

# Update version in Cargo.toml (if needed)
vim Cargo.toml  # Change version = "0.4.0" to "0.5.0"

# Update Cargo.lock
cargo build --release

# Run all checks
cargo test --all
cargo clippy --all-targets -- -D warnings
```

### 2. Commit changes

```bash
git add -A
git commit -m "Release v0.5.0

- Feature 1
- Feature 2
- Bug fix 3

Co-Authored-By: Claude Haiku 4.5 <noreply@anthropic.com>"
```

### 3. Create and push tag

```bash
# Create annotated tag
git tag -a v0.5.0 -m "v0.5.0

- Feature 1
- Feature 2
- Bug fix 3"

# Push commit and tag
git push origin main
git push origin v0.5.0
```

### 4. Monitor GitHub Actions

1. Go to https://github.com/zkoranges/flat/actions
2. Watch the "Release" workflow
3. Verify all jobs succeed:
   - Build (macOS, Linux, Windows)
   - Tests
   - Homebrew formula update

### 5. Verify the release

```bash
# Check GitHub release page
open https://github.com/zkoranges/flat/releases/tag/v0.5.0

# Test crates.io (may take a few minutes to sync)
cargo install flat-cli --force

# Test Homebrew (formula auto-updated)
brew upgrade flat
```

## Release Checklist

Before releasing:
- [ ] All features complete and tested
- [ ] README.md updated with new features (if applicable)
- [ ] ROADMAP.md updated (if roadmap changed)
- [ ] Version bumped in Cargo.toml (if not auto-done)
- [ ] Tests passing (`cargo test --all`)
- [ ] Clippy clean (`cargo clippy --all-targets -- -D warnings`)

After releasing:
- [ ] GitHub Actions passed (check Actions tab)
- [ ] GitHub release created with binaries
- [ ] crates.io updated (https://crates.io/crates/flat-cli)
- [ ] Homebrew formula updated
- [ ] Download and test at least one binary
- [ ] Announce release (optional: Twitter, Reddit, etc.)

## Troubleshooting

### GitHub Actions fails

Check the Actions tab for error messages. Common issues:
- **Test failures:** Run locally first with `cargo test --all`
- **Clippy warnings:** Run `cargo clippy --all-targets -- -D warnings`
- **Cross-compilation errors:** Check the build matrix in `.github/workflows/release.yml`

**Fix:** Commit and push the fix, then retry release

### Homebrew formula update fails

- Check `HOMEBREW_TAP_TOKEN` secret is set in GitHub
- Verify the tap repository exists: https://github.com/zkoranges/homebrew-tap
- Formula may need manual update if automation fails

### crates.io publish fails

- For v0.4.0: Already published to crates.io
- For subsequent versions: Published automatically via GitHub Actions
- Manual publish: `cargo publish` (requires token: `cargo login`)

## Release Cadence

- **Patch releases** (0.4.1): Bug fixes, every 1-2 weeks as needed
- **Minor releases** (0.5.0): New features, every 4-8 weeks
- **Major releases** (1.0.0): Breaking changes or major milestones

## Version Numbering

Follow Semantic Versioning (semver):
- **MAJOR**: Breaking API/CLI changes (1.0.0 → 2.0.0)
- **MINOR**: New features, backwards compatible (0.4.0 → 0.5.0)
- **PATCH**: Bug fixes, backwards compatible (0.4.0 → 0.4.1)

## Platform Support

Current targets (auto-built via GitHub Actions):
- `x86_64-apple-darwin` (macOS Intel)
- `aarch64-apple-darwin` (macOS Apple Silicon)
- `x86_64-unknown-linux-gnu` (Linux)
- `x86_64-pc-windows-msvc` (Windows)

Binaries available via:
- Direct download: GitHub Releases
- Homebrew: `brew install flat`
- Cargo: `cargo install flat-cli`

## Installation Instructions for Users

**Homebrew (macOS):**
```bash
brew install zkoranges/tap/flat
```

**Cargo (all platforms):**
```bash
cargo install flat-cli
```

**Binary download:**
Visit https://github.com/zkoranges/flat/releases

## CI/CD Details

Release workflow (`.github/workflows/release.yml`):
1. Triggered by git tag push (v*.*.*)
2. Builds for all platforms
3. Runs tests
4. Creates GitHub release
5. Updates Homebrew formula via HOMEBREW_TAP_TOKEN
6. Auto-publishes to crates.io (if configured)

Credentials in GitHub Secrets:
- `HOMEBREW_TAP_TOKEN` - For auto-updating Homebrew formula
- `CARGO_REGISTRY_TOKEN` - For crates.io publish (if applicable)
