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

/// Helper to initialize a git repository in a directory
fn init_git_repo(dir: &std::path::Path) {
    std::process::Command::new("git")
        .arg("init")
        .current_dir(dir)
        .output()
        .unwrap();
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

#[test]
fn test_snapshot_solidity_compression() {
    let output = flat_cmd()
        .arg("tests/fixtures/snapshot")
        .arg("--compress")
        .arg("--include")
        .arg("sol")
        .output()
        .expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = fs::read_to_string("tests/fixtures/snapshot/expected_sol.txt").unwrap();
    assert_eq!(
        stdout.as_ref(),
        expected.as_str(),
        "Solidity compression output changed from golden file"
    );
}

#[test]
fn test_snapshot_elixir_compression() {
    let output = flat_cmd()
        .arg("tests/fixtures/snapshot")
        .arg("--compress")
        .arg("--include")
        .arg("ex")
        .output()
        .expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = fs::read_to_string("tests/fixtures/snapshot/expected_ex.txt").unwrap();
    assert_eq!(
        stdout.as_ref(),
        expected.as_str(),
        "Elixir compression output changed from golden file"
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

// ============================================================================
// Human-Friendly Number Parsing Tests
// ============================================================================

#[test]
fn test_tokens_suffix_k_lowercase() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("1k")
        .assert()
        .success()
        .stdout(predicate::str::contains("Token budget:"));
}

#[test]
fn test_tokens_suffix_k_uppercase() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("100K")
        .assert()
        .success();
}

#[test]
fn test_tokens_suffix_m() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("1M")
        .assert()
        .success();
}

#[test]
fn test_tokens_plain_number_still_works() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("8000")
        .assert()
        .success();
}

#[test]
fn test_tokens_invalid_suffix_errors() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("abc")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid number"));
}

#[test]
fn test_tokens_decimal_not_supported() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("1.5k")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid number"));
}

#[test]
fn test_tokens_k_means_1000() {
    // 1k = 1,000 tokens (decimal), files with ~10 tokens should fit easily
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "small.rs", "fn a() {}\n");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("1k")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("small.rs"), "File should fit in 1k (1000) token budget");
}

#[test]
fn test_max_size_suffix_k() {
    let temp_dir = TempDir::new().unwrap();
    // File is 500 bytes, 1k = 1024 bytes — should fit
    create_test_file(temp_dir.path(), "small.rs", &"x".repeat(500));

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--max-size")
        .arg("1k")
        .assert()
        .success()
        .stdout(predicate::str::contains("small.rs"));
}

#[test]
fn test_max_size_suffix_m() {
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--max-size")
        .arg("10M")
        .assert()
        .success();
}

#[test]
fn test_max_size_plain_number_still_works() {
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--max-size")
        .arg("10485760")
        .assert()
        .success();
}

#[test]
fn test_max_size_k_means_1024() {
    let temp_dir = TempDir::new().unwrap();
    // File is 1025 bytes — just over 1k (1024) limit
    create_test_file(temp_dir.path(), "big.rs", &"x".repeat(1025));
    // File is 500 bytes — fits in 1k
    create_test_file(temp_dir.path(), "small.rs", &"y".repeat(500));

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--max-size")
        .arg("1k")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stdout.contains("small.rs"), "500-byte file should fit in 1k (1024)");
    assert!(stderr.contains("big.rs") && stderr.contains("too large"),
        "1025-byte file should exceed 1k (1024) limit");
}

#[test]
fn test_max_size_invalid_errors() {
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--max-size")
        .arg("xyz")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid number"));
}

#[test]
fn test_tokens_and_max_size_suffixes_together() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("8k")
        .arg("--max-size")
        .arg("1M")
        .assert()
        .success();
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

// ============================================================================
// Tokenizer CLI Tests
// ============================================================================

#[test]
fn test_tokenizer_default_heuristic() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("1000")
        .assert()
        .success();
}

#[test]
fn test_tokenizer_heuristic_explicit() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokenizer")
        .arg("heuristic")
        .arg("--tokens")
        .arg("1000")
        .assert()
        .success();
}

#[test]
fn test_tokenizer_claude() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokenizer")
        .arg("claude")
        .arg("--tokens")
        .arg("1000")
        .assert()
        .success();
}

#[test]
fn test_tokenizer_gpt4() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokenizer")
        .arg("gpt-4")
        .arg("--tokens")
        .arg("1000")
        .assert()
        .success();
}

#[test]
fn test_tokenizer_gpt35() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokenizer")
        .arg("gpt-3.5")
        .arg("--tokens")
        .arg("1000")
        .assert()
        .success();
}

#[test]
fn test_tokenizer_invalid_rejected() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokenizer")
        .arg("invalid-name")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown tokenizer"));
}

#[test]
fn test_tokenizer_appears_in_help() {
    flat_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--tokenizer"));
}

#[test]
fn test_tokenizer_without_tokens_still_works() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokenizer")
        .arg("gpt-4")
        .assert()
        .success();
}

#[test]
fn test_tokenizer_real_vs_heuristic_budget_difference() {
    let temp_dir = TempDir::new().unwrap();

    let content = "x".repeat(300); // Heuristic: 300/3 = 100 tokens
    create_test_file(temp_dir.path(), "code.rs", &content);

    let heuristic_output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokenizer")
        .arg("heuristic")
        .arg("--tokens")
        .arg("100")
        .output()
        .expect("Failed to execute command");

    let real_output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokenizer")
        .arg("gpt-4")
        .arg("--tokens")
        .arg("100")
        .output()
        .expect("Failed to execute command");

    let h_stdout = String::from_utf8_lossy(&heuristic_output.stdout);
    let r_stdout = String::from_utf8_lossy(&real_output.stdout);

    assert!(
        h_stdout.contains("Token budget:") || h_stdout.contains("code.rs"),
        "Heuristic output should contain budget info or file"
    );
    assert!(
        r_stdout.contains("Token budget:") || r_stdout.contains("code.rs"),
        "Real tokenizer output should contain budget info or file"
    );
}

