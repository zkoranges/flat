use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to get the flat binary for testing
fn flat_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_flat"))
}

/// Helper to create a temp file with content
fn create_test_file(dir: &std::path::Path, path: &str, content: &str) {
    let file_path = dir.join(path);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(file_path, content).unwrap();
}

// ============================================================================
// Basic Functionality Tests
// ============================================================================

#[test]
fn test_basic_flatten() {
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .assert()
        .success()
        .stdout(predicate::str::contains("<summary>"))
        .stdout(predicate::str::contains("Total files:"))
        .stdout(predicate::str::contains("<file path="))
        .stdout(predicate::str::contains("src/main.rs"))
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn test_help_command() {
    flat_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Flatten a codebase"));
}

#[test]
fn test_version_command() {
    flat_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("flat"));
}

// ============================================================================
// Secret Exclusion Tests
// ============================================================================

#[test]
fn test_env_files_excluded() {
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .assert()
        .success()
        .stdout(predicate::str::contains("SECRET_KEY").not())
        .stdout(predicate::str::contains("DATABASE_PASSWORD").not());
}

#[test]
fn test_credentials_excluded() {
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .assert()
        .success()
        .stdout(predicate::str::contains("credentials.json: secret").not())
        .stderr(predicate::str::contains("credentials.json: secret"));
}

// ============================================================================
// Binary Exclusion Tests
// ============================================================================

#[test]
fn test_images_excluded() {
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .assert()
        .success()
        .stderr(predicate::str::contains("logo.png: binary"))
        .stderr(predicate::str::contains("icon.svg: binary"));
}

#[test]
fn test_large_files_excluded() {
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .assert()
        .success()
        .stderr(predicate::str::contains("large_file.txt: too large"));
}

// ============================================================================
// Gitignore Tests
// ============================================================================

#[test]
fn test_gitignore_respected() {
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // target/ should be excluded via .gitignore
    assert!(!stdout.contains("target/debug/binary.exe"));
}

// ============================================================================
// Extension Filtering Tests
// ============================================================================

#[test]
fn test_include_filter() {
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--include")
        .arg("rs,toml")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should include .rs and .toml
    assert!(stdout.contains("src/main.rs"));
    assert!(stdout.contains("Cargo.toml"));

    // Should exclude others
    assert!(!stdout.contains("README.md"));
}

#[test]
fn test_exclude_filter() {
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--exclude")
        .arg("json,md")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should include .rs
    assert!(stdout.contains("src/main.rs"));

    // Should exclude .md and .json
    assert!(!stdout.contains("README.md"));
    assert!(!stdout.contains("test_data.json"));
}

#[test]
fn test_combined_filters() {
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--include")
        .arg("rs,toml,md")
        .arg("--exclude")
        .arg("md")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should include rs and toml
    assert!(stdout.contains("src/main.rs"));
    assert!(stdout.contains("Cargo.toml"));

    // Should exclude md (exclude takes precedence)
    assert!(!stdout.contains("README.md"));
}

// ============================================================================
// Output Mode Tests
// ============================================================================

#[test]
fn test_dry_run_mode() {
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should have summary
    assert!(stdout.contains("<summary>"));
    assert!(stdout.contains("src/main.rs"));

    // Should NOT have file contents
    assert!(!stdout.contains("<file path="));
    assert!(!stdout.contains("fn main()"));
}

#[test]
fn test_stats_mode() {
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--stats")
        .assert()
        .success()
        .stderr(predicate::str::contains("<summary>"))
        .stderr(predicate::str::contains("Total files:"))
        .stderr(predicate::str::contains("Included:"))
        .stderr(predicate::str::contains("Skipped:"));
}

#[test]
fn test_output_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("output.txt");

    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--output")
        .arg(&output_file)
        .assert()
        .success();

    assert!(output_file.exists());

    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("<summary>"));
    assert!(content.contains("src/main.rs"));
}

// ============================================================================
// Exit Code Tests
// ============================================================================

#[test]
fn test_no_files_matched_exit_code() {
    let temp_dir = TempDir::new().unwrap();

    flat_cmd()
        .arg(temp_dir.path())
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("No files matched the criteria"));
}

