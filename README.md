# flat

[![Crates.io](https://img.shields.io/crates/v/flat.svg)](https://crates.io/crates/flat)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/zkoranges/flat)](https://github.com/zkoranges/flat/releases)

Pack an entire codebase into a single file, ready to paste into any AI.

Automatically respects `.gitignore`, strips secrets, skips binaries — and optionally **compresses source code to its signatures** so you can fit more into a context window.

```
$ flat --compress --tokens 8000 --include py

<file path="flask/app.py" mode="compressed">
from flask import Flask
import typing as t

class Flask(App):
    def route(self, rule: str, **options: t.Any) -> t.Callable:
        """Decorate a view function to register it with the given URL rule."""
        ...
    def add_url_rule(self, rule: str, endpoint: str | None = None, ...) -> None:
        ...
</file>

<summary>
Included: 24 (24 .py)
Compressed: 6 files
Token budget: 7,995 / 8,000 used
Excluded by budget: 18 files
</summary>
```

## Install

**Homebrew**

```bash
brew install zkoranges/tap/flat
```

**Cargo**

```bash
cargo install --git https://github.com/zkoranges/flat.git
```

**Binary**

```bash
curl -sSL https://raw.githubusercontent.com/zkoranges/flat/main/install.sh | bash
```

## Quick Start

```bash
flat | pbcopy                     # copy entire project to clipboard
flat src/ -o context.xml          # save to file
flat --include rs,toml | pbcopy   # just Rust files
flat --stats                      # preview size before copying
```

## Compression

`--compress` uses [tree-sitter](https://tree-sitter.github.io/) to parse source files, extract declarations and signatures, and replace function bodies with `...` or `{ ... }`. The result is a structural overview of the codebase that preserves everything an AI needs to understand the architecture.

**What's kept:** imports, type definitions, interfaces, structs, enums, class declarations, function signatures, decorators, docstrings, comments, module-level assignments.

**What's stripped:** function and method bodies (the implementation details).

**Supported languages:** Rust, TypeScript, JavaScript, Python, Go. Files in other languages pass through unmodified.

**Real-world compression:**

| Codebase | Files | Full | Compressed | Reduction |
|----------|------:|-----:|-----------:|----------:|
| [Express](https://github.com/expressjs/express) | 6 | 61 KB | 28 KB | **54%** |
| [Flask](https://github.com/pallets/flask) | 24 | 339 KB | 214 KB | **37%** |
| [Next.js](https://github.com/vercel/next.js) `packages/next/src` | 1,605 | 8.0 MB | 5.6 MB | **31%** |

Fallback is safe: parse errors, unsupported syntax, or empty output all fall back to including the original source in full. Nothing is silently dropped.

### Full-match: keep specific files uncompressed

```bash
flat --compress --full-match 'main.rs,app.py'
```

Matched files keep complete content (`mode="full"`). Everything else gets compressed (`mode="compressed"`). Useful when you want an architectural overview of the project but need full implementation detail in specific files.

## Token Budget

`--tokens` caps output to fit a context window. Files are scored by importance and packed greedily — high-value files first, low-value files dropped:

| Priority | Score | Examples |
|----------|------:|---------|
| README | 100 | `README.md`, `README.rst` |
| Entry points | 90 | `main.rs`, `index.ts`, `app.py` |
| Config | 80 | `Cargo.toml`, `package.json`, `tsconfig.json` |
| Source | 70* | `handler.rs`, `utils.ts` *(decreases with nesting depth)* |
| Tests | 30 | `*_test.go`, `test_*.py` |
| Fixtures | 5 | `tests/fixtures/*`, `__snapshots__/*` |

When combined with `--compress`, files that don't fit at full size are tried at compressed size before being excluded:

```bash
flat --compress --tokens 16000 --include rs
```

Preview what would be included or excluded:

```
$ flat --compress --tokens 2000 --include rs --dry-run

src/lib.rs [COMPRESSED]
src/main.rs [COMPRESSED]
src/config.rs [EXCLUDED]
src/compress.rs [EXCLUDED]
src/filters.rs [EXCLUDED]
...
```

## Filtering

```bash
flat --include rs,toml,md             # only these extensions
flat --exclude test,spec,lock         # skip these extensions
flat --match '*_test.go'              # glob on filename (repeatable)
flat --match '*.spec.ts'
flat --max-size 10485760              # increase file size limit to 10 MB
```

Filters compose: `--include` and `--exclude` operate on extensions, `--match` operates on filenames, and they all apply before compression and budget allocation.

## Output Modes

| Flag | Output |
|------|--------|
| *(none)* | XML-wrapped file contents to stdout |
| `-o FILE` | Same, written to a file |
| `--dry-run` | File list only, no content |
| `--stats` | Summary statistics only |
| `--compress --dry-run` | File list with `[FULL]`/`[COMPRESSED]`/`[EXCLUDED]` annotations |

## Performance

flat is fast. The entire Next.js monorepo (25,000+ files) processes in under 3 seconds:

```
$ time flat /path/to/nextjs --compress --stats

Included: 24,327
Compressed: 19,771 files
Skipped: 894 (873 binary, 14 secret, 7 too large)

real    0m2.883s
```

Compression streams file-by-file — only `--tokens` mode buffers (it needs all files scored before allocating). Memory usage stays low for typical projects.

## Safety

Secrets are excluded automatically — you don't need to remember to filter them:

| Pattern | Examples |
|---------|----------|
| Environment | `.env`, `.env.local`, `.env.production` |
| Keys | `*.key`, `*.pem`, `*.p12`, `*.pfx` |
| SSH | `id_rsa`, `id_dsa`, `id_ecdsa`, `id_ed25519` |
| Credentials | `credentials.json`, `serviceAccount.json` |
| Tokens | Files containing `secret`, `password`, or `credential` in the name |

Binary files are also always excluded (images, media, archives, executables, compiled artifacts).

> Always use `--dry-run` to preview before sharing code with any external service.

## Recipes

```bash
# Paste a project into ChatGPT / Claude
flat --include ts,tsx | pbcopy

# Fit into a specific model's context window
flat --compress --tokens 100000 | pbcopy

# Compressed overview, but keep the file you're debugging full
flat src/ --compress --full-match 'handler.rs' | pbcopy

# Save a snapshot of your project structure
flat --compress -o snapshot.xml

# Just the API layer
flat src/api --include ts --exclude test,spec | pbcopy

# Go test files only
flat --match '*_test.go' | pbcopy

# Check what you're about to share
flat --stats && flat --dry-run
```

## How It Works

```
                        ┌─────────────┐
                        │  Directory   │
                        │    Walk      │
                        └──────┬──────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
        ┌─────▼─────┐  ┌──────▼──────┐  ┌──────▼──────┐
        │ .gitignore │  │   Secret    │  │   Binary    │
        │  filter    │  │ detection   │  │ detection   │
        └─────┬─────┘  └──────┬──────┘  └──────┬──────┘
              │                │                │
              └────────────────┼────────────────┘
                               │
                      ┌────────▼────────┐
                      │ --include/      │
                      │ --exclude/      │
                      │ --match filters │
                      └────────┬────────┘
                               │
                ┌──────────────┼──────────────┐
                │              │              │
          ┌─────▼─────┐ ┌─────▼─────┐ ┌──────▼──────┐
          │  Normal    │ │ Compress  │ │   Token     │
          │  output    │ │ (tree-    │ │  budget     │
          │            │ │  sitter)  │ │ allocation  │
          └─────┬─────┘ └─────┬─────┘ └──────┬──────┘
                │              │              │
                └──────────────┼──────────────┘
                               │
                        ┌──────▼──────┐
                        │  XML output │
                        │  + summary  │
                        └─────────────┘
```

## Project

```
src/
├── main.rs        CLI entry point
├── lib.rs         Public API
├── walker.rs      Directory traversal, two-pass budget allocation
├── compress.rs    Tree-sitter compression engine
├── priority.rs    File importance scoring
├── tokens.rs      Token estimation
├── filters.rs     Secret and binary detection
├── output.rs      XML formatting and statistics
└── config.rs      Configuration
```

100 tests (43 unit + 57 integration). Tested against Flask, FastAPI, Express, and Next.js.

```bash
cargo test --all && cargo clippy --all-targets -- -D warnings
```

## License

MIT — see [LICENSE](LICENSE).