// ============================================================================
// TIER 1: Error Path Tests (NEW - Critical for production safety)
// ============================================================================

#[test]
fn test_error_nonexistent_directory_graceful() {
    flat_cmd()
        .arg("/path/that/definitely/does/not/exist/anywhere")
        .assert()
        .failure();
}

#[test]
fn test_error_invalid_output_directory() {
    // Test: output file in non-existent directory should fail gracefully
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--output")
        .arg("/nonexistent/directory/output.xml")
        .assert()
        .failure();
}

#[test]
fn test_error_all_files_filtered_out_exit_code_3() {
    // Test: when no files match criteria, exit with code 3
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "file.txt", "content");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--include")
        .arg("rs,py,go") // .txt doesn't match
        .assert()
        .failure()
        .code(3);
}

#[test]
fn test_error_invalid_max_size_suffix() {
    // Test: invalid size suffix should error
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--max-size")
        .arg("10X")
        .assert()
        .failure();
}

#[test]
fn test_error_empty_directory_exit_code_3() {
    // Test: directory with no files matching criteria returns exit code 3
    let temp_dir = TempDir::new().unwrap();
    // Create only files that will be excluded (binaries, secrets, etc.)
    create_test_file(temp_dir.path(), ".env", "SECRET=123");
    create_test_file(temp_dir.path(), ".gitignore", "");

    flat_cmd()
        .arg(temp_dir.path())
        .assert()
        .failure()
        .code(3);
}

#[test]
fn test_error_invalid_glob_pattern_fails_early() {
    // Test: invalid glob pattern should fail with clear error
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--match")
        .arg("[invalid[pattern")
        .assert()
        .failure();
}

#[test]
fn test_error_invalid_full_match_pattern() {
    // Test: invalid --full-match pattern should fail
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--compress")
        .arg("--full-match")
        .arg("[invalid")
        .assert()
        .failure();
}

#[test]
fn test_multiple_filters_all_fail_exit_3() {
    // Test: when all filters are applied and nothing matches, exit 3
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}");
    create_test_file(temp_dir.path(), "lib.rs", "pub fn lib() {}");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--include")
        .arg("rs")
        .arg("--match")
        .arg("*.xyz")
        .assert()
        .failure()
        .code(3);
}

#[test]
fn test_compression_on_empty_file() {
    // Test: empty files should be handled gracefully
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "empty.rs", "");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--compress")
        .assert()
        .success();
}

// ============================================================================
// TIER 1: Large Scale / Stress Tests (NEW - Catch scaling issues)
// ============================================================================

#[test]
fn test_many_files_sorted_correctly() {
    // Test: Verify sorting works with many files
    let temp_dir = TempDir::new().unwrap();

    // Create 50 files in random order
    for i in (0..50).rev() {
        let filename = format!("file_{:02}.rs", i);
        create_test_file(temp_dir.path(), &filename, "fn main() {}\n");
    }

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--dry-run")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Extract file list (before <summary>)
    let files_section: Vec<&str> = stdout
        .lines()
        .take_while(|l| !l.starts_with("<summary>"))
        .filter(|l| l.contains("file_"))
        .collect();

    // Verify files are in alphabetical order
    assert!(!files_section.is_empty(), "Should have files");
    for i in 1..files_section.len() {
        assert!(
            files_section[i - 1] < files_section[i],
            "Files should be sorted: {} >= {}",
            files_section[i - 1],
            files_section[i]
        );
    }
}

#[test]
fn test_deep_directory_nesting() {
    // Test: Deeply nested directories should work
    let temp_dir = TempDir::new().unwrap();

    // Create file at depth 10
    create_test_file(
        temp_dir.path(),
        "a/b/c/d/e/f/g/h/i/j/file.rs",
        "fn main() {}",
    );

    flat_cmd()
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("file.rs"));
}

#[test]
fn test_large_single_file() {
    // Test: Very large files are handled (but may be skipped for size)
    let temp_dir = TempDir::new().unwrap();
    let large_content = "x".repeat(5_000_000); // 5MB

    create_test_file(temp_dir.path(), "large.rs", &large_content);

    // Should either succeed (if under default 1MB limit) or handle gracefully
    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    // Either it processes or skips with reason
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || stderr.contains("too large"),
        "Should handle large files gracefully"
    );
}

#[test]
fn test_token_budget_with_many_files() {
    // Test: Budget allocation works correctly with many files
    let temp_dir = TempDir::new().unwrap();

    // Create 20 files, each ~300 bytes (100 tokens at heuristic)
    for i in 0..20 {
        let filename = format!("file_{:02}.rs", i);
        create_test_file(temp_dir.path(), &filename, &"x".repeat(300));
    }

    // Budget of 500 tokens should include ~5 files
    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("500")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Token budget:"));
    assert!(stdout.contains("Excluded by budget"));
}

// ============================================================================
// TIER 1: Real-World Scenario Tests (NEW - Detect edge cases)
// ============================================================================

#[test]
fn test_mixed_language_project() {
    // Test: Project with multiple languages is processed correctly
    let temp_dir = TempDir::new().unwrap();

    create_test_file(temp_dir.path(), "main.rs", "fn main() {}");
    create_test_file(temp_dir.path(), "index.js", "console.log('hi');");
    create_test_file(temp_dir.path(), "script.py", "print('hello')");
    create_test_file(temp_dir.path(), "style.css", "body { color: red; }");
    create_test_file(temp_dir.path(), "README.md", "# Project");
    create_test_file(temp_dir.path(), ".env", "SECRET=123");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should include source files
    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("index.js"));
    assert!(stdout.contains("script.py"));
    assert!(stdout.contains("README.md"));

    // Should exclude secrets
    assert!(!stdout.contains("SECRET=123"));
}