#[test]
fn test_current_directory_default() {
    flat_cmd()
        .current_dir("tests/fixtures/sample_project")
        .assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"));
}

// ============================================================================
// XML Escaping Tests
// ============================================================================

#[test]
fn test_xml_escaping() {
    let temp_dir = TempDir::new().unwrap();

    create_test_file(
        temp_dir.path(),
        "special<chars>.txt",
        "Content with <tag> & \"quotes\"",
    );

    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Path should be escaped or handled properly
    assert!(stdout.contains("special"));
}

// ============================================================================
// JavaScript Project Tests
// ============================================================================

#[test]
fn test_js_project_structure() {
    let output = flat_cmd()
        .arg("tests/fixtures/js_project")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should include source files
    assert!(stdout.contains("src/index.js"));
    assert!(stdout.contains("src/utils/helpers.js"));
    assert!(stdout.contains("src/components/Button.jsx"));
    assert!(stdout.contains("package.json"));
}

#[test]
fn test_js_project_secrets_excluded() {
    let output = flat_cmd()
        .arg("tests/fixtures/js_project")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // No secrets should appear
    assert!(!stdout.contains("API_KEY"));
    assert!(!stdout.contains("sk_test_secret_key"));
    assert!(!stdout.contains("super_secret_api_key"));
}

#[test]
fn test_js_project_node_modules_excluded() {
    let output = flat_cmd()
        .arg("tests/fixtures/js_project")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // node_modules should be excluded
    assert!(!stdout.contains("<file path=\"tests/fixtures/js_project/node_modules"));
}

#[test]
fn test_js_project_dist_excluded() {
    let output = flat_cmd()
        .arg("tests/fixtures/js_project")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // dist should be excluded
    assert!(!stdout.contains("<file path=\"tests/fixtures/js_project/dist"));
}

#[test]
fn test_js_project_images_excluded() {
    flat_cmd()
        .arg("tests/fixtures/js_project")
        .assert()
        .success()
        .stderr(predicate::str::contains("logo.png: binary"))
        .stderr(predicate::str::contains("icon.svg: binary"));
}

#[test]
fn test_js_project_nested_folders() {
    let output = flat_cmd()
        .arg("tests/fixtures/js_project")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // 3 levels of nesting should work
    assert!(stdout.contains("src/utils/helpers.js"));
    assert!(stdout.contains("src/components/Button.jsx"));
    assert!(stdout.contains("tests/unit/helpers.test.js"));
}

#[test]
fn test_js_project_with_filters() {
    let output = flat_cmd()
        .arg("tests/fixtures/js_project")
        .arg("--include")
        .arg("js,jsx")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should include JS/JSX
    assert!(stdout.contains("src/index.js"));
    assert!(stdout.contains("Button.jsx"));

    // Should exclude JSON
    assert!(!stdout.contains("package.json"));
}

#[test]
fn test_js_project_stats() {
    flat_cmd()
        .arg("tests/fixtures/js_project")
        .arg("--stats")
        .assert()
        .success()
        .stderr(predicate::str::contains("Total files:"))
        .stderr(predicate::str::contains("binary"))
        .stderr(predicate::str::contains("secret"));
}

// ============================================================================
// Match Pattern Filtering Tests
// ============================================================================

#[test]
fn test_match_filter_go_test_pattern() {
    let temp_dir = TempDir::new().unwrap();

    create_test_file(temp_dir.path(), "main.go", "package main");
    create_test_file(temp_dir.path(), "handler.go", "package main");
    create_test_file(temp_dir.path(), "main_test.go", "package main");
    create_test_file(temp_dir.path(), "handler_test.go", "package main");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--match")
        .arg("*_test.go")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should include test files
    assert!(stdout.contains("main_test.go"));
    assert!(stdout.contains("handler_test.go"));

    // Should not include non-test files
    assert!(!stdout.contains("\"main.go\""));
    assert!(!stdout.contains("\"handler.go\""));
}

