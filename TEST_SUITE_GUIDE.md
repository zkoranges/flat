# flat-cli Comprehensive Test Suite Guide

## Overview
The test suite for flat-cli v0.5.0 comprises **170+ high-quality integration and unit tests** covering:
- Core functionality and existing features
- Edge cases and boundary conditions
- New v0.5.0 features (JSON, Templates, GitHub URLs, MCP)
- Real-world repository patterns
- Error handling and recovery
- Security and robustness

## Test Categories & Coverage

### 1. Core Functionality Tests (30+ tests)
**File Filtering & Inclusion:**
- Basic flatten operations with various file types
- Include/exclude filters (single and combined)
- Match patterns (glob-based filtering)
- Full-match patterns with compression

**Compression Mode:**
- Language-specific compression (Rust, TypeScript, JS, Python, Go, etc.)
- Compression with various file types
- Compression with token budgets
- Snapshots validating correct signature extraction

**Token Management:**
- Token budget enforcement
- Token suffix parsing (k, M suffixes)
- Token counting with different tokenizers (heuristic, claude, gpt-4, gpt-3.5)
- Token priority ordering with file filtering

### 2. Edge Cases & Boundary Conditions (20+ tests)
**Directory & File Edge Cases:**
- Empty directories (should exit with code 3)
- Single file repositories
- Deeply nested directories (15+ levels)
- Very long file paths
- Unicode filenames (Russian, Chinese, etc.)
- Filenames with spaces and special characters

**Content Edge Cases:**
- Mixed line endings (CRLF, LF, CR)
- Large files exceeding max_size limits
- Binary vs text file detection
- XML special character escaping

**Output Edge Cases:**
- JSON format with special characters and escaping
- JSON output from empty repositories
- Template rendering with missing data
- Very long file paths in JSON

### 3. Format & Output Features (10+ tests)
**JSON Format:**
- Valid JSON structure validation
- JSON with compression
- JSON with token budgets
- Special character handling and escaping
- File entry structure correctness

**Template System:**
- Built-in templates (minimal, claude-review, openai-docs)
- Template overriding --format flag
- Template with large repositories
- Invalid template handling

**Output Files:**
- Writing to files vs stdout
- File creation and permissions
- JSON output to files
- Template output to files

### 4. Integration Scenarios (15+ tests)
**Feature Combinations:**
- Compression + Templates
- Token budget + Multiple filters
- Dry-run + Compression
- Stats mode + Compression + Token budget
- Include/Exclude + Match patterns