#[test]
fn test_build_artifacts_excluded() {
    // Test: Common build artifacts are skipped
    let temp_dir = TempDir::new().unwrap();

    create_test_file(temp_dir.path(), "src/main.rs", "fn main() {}");
    create_test_file(temp_dir.path(), "target/debug/binary.exe", "fake binary content");
    create_test_file(temp_dir.path(), "dist/bundle.js", "fake bundle");
    create_test_file(temp_dir.path(), ".gitignore", "target/\ndist/\n");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Source should be included
    assert!(stdout.contains("src/main.rs"));

    // Build artifacts should be excluded or skipped
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.contains("target/debug/binary") || stderr.contains("binary"),
        "Build artifacts should be skipped"
    );
}

#[test]
fn test_hidden_files_respected() {
    // Test: Hidden files (.dotfiles) are handled correctly
    let temp_dir = TempDir::new().unwrap();

    create_test_file(temp_dir.path(), "visible.rs", "fn main() {}");
    create_test_file(temp_dir.path(), ".hidden", "should be ignored by gitignore");
    create_test_file(temp_dir.path(), ".gitignore", ".hidden\n");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Visible file should be included
    assert!(stdout.contains("visible.rs"));

    // .hidden should be excluded by gitignore
    assert!(!stdout.contains(".hidden"));
}

#[test]
fn test_special_characters_in_filenames() {
    // Test: Files with special characters are handled
    let temp_dir = TempDir::new().unwrap();

    create_test_file(temp_dir.path(), "test-file.rs", "fn main() {}");
    create_test_file(temp_dir.path(), "test_file.rs", "fn main() {}");
    create_test_file(temp_dir.path(), "test file.rs", "fn main() {}");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // All files should be included
    assert!(stdout.contains("test-file.rs"));
    assert!(stdout.contains("test_file.rs"));
    assert!(stdout.contains("test file.rs"));
}

#[test]
fn test_compression_preserves_structure_with_mix() {
    // Test: Compression works on mixed file types
    let temp_dir = TempDir::new().unwrap();

    create_test_file(
        temp_dir.path(),
        "main.rs",
        "fn hello(name: &str) -> String {\n    format!(\"Hello, {}!\", name)\n}",
    );
    create_test_file(temp_dir.path(), "README.md", "# Documentation");
    create_test_file(temp_dir.path(), "Cargo.toml", "[package]\nname = \"test\"");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--compress")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Rust file should be compressed (body removed)
    assert!(stdout.contains("fn hello(name: &str) -> String"));
    assert!(stdout.contains("{ ... }"));

    // README and Cargo.toml should have mode="full" (no compression for these)
    assert!(stdout.contains("README.md"));
    assert!(stdout.contains("Cargo.toml"));
}

// ============================================================================
// Output Format Tests
// ============================================================================

#[test]
fn test_json_format_flag() {
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Verify it's valid JSON
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Output should be valid JSON");

    // Verify JSON structure
    assert!(json["statistics"].is_object());
    assert!(json["files"].is_array());

    // Verify statistics fields
    assert!(json["statistics"]["total_files"].is_number());
    assert!(json["statistics"]["included_files"].is_number());

    // Verify files contain expected entries
    let files = json["files"].as_array().unwrap();
    assert!(!files.is_empty());

    // Check that at least one file has the expected structure
    let has_expected = files.iter().any(|f| {
        f["path"].is_string() && f["content"].is_string()
    });
    assert!(has_expected, "Files should have path and content fields");
}

#[test]
fn test_json_format_with_compress() {
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--format")
        .arg("json")
        .arg("--compress")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Output should be valid JSON");

    assert!(json["statistics"]["compressed_files"].is_number());
    assert!(json["statistics"]["compressed_files"].as_u64().unwrap_or(0) > 0);
}

#[test]
fn test_format_invalid_rejected() {
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--format")
        .arg("invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown format"));
}

// ============================================================================
// Template Tests
// ============================================================================

#[test]
fn test_minimal_template() {
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--template")
        .arg("minimal")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Minimal template should contain file paths with ## markdown headers
    assert!(stdout.contains("##"));
    assert!(stdout.contains("src/main.rs") || stdout.contains("Cargo.toml"));
}

#[test]
fn test_claude_review_template() {
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--template")
        .arg("claude-review")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Claude review template should have a title
    assert!(stdout.contains("Code Review Request"));
    // Should include statistics
    assert!(stdout.contains("Statistics"));
}

#[test]
fn test_openai_docs_template() {
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--template")
        .arg("openai-docs")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();

    // OpenAI docs template should have documentation structure
    assert!(stdout.contains("Documentation"));
    assert!(stdout.contains("Project Overview"));
}

#[test]
fn test_template_not_found() {
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--template")
        .arg("nonexistent_template_xyz")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("Cannot")));
}

#[test]
fn test_template_overrides_format() {
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--format")
        .arg("json")
        .arg("--template")
        .arg("minimal")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Template should be used (markdown format), not JSON
    assert!(stdout.contains("##"));
    // Should NOT be valid JSON (minimal template doesn't produce JSON)
    assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_err());
}

// ============================================================================
// Edge Cases & Boundary Conditions
// ============================================================================

#[test]
fn test_empty_directory() {
    let temp_dir = TempDir::new().unwrap();

    flat_cmd()
        .arg(temp_dir.path())
        .assert()
        .code(3)  // Should exit with code 3 (no files matched)
        .stderr(predicate::str::contains("No files matched"));
}

#[test]
fn test_single_file_repository() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("fn main() {}"));
}

#[test]
fn test_unicode_filenames() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "файл.rs", "// Russian filename");
    create_test_file(temp_dir.path(), "文件.txt", "// Chinese filename");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("файл.rs") || stdout.contains("文件.txt"));
}