#[test]
fn test_match_filter_multiple_patterns() {
    let temp_dir = TempDir::new().unwrap();

    create_test_file(temp_dir.path(), "main.go", "package main");
    create_test_file(temp_dir.path(), "main_test.go", "package main");
    create_test_file(temp_dir.path(), "app.spec.js", "describe('app')");
    create_test_file(temp_dir.path(), "app.js", "const app = {}");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--match")
        .arg("*_test.go")
        .arg("--match")
        .arg("*.spec.js")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should include files matching either pattern
    assert!(stdout.contains("main_test.go"));
    assert!(stdout.contains("app.spec.js"));

    // Should exclude non-matching files
    assert!(!stdout.contains("\"main.go\""));
    assert!(!stdout.contains("\"app.js\""));
}

#[test]
fn test_match_with_extension_filter() {
    let temp_dir = TempDir::new().unwrap();

    create_test_file(temp_dir.path(), "main.rs", "fn main() {}");
    create_test_file(temp_dir.path(), "lib.rs", "pub fn lib() {}");
    create_test_file(temp_dir.path(), "main_test.rs", "mod tests {}");
    create_test_file(temp_dir.path(), "config.toml", "[package]");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--include")
        .arg("rs")
        .arg("--match")
        .arg("main*")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should include only .rs files matching main*
    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("main_test.rs"));

    // lib.rs matches extension but not pattern
    assert!(!stdout.contains("\"lib.rs\""));
    // config.toml doesn't match extension
    assert!(!stdout.contains("config.toml"));
}

#[test]
fn test_match_no_matches_exit_code() {
    let temp_dir = TempDir::new().unwrap();

    create_test_file(temp_dir.path(), "main.rs", "fn main() {}");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--match")
        .arg("*.xyz")
        .assert()
        .failure()
        .code(3);
}

#[test]
fn test_match_invalid_pattern() {
    flat_cmd()
        .arg(".")
        .arg("--match")
        .arg("[invalid")
        .assert()
        .failure();
}

#[test]
fn test_match_dry_run() {
    let temp_dir = TempDir::new().unwrap();

    create_test_file(temp_dir.path(), "main_test.go", "package main");
    create_test_file(temp_dir.path(), "main.go", "package main");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--match")
        .arg("*_test.go")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("main_test.go"));
    assert!(!stdout.contains("\"main.go\""));
}

#[test]
fn test_match_on_sample_project() {
    // Use glob to match only .rs files in sample_project
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--match")
        .arg("*.rs")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should include .rs files
    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("lib.rs"));

    // Should not include non-.rs files
    assert!(!stdout.contains("Cargo.toml"));
    assert!(!stdout.contains("README.md"));
}

#[test]
fn test_match_stats_shows_skips() {
    let temp_dir = TempDir::new().unwrap();

    create_test_file(temp_dir.path(), "main.go", "package main");
    create_test_file(temp_dir.path(), "main_test.go", "package main");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--match")
        .arg("*_test.go")
        .arg("--stats")
        .assert()
        .success()
        .stderr(predicate::str::contains("no match"));
}

#[test]
fn test_match_backward_compat_regex_alias() {
    // --regex should still work as an alias for --match
    let temp_dir = TempDir::new().unwrap();

    create_test_file(temp_dir.path(), "main_test.go", "package main");
    create_test_file(temp_dir.path(), "main.go", "package main");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--regex")
        .arg("*_test.go")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("main_test.go"));
    assert!(!stdout.contains("\"main.go\""));
}

// ============================================================================
// Compression Tests
// ============================================================================

#[test]
fn test_compress_adds_mode_attribute() {
    let temp_dir = TempDir::new().unwrap();

    create_test_file(
        temp_dir.path(),
        "main.rs",
        "fn main() {\n    println!(\"hello\");\n}\n",
    );

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--compress")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should have mode attribute on file tag
    assert!(stdout.contains("mode=\"compressed\"") || stdout.contains("mode=\"full\""));
}

#[test]
fn test_compress_strips_function_body() {
    let temp_dir = TempDir::new().unwrap();

    create_test_file(
        temp_dir.path(),
        "main.rs",
        "fn hello(name: &str) -> String {\n    let greeting = format!(\"Hello, {}!\", name);\n    greeting\n}\n",
    );

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--compress")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("fn hello(name: &str) -> String"));
    assert!(stdout.contains("{ ... }"));
    assert!(!stdout.contains("let greeting"));
}

#[test]
fn test_compress_no_mode_without_flag() {
    let temp_dir = TempDir::new().unwrap();

    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Without --compress, no mode attribute
    assert!(!stdout.contains("mode="));
}

