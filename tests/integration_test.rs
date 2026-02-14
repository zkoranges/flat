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

    create_test_file(temp_dir.path(), "config.toml", "[package]\nname = \"test\"\n");

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