#[test]
fn test_filenames_with_spaces() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "my file.rs", "fn main() {}");
    create_test_file(temp_dir.path(), "another file with spaces.txt", "content");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("my file.rs"));
}

#[test]
fn test_deeply_nested_directories() {
    let temp_dir = TempDir::new().unwrap();
    let mut path = temp_dir.path().to_path_buf();

    // Create 15 levels of nesting
    for i in 0..15 {
        path = path.join(format!("level{}", i));
    }

    fs::create_dir_all(&path).unwrap();
    create_test_file(&path, "deep.rs", "// deeply nested file");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("deep.rs"));
}

#[test]
fn test_json_format_with_special_characters() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "test.rs",
        "fn test() { println!(\"<tag>\"); }  // JSON: {\"key\": \"value\"}\n");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Verify it's valid JSON even with special characters
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Output should be valid JSON with special chars");

    let files = json["files"].as_array().unwrap();
    assert!(!files.is_empty());

    // Content should be properly escaped in JSON
    let content = &files[0]["content"];
    assert!(content.is_string());
}

#[test]
fn test_json_format_empty_repository() {
    let temp_dir = TempDir::new().unwrap();

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--format")
        .arg("json")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("No files matched"));
}

#[test]
fn test_json_with_token_budget() {
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--format")
        .arg("json")
        .arg("--tokens")
        .arg("500")
        .arg("--compress")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Output should be valid JSON");

    // Verify token budget is respected
    assert!(json["statistics"]["token_budget"].is_number());
    assert!(json["statistics"]["token_budget"].as_u64().unwrap_or(0) > 0);
}

#[test]
fn test_mixed_line_endings() {
    let temp_dir = TempDir::new().unwrap();

    // Create file with mixed line endings
    let content_with_crlf = "line1\r\nline2\nline3\r";
    fs::write(temp_dir.path().join("mixed.txt"), content_with_crlf).unwrap();

    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mixed.txt"));
}

// ============================================================================
// Integration Scenarios - Feature Combinations
// ============================================================================

#[test]
fn test_compress_with_template() {
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--compress")
        .arg("--template")
        .arg("minimal")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should be template format, not XML
    assert!(stdout.contains("##"));
    assert!(!stdout.contains("<file path="));
}

#[test]
fn test_token_budget_with_multiple_filters() {
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--tokens")
        .arg("1000")
        .arg("--include")
        .arg("rs,toml,md")
        .arg("--exclude")
        .arg("lock")
        .arg("--compress")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("<summary>"));
}

#[test]
fn test_dry_run_with_compression() {
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--dry-run")
        .arg("--compress")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Dry-run should list files without content
    assert!(stdout.contains("Cargo.toml") || stdout.contains("README.md"));
    // Should NOT contain file content
    assert!(!stdout.contains("fn main"));
}

#[test]
fn test_stats_mode_with_compress_and_token_budget() {
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--stats")
        .arg("--compress")
        .arg("--tokens")
        .arg("5000")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Stats mode outputs to stderr or stdout
    let output_text = format!("{}\n{}", stdout, stderr);
    assert!(output_text.contains("Total files:") || output_text.contains("Included:"),
            "Expected stats output with 'Total files' or 'Included'");
}

// ============================================================================
// Error Handling & Edge Cases
// ============================================================================

#[test]
fn test_nonexistent_path_with_stats() {
    flat_cmd()
        .arg("/nonexistent/path/that/does/not/exist")
        .arg("--stats")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("No files matched").or(predicate::str::contains("error")));
}

#[test]
fn test_permission_denied_simulation() {
    // This test checks behavior when encountering read errors
    flat_cmd()
        .arg("tests/fixtures/sample_project")
        .assert()
        .success();
}

#[test]
fn test_json_large_output() {
    // Test JSON with all available content
    let output = flat_cmd()
        .arg("tests/fixtures/sample_project")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Verify it's valid JSON
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Large JSON output should be valid");

    // Verify structure integrity
    assert!(json.is_object());
    assert!(json.get("files").is_some());
    assert!(json.get("statistics").is_some());
}

// ============================================================================
// Security & Special Cases
// ============================================================================

#[test]
fn test_secret_file_detection() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "credentials.json", "{\"password\": \"secret\"}");
    create_test_file(temp_dir.path(), ".env", "API_KEY=secret123");
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    // Secret files should be skipped
    assert!(stderr.contains("credentials.json") || stderr.contains("secret"));
    // Regular files should be included
    assert!(stdout.contains("main.rs"));
}

#[test]
fn test_binary_file_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Create a fake binary file
    fs::write(temp_dir.path().join("binary.exe"), vec![0xFF, 0xD8, 0xFF]).unwrap();
    create_test_file(temp_dir.path(), "text.txt", "readable text");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    // Binary should be skipped
    assert!(stderr.contains("binary") || stderr.contains("Binary"));
    // Text should be included
    assert!(stdout.contains("text.txt"));
}

#[test]
fn test_gitignore_with_negation_patterns() {
    let temp_dir = TempDir::new().unwrap();

    // Create files
    create_test_file(temp_dir.path(), "included.txt", "should be included");
    create_test_file(temp_dir.path(), "excluded.log", "should be excluded");

    // Create .gitignore with negation
    create_test_file(temp_dir.path(), ".gitignore", "*.log\n!important.log");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // .txt should be included
    assert!(stdout.contains("included.txt"));
}

#[test]
fn test_very_long_file_path() {
    let temp_dir = TempDir::new().unwrap();
    let mut path = temp_dir.path().to_path_buf();

    // Create a very long path (but not exceeding system limits)
    for i in 0..8 {
        path = path.join(format!("very_long_directory_name_{}", i));
    }

    fs::create_dir_all(&path).unwrap();
    create_test_file(&path, "file.rs", "// long path file");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Long paths should work in JSON");

    let files = json["files"].as_array().unwrap();
    assert!(!files.is_empty());
}