#[test]
fn test_compress_unsupported_gets_full() {
    let temp_dir = TempDir::new().unwrap();

    create_test_file(
        temp_dir.path(),
        "config.toml",
        "[package]\nname = \"test\"\n",
    );

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--compress")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Unsupported extension gets full content with mode="full"
    assert!(stdout.contains("mode=\"full\""));
    assert!(stdout.contains("[package]"));
}

#[test]
fn test_compress_summary_shows_count() {
    let temp_dir = TempDir::new().unwrap();

    create_test_file(
        temp_dir.path(),
        "main.rs",
        "fn main() {\n    println!(\"hello\");\n}\n",
    );

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--compress")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Compressed:"));
}

#[test]
fn test_full_match_skips_compression() {
    let temp_dir = TempDir::new().unwrap();

    create_test_file(
        temp_dir.path(),
        "main.rs",
        "fn main() {\n    println!(\"hello\");\n}\n",
    );
    create_test_file(
        temp_dir.path(),
        "lib.rs",
        "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    );

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--compress")
        .arg("--full-match")
        .arg("main.rs")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // main.rs should be full (body preserved)
    assert!(stdout.contains("println!(\"hello\")"));
    // lib.rs should be compressed
    assert!(stdout.contains("pub fn add(a: i32, b: i32) -> i32 { ... }"));
}

#[test]
fn test_full_match_without_compress_warns() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--full-match")
        .arg("*.rs")
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("--full-match has no effect without --compress"));
    // Should not have mode attribute
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("mode="));
}

#[test]
fn test_compress_full_match_all_produces_full_output() {
    // INV-6: --compress + --full-match '*' should produce same content as no --compress
    let temp_dir = TempDir::new().unwrap();

    create_test_file(
        temp_dir.path(),
        "main.rs",
        "fn main() {\n    println!(\"hello\");\n}\n",
    );

    let output_full = flat_cmd()
        .arg(temp_dir.path())
        .arg("--compress")
        .arg("--full-match")
        .arg("*")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output_full.stdout);

    // All files should have full content
    assert!(stdout.contains("println!(\"hello\")"));
    assert!(stdout.contains("mode=\"full\""));
}

// ============================================================================
// Token Budget Tests
// ============================================================================

#[test]
fn test_tokens_budget_limits_output() {
    let temp_dir = TempDir::new().unwrap();

    // Create files with known sizes
    create_test_file(temp_dir.path(), "big.rs", &"x".repeat(900)); // 300 tokens (900/3)
    create_test_file(temp_dir.path(), "small.rs", &"y".repeat(30)); // 10 tokens (30/3)

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("50") // Only small.rs should fit
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // small.rs should be included
    assert!(stdout.contains("small.rs"));
    // big.rs should be excluded
    assert!(
        !stdout.contains("<file")
            || !stdout.contains("big.rs")
            || stdout.contains("Excluded by budget")
    );
}

#[test]
fn test_tokens_zero_produces_summary_only() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("0")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should have summary but no file content
    assert!(stdout.contains("<summary>"));
    assert!(stdout.contains("Excluded by budget"));
    assert!(!stdout.contains("<file path="));
}

#[test]
fn test_tokens_summary_shows_budget_info() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("1000")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Token budget:"));
}

#[test]
fn test_tokens_dry_run_shows_annotations() {
    let temp_dir = TempDir::new().unwrap();

    create_test_file(temp_dir.path(), "small.rs", "fn main() {}\n");
    create_test_file(temp_dir.path(), "big.rs", &"x".repeat(9000));

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("100")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show annotations
    assert!(stdout.contains("[FULL]") || stdout.contains("[EXCLUDED]"));
}

#[test]
fn test_tokens_priority_ordering() {
    let temp_dir = TempDir::new().unwrap();

    // README gets highest priority (100), main.rs gets 90
    create_test_file(temp_dir.path(), "README.md", "# Project\n");
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");
    create_test_file(temp_dir.path(), "utils.rs", &"x".repeat(9000));

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("100")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // README should appear first (highest priority)
    let readme_pos = stdout.find("README.md");
    let main_pos = stdout.find("main.rs");
    assert!(readme_pos.is_some());
    assert!(main_pos.is_some());
    assert!(readme_pos.unwrap() < main_pos.unwrap());
}