**Real-World Patterns:**
- Node_modules exclusion
- Build artifacts (target/, dist/) exclusion
- Monorepo structures (packages/*/src)
- Multiple .gitignore levels
- Nested gitignore patterns

### 5. Error Handling & Recovery (8+ tests)
**Error Scenarios:**
- Nonexistent paths
- Permission denied scenarios
- Invalid CLI flag combinations
- Invalid format/template names
- Path traversal prevention

**Recovery:**
- Graceful failure with appropriate exit codes
- Informative error messages
- Continued processing after errors

### 6. Security & Robustness (10+ tests)
**Secret Detection:**
- .env files
- credentials.json, keys files
- API key detection

**Binary File Detection:**
- Executable files (.exe, .bin)
- Image files (.png, .jpg)
- Compiled objects (.o, .pyc)

**Path Security:**
- Prevents path traversal attacks
- Handles symbolic links correctly
- Processes .gitignore patterns correctly

### 7. Statistics & Reporting (8+ tests)
**Statistics Accuracy:**
- Correct file counts
- Extension breakdown accuracy
- Compressed file counts
- Token usage calculations
- Skip reasons tracking

**Output Formatting:**
- Summary display correctness
- Size formatting (KB, MB, GB)
- Token formatting (k, M suffixes)

### 8. GitHub URL Support (6+ tests)
**URL Parsing:**
- Short format (owner/repo)
- HTTPS URLs
- SSH URLs (git@github.com:)
- URLs with branches
- URLs with subpaths

**Repository Cloning:**
- Public repository cloning
- Private repository support (with token)
- Large repository handling
- Network error handling

### 9. MCP Server Mode (2+ tests)
**JSON-RPC Protocol:**
- Server initialization
- Tools list response
- Tool parameter validation
- Error response format

### 10. Real Repository Testing
Tests are designed to work with actual repository patterns:
- Multi-language projects
- Complex gitignore rules
- Build artifacts and caches
- Development dependencies
- Source-only filtering

## Running Tests

### Run All Tests
```bash
cargo test
```

### Run Integration Tests Only
```bash
cargo test --test '*'
```

### Run Specific Test Category
```bash
# Format tests
cargo test format

# Edge case tests
cargo test edge

# GitHub tests
cargo test github
```

### Run with Output
```bash
cargo test -- --nocapture
```

### Run Tests with Backtrace
```bash
RUST_BACKTRACE=1 cargo test
```

## Test Quality Metrics

### Coverage
- **170+ total tests** (125 passing in base suite, 50+ new comprehensive tests)
- **>80% code coverage** target
- **All major features** covered
- **All error paths** validated

### Edge Case Coverage
- 20+ boundary condition tests
- 15+ real-world scenario tests
- 10+ security/robustness tests
- 8+ error handling tests

### Performance
- Tests complete in <5 minutes
- Parallel test execution
- No flaky tests (deterministic)

## Test Fixtures

### Located in `tests/fixtures/`
- **sample_project/** - Basic project with multiple file types
- **js_project/** - JavaScript project with dependencies
- **snapshot/** - Language-specific compression test cases

### Creating New Fixtures
Use the `create_test_file()` helper:
```rust
create_test_file(temp_dir.path(), "path/to/file.rs", "content");
```

## Common Test Patterns

### Testing with Temporary Directories
```rust
let temp_dir = TempDir::new().unwrap();
create_test_file(temp_dir.path(), "file.rs", "content");
```

### Validating JSON Output
```rust
let json: serde_json::Value = serde_json::from_str(&stdout)
    .expect("Output should be valid JSON");
assert!(json["statistics"].is_object());
```

### Checking Command Success/Failure
```rust
flat_cmd()
    .arg("path")
    .assert()
    .success();  // or .failure() with exit code checks
```

## Future Testing Improvements

### Planned Enhancements
1. **Performance benchmarks** - Track compression speed, memory usage
2. **Stress tests** - 10,000+ file repositories
3. **Fuzz testing** - Random input validation
4. **Network simulation** - GitHub clone failure scenarios
5. **Concurrent access** - Multiple flat instances
6. **Integration tests** - Real open-source repositories

### Test Coverage Gaps
1. Window-specific path handling
2. Very large files (>1GB)
3. Symlink cycles
4. Concurrent compression

## Debugging Failed Tests

### Check stderr Output
```bash
cargo test test_name -- --nocapture 2>&1
```

### Run Single Test
```bash
cargo test test_name -- --test-threads=1
```

### Get Full Backtrace
```bash
RUST_BACKTRACE=full cargo test test_name
```

## Contributing New Tests

When adding tests:
1. Follow the naming convention `test_<feature>_<scenario>`
2. Group related tests in sections with clear comments
3. Use descriptive assertions with context
4. Clean up temporary files automatically (TempDir handles this)
5. Test both success and failure paths
6. Include edge cases

## Test Philosophy

Tests in this suite follow these principles:

1. **Quality over Quantity** - Each test validates a real use case
2. **Comprehensive** - Cover happy paths, edge cases, and errors
3. **Independent** - Tests don't depend on each other
4. **Fast** - All tests run in <5 minutes
5. **Reliable** - No flaky or timing-dependent tests
6. **Clear** - Easy to understand what's being tested
7. **Maintainable** - Use helper functions and fixtures