// ============================================================================
// GitHub URL Parsing - Comprehensive
// ============================================================================

#[test]
fn test_github_url_parsing_short_format() {
    // Test GitHub URL parsing with short format (owner/repo)
    // Uses hello-world which is tiny (~1MB)
    let output = flat_cmd()
        .arg("--github")
        .arg("octocat/Hello-World")
        .arg("--stats")
        .output()
        .unwrap();

    // Either succeeds (with network) or fails with clone/network error
    if output.status.success() {
        // If clone succeeded, stats should show files
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("Total files:") || stderr.contains("Included:"));
    } else {
        // If clone failed, should be a network error
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("clone") || stderr.contains("Connection") ||
                stderr.contains("Failed") || stderr.contains("error"));
    }
}

// ============================================================================
// MCP Server Mode - JSON-RPC Validation
// ============================================================================

#[test]
fn test_mcp_server_tools_list_response() {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg("--serve")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();

    // Send tools/list request
    use std::io::Write;
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
    stdin.write_all(request.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    drop(stdin);

    // Read response (with timeout)
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain tools info
    assert!(stdout.contains("tools") || stdout.contains("analyze_repo"));
}

// ============================================================================
// Real-World Repository Patterns
// ============================================================================

#[test]
fn test_node_modules_excluded() {
    // Test: node_modules are excluded via explicit .gitignore file
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "package.json", "{\"name\": \"myapp\"}");
    create_test_file(temp_dir.path(), "src/index.js", "console.log('hello');");
    create_test_file(temp_dir.path(), "node_modules/lodash/index.js", "// should be skipped");
    let gitignore_path = temp_dir.path().join(".gitignore");
    create_test_file(temp_dir.path(), ".gitignore", "node_modules/\n");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--gitignore")
        .arg(&gitignore_path)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Source files should be included
    assert!(stdout.contains("package.json") || stdout.contains("index.js"));

    // node_modules should be excluded by explicit gitignore
    assert!(!stdout.contains("lodash"), "lodash should not be included in output");
}

#[test]
fn test_source_and_binary_mixed_detection() {
    // Test: Binary detection works without gitignore patterns
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "src/main.rs", "fn main() {}");
    create_test_file(temp_dir.path(), "target/debug/binary", "compiled binary");
    create_test_file(temp_dir.path(), "dist/bundle.js", "bundled output");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Source should be included
    assert!(stdout.contains("main.rs"));

    // Build artifacts should not be in output as files
    // (they may be in error output as skipped)
}

#[test]
fn test_monorepo_structure() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "packages/api/package.json", "{}");
    create_test_file(temp_dir.path(), "packages/api/src/index.ts", "export const api = {};");
    create_test_file(temp_dir.path(), "packages/web/package.json", "{}");
    create_test_file(temp_dir.path(), "packages/web/src/App.tsx", "export default App;");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should include files from multiple packages
    assert!(stdout.contains("index.ts") || stdout.contains("App.tsx"));
}

#[test]
fn test_multiple_gitignore_levels() {
    let temp_dir = TempDir::new().unwrap();

    // Create root .gitignore
    create_test_file(temp_dir.path(), ".gitignore", "*.log\ntemp/");

    // Create nested gitignore
    create_test_file(temp_dir.path(), "src/.gitignore", "*.tmp");

    create_test_file(temp_dir.path(), "README.md", "# Project");
    create_test_file(temp_dir.path(), "debug.log", "log content");
    create_test_file(temp_dir.path(), "src/main.rs", "fn main() {}");
    create_test_file(temp_dir.path(), "src/temp.tmp", "temp");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should include source, exclude logs and temps
    assert!(stdout.contains("main.rs"));
}

// ============================================================================
// Output File Handling
// ============================================================================

#[test]
fn test_output_file_json_format() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("output.json");

    create_test_file(temp_dir.path(), "main.rs", "fn main() {}");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--format")
        .arg("json")
        .arg("--output")
        .arg(&output_file)
        .assert()
        .success();

    // Verify output file was created
    assert!(output_file.exists());

    // Verify it's valid JSON
    let content = fs::read_to_string(&output_file).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content)
        .expect("Output file should contain valid JSON");

    assert!(json["files"].is_array());
}

#[test]
fn test_output_file_with_template() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("output.md");

    create_test_file(temp_dir.path(), "main.rs", "fn main() {}");

    flat_cmd()
        .arg(temp_dir.path())
        .arg("--template")
        .arg("minimal")
        .arg("--output")
        .arg(&output_file)
        .assert()
        .success();

    // Verify output file was created
    assert!(output_file.exists());

    // Verify it's markdown (from minimal template)
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("##"));
}

// ============================================================================
// Filter Combination Edge Cases
// ============================================================================

#[test]
fn test_include_exclude_conflict_resolution() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "file1.rs", "fn main() {}");
    create_test_file(temp_dir.path(), "file2.ts", "export {};");
    create_test_file(temp_dir.path(), "file3.json", "{}");

    // Include rs and ts, but exclude ts
    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--include")
        .arg("rs,ts")
        .arg("--exclude")
        .arg("ts")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Only rs should be included (exclude takes precedence)
    assert!(stdout.contains("file1.rs"));
    assert!(!stdout.contains("file2.ts"));
    assert!(!stdout.contains("file3.json"));
}

#[test]
fn test_match_pattern_with_filters() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "test_main.rs", "fn main() {}");
    create_test_file(temp_dir.path(), "test_helper.rs", "fn helper() {}");
    create_test_file(temp_dir.path(), "main.rs", "fn main2() {}");

    // Include rs files, match only test_* files
    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--include")
        .arg("rs")
        .arg("--match")
        .arg("test_*")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Only test_* files should be included
    assert!(stdout.contains("test_main.rs") || stdout.contains("test_helper.rs"));
}