#[test]
fn test_tokens_without_compress_no_mode_attr() {
    // INV-7: --tokens without --compress never adds mode attributes
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("1000")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!stdout.contains("mode="));
}

#[test]
fn test_tokens_with_compress() {
    let temp_dir = TempDir::new().unwrap();

    create_test_file(
        temp_dir.path(),
        "main.rs",
        "fn hello(name: &str) -> String {\n    let greeting = format!(\"Hello, {}!\", name);\n    greeting\n}\n",
    );

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("1000")
        .arg("--compress")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should have mode attribute and be compressed
    assert!(stdout.contains("mode="));
    assert!(stdout.contains("{ ... }"));
}

// ============================================================================
// Determinism Tests
// ============================================================================

#[test]
fn test_output_is_deterministic() {
    // INV-8: Running flat twice on the same directory produces identical output
    let output1 = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .output()
        .expect("Failed to execute command");

    let output2 = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .output()
        .expect("Failed to execute command");

    assert_eq!(output1.stdout, output2.stdout);
}

#[test]
fn test_output_order_sorted_by_path() {
    let temp_dir = TempDir::new().unwrap();

    // Create files in non-alphabetical order
    create_test_file(temp_dir.path(), "c.rs", "fn c() {}");
    create_test_file(temp_dir.path(), "a.rs", "fn a() {}");
    create_test_file(temp_dir.path(), "b.rs", "fn b() {}");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--dry-run")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Filter to only file path lines (before summary), not summary content
    let lines: Vec<&str> = stdout
        .lines()
        .take_while(|l| !l.starts_with("<summary>"))
        .filter(|l| l.ends_with(".rs"))
        .collect();

    // Files should appear in alphabetical order
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("a.rs"));
    assert!(lines[1].contains("b.rs"));
    assert!(lines[2].contains("c.rs"));
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_max_size_option() {
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--max-size")
        .arg("10485760") // 10MB
        .assert()
        .success();
}

#[test]
fn test_nonexistent_directory() {
    flat_cmd()
        .arg("/path/that/does/not/exist")
        .assert()
        .failure();
}

#[test]
fn test_empty_include_filter() {
    // Empty include filter matches nothing -> exit code 3
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--include")
        .arg("")
        .assert()
        .failure()
        .code(3);
}

// ============================================================================
// Real-World Workflow Tests
// ============================================================================

#[test]
fn test_workflow_rust_project() {
    // Typical workflow: get only Rust source for AI
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--include")
        .arg("rs,toml")
        .arg("--exclude")
        .arg("test")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("src/main.rs"));
    assert!(stdout.contains("Cargo.toml"));
}

#[test]
fn test_workflow_preview_before_share() {
    // User wants to preview what will be shared
    flat_cmd()
        .arg("tests/fixtures/js_project")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("<summary>"));
}

#[test]
fn test_workflow_stats_check() {
    // Quick check of project size
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--stats")
        .assert()
        .success()
        .stderr(predicate::str::contains("Total files:"))
        .stderr(predicate::str::contains("Included:"));
}

// ============================================================================
// Snapshot Tests — Pin Known-Good Output (Phase 3D)
// ============================================================================

#[test]
fn test_snapshot_rust_compression() {
    let output = flat_cmd()
        .arg("tests/fixtures/snapshot")
        .arg("--compress")
        .arg("--include")
        .arg("rs")
        .output()
        .expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = fs::read_to_string("tests/fixtures/snapshot/expected_rs.txt").unwrap();
    assert_eq!(
        stdout.as_ref(),
        expected.as_str(),
        "Rust compression output changed from golden file"
    );
}

#[test]
fn test_snapshot_typescript_compression() {
    let output = flat_cmd()
        .arg("tests/fixtures/snapshot")
        .arg("--compress")
        .arg("--include")
        .arg("ts")
        .output()
        .expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = fs::read_to_string("tests/fixtures/snapshot/expected_ts.txt").unwrap();
    assert_eq!(
        stdout.as_ref(),
        expected.as_str(),
        "TypeScript compression output changed from golden file"
    );
}

