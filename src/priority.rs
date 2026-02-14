use std::path::Path;

/// Score a file for priority ordering in token budget allocation.
///
/// Higher scores = higher priority (included first in budget).
/// Per PDR spec:
/// - READMEs: 100
/// - Entry points (main.*, index.*, app.*): 90
/// - Config files: 80
/// - Source code: 70 - (depth * 10), min 10
/// - Tests: 30
/// - Fixtures/generated: 5
pub fn score_file(path: &Path, base_path: &Path) -> u32 {
    let file_name = path
        .file_name()
        .map(|f| f.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let relative = path.strip_prefix(base_path).unwrap_or(path);
    let depth = relative.components().count().saturating_sub(1); // depth of file, not dir

    let path_str = relative.to_string_lossy().to_lowercase();

    // Check categories in priority order (highest score wins)
    if is_fixture(&path_str) {
        5
    } else if is_test(&path_str, &file_name) {
        30
    } else if is_readme(&file_name) {
        100
    } else if is_entry_point(&file_name) {
        90
    } else if is_config(&file_name) {
        80
    } else {
        // Source code with depth penalty
        let score = 70u32.saturating_sub((depth as u32) * 10);
        score.max(10)
    }
}

fn is_readme(file_name: &str) -> bool {
    file_name.starts_with("readme")
}

fn is_entry_point(file_name: &str) -> bool {
    let stem = file_name.split('.').next().unwrap_or("");
    matches!(stem, "main" | "index" | "app" | "lib" | "mod")
}

fn is_config(file_name: &str) -> bool {
    let stem = file_name.split('.').next().unwrap_or("");
    matches!(
        stem,
        "config"
            | "settings"
            | "package"
            | "cargo"
            | "tsconfig"
            | "webpack"
            | "vite"
            | "eslint"
            | "prettier"
            | "jest"
            | "pyproject"
            | "setup"
            | "makefile"
            | "dockerfile"
            | "docker-compose"
            | "go"
    ) || file_name.ends_with(".toml")
        || file_name.ends_with(".yaml")
        || file_name.ends_with(".yml")
        || file_name.ends_with(".json") && !file_name.contains("test")
}

fn is_test(path_str: &str, file_name: &str) -> bool {
    path_str.contains("test")
        || path_str.contains("spec")
        || file_name.contains("test")
        || file_name.contains("spec")
}

fn is_fixture(path_str: &str) -> bool {
    path_str.contains("fixture")
        || path_str.contains("testdata")
        || path_str.contains("test_data")
        || path_str.contains("__snapshots__")
        || path_str.contains("generated")
        || path_str.contains("vendor")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn score(path: &str) -> u32 {
        score_file(Path::new(path), Path::new("/project"))
    }

    #[test]
    fn test_readme_highest() {
        assert_eq!(score("/project/README.md"), 100);
        assert_eq!(score("/project/readme.txt"), 100);
    }

    #[test]
    fn test_entry_points() {
        assert_eq!(score("/project/src/main.rs"), 90);
        assert_eq!(score("/project/src/index.ts"), 90);
        assert_eq!(score("/project/src/lib.rs"), 90);
    }

    #[test]
    fn test_config_files() {
        assert_eq!(score("/project/Cargo.toml"), 80);
        assert_eq!(score("/project/package.json"), 80);
    }

    #[test]
    fn test_source_with_depth_penalty() {
        // depth 0 (file at root)
        assert_eq!(score("/project/foo.rs"), 70);
        // depth 1
        assert_eq!(score("/project/src/foo.rs"), 60);
        // depth 2
        assert_eq!(score("/project/src/utils/foo.rs"), 50);
        // depth 6+ → min 10
        assert_eq!(score("/project/a/b/c/d/e/f/foo.rs"), 10);
    }

    #[test]
    fn test_tests_scored_low() {
        assert_eq!(score("/project/tests/unit_test.rs"), 30);
        assert_eq!(score("/project/src/foo_test.go"), 30);
    }

    #[test]
    fn test_fixtures_lowest() {
        assert_eq!(score("/project/tests/fixtures/data.json"), 5);
        assert_eq!(score("/project/testdata/input.txt"), 5);
    }

    #[test]
    fn test_readme_in_subdirectory() {
        // README in tests/fixtures/ → fixture (5), not README (100)
        assert_eq!(score("/project/tests/fixtures/README.md"), 5);
    }

    #[test]
    fn test_sorting_order() {
        let base = PathBuf::from("/project");
        let mut files = [
            PathBuf::from("/project/tests/fixture/data.json"),
            PathBuf::from("/project/src/utils.rs"),
            PathBuf::from("/project/README.md"),
            PathBuf::from("/project/src/main.rs"),
            PathBuf::from("/project/Cargo.toml"),
            PathBuf::from("/project/tests/test_foo.rs"),
        ];

        files.sort_by(|a, b| {
            let sa = score_file(a, &base);
            let sb = score_file(b, &base);
            sb.cmp(&sa).then_with(|| a.cmp(b))
        });

        let names: Vec<&str> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect();
        assert_eq!(names[0], "README.md");
        assert_eq!(names[1], "main.rs");
        assert_eq!(names[2], "Cargo.toml");
    }
}