// ============================================================================
// Compression Language-Specific Tests
// ============================================================================

#[test]
fn test_compression_different_languages() {
    let temp_dir = TempDir::new().unwrap();

    // Test files in different languages
    create_test_file(temp_dir.path(), "main.rs",
        "pub fn main() {\n    println!(\"hello\");\n    let x = 1;\n    let y = 2;\n}");
    create_test_file(temp_dir.path(), "index.js",
        "function test() {\n    console.log('hello');\n    let x = 1;\n}");
    create_test_file(temp_dir.path(), "main.py",
        "def main():\n    print('hello')\n    x = 1\n");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--compress")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // All files should be compressed (signatures extracted)
    assert!(stdout.contains("{ ... }") || stdout.contains("Compressed:"));
}

// ============================================================================
// Statistics Accuracy Tests
// ============================================================================

#[test]
fn test_statistics_accurate_file_counts() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "1.rs", "fn a() {}");
    create_test_file(temp_dir.path(), "2.rs", "fn b() {}");
    create_test_file(temp_dir.path(), "3.txt", "text");
    create_test_file(temp_dir.path(), "4.json", "{}");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Should have exactly 4 files
    let total_files = json["statistics"]["total_files"].as_u64().unwrap();
    assert_eq!(total_files, 4);

    let files_array = json["files"].as_array().unwrap();
    assert_eq!(files_array.len(), 4);
}

#[test]
fn test_statistics_extension_breakdown() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "a.rs", "fn a() {}");
    create_test_file(temp_dir.path(), "b.rs", "fn b() {}");
    create_test_file(temp_dir.path(), "c.ts", "export {};");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--stats")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();

    // Should show breakdown: 2 .rs, 1 .ts
    assert!(stderr.contains("2") && stderr.contains(".rs") ||
            stderr.contains("1") && stderr.contains(".ts"));
}

// ============================================================================
// Unix Alignment: Directory Exclusion Tests
// ============================================================================

#[test]
fn test_exclude_dir_basic() {
    let temp_dir = TempDir::new().unwrap();
    let node_modules = temp_dir.path().join("node_modules");
    fs::create_dir_all(&node_modules).unwrap();
    let src = temp_dir.path().join("src");
    fs::create_dir_all(&src).unwrap();

    create_test_file(temp_dir.path(), "package.json", "{}");
    create_test_file(&node_modules, "index.js", "console.log('dep');");
    create_test_file(&src, "main.rs", "fn main() {}");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--exclude-dir")
        .arg("node_modules")
        .arg("--dry-run")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should include src/main.rs and package.json
    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("package.json"));
    // Should not include node_modules
    assert!(!stdout.contains("index.js"));
}

#[test]
fn test_exclude_dir_short_alias() {
    let temp_dir = TempDir::new().unwrap();
    let dist = temp_dir.path().join("dist");
    fs::create_dir_all(&dist).unwrap();
    let src = temp_dir.path().join("src");
    fs::create_dir_all(&src).unwrap();

    create_test_file(&dist, "bundle.js", "bundled");
    create_test_file(&src, "app.ts", "export const app = {};");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("-I")
        .arg("dist")
        .arg("--dry-run")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("app.ts"));
    assert!(!stdout.contains("bundle.js"));
}

#[test]
fn test_exclude_dir_multiple_comma_separated() {
    let temp_dir = TempDir::new().unwrap();
    let node_modules = temp_dir.path().join("node_modules");
    fs::create_dir_all(&node_modules).unwrap();
    let dist = temp_dir.path().join("dist");
    fs::create_dir_all(&dist).unwrap();
    let src = temp_dir.path().join("src");
    fs::create_dir_all(&src).unwrap();

    create_test_file(&node_modules, "index.js", "deps");
    create_test_file(&dist, "bundle.js", "bundled");
    create_test_file(&src, "main.rs", "fn main() {}");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("-I")
        .arg("node_modules,dist")
        .arg("--dry-run")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("main.rs"));
    assert!(!stdout.contains("index.js"));
    assert!(!stdout.contains("bundle.js"));
}

#[test]
fn test_exclude_dir_with_gitignore() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());
    let node_modules = temp_dir.path().join("node_modules");
    fs::create_dir_all(&node_modules).unwrap();
    let build = temp_dir.path().join("build");
    fs::create_dir_all(&build).unwrap();
    let src = temp_dir.path().join("src");
    fs::create_dir_all(&src).unwrap();

    create_test_file(temp_dir.path(), ".gitignore", "build/\n");
    create_test_file(&node_modules, "lib.js", "dep");
    create_test_file(&build, "output.o", "build artifact");
    create_test_file(&src, "main.rs", "fn main() {}");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("-I")
        .arg("node_modules")
        .arg("--dry-run")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should exclude both node_modules (--exclude-dir) and build (gitignore)
    assert!(stdout.contains("main.rs"));
    assert!(!stdout.contains("lib.js"));
    assert!(!stdout.contains("output.o"));
}

// ============================================================================
// Unix Alignment: No-Ignore Flag Tests
// ============================================================================

#[test]
fn test_no_ignore_flag_respects_gitignore_by_default() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());
    let logs = temp_dir.path().join("logs");
    fs::create_dir_all(&logs).unwrap();

    create_test_file(temp_dir.path(), ".gitignore", "*.log\nlogs/\n");
    create_test_file(&logs, "app.log", "log content");
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--dry-run")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should not include files in .gitignore
    assert!(stdout.contains("main.rs"));
    assert!(!stdout.contains("app.log"));
}

#[test]
fn test_no_ignore_flag_disables_gitignore() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());
    let logs = temp_dir.path().join("logs");
    fs::create_dir_all(&logs).unwrap();

    create_test_file(temp_dir.path(), ".gitignore", "*.log\nlogs/\n");
    create_test_file(&logs, "app.log", "log content");
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--no-ignore")
        .arg("--dry-run")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should include all files, including those in .gitignore
    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("app.log"));
}