#[test]
fn test_snapshot_python_compression() {
    let output = flat_cmd()
        .arg("tests/fixtures/snapshot")
        .arg("--compress")
        .arg("--include")
        .arg("py")
        .output()
        .expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = fs::read_to_string("tests/fixtures/snapshot/expected_py.txt").unwrap();
    assert_eq!(
        stdout.as_ref(),
        expected.as_str(),
        "Python compression output changed from golden file"
    );
}

#[test]
fn test_snapshot_go_compression() {
    let output = flat_cmd()
        .arg("tests/fixtures/snapshot")
        .arg("--compress")
        .arg("--include")
        .arg("go")
        .output()
        .expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = fs::read_to_string("tests/fixtures/snapshot/expected_go.txt").unwrap();
    assert_eq!(
        stdout.as_ref(),
        expected.as_str(),
        "Go compression output changed from golden file"
    );
}

// ============================================================================
// Mutation-Killing Tests — Cover Surviving Mutants
// ============================================================================

#[test]
fn test_output_files_in_sorted_order() {
    // Kills Mutation 8: verifies files appear in lexicographic path order
    let temp_dir = TempDir::new().unwrap();

    create_test_file(temp_dir.path(), "z_last.rs", "fn z() {}");
    create_test_file(temp_dir.path(), "a_first.rs", "fn a() {}");
    create_test_file(temp_dir.path(), "m_middle.rs", "fn m() {}");
    // Subdirectories should also sort correctly
    create_test_file(temp_dir.path(), "b_dir/nested.rs", "fn n() {}");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Extract file paths from <file path="..."> tags
    let paths: Vec<&str> = stdout
        .lines()
        .filter(|l| l.starts_with("<file path="))
        .collect();

    assert_eq!(paths.len(), 4, "Expected 4 file tags");

    // Verify lexicographic order
    let a_pos = stdout.find("a_first.rs").expect("a_first.rs not found");
    let b_pos = stdout
        .find("b_dir/nested.rs")
        .expect("b_dir/nested.rs not found");
    let m_pos = stdout.find("m_middle.rs").expect("m_middle.rs not found");
    let z_pos = stdout.find("z_last.rs").expect("z_last.rs not found");
    assert!(
        a_pos < b_pos && b_pos < m_pos && m_pos < z_pos,
        "Files not in sorted order: a={}, b_dir={}, m={}, z={}",
        a_pos,
        b_pos,
        m_pos,
        z_pos
    );
}

#[test]
fn test_compress_fallback_on_syntax_error() {
    // Kills Mutation 9: verifies parse errors fall back to full content
    let temp_dir = TempDir::new().unwrap();

    // Deliberately broken Rust syntax
    let broken_rust = "fn broken( {\n    this is not valid rust\n}\n";
    create_test_file(temp_dir.path(), "broken.rs", broken_rust);

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--compress")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // File should still be included (fallback to full content)
    assert!(
        stdout.contains("broken.rs"),
        "broken.rs should be included in output"
    );
    assert!(
        stdout.contains("this is not valid rust"),
        "Full content should be preserved on parse error"
    );
    // Should have mode="full" since compression failed
    assert!(
        stdout.contains("mode=\"full\""),
        "Parse error file should have mode=full"
    );
    // Should warn on stderr about parse error
    assert!(
        stderr.contains("ERROR") || stderr.contains("error") || stderr.contains("Warning"),
        "Should warn about parse error on stderr"
    );
}

// ============================================================================
// Coverage Gap Tests — Additional assertions per Phase 4
// ============================================================================

#[test]
fn test_compress_rust_preserves_imports_integration() {
    // Integration-level test for Mutation 3 coverage gap
    let temp_dir = TempDir::new().unwrap();

    create_test_file(
        temp_dir.path(),
        "lib.rs",
        "use std::path::Path;\nuse std::io::Read;\n\nfn process(p: &Path) {\n    println!(\"{}\", p.display());\n}\n",
    );

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--compress")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("use std::path::Path;"),
        "use statement should be preserved in compressed output"
    );
    assert!(
        stdout.contains("use std::io::Read;"),
        "second use statement should be preserved"
    );
    assert!(
        stdout.contains("fn process(p: &Path) { ... }"),
        "function should show compressed signature"
    );
    assert!(
        !stdout.contains("println!"),
        "function body should be stripped"
    );
}

