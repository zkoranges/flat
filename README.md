# flat

[![Crates.io](https://img.shields.io/crates/v/flat-cli.svg)](https://crates.io/crates/flat-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![GitHub Release](https://img.shields.io/github/v/release/zkoranges/flat)](https://github.com/zkoranges/flat/releases)
[![CI/CD](https://github.com/zkoranges/flat/actions/workflows/release.yml/badge.svg)](https://github.com/zkoranges/flat/actions)

**Pack entire codebases into a single file for AI consumption.**

Compress source code to its structure (signatures, types, imports — no function bodies), pack files by importance, and fit any project into any context window.

```bash
flat | pbcopy
```

That's it. `.gitignore` respected, secrets stripped, binaries excluded — automatically. Works on macOS, Linux, and Windows with native binaries.

## Quick Start

```bash
# Install once
cargo install flat-cli

# Use it
flat | pbcopy                                          # everything to clipboard
flat --format markdown | pbcopy                        # markdown for AI chat
flat --compress --tokens 100k | pbcopy                 # fit into 100k context
flat --tokenizer claude --tokens 10k | pbcopy          # accurate Claude token count
```

## Why flat?

**Three features that compose together:**

1. **`--compress`** — Structural compression using tree-sitter (12 languages)
   - Keeps: imports, type definitions, function signatures, docstrings
   - Strips: function bodies, loop contents, variable assignments
   - Result: 30-60% token reduction while preserving API surface

2. **`--tokens N`** — Context window budgeting
   - Scores files by importance (README=100, entry points=90, tests=30, fixtures=5)
   - Packs greedily: high-value files first, low-value files dropped
   - Fits any codebase into any context window

3. **`--format`** — Choose your output
   - `xml` (default) — structured XML, for programmatic parsing
   - `markdown` — human-readable, for pasting into Claude/ChatGPT

**The power move:**

```bash
flat --compress --tokenizer claude --tokens 100k --format markdown | pbcopy
```

This gives you:
- **Compressed** code (structure, no bodies)
- **Accurate** token count (Claude tokenizer, not heuristic)
- **Bounded** output (fits 100k context exactly)
- **Human-readable** markdown (perfect for copy-paste into AI)

## Features

| Feature | v0.4.0 | Details |
|---------|:------:|---------|
| **Tree-sitter compression** | ✓ | 12 languages, 30-60% reduction |
| **Real tokenizers** | ✓ | Claude/GPT-4/GPT-3.5 (accurate) or heuristic (fast) |
| **Token budgeting** | ✓ | Fit any codebase into any context window |
| **Priority scoring** | ✓ | README=100, entry points=90, tests=30, fixtures=5 |
| **Multiple formats** | ✓ | XML (structured) or Markdown (human-readable) |
| **Multi-platform** | ✓ | macOS, Linux, Windows with native binaries |
| **Safety by default** | ✓ | Secrets excluded, binaries skipped, `.gitignore` respected |
| **12 languages** | ✓ | Rust, TS/JS, Python, Go, Java, C#, C, C++, Ruby, PHP, Solidity, Elixir |
| **Fast** | ✓ | 25k files in <3 seconds |
| **Test coverage** | ✓ | 200+ tests (unit + integration) |

## Installation

### Via cargo (recommended)

```bash
cargo install flat-cli
```

Works on macOS, Linux, Windows. Binary name: `flat`.

### Via Homebrew (macOS)

```bash
brew install zkoranges/tap/flat
```

### Prebuilt binaries

Download from [GitHub Releases](https://github.com/zkoranges/flat/releases):
- `flat-x86_64-apple-darwin.tar.gz` (macOS Intel)
- `flat-aarch64-apple-darwin.tar.gz` (macOS Apple Silicon)
- `flat-x86_64-unknown-linux-gnu.tar.gz` (Linux)
- `flat-x86_64-pc-windows-msvc.zip` (Windows)

### Requirements

- macOS 10.15+, Linux (glibc 2.17+), Windows 10+
- No external dependencies (fully static linked)
- Rust 1.70+ (if building from source)

## Usage

### Basic usage

```bash
flat                                  # all files in current directory
flat src/                             # all files in src/
flat src/ --include rs,toml          # only .rs and .toml files
flat src/ --exclude test,spec        # skip test/spec files
flat src/ --match '*_test.go'        # glob patterns (repeatable)
flat src/ --max-size 10M             # files up to 10 MiB
```

### Compression

```bash
flat --compress                       # signatures only (30-60% smaller)
flat --compress --full-match '*.rs'   # compress most files, keep *.rs in full
```

### Output formats

```bash
flat --format xml                     # XML (default, structured)
flat --format markdown | pbcopy       # Markdown (human-readable)
flat -o snapshot.xml                  # write to file instead of stdout
```

### Token budgeting

```bash
flat --tokens 100k                    # fit output into 100k tokens
flat --compress --tokens 100k         # compress to fit budget
flat --tokens 100k --dry-run          # preview what fits
flat --tokens 100k --stats            # summary only
```

### Real tokenizers

```bash
flat --tokenizer heuristic            # fast estimate (default, conservative)
flat --tokenizer claude               # Claude 3/4 tokenizer (exact)
flat --tokenizer gpt-4                # GPT-4 tokenizer (exact)
flat --tokenizer gpt-3.5              # GPT-3.5 tokenizer (exact)
```

**Tokenizer comparison:**

| Tokenizer | Speed | Accuracy | Use case |
|-----------|-------|----------|----------|
| `heuristic` | Instant | ~20-30% overestimate | Quick previews, safety margin |
| `claude` | ~1s | Exact | Pasting into Claude |
| `gpt-4` | ~1s | Exact | OpenAI API (GPT-4) |
| `gpt-3.5` | ~1s | Exact | OpenAI API (GPT-3.5) |

The heuristic intentionally overestimates so you stay within budget. Real tokenizers give exact counts when precision matters.

## Compression Details

### What gets compressed

Uses [tree-sitter](https://tree-sitter.github.io/) to parse and selectively strip:

```
Kept                              Stripped
─────────────────────────────     ──────────────────────
imports, require(), use            function/method bodies
type definitions, interfaces       loop contents
struct/class declarations          if/else branches
function signatures                variable assignments
decorators, attributes             inside functions
docstrings, comments
module-level constants
enums, preprocessor directives
```

### Supported languages

| Language | Keeps | Body placeholder |
|----------|-------|:----------------:|
| **Rust** | `use`/`mod`/`extern crate`, attributes, macros, structs, enums, trait/impl signatures | `{ ... }` |
| **TS/JS** (JSX/TSX) | imports, interfaces, type aliases, enums, class member signatures, exports | `{ ... }` |
| **Python** | imports, docstrings, decorators, class variables, module constants | `...` |
| **Go** | `package`, imports, type/const/var declarations | `{ ... }` |
| **Java** | `package`, imports, class/interface/enum declarations, fields, constants | `{ ... }` |
| **C#** | `using`, namespaces, class/struct/record/interface, properties, events | `{ ... }` |
| **C** | `#include`/`#define`/preprocessor, typedefs, struct/enum/union | `{ ... }` |
| **C++** | preprocessor, templates, namespaces, classes with members, `using`/aliases | `{ ... }` |
| **Ruby** | `require`, assignments, class/module structure | `...\nend` |
| **PHP** | `<?php`, `use`/`namespace`, class/interface/trait/enum, properties | `{ ... }` |
| **Solidity** | `pragma`, imports, contract/interface/library, event/error/struct/enum declarations | `{ ... }` |
| **Elixir** | `defmodule`, `use`/`import`/`alias`/`require`, module attributes, typespecs | `...\nend` |

Files in other languages pass through in full. If tree-sitter can't parse a file, the original is included with a stderr warning.

### Real-world results

| Codebase | Files | Full | Compressed | Reduction |
|----------|------:|-----:|-----------:|----------:|
| [Express](https://github.com/expressjs/express) | 6 | 61 KB | 28 KB | **54%** |
| [Flask](https://github.com/pallets/flask) | 24 | 339 KB | 214 KB | **37%** |
| [Next.js](https://github.com/vercel/next.js) `packages/next/src` | 1,605 | 8.0 MB | 5.6 MB | **31%** |

## Priority Scoring

Files are ranked by importance for `--tokens` budgeting:

| Priority | Score | Examples |
|----------|------:|----------|
| README | 100 | `README.md`, `README.rst` |
| Entry points | 90 | `main.rs`, `index.ts`, `app.py` |
| Config | 80 | `Cargo.toml`, `package.json`, `tsconfig.json` |
| Source | 70* | `handler.rs`, `utils.ts` *(decreases with nesting depth)* |
| Tests | 30 | `*_test.go`, `test_*.py` |
| Fixtures | 5 | `tests/fixtures/*`, `__snapshots__/*` |

When a token budget is specified, high-priority files are included first. Low-priority files are dropped first.

## Safety

Secrets are **always** excluded — no flag needed:

| Pattern | Examples |
|---------|----------|
| Environment | `.env`, `.env.local`, `.env.production` |
| Keys | `*.key`, `*.pem`, `*.p12`, `*.pfx` |
| SSH | `id_rsa`, `id_dsa`, `id_ecdsa`, `id_ed25519` |
| Credentials | `credentials.json`, `serviceAccount.json` |

Binary files are automatically skipped (images, media, archives, executables, compiled artifacts). All `.gitignore` patterns are respected.

**Use `--dry-run` to preview before sharing code with external services.**

## Performance

Processes large codebases efficiently:

```bash
$ time flat /path/to/nextjs --compress --stats
# 25,000+ files processed in ~3 seconds

Included: 24,327
Compressed: 19,771 files
Skipped: 894

real    0m2.883s
```

Without `--tokens`, compression streams file-by-file (constant memory). With `--tokens`, all candidate files are buffered for scoring — but even that is fast.

## Recipes

```bash
# The basics
flat | pbcopy                                    # everything, to clipboard
flat --include rs,toml | pbcopy                  # just Rust files
flat --stats                                     # preview before copying

# Output formats
flat --format markdown | pbcopy                  # markdown for AI chat
flat --format xml -o snapshot.xml                # XML for tools

# Compression
flat --compress | pbcopy                         # structural overview
flat --compress --full-match 'main.rs' | pbcopy  # overview + one file in full

# Token budgets
flat --compress --tokens 100k | pbcopy            # fit into 100k context
flat --compress --tokens 8k --dry-run             # preview what fits
flat --tokenizer claude --tokens 100k | pbcopy    # exact Claude token count
flat --tokenizer gpt-4 --tokens 128k | pbcopy     # exact GPT-4 token count

# Targeted analysis
flat src/api --include ts --exclude spec          # just the API layer
flat --match '*_test.go' | pbcopy                 # only test files
flat src/ --compress --full-match 'handler.rs'    # debug one file in context

# The full pipeline
flat src/ --compress --tokens 100k \
  --tokenizer claude --format markdown \
  --full-match 'main.rs' | pbcopy                # everything combined

# Save to file
flat --compress -o snapshot.xml                   # compressed snapshot
```

## Architecture

```
src/
├── main.rs        CLI entry point, argument parsing
├── lib.rs         Public API
├── walker.rs      Directory traversal, two-pass budget allocation
├── compress.rs    Tree-sitter compression engine (12 languages)
├── priority.rs    File importance scoring
├── tokens.rs      Token estimation and real tokenizer support (Claude/GPT-4)
├── filters.rs     Secret detection, binary file skipping
├── output.rs      Output formatting (XML/Markdown) and statistics
├── parse.rs       Number parsing (k/M/G suffixes)
└── config.rs      Configuration and CLI options
```

**Quality metrics:**
- 200+ tests (unit + integration)
- Zero clippy warnings
- Full tree-sitter support for 12 languages
- Validated against Flask, FastAPI, Express, Next.js

```bash
cargo test --all && cargo clippy --all-targets -- -D warnings
```

## Contributing

Contributions welcome! Areas of interest:

- Additional language support (via tree-sitter)
- Tokenizer optimizations
- Template system (v0.5.0)
- MCP server integration (v0.5.0)

See [LICENSE](LICENSE) for details.

## Roadmap

### v0.4.0 ✅ Accessibility
- Real tokenizer support (Claude/GPT-4/GPT-3.5)
- Markdown output format
- Multi-platform binaries (macOS/Linux/Windows)
- Published to crates.io

### v0.5.0 Integration (planned)
- MCP server mode (`flat serve`)
- GitHub URL support (`--github org/repo`)
- Template system (custom output formats)
- JSON output format

### v0.6.0 Performance (planned)
- Incremental processing with caching
- Watch mode for auto-regeneration
- Parallel compression

### v1.0 Intelligence (research)
- Hierarchical summarization (18:1 compression)
- Semantic code understanding
- IDE extensions

## License

MIT — see [LICENSE](LICENSE).