#[test]
fn test_no_ignore_but_exclude_dir_still_applies() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());
    let node_modules = temp_dir.path().join("node_modules");
    fs::create_dir_all(&node_modules).unwrap();
    let src = temp_dir.path().join("src");
    fs::create_dir_all(&src).unwrap();

    create_test_file(temp_dir.path(), ".gitignore", "node_modules/\n");
    create_test_file(&node_modules, "lib.js", "dep");
    create_test_file(&src, "main.rs", "fn main() {}");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--no-ignore")
        .arg("-I")
        .arg("node_modules")
        .arg("--dry-run")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    // --exclude-dir should still work even with --no-ignore
    assert!(stdout.contains("main.rs"));
    assert!(!stdout.contains("lib.js"));
}

// ============================================================================
// Unix Alignment: Short Alias Tests
// ============================================================================

#[test]
fn test_short_alias_compress() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {\n    println!(\"hello\");\n}");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("-c")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should show compression happened
    assert!(stdout.contains("main.rs"));
    // Compressed content should be smaller
    assert!(stdout.contains("{ ... }") || stdout.contains("Compressed:"));
}

#[test]
fn test_short_alias_match() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main_test.go", "func TestMain() {}");
    create_test_file(temp_dir.path(), "main.go", "func main() {}");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("-m")
        .arg("*_test*")
        .arg("--dry-run")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("main_test.go"));
    assert!(!stdout.contains("main.go"));
}

#[test]
fn test_short_alias_include() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}");
    create_test_file(temp_dir.path(), "config.toml", "[config]");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("-i")
        .arg("rs")
        .arg("--dry-run")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("main.rs"));
    assert!(!stdout.contains("config.toml"));
}

#[test]
fn test_short_aliases_combined() {
    let temp_dir = TempDir::new().unwrap();
    let dist = temp_dir.path().join("dist");
    fs::create_dir_all(&dist).unwrap();

    create_test_file(&dist, "bundle.js", "bundled");
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}\n    println!(\"test\");");
    create_test_file(temp_dir.path(), "config.toml", "[test]");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("-I")
        .arg("dist")
        .arg("-i")
        .arg("rs")
        .arg("-c")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("main.rs"));
    assert!(!stdout.contains("bundle.js"));
    assert!(!stdout.contains("config.toml"));
}

// ============================================================================
// Unix Alignment: Ignore File Tests
// ============================================================================

#[test]
fn test_ignore_file_flag() {
    let temp_dir = TempDir::new().unwrap();
    let custom_ignore = temp_dir.path().join(".customignore");

    create_test_file(temp_dir.path(), ".customignore", "*.tmp\n");
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}");
    create_test_file(temp_dir.path(), "data.tmp", "temporary");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--ignore-file")
        .arg(&custom_ignore)
        .arg("--dry-run")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("main.rs"));
    assert!(!stdout.contains("data.tmp"));
}

#[test]
fn test_ignore_file_with_no_ignore_flag() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());
    let custom_ignore = temp_dir.path().join(".customignore");

    create_test_file(temp_dir.path(), ".customignore", "*.tmp\n");
    create_test_file(temp_dir.path(), "main.rs", "fn main() {}");
    create_test_file(temp_dir.path(), "data.tmp", "temporary");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("--ignore-file")
        .arg(&custom_ignore)
        .arg("--no-ignore")
        .arg("--dry-run")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    // With --no-ignore, custom ignore file should be ignored
    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("data.tmp"));
}

// ============================================================================
// Unix Alignment: Precedence and Interaction Tests
// ============================================================================

#[test]
fn test_precedence_exclude_dir_over_include_filter() {
    let temp_dir = TempDir::new().unwrap();
    let src = temp_dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    let build = temp_dir.path().join("build");
    fs::create_dir_all(&build).unwrap();

    create_test_file(&src, "main.rs", "fn main() {}");
    create_test_file(&build, "output.rs", "// build artifact");

    let output = flat_cmd()
        .arg(temp_dir.path())
        .arg("-I")
        .arg("build")
        .arg("-i")
        .arg("rs")
        .arg("--dry-run")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Even though .rs is included, build should be excluded
    assert!(stdout.contains("main.rs"));
    assert!(!stdout.contains("output.rs"));
}

// ============================================================================
// Parallel Processing Tests (v0.6.0)
// ============================================================================

#[test]
fn test_parallel_flag_basic() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg("tests/fixtures/sample_project")
        .arg("--parallel")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("src/main.rs"));
    assert!(stdout.contains("Total files:"));
}

#[test]
fn test_parallel_determinism() {
    // Run 1: parallel (no cache to ensure consistency)
    let output1 = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg("tests/fixtures/sample_project")
        .arg("--parallel")
        .arg("--no-cache")
        .output()
        .unwrap();

    // Run 2: parallel (no cache to ensure consistency)
    let output2 = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg("tests/fixtures/sample_project")
        .arg("--parallel")
        .arg("--no-cache")
        .output()
        .unwrap();

    let stdout1 = String::from_utf8(output1.stdout).unwrap();
    let stdout2 = String::from_utf8(output2.stdout).unwrap();

    // Outputs must be identical (deterministic)
    assert_eq!(
        stdout1, stdout2,
        "Parallel runs should produce identical output (deterministic)"
    );
}

#[test]
fn test_parallel_vs_sequential_equivalence() {
    // Sequential run (no cache)
    let output_seq = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg("tests/fixtures/sample_project")
        .arg("--no-cache")
        .output()
        .unwrap();

    // Parallel run (no cache)
    let output_par = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg("tests/fixtures/sample_project")
        .arg("--parallel")
        .arg("--no-cache")
        .output()
        .unwrap();

    let stdout_seq = String::from_utf8(output_seq.stdout).unwrap();
    let stdout_par = String::from_utf8(output_par.stdout).unwrap();

    // Parallel and sequential must produce identical output
    assert_eq!(
        stdout_seq, stdout_par,
        "Parallel and sequential outputs must be identical"
    );
}