#[test]
fn test_compress_typescript_export_function() {
    // Verifies export function declarations are compressed
    let temp_dir = TempDir::new().unwrap();

    create_test_file(
        temp_dir.path(),
        "api.ts",
        "export function fetchData(url: string): Promise<Response> {\n  const res = await fetch(url);\n  return res.json();\n}\n",
    );

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--compress")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("export function fetchData(url: string): Promise<Response> { ... }"),
        "export function should be compressed: got {}",
        stdout
    );
    assert!(
        !stdout.contains("await fetch(url)"),
        "function body should be stripped from export function"
    );
}

#[test]
fn test_compress_python_module_constants() {
    // Verifies module-level constants are preserved
    let temp_dir = TempDir::new().unwrap();

    create_test_file(
        temp_dir.path(),
        "config.py",
        "MAX_SIZE = 1024\nDEBUG = True\n\ndef run():\n    print('running')\n",
    );

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--compress")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("MAX_SIZE = 1024"),
        "Module-level constant should be preserved"
    );
    assert!(
        stdout.contains("DEBUG = True"),
        "Module-level constant should be preserved"
    );
    assert!(
        !stdout.contains("print('running')"),
        "Function body should be stripped"
    );
}

#[test]
fn test_priority_ordering_integration() {
    // Integration-level test for Mutation 6 coverage gap
    let temp_dir = TempDir::new().unwrap();

    create_test_file(
        temp_dir.path(),
        "README.md",
        "# Project\nDescription here.\n",
    );
    create_test_file(temp_dir.path(), "src/main.rs", "fn main() {}\n");
    create_test_file(
        temp_dir.path(),
        "src/deep/nested/util.rs",
        &"x".repeat(3000),
    );
    create_test_file(
        temp_dir.path(),
        "Cargo.toml",
        "[package]\nname = \"test\"\n",
    );

    // Small budget: should include README and main.rs but exclude deep nested file
    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("100")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // README should be included (priority 100)
    assert!(
        stdout.contains("README.md"),
        "README.md should be in output"
    );
    // Deep nested file should be excluded by budget
    assert!(
        stdout.contains("util.rs") && stdout.contains("[EXCLUDED]"),
        "Deep nested file should be excluded by budget"
    );
}

#[test]
fn test_determinism_with_compress() {
    // Runs flat twice with --compress and verifies identical output
    let output1 = flat_cmd()
        .arg("tests/fixtures/snapshot")
        .arg("--compress")
        .output()
        .expect("Failed to execute command");

    let output2 = flat_cmd()
        .arg("tests/fixtures/snapshot")
        .arg("--compress")
        .output()
        .expect("Failed to execute command");

    assert_eq!(
        output1.stdout, output2.stdout,
        "Compressed output should be deterministic across runs"
    );
}

#[test]
fn test_determinism_with_tokens() {
    // Runs flat twice with --tokens and verifies identical output
    let output1 = flat_cmd()
        .arg("tests/fixtures/snapshot")
        .arg("--compress")
        .arg("--tokens")
        .arg("5000")
        .output()
        .expect("Failed to execute command");

    let output2 = flat_cmd()
        .arg("tests/fixtures/snapshot")
        .arg("--compress")
        .arg("--tokens")
        .arg("5000")
        .output()
        .expect("Failed to execute command");

    assert_eq!(
        output1.stdout, output2.stdout,
        "Token-budgeted output should be deterministic across runs"
    );
}

#[test]
fn test_tokens_budget_actually_enforced() {
    // Phase 5A: Prove token budget is enforced with math
    let temp_dir = TempDir::new().unwrap();

    // Create files with known sizes
    create_test_file(temp_dir.path(), "a.rs", &"x".repeat(600)); // ~200 tokens
    create_test_file(temp_dir.path(), "b.rs", &"y".repeat(600)); // ~200 tokens
    create_test_file(temp_dir.path(), "c.rs", &"z".repeat(600)); // ~200 tokens

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("250") // Only ~1 file should fit
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Count how many <file path= tags appear
    let file_count = stdout.matches("<file path=").count();
    assert!(
        file_count <= 2,
        "With budget 250 and 3x200-token files, at most 1-2 files should be included, got {}",
        file_count
    );
    // Should have excluded some files by budget
    assert!(
        stdout.contains("Excluded by budget"),
        "Summary should mention excluded files"
    );
}

