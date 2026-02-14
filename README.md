# flat

[![Crates.io](https://img.shields.io/crates/v/flat.svg)](https://crates.io/crates/flat)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/zkoranges/flat)](https://github.com/zkoranges/flat/releases)

Pack an entire codebase into a single file, ready to paste into any AI.

```bash
flat | pbcopy
```

That's it. `.gitignore` respected, secrets stripped, binaries skipped — automatically.

But the real power is fitting *more* code into a context window:

```bash
flat --compress --tokens 128000 | pbcopy
```

This **compresses source code to its signatures** (stripping function bodies, keeping structure) and **packs files by priority** until the token budget is full. README and entry points go in first. Test fixtures get cut first.

## Install

```bash
brew install zkoranges/tap/flat           # Homebrew
cargo install --git https://github.com/zkoranges/flat.git   # Cargo
```

## What You Get

```
$ flat src/ --include rs

<file path="src/tokens.rs">
pub fn estimate_tokens(content: &str, is_prose: bool) -> usize {
    let byte_count = content.len();
    if is_prose {
        byte_count / 4
    } else {
        byte_count / 3
    }
}

pub fn is_prose_extension(ext: &str) -> bool {
    matches!(ext.to_lowercase().as_str(), "md" | "txt" | "rst" ...)
}
</file>
```

```
$ flat src/ --compress --include rs

<file path="src/tokens.rs" mode="compressed">
pub fn estimate_tokens(content: &str, is_prose: bool) -> usize { ... }
pub fn is_prose_extension(ext: &str) -> bool { ... }
</file>
```

Same file. Same API surface. 60% fewer tokens.

## The Three Powers

flat has three features that compose together. Each is useful alone. Combined, they let you fit any codebase into any context window.

### 1. `--compress` — structural compression

Uses [tree-sitter](https://tree-sitter.github.io/) to parse source files, keep the structure, strip the implementation:

```
 Kept                              Stripped
 ─────────────────────────────     ──────────────────────
 imports, require(), use            function/method bodies
 type definitions, interfaces      loop contents
 struct/class declarations         if/else branches
 function signatures               variable assignments
 decorators, attributes              inside functions
 docstrings, comments
 module-level constants
 enums, preprocessor directives
```

**Supported languages:** Rust, TypeScript/JavaScript (JSX/TSX), Python, Go, Java, C#, C, C++, Ruby, PHP.

<details>
<summary>What each compressor preserves</summary>

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

</details>

Files in other languages pass through in full — nothing is silently dropped. If tree-sitter can't parse a file (syntax errors, unsupported features), the original is included with a stderr warning.

**Real-world results:**

| Codebase | Files | Full | Compressed | Reduction |
|----------|------:|-----:|-----------:|----------:|
| [Express](https://github.com/expressjs/express) | 6 | 61 KB | 28 KB | **54%** |
| [Flask](https://github.com/pallets/flask) | 24 | 339 KB | 214 KB | **37%** |
| [Next.js](https://github.com/vercel/next.js) `packages/next/src` | 1,605 | 8.0 MB | 5.6 MB | **31%** |

### 2. `--tokens N` — token budget

Caps output to fit a context window. Files are scored by importance and packed greedily — high-value files first, low-value files dropped:

| Priority | Score | Examples |
|----------|------:|---------|
| README | 100 | `README.md`, `README.rst` |
| Entry points | 90 | `main.rs`, `index.ts`, `app.py` |
| Config | 80 | `Cargo.toml`, `package.json`, `tsconfig.json` |
| Source | 70* | `handler.rs`, `utils.ts` *(decreases with nesting depth)* |
| Tests | 30 | `*_test.go`, `test_*.py` |
| Fixtures | 5 | `tests/fixtures/*`, `__snapshots__/*` |

### 3. `--full-match GLOB` — selective full content

When compressing, keep specific files in full:

```bash
flat --compress --full-match 'app.py'
```

`app.py` gets `mode="full"` with complete source. Everything else gets `mode="compressed"` with signatures only. Useful when you want a project overview but need complete implementation detail in the file you're debugging.

## Composing Flags

**Every combination works.** Flags operate in a pipeline — filters narrow the file set, transforms shape the content, output controls the format:

```
  Filters (narrow files)          Transforms (shape content)       Output
  ─────────────────────           ──────────────────────────       ──────
  --include / --exclude           --compress                       (stdout)
  --match                         --full-match                     -o FILE
  --max-size                      --tokens                         --dry-run
  --gitignore                                                      --stats
```

All filters compose with all transforms and all output modes. Here's what each transform combination does:

```
  flat                                    Full content
  flat --compress                         Signatures only
  flat --tokens 8000                      Full content, capped to budget
  flat --compress --tokens 8000           Signatures, capped to budget
  flat --compress --full-match '*.rs'     Matched files full, rest compressed
  flat --compress --full-match '*.rs' \
       --tokens 8000                      The full pipeline (see below)
```

### The full pipeline

```bash
flat src/ \
  --include py \
  --compress \
  --full-match 'app.py' \
  --tokens 30000
```

Here's what happens:

1. **Filter** — walk `src/`, keep only `.py` files
2. **Score** — rank every file by importance (README=100, entry points=90, ...)
3. **Allocate** — `app.py` matches `--full-match`, so reserve its full content first
4. **Fill** — pack remaining files in priority order, compressing each to save space
5. **Cut** — when the 30k token budget is full, exclude the rest

Preview the result without generating output:

```
$ flat src/ --include py --compress --full-match 'app.py' --tokens 30000 --dry-run

flask/app.py [FULL]
flask/config.py [COMPRESSED]
flask/__init__.py [COMPRESSED]
flask/blueprints.py [COMPRESSED]
flask/cli.py [EXCLUDED]
flask/ctx.py [EXCLUDED]
...
Token budget: 29.8k / 30.0k used
Excluded by budget: 16 files
```

`app.py` is in full (you can debug it). The most important modules are compressed (you can see the API surface). Low-priority files are cut. Everything fits in 30k tokens.

### What `--full-match` does NOT do

`--full-match` does not override the token budget. If `app.py` is 20k tokens and your budget is 10k, `app.py` gets excluded — the budget is a hard ceiling. This is intentional: if flat silently overran the budget, you'd overflow context windows.

## Filtering

```bash
flat --include rs,toml,md             # only these extensions
flat --exclude test,spec,lock         # skip these extensions
flat --match '*_test.go'              # glob on filename (repeatable)
flat --max-size 10M                   # increase size limit to 10 MiB
```

All numeric arguments accept human-friendly suffixes: `k`/`K`, `M`, `G`. Token counts use decimal multipliers (10k = 10,000). Byte sizes use binary multipliers (10M = 10 MiB = 10,485,760 bytes).

Filters compose: `--include`/`--exclude` operate on extensions, `--match` operates on filenames. They all apply before compression and budget allocation.

## Output Modes

| Flag | Output |
|------|--------|
| *(none)* | XML-wrapped file contents to stdout |
| `-o FILE` | Same, written to a file |
| `--dry-run` | File list only, no content |
| `--stats` | Summary statistics only |
| `--dry-run` + `--tokens` | File list annotated `[FULL]` / `[COMPRESSED]` / `[EXCLUDED]` |

## Performance

The entire Next.js monorepo — 25,000+ files — processes in under 3 seconds:

```
$ time flat /path/to/nextjs --compress --stats

Included: 24,327
Compressed: 19,771 files
Skipped: 894

real    0m2.883s
```

Without `--tokens`, compression streams file-by-file (constant memory). With `--tokens`, all candidate files are buffered for scoring — but even that is fast.

## Safety

Secrets are **always** excluded — no flag needed:

| Pattern | Examples |
|---------|----------|
| Environment | `.env`, `.env.local`, `.env.production` |
| Keys | `*.key`, `*.pem`, `*.p12`, `*.pfx` |
| SSH | `id_rsa`, `id_dsa`, `id_ecdsa`, `id_ed25519` |
| Credentials | `credentials.json`, `serviceAccount.json` |

Binary files are always excluded (images, media, archives, executables, compiled artifacts). All `.gitignore` patterns are respected via [ripgrep's parser](https://github.com/BurntSushi/ripgrep).

> Use `--dry-run` to preview before sharing code with any external service.

## Recipes

```bash
# The basics
flat | pbcopy                                    # everything, to clipboard
flat --include rs,toml | pbcopy                  # just Rust files
flat --stats                                     # preview before copying

# Compression
flat --compress | pbcopy                         # structural overview
flat --compress --full-match 'main.rs' | pbcopy  # overview + one file in full

# Token budgets
flat --compress --tokens 100k | pbcopy            # fit into 100k context
flat --compress --tokens 8k --dry-run             # preview what fits

# Targeted
flat src/api --include ts --exclude spec          # just the API layer
flat --match '*_test.go' | pbcopy                 # only test files
flat src/ --compress --full-match 'handler.rs'    # debug one file in context

# Save to file
flat --compress -o snapshot.xml                   # compressed snapshot
```

## Project

```
src/
├── main.rs        CLI entry point
├── walker.rs      Directory traversal, two-pass budget allocation
├── compress.rs    Tree-sitter compression engine (13 languages)
├── priority.rs    File importance scoring
├── tokens.rs      Token estimation
├── filters.rs     Secret and binary detection
├── output.rs      XML formatting and statistics
├── config.rs      Configuration
└── lib.rs         Public API
```

139 tests (64 unit + 75 integration), validated against Flask, FastAPI, Express, and Next.js.

```bash
cargo test --all && cargo clippy --all-targets -- -D warnings
```

## License

MIT — see [LICENSE](LICENSE).