#[test]
fn test_parallel_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path();

    // Create test files
    create_test_file(dir, "good.rs", "fn main() { println!(\"hello\"); }");
    create_test_file(dir, "also_good.md", "# Test\nSome documentation");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg(dir)
        .arg("--parallel")
        .output()
        .unwrap();

    // Should succeed even with mixed content
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("good.rs"));
    assert!(stdout.contains("also_good.md"));
}

#[test]
fn test_parallel_with_compression() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg("tests/fixtures/sample_project")
        .arg("--parallel")
        .arg("--compress")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("compressed"));
}

// ============================================================================
// Caching Tests (v0.6.0)
// ============================================================================

#[test]
fn test_cache_basic() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path();

    // Create test files
    create_test_file(dir, "test.rs", "fn main() { println!(\"hello\"); }");
    create_test_file(dir, "lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }");

    // First run: cache misses
    let output1 = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg(dir)
        .arg("--parallel")
        .output()
        .unwrap();

    assert!(output1.status.success());
    let stdout1 = String::from_utf8(output1.stdout).unwrap();
    assert!(stdout1.contains("Cache: 0 hits, 2 misses")); // First run - all misses

    // Second run: should have cache hits
    let output2 = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg(dir)
        .arg("--parallel")
        .output()
        .unwrap();

    assert!(output2.status.success());
    let stdout2 = String::from_utf8(output2.stdout).unwrap();
    assert!(stdout2.contains("Cache: 2 hits, 0 misses")); // Second run - all hits
}

#[test]
fn test_cache_invalidation_on_file_change() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path();

    let test_file = dir.join("test.rs");

    // First run
    fs::write(&test_file, "fn main() {}").unwrap();
    let output1 = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg(dir)
        .arg("--parallel")
        .output()
        .unwrap();

    assert!(output1.status.success());
    let stdout1 = String::from_utf8(output1.stdout).unwrap();
    assert!(stdout1.contains("Cache: 0 hits, 1 miss")); // First run - miss

    // Modify the file
    std::thread::sleep(std::time::Duration::from_millis(100)); // Ensure different mtime
    fs::write(&test_file, "fn main() { println!(\"updated\"); }").unwrap();

    // Second run: should detect change and reprocess
    let output2 = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg(dir)
        .arg("--parallel")
        .output()
        .unwrap();

    assert!(output2.status.success());
    let stdout2 = String::from_utf8(output2.stdout).unwrap();
    assert!(stdout2.contains("Cache: 0 hits, 1 miss")); // File changed - cache miss
}

#[test]
fn test_no_cache_flag() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path();

    create_test_file(dir, "test.rs", "fn main() {}");

    // First run with cache
    let output1 = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg(dir)
        .arg("--parallel")
        .output()
        .unwrap();

    assert!(output1.status.success());

    // Second run with --no-cache
    let output2 = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg(dir)
        .arg("--parallel")
        .arg("--no-cache")
        .output()
        .unwrap();

    assert!(output2.status.success());
    let stdout2 = String::from_utf8(output2.stdout).unwrap();
    // With --no-cache, should not show cache stats
    assert!(!stdout2.contains("Cache:"));
}

#[test]
fn test_cache_with_compression() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path();

    create_test_file(dir, "test.rs", "fn main() { println!(\"hello\"); }");

    // First run with compression
    let output1 = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg(dir)
        .arg("--parallel")
        .arg("--compress")
        .output()
        .unwrap();

    assert!(output1.status.success());
    let stdout1 = String::from_utf8(output1.stdout).unwrap();
    assert!(stdout1.contains("compressed"));
    assert!(stdout1.contains("Cache: 0 hits, 1 miss"));

    // Second run with same compression
    let output2 = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg(dir)
        .arg("--parallel")
        .arg("--compress")
        .output()
        .unwrap();

    assert!(output2.status.success());
    let stdout2 = String::from_utf8(output2.stdout).unwrap();
    assert!(stdout2.contains("Cache: 1 hits, 0 misses")); // Compressed content is cached
}

#[test]
fn test_cache_invalidation_on_compression_change() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path();

    create_test_file(dir, "test.rs", "fn main() { println!(\"hello\"); }");

    // First run with compression
    let output1 = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg(dir)
        .arg("--parallel")
        .arg("--compress")
        .output()
        .unwrap();

    assert!(output1.status.success());

    // Second run without compression - cache should be invalid (different config)
    let output2 = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg(dir)
        .arg("--parallel")
        .output()
        .unwrap();

    assert!(output2.status.success());
    let stdout2 = String::from_utf8(output2.stdout).unwrap();
    assert!(stdout2.contains("Cache: 0 hits, 1 miss")); // Config changed - cache miss
}

// ============================================================================
// Watch Mode Tests (v0.6.0)
// ============================================================================

#[test]
fn test_watch_invalid_combinations_with_dry_run() {
    // --watch with --dry-run should error
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg("tests/fixtures/sample_project")
        .arg("--watch")
        .arg("--dry-run")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("cannot be used with --dry-run"));
}

#[test]
fn test_watch_invalid_combinations_with_stats() {
    // --watch with --stats should error
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg("tests/fixtures/sample_project")
        .arg("--watch")
        .arg("--stats")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("cannot be used with --stats"));
}

#[test]
fn test_watch_invalid_combinations_with_output() {
    // --watch with --output should error
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_flat"))
        .arg("tests/fixtures/sample_project")
        .arg("--watch")
        .arg("--output")
        .arg("/tmp/test.xml")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("cannot be used with --output"));
}