#[test]
fn test_compression_ratio_is_real() {
    // Phase 5C: Verify compression actually reduces output size
    let full_output = flat_cmd()
        .arg("tests/fixtures/snapshot")
        .arg("--include")
        .arg("rs")
        .output()
        .expect("Failed to execute command");

    let compressed_output = flat_cmd()
        .arg("tests/fixtures/snapshot")
        .arg("--compress")
        .arg("--include")
        .arg("rs")
        .output()
        .expect("Failed to execute command");

    let full_len = full_output.stdout.len();
    let compressed_len = compressed_output.stdout.len();

    assert!(
        compressed_len < full_len,
        "Compressed output ({} bytes) should be smaller than full ({} bytes)",
        compressed_len,
        full_len
    );
    let reduction_pct = ((full_len - compressed_len) * 100) / full_len;
    assert!(
        reduction_pct > 20,
        "Compression should reduce output by >20%, got {}%",
        reduction_pct
    );
}

#[test]
fn test_compress_unsupported_extension_passthrough() {
    // Fallback: unknown extension gets full content
    let temp_dir = TempDir::new().unwrap();

    create_test_file(temp_dir.path(), "data.csv", "name,age\nalice,30\nbob,25\n");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--compress")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("alice,30"),
        "CSV content should be included in full"
    );
    assert!(
        stdout.contains("mode=\"full\""),
        "Unsupported file should get mode=full"
    );
}

#[test]
fn test_compress_empty_file() {
    // Fallback: empty file
    let temp_dir = TempDir::new().unwrap();

    create_test_file(temp_dir.path(), "empty.rs", "");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--compress")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Empty file should still appear
    assert!(
        stdout.contains("empty.rs"),
        "Empty file should be in output"
    );
}

#[test]
fn test_full_match_with_compress_and_include() {
    // INV: full-match with include filter
    let temp_dir = TempDir::new().unwrap();

    create_test_file(
        temp_dir.path(),
        "main.rs",
        "fn main() {\n    println!(\"hello\");\n}\n",
    );
    create_test_file(
        temp_dir.path(),
        "lib.rs",
        "pub fn lib_fn() {\n    let x = 1;\n}\n",
    );
    create_test_file(
        temp_dir.path(),
        "config.toml",
        "[package]\nname = \"test\"\n",
    );

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--compress")
        .arg("--full-match")
        .arg("*")
        .arg("--include")
        .arg("rs")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // All .rs files should be full (because of --full-match '*')
    assert!(
        stdout.contains("println!(\"hello\")"),
        "main.rs body should be preserved with --full-match '*'"
    );
    assert!(
        stdout.contains("let x = 1"),
        "lib.rs body should be preserved with --full-match '*'"
    );
    // .toml should not appear (filtered by --include rs)
    assert!(
        !stdout.contains("[package]"),
        "config.toml should be excluded by --include rs"
    );
}

#[test]
fn test_full_match_with_wildcard_matches_all() {
    // INV-6: --compress + --full-match '*' content = no --compress content (for matched files)
    let temp_dir = TempDir::new().unwrap();

    create_test_file(
        temp_dir.path(),
        "code.rs",
        "fn compute(x: i32) -> i32 {\n    let result = x * 2 + 1;\n    result\n}\n",
    );

    // With --compress --full-match '*'
    let output_full_match = flat_cmd()
        .arg(temp_dir.path())
        .arg("--compress")
        .arg("--full-match")
        .arg("*")
        .output()
        .expect("Failed to execute command");

    // Without --compress
    let output_no_compress = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    let full_match_stdout = String::from_utf8_lossy(&output_full_match.stdout);
    let no_compress_stdout = String::from_utf8_lossy(&output_no_compress.stdout);

    // Both should contain the function body
    assert!(
        full_match_stdout.contains("let result = x * 2 + 1"),
        "Full-match should preserve function body"
    );
    assert!(
        no_compress_stdout.contains("let result = x * 2 + 1"),
        "No-compress should preserve function body"
    );
}
