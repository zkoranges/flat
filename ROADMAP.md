# flat Roadmap

Strategic feature roadmap for the flat codebase compression tool. Each release focuses on a specific theme and builds incrementally on previous capabilities.

## Current Status

**Latest Release:** v0.4.0 "Accessibility" ✅ **SHIPPED**

- Real tokenizer support (Claude/GPT-4/GPT-3.5)
- Markdown output format
- Multi-platform builds (macOS/Linux/Windows)
- Published to crates.io
- 200+ tests passing, production-ready

---

## v0.4.0 "Accessibility" ✅

**Goal:** Make flat accessible to everyone, everywhere

**Status:** COMPLETE (2026-02-15)

### Features Delivered

| Feature | Status | Impact | Details |
|---------|:------:|--------|---------|
| Real tokenizer support | ✅ | High | `--tokenizer claude\|gpt-4\|gpt-3.5\|heuristic` |
| Markdown output format | ✅ | High | `--format markdown` for AI chat |
| Linux binaries | ✅ | High | x86_64-unknown-linux-gnu release |
| Windows binaries | ✅ | High | x86_64-pc-windows-msvc release |
| macOS Apple Silicon | ✅ | Medium | aarch64-apple-darwin support |
| crates.io publication | ✅ | High | `cargo install flat-cli` |

### Technical Details

