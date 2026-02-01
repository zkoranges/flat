# flat

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Release](https://img.shields.io/github/v/release/zkoranges/flat)](https://github.com/zkoranges/flat/releases)

A command-line tool to flatten your codebase into a single file for AI context.

**TL;DR:** Copy your entire codebase to share with AI, without secrets or binaries.

```bash
# Install
curl -sSL https://raw.githubusercontent.com/zkoranges/flat/main/install.sh | bash

# Use
flat | pbcopy                    # Copy everything
flat --include rs,toml | pbcopy  # Copy only Rust files
flat --stats                     # See what would be included
```

## Overview

`flat` recursively processes directories and consolidates your code into a single, structured output that's easy to share with AI assistants like Claude, GPT-4, or Copilot.

**Key Features:**
- Automatically respects `.gitignore` rules
- Excludes secrets (`.env` files, credentials, API keys)
- Skips binary files and build artifacts
- Supports extension-based filtering
- Fast and memory-efficient (streaming architecture)

## Why This Tool?

**"Can't I just use `find` and `cat`?"**

Yes, you can. Here's what that looks like:

```bash
# Naive approach
find . -type f -name "*.rs" -exec cat {} \;

# Better, but still incomplete
find . -type f \
  -not -path "*/target/*" \
  -not -path "*/.git/*" \
  -not -path "*/node_modules/*" \
  -not -name "*.png" \
  -not -name "*.jpg" \
  -name "*.rs" \
  -exec sh -c 'echo "=== {} ===" && cat {}' \;
```

**The problems:**
- **Boilerplate**: You need to remember all the exclusion patterns
- **No gitignore**: Have to manually list every ignored directory
- **No secret detection**: Easy to accidentally include `.env` or `credentials.json`
- **No structure**: Output is hard to parse (where does one file end?)
- **Platform-specific**: Different syntax on macOS vs Linux vs Windows
- **Error-prone**: One mistake and you leak secrets or include 10MB of dependencies

**What `flat` does differently:**
- Reads your `.gitignore` automatically (using ripgrep's battle-tested parser)
- Detects and excludes secrets by pattern matching
- Detects binary files (extension + content inspection)
- Wraps output in XML tags for clear file boundaries
- Works the same on macOS, Linux, and Windows
- Provides statistics and dry-run mode
- One command: `flat | pbcopy`

**When to use Unix commands instead:**
- You only need 2-3 specific files: `cat file1.rs file2.rs`
- You're on a server without `flat` installed
- You need a custom one-off filter that `flat` doesn't support

**When to use `flat`:**
- Sharing code with AI assistants (the primary use case)
- You want safety (automatic secret exclusion)
- You want convenience (respects `.gitignore`)
- You want it to just work

This tool exists because I got tired of manually crafting `find` commands and accidentally including `node_modules/` or `.env` files when sharing code with Claude.

## Installation

### Quick Install (macOS)

```bash
curl -sSL https://raw.githubusercontent.com/zkoranges/flat/main/install.sh | bash
```

This downloads and installs the latest version to `/usr/local/bin/`.

**Note:** If pre-built binaries aren't available yet, the script will automatically build from source (requires Rust).

### Cargo (Rust Users)

If you have Rust installed:

```bash
# Install from source
git clone https://github.com/zkoranges/flat.git
cd flat
cargo install --path .
```

### Build from Source

**Prerequisites:**
- Rust 1.75 or higher
- Cargo (comes with Rust)

```bash
git clone https://github.com/zkoranges/flat.git
cd flat
cargo build --release
```

The compiled binary will be at `target/release/flat`.

### System-Wide Installation

```bash
# After building
sudo cp target/release/flat /usr/local/bin/
```

## Quick Start

```bash
# View statistics about current directory
flat --stats

# Flatten to stdout
flat

# Copy to clipboard (macOS)
flat | pbcopy

# Save to file
flat --output codebase.txt

# Preview what would be included
flat --dry-run
```

## Usage

### Basic Commands

```bash
# Process current directory
flat

# Process specific directory
flat ./src

# Output to file
flat --output output.txt

# Show statistics only
flat --stats

# Preview files without content (dry-run)
flat --dry-run
```

### Extension Filtering

Filter files by extension (without the leading dot):

```bash
# Include only specific extensions
flat --include rs,toml,md

# Exclude specific extensions
flat --exclude test,spec,json

# Combine filters (exclude takes precedence over include)
flat --include js,jsx,ts,tsx --exclude test,spec
```

**Examples:**
```bash
# Rust project: source code and config only
flat --include rs,toml

# JavaScript/TypeScript: no tests
flat --include js,jsx,ts,tsx --exclude test,spec

# Python: exclude notebooks
flat --include py --exclude ipynb

# Documentation only
flat --include md,txt
```

### Advanced Options

```bash
# Custom file size limit (bytes)
flat --max-size 10485760  # 10MB instead of default 1MB

# Custom .gitignore file
flat --gitignore /path/to/custom/.gitignore

# Combine options
flat ./src --include rs --exclude test --output rust-src.txt
```

### Output Modes

| Mode | Command | Description |
|------|---------|-------------|
| **Normal** | `flat` | Full file contents to stdout |
| **File** | `flat --output file.txt` | Write to specified file |
| **Dry-run** | `flat --dry-run` | List files only, no content |
| **Stats** | `flat --stats` | Show summary statistics only |

## Output Format

Files are wrapped in XML-style tags:

```xml
<summary>
Total files: 45
Included: 32
Skipped: 13 (8 binary, 3 too large, 2 secrets)
</summary>

<file path="src/main.rs">
fn main() {
    println!("Hello, world!");
}
</file>

<file path="package.json">
{
  "name": "my-project",
  "version": "1.0.0"
}
</file>
```

**Why XML-style?**
- Clear file boundaries for AI parsing
- Easy to grep and search
- Human-readable

## Automatic Exclusions

### Secrets (Always Excluded)

| Pattern | Examples |
|---------|----------|
| `.env*` files | `.env`, `.env.local`, `.env.production` |
| Key files | `*.key`, `*.pem`, `*.p12`, `*.pfx` |
| SSH keys | `id_rsa`, `id_dsa`, `id_ecdsa`, `id_ed25519` |
| Credentials | `credentials.json`, `serviceAccount.json` |
| Pattern matching | Any file containing `secret`, `password`, or `credential` |

### Binary Files (Always Excluded)

| Type | Extensions |
|------|-----------|
| Images | `.png`, `.jpg`, `.jpeg`, `.gif`, `.bmp`, `.ico`, `.svg`, `.webp` |
| Media | `.mp4`, `.mp3`, `.wav`, `.avi`, `.mov`, `.flac`, `.ogg` |
| Archives | `.zip`, `.tar`, `.gz`, `.7z`, `.rar`, `.bz2`, `.xz` |
| Executables | `.exe`, `.dll`, `.so`, `.dylib`, `.bin` |
| Compiled | `.wasm`, `.class`, `.pyc`, `.o`, `.a`, `.lib` |
| Documents | `.pdf`, `.doc`, `.docx`, `.xls`, `.xlsx`, `.ppt`, `.pptx` |

### Gitignore Patterns

All patterns in `.gitignore` are respected:
- `node_modules/`
- `dist/`, `build/`, `target/`
- `*.log`
- Custom patterns

### Size Limits

Files larger than **1MB** are skipped by default.
- Configurable with `--max-size <bytes>`

## Real-World Workflows

### "Help me debug this React app"

```bash
# Check what will be shared
flat --include js,jsx,ts,tsx --stats

# Copy source to clipboard (excludes tests, node_modules, .env automatically)
flat --include js,jsx,ts,tsx --exclude test,spec,stories | pbcopy

# Paste into Claude → "Here's my React app, help me fix the routing bug"
```

### "Review my Rust code"

```bash
# Preview files
flat --include rs,toml --dry-run

# Copy just the source
flat --include rs,toml | pbcopy

# Or save for later
flat --include rs,toml --output review.txt
```

### "Explain this Python project"

```bash
# Quick stats
flat --stats

# Get all Python (auto-excludes .pyc, __pycache__, .env)
flat --include py | pbcopy

# Or exclude tests and notebooks
flat --include py --exclude test,ipynb | pbcopy
```

### "Document this API"

```bash
# Get just the docs
flat --include md --output docs.txt

# Or API routes + config
flat --include js,json --dry-run  # preview first
flat --include js,json | pbcopy
```

### Before Sharing: Check What's Included

```bash
# Always run this first
flat --stats

# See file list without content
flat --dry-run

# Verify secrets are excluded
flat --dry-run | grep -i "\.env\|secret\|credential"  # Should be empty
```

## Command Reference

```
flat [OPTIONS] [PATH]

Arguments:
  [PATH]  Directory to process (default: current directory)

Options:
      --include <EXTENSIONS>    Include only these extensions (comma-separated)
      --exclude <EXTENSIONS>    Exclude these extensions (comma-separated)
  -o, --output <FILE>           Write output to file instead of stdout
      --dry-run                 List files without content
      --stats                   Show statistics only
      --gitignore <PATH>        Use custom .gitignore file
      --max-size <BYTES>        Maximum file size [default: 1048576]
  -h, --help                    Print help
  -V, --version                 Print version
```

## Exit Codes

| Code | Meaning | Explanation |
|------|---------|-------------|
| `0` | Success | Files were processed successfully |
| `1` | Invalid arguments | Check command syntax |
| `2` | File I/O error | Permission or disk issues |
| `3` | No files matched | All files were filtered out |

## How It Works

1. **Directory Walking**: Recursively traverses directories starting from the specified path
2. **Gitignore Filtering**: Applies `.gitignore` rules using the `ignore` crate
3. **Secret Detection**: Checks filenames and patterns for sensitive data
4. **Binary Detection**:
   - Checks file extensions
   - Reads first 8KB to detect null bytes
5. **Size Filtering**: Skips files over the size limit
6. **Extension Filtering**: Applies `--include` and `--exclude` rules
7. **Output Generation**: Streams results in XML format

**Performance:**
- Streaming architecture (low memory usage)
- Early filtering (minimal disk I/O)
- Efficient for projects of any size

## Troubleshooting

### "No files matched the criteria" (Exit Code 3)

All files were filtered out. Check:

```bash
# See what's being skipped
flat --dry-run

# View statistics
flat --stats

# Try without filters
flat
```

### Expected Files Are Missing

Files may be excluded because they're:
1. In `.gitignore`
2. Secret files (`.env`, `*.key`, etc.)
3. Binary files
4. Over size limit (default 1MB)

**Debug:**
```bash
# Check what's included
flat --dry-run

# Increase size limit
flat --max-size 10485760

# Check gitignore
cat .gitignore
```

### Too Many Files Included

Add filters to narrow down:

```bash
# By extension
flat --include rs,toml,md

# Exclude unwanted
flat --exclude test,spec,generated
```

## FAQ

**Q: Does this work with monorepos?**
A: Yes. Point it at the specific package:
```bash
flat packages/backend --include ts
```

**Q: What about symlinks?**
A: Symbolic links are followed by default.

**Q: Does it work on Windows?**
A: Yes, the tool is cross-platform (uses Rust's `std::path`).

**Q: How do I exclude a specific directory?**
A: Add it to `.gitignore`, or use extension filters to select only what you want.

**Q: Can I use this in CI/CD?**
A: Yes:
```bash
flat --output codebase.txt || exit 1
```

**Q: What if I have a custom .gitignore location?**
A: Use `--gitignore <path>` to specify a custom file.

**Q: Does it handle UTF-8?**
A: Yes, all files are read as UTF-8 text.

## Technical Details

### Dependencies

- `clap` - CLI argument parsing
- `ignore` - Gitignore handling (from ripgrep)
- `walkdir` - Directory traversal
- `anyhow` - Error handling
- `content_inspector` - Binary detection

### Architecture

```
src/
├── main.rs      CLI interface and argument parsing
├── lib.rs       Public library interface
├── config.rs    Configuration management
├── filters.rs   Secret and binary detection
├── output.rs    XML formatting and statistics
└── walker.rs    Directory traversal and filtering
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_secret_exclusion

# With output
cargo test -- --nocapture

# Integration tests only
cargo test --test integration_test
```

**Test Coverage:**
- 10 unit tests
- 25 integration tests
- Tests for: secrets, binaries, gitignore, filters, output modes

### Code Quality

```bash
# Lint with clippy
cargo clippy -- -D warnings

# Format code
cargo fmt

# Check formatting
cargo fmt -- --check
```

## Development

### Project Structure

```
flat/
├── Cargo.toml
├── src/
│   ├── main.rs       # CLI entry point (62 lines)
│   ├── lib.rs        # Public API (6 lines)
│   ├── config.rs     # Config (85 lines)
│   ├── filters.rs    # Filtering (152 lines)
│   ├── output.rs     # Output (85 lines)
│   └── walker.rs     # Walking (169 lines)
├── tests/
│   ├── integration_test.rs        # 25 integration tests
│   └── fixtures/
│       ├── sample_project/        # Rust test project
│       └── js_project/            # JavaScript test project
└── README.md
```

### Running Tests

All tests must pass before committing:

```bash
cargo test --all
```

### Adding Features

1. Write tests first
2. Implement feature
3. Run `cargo test`
4. Run `cargo clippy -- -D warnings`
5. Run `cargo fmt`

## Limitations

- **Text Files Only**: Binary files are automatically skipped
- **Size Limit**: Files >1MB skipped by default (configurable)
- **UTF-8 Only**: Non-UTF-8 files will cause errors
- **Memory**: Very large single files may use significant memory when reading

## Security

**Automatic Secret Exclusion:**
- Never includes `.env` files
- Never includes credential files
- Pattern-based detection for common secrets

**Caution:**
- Always review output before sharing
- Check that `.gitignore` is configured properly
- Use `--dry-run` to preview what will be included

**Not a replacement for:**
- Proper secret management
- Security audits
- Code review

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Credits

- Built with Rust
- Uses the `ignore` crate from ripgrep for gitignore handling
- CLI powered by `clap`

---

## Best Practices

**Before you `| pbcopy`:**
```bash
# 1. Check stats first (are you about to copy 500 files?)
flat --stats

# 2. Preview the file list
flat --dry-run

# 3. Look for anything unexpected
flat --dry-run | grep -i "secret\|password\|node_modules"
```

**For large projects:**
```bash
# Don't copy the entire monorepo
flat | pbcopy  # ❌ Too much context

# Be specific about what you need
flat packages/backend --include ts,json | pbcopy  # ✅ Just the backend
flat src/components --include tsx,css | pbcopy    # ✅ Just components
```

**Common workflows:**
```bash
# The usual flow
flat --stats                          # See what you have
flat --include rs,toml --dry-run      # Preview
flat --include rs,toml | pbcopy       # Copy
# → Paste into AI

# Debug a specific directory
flat src/auth --include ts | pbcopy

# Share only API routes
flat --include ts --exclude test,spec,stories | pbcopy

# Get everything except tests
flat --exclude test,spec,mock | pbcopy
```

**Command Cheat Sheet:**
```bash
flat --stats                          # Quick overview
flat --dry-run                        # Preview files
flat --include rs,toml                # Rust project
flat --include js,jsx,ts,tsx          # React/Next.js
flat --include py --exclude test      # Python (no tests)
flat --output code.txt                # Save to file
flat | pbcopy                         # Copy to clipboard (macOS)
```