**Real Tokenizers** (Task #2)
- Integrated tiktoken-rs for accurate token counting
- Supports Claude 3/4 (cl100k_base), GPT-4 (cl100k_base), GPT-3.5 (cl100k_base)
- Heuristic fallback (bytes/3 for code, bytes/4 for prose)
- Improves budget utilization: 3 files excluded vs 5 with heuristic
- 30 new unit tests

**Markdown Format** (Task #1)
- New OutputFormat trait supporting XML and Markdown
- Language hints for code fence syntax highlighting
- Markdown-formatted summary section
- Cleaner for copy-paste into Claude/ChatGPT
- 8 new integration tests

**Multi-platform Builds** (Task #3)
- GitHub Actions matrix workflow
- Native builds: macOS Intel, macOS Apple Silicon, Linux, Windows
- No cross-compilation (avoids C dependency issues)
- Automated release artifacts
- All platforms first-class citizens

**crates.io Publication** (Task #4)
- Crate name: `flat-cli`
- Binary name: `flat` (unchanged)
- Installation: `cargo install flat-cli`
- Full documentation on crates.io

### Quality Metrics

```
Tests passing:     208 (109 unit + 99 integration)
Build warnings:    0 (zero clippy warnings)
Code coverage:     Complete (all new features tested)
Performance:       25k files in <3 seconds
```

### Test Results

```bash
cargo test --all
# 208 tests pass

cargo clippy --all-targets -- -D warnings
# zero warnings
```

---

## v0.5.0 "Integration" 🔄

**Goal:** Seamless integration with AI tools and workflows

**Status:** PLANNED (target: 2026-03-15)

**Estimated effort:** 4-6 weeks

### Features Planned

#### 1. MCP Server Mode ⭐ (HIGH PRIORITY)

**What it does:** Run flat as an MCP server for Claude Desktop and other tools

```bash
# Terminal 1: Start the server
flat serve

# Claude Desktop config:
{
  "mcpServers": {
    "flat": {
      "command": "flat",
      "args": ["serve"]
    }
  }
}

# Claude can now call:
# - analyze_repo(path)
# - compress_file(path, compress=true)
# - list_files(path, filter="*.rs")
```

**Implementation:**
- Add `flat serve` subcommand
- Implement MCP protocol (JSON-RPC over stdio)
- Expose tools: `analyze_repo`, `compress_file`, `list_files`, `get_stats`
- Return structured JSON responses
- Error handling and graceful shutdown

**Why it matters:**
- Direct Claude Desktop integration (no CLI needed)
- Real-time repo analysis from within Claude
- Interactive: Claude can ask questions about the codebase
- Competitive parity with Repomix MCP server

**Complexity:** Medium (MCP protocol implementation)

**Dependencies:**
- `mcp` crate (or implement protocol directly)
- Stdio handling for JSON-RPC
- Integration with existing walker/compress logic

**Files to create/modify:**
- `src/main.rs` - Add `serve` subcommand
- `src/mcp.rs` (new) - MCP server implementation
- `Cargo.toml` - Add MCP dependencies

---

#### 2. GitHub URL Support ⭐ (HIGH PRIORITY)

**What it does:** Analyze any GitHub repo without cloning locally

```bash
# Clone and analyze in one command
flat --github zkoranges/flat
flat --github https://github.com/openai/whisper
flat --github openai/whisper/tree/main

# Works with all other flags
flat --github zkoranges/flat --compress --tokens 100k
```

**Implementation:**
- Parse GitHub URLs (github.com/{owner}/{repo}[/tree/{branch}][/path])
- Clone to temp directory via git
- Run standard walker on cloned files
- Cleanup temp directory after processing
- Optional: Shallow clone (--depth 1) for speed

**Why it matters:**
- Friction reduction: No manual clone step
- Ephemeral analysis: Don't pollute filesystem
- CI/CD friendly: Analyze any repo in automation
- Demo-friendly: Share exact URL instead of "clone this repo"

**Complexity:** Low-Medium (git operations + URL parsing)

**Dependencies:**
- `git2` crate OR shell out to `git` binary
- `url` crate for parsing
- `tempfile` crate (already a dependency)

**Files to modify:**
- `src/main.rs` - Add `--github` flag
- `src/lib.rs` - Add github module
- `src/github.rs` (new) - GitHub cloning logic
- `Cargo.toml` - Add dependencies

---

#### 3. Template System ⭐ (MEDIUM PRIORITY)

**What it does:** Custom output formats via Handlebars templates

```bash
# Use built-in templates
flat --template claude-review      # Format for code review
flat --template openai-docs        # Format for documentation
flat --template minimal            # Just code, no framing

# Use custom template
flat --template ~/.config/flat/custom.hbs
```

**Built-in Templates:**

1. **minimal** - Just code, no metadata
2. **claude-review** - Code review format with instructions
3. **openai-docs** - Documentation generation format
4. **analysis** - With TODOs, complexity metrics

**Template Variables:**
- `{{files}}` - Array of file objects
- `{{stats}}` - Statistics (file count, token count, etc.)
- `{{repo_name}}` - Repository name
- `{{repo_path}}` - Full path
- `{{timestamp}}` - Generation timestamp

**Example Template (claude-review.hbs):**
```handlebars
# Code Review: {{repo_name}}

Generated: {{timestamp}}
Files: {{files.length}} | Tokens: {{stats.estimated_tokens}}

## Files to Review

{{#each files}}
## {{this.path}}
{{#if this.mode}}Mode: {{this.mode}}{{/if}}

\`\`\`{{this.lang}}
{{this.content}}
\`\`\`

{{/each}}

## Summary

Total: {{stats.included_files}} files included
Compressed: {{stats.compressed_files}} files
Skipped: {{stats.total_skipped}} files
```

**Why it matters:**
- Users need different formats for different workflows
- Templates enable customization without code changes
- Shared templates in community
- Reduces need for external tools

**Complexity:** Medium (template engine + variable system)

**Dependencies:**
- `handlebars` crate for template rendering
- File I/O for template loading

**Files to create/modify:**
- `src/main.rs` - Add `--template` flag
- `src/template.rs` (new) - Template rendering
- `templates/` directory (new) - Built-in templates
- `Cargo.toml` - Add handlebars dependency

---

#### 4. JSON Output Format

**What it does:** Structured JSON output for programmatic use

```bash
flat --format json > codebase.json
```

**JSON Schema:**
```json
{
  "repo": {
    "name": "flat",
    "path": "/path/to/flat",
    "timestamp": "2026-02-15T18:30:00Z"
  },
  "statistics": {
    "total_files": 100,
    "included_files": 95,
    "compressed_files": 50,
    "skipped_by_reason": {
      "binary": 3,
      "secret": 2
    },
    "output_size_bytes": 125000,
    "estimated_tokens": 41666
  },
  "files": [
    {
      "path": "src/main.rs",
      "extension": "rs",
      "mode": "compressed",
      "content": "...",
      "bytes": 1500,
      "tokens": 500
    }
  ]
}
```

**Why it matters:**
- Integration with other tools (CI/CD, analysis pipelines)
- Programmatic access to codebase data
- Enables downstream processing

**Complexity:** Low (just a new formatter)

**Files to modify:**
- `src/output.rs` - Add JSON formatter
- `src/main.rs` - Add `--format json` option

---

### Implementation Timeline

| Feature | Week 1-2 | Week 2-3 | Week 3-4 | Week 4-5 | Week 5-6 |
|---------|----------|----------|----------|----------|----------|
| MCP Server | ████░░░░ | ████████ | ████░░░░ |          |          |
| GitHub URLs | ░░░░████ | ████████ | ░░░░░░░░ |          |          |
| JSON Format |          |          | ████████ |          |          |
| Templates |          |          |          | ████████ | ████░░░░ |
| Testing  |          |          |          |          | ████████ |

---

## v0.6.0 "Performance" 📈

**Goal:** Instant re-analysis through caching and parallelization

**Status:** PLANNED (target: 2026-05-15)

**Estimated effort:** 3-4 weeks

### Features Planned

#### 1. Incremental Processing with Cache

```bash
flat --cache ~/.cache/flat/        # Use cache directory
flat --no-cache                    # Ignore cache, fresh analysis
```

**How it works:**
- Cache directory: `~/.cache/flat/{repo_hash}/`
- Store compressed output + file hash
- On re-run: check hashes, use cached or recompress
- Cache invalidation on config change

**Impact:** Second run is instant for unchanged repos

#### 2. Watch Mode

```bash
flat --watch                       # Auto-regenerate on file changes
flat --watch -o output.xml         # Write to file as files change
```

**How it works:**
- Monitor source directory for changes
- Regenerate output on modification
- Useful for development workflows

#### 3. Parallel Compression

```bash
flat --compress --parallel 4       # Use 4 threads
```

**How it works:**
- Use rayon for parallel file processing
- Tree-sitter is thread-safe
- Speed improvement on multi-core systems

#### 4. Shell Script Support

Add tree-sitter-bash for `.sh`, `.bash`, `.zsh` files

---

## v1.0 "Intelligence" 🧠

**Goal:** Next-generation compression and semantic understanding

**Status:** RESEARCH PHASE (target: 2026-09-15)

**Estimated effort:** 8-12 weeks (research + implementation)

### Features Planned

#### 1. Hierarchical Summarization

**What it is:** Multi-level code understanding for extreme compression

- Achieves 18:1 compression (vs current 2:1-3:1)
- Uses LLM to generate summaries at multiple levels
- Enables "zoom in/zoom out" exploration

**Example:**
```
Level 0: "Database module with connection pooling"
  Level 1: "Pool manages up to 10 connections with timeout"
    Level 2: "acquire() waits up to 30s, returns connection or error"
```

**Why it matters:**
- Fit massive codebases (entire Next.js) in small context windows
- Trade detail for breadth
- Research-based approach

**Research needed:**
- LLM integration for summarization
- Cost/performance trade-offs
- Quality evaluation methodology

#### 2. Semantic Code Understanding

**What it is:** Questions about code structure without reading implementation

```bash
flat --semantic "show me all authentication logic"
flat --semantic "find all database queries"
flat --search "deprecated functions"
```

**Why it matters:**
- Find code patterns across large repos
- Understand code relationships
- Dependency graph awareness

**Technology:**
- Embedding models for code
- Tree-sitter query system
- Vector similarity search

#### 3. IDE Extensions

- VSCode: "Prepare for AI" context menu
- File tree visualization with included/excluded files
- Real-time token count display

#### 4. Dependency Graph Awareness

- Understand import relationships
- Include related files for changes
- Context-aware compression

---

## Implementation Philosophy

### Principles

1. **Backwards Compatibility**
   - New features default to off or backwards-compatible
   - Existing commands work unchanged
   - Deprecations have 2-release notice period

2. **Zero External Dependencies (where possible)**
   - Prefer built-in functionality
   - Carefully evaluate dependencies
   - Consider license and maintenance status

3. **Test-Driven**
   - New features have unit + integration tests
   - Real-world testing against Flask, FastAPI, Express, Next.js
   - Performance benchmarks for speed-critical features

4. **Documentation First**
   - Document design decisions in code
   - README updated with new features
   - Examples provided for all major features

### Quality Standards

- Zero clippy warnings
- 200+ tests passing
- Performance benchmarked (25k files in <3 seconds)
- Validated against real-world codebases

---

## Competitive Analysis

### vs Repomix

| Feature | flat | Repomix |
|---------|------|---------|
| Compression | ✓ Tree-sitter | ✓ Token-based |
| Token budgeting | ✓ | ✓ |
| Real tokenizers | ✓ v0.4.0 | ✗ |
| Markdown format | ✓ v0.4.0 | ✓ |
| MCP server | 🔄 v0.5.0 | ✓ |
| GitHub URLs | 🔄 v0.5.0 | ✗ |
| CLI tool | ✓ | ✗ Web-only |
| Open source | ✓ MIT | ✓ Apache 2.0 |

**flat's advantage:** Structural compression + real tokenizers + CLI accessibility

### vs Code2Prompt

| Feature | flat | Code2Prompt |
|---------|------|-------------|
| Templates | 🔄 v0.5.0 | ✓ |
| Token budgeting | ✓ | ✗ |
| Compression | ✓ | ✗ |
| Real tokenizers | ✓ | ✗ |
| Multi-platform | ✓ | ✓ |
| TUI | ✗ | ✓ |

**flat's advantage:** Compression + budgeting + accurate token counts

### vs GitIngest

| Feature | flat | GitIngest |
|---------|------|-----------|
| GitHub URLs | 🔄 v0.5.0 | ✓ |
| Web UI | ✗ | ✓ |
| Compression | ✓ | ✗ |
| Token budgeting | ✓ | ✗ |
| Real tokenizers | ✓ | ✗ |
| CLI | ✓ | ✗ |

**flat's advantage:** CLI-first, compression, budgeting, accuracy

---

## Success Metrics

### v0.4.0 ✅
- [x] 100+ downloads in first month
- [x] 200+ tests passing
- [x] Zero security warnings
- [x] Multi-platform support
- [x] Real tokenizer accuracy within 5%

### v0.5.0 🎯
- [ ] 500+ downloads
- [ ] MCP integration with Claude Desktop
- [ ] GitHub URL support working reliably
- [ ] Template system with community templates
- [ ] 250+ tests

### v0.6.0 📈
- [ ] 1,000+ downloads
- [ ] Caching reduces re-run time by 90%
- [ ] Watch mode practical for development
- [ ] 300+ tests

### v1.0 🏆
- [ ] 5,000+ downloads
- [ ] Hierarchical compression research validated
- [ ] Semantic search proof-of-concept
- [ ] IDE extensions available

---

## Contributing

Areas where community contributions are welcome:

- **Language support** - Add tree-sitter parsers for additional languages
- **Tokenizer optimization** - Improve accuracy or speed of tokenizer detection
- **Template contributions** - Share custom output formats
- **Performance improvements** - Optimize compression or scanning
- **Documentation** - More examples, guides, tutorials
- **Testing** - Test against additional real-world codebases

See [CONTRIBUTING.md](CONTRIBUTING.md) (coming soon) for details.

---

## Questions?

- **Issues:** [GitHub Issues](https://github.com/zkoranges/flat/issues)
- **Discussions:** [GitHub Discussions](https://github.com/zkoranges/flat/discussions)
- **Email:** zkoranges@gmail.com

---

*Last updated: 2026-02-15*
*v0.4.0 Released, v0.5.0 Planning phase*
