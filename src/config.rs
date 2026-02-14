use globset::GlobMatcher;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub path: PathBuf,
    pub include_extensions: Option<Vec<String>>,
    pub exclude_extensions: Option<Vec<String>>,
    pub match_patterns: Option<Vec<GlobMatcher>>,
    pub output_file: Option<PathBuf>,
    pub dry_run: bool,
    pub stats_only: bool,
    pub gitignore_path: Option<PathBuf>,
    pub max_file_size: u64,
    pub compress: bool,
    pub full_match_patterns: Option<Vec<GlobMatcher>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            include_extensions: None,
            exclude_extensions: None,
            match_patterns: None,
            output_file: None,
            dry_run: false,
            stats_only: false,
            gitignore_path: None,
            max_file_size: 1024 * 1024, // 1MB
            compress: false,
            full_match_patterns: None,
        }
    }
}

impl Config {
    pub fn should_include_extension(&self, ext: &str) -> bool {
        // If include list is specified, extension must be in it
        if let Some(ref include) = self.include_extensions {
            if !include.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                return false;
            }
        }

        // If exclude list is specified, extension must not be in it
        if let Some(ref exclude) = self.exclude_extensions {
            if exclude.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                return false;
            }
        }

        true
    }

    /// Check if a file name matches any of the configured glob patterns.
    /// Returns true if no patterns are set or if the name matches at least one pattern.
    pub fn should_include_by_match(&self, file_name: &str) -> bool {
        match &self.match_patterns {
            Some(patterns) => patterns.iter().any(|m| m.is_match(file_name)),
            None => true,
        }
    }

    /// Check if a file should always get full content (skip compression).
    /// Returns true if --full-match patterns are set and the file name matches.
    pub fn is_full_match(&self, file_name: &str) -> bool {
        match &self.full_match_patterns {
            Some(patterns) => patterns.iter().any(|m| m.is_match(file_name)),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use globset::Glob;

    #[test]
    fn test_include_only() {
        let config = Config {
            include_extensions: Some(vec!["rs".to_string(), "toml".to_string()]),
            ..Default::default()
        };

        assert!(config.should_include_extension("rs"));
        assert!(config.should_include_extension("toml"));
        assert!(!config.should_include_extension("json"));
    }

    #[test]
    fn test_exclude_only() {
        let config = Config {
            exclude_extensions: Some(vec!["test".to_string(), "json".to_string()]),
            ..Default::default()
        };

        assert!(config.should_include_extension("rs"));
        assert!(!config.should_include_extension("test"));
        assert!(!config.should_include_extension("json"));
    }

    #[test]
    fn test_include_and_exclude() {
        let config = Config {
            include_extensions: Some(vec!["rs".to_string(), "toml".to_string()]),
            exclude_extensions: Some(vec!["toml".to_string()]),
            ..Default::default()
        };

        assert!(config.should_include_extension("rs"));
        assert!(!config.should_include_extension("toml")); // Excluded even though included
        assert!(!config.should_include_extension("json"));
    }

    #[test]
    fn test_match_no_patterns() {
        let config = Config::default();
        assert!(config.should_include_by_match("anything.rs"));
    }

    #[test]
    fn test_match_single_pattern() {
        let config = Config {
            match_patterns: Some(vec![Glob::new("*_test.go").unwrap().compile_matcher()]),
            ..Default::default()
        };

        assert!(config.should_include_by_match("user_test.go"));
        assert!(config.should_include_by_match("auth_test.go"));
        assert!(!config.should_include_by_match("main.go"));
        assert!(!config.should_include_by_match("test.rs"));
    }

    #[test]
    fn test_match_multiple_patterns() {
        let config = Config {
            match_patterns: Some(vec![
                Glob::new("*_test.go").unwrap().compile_matcher(),
                Glob::new("*.spec.js").unwrap().compile_matcher(),
            ]),
            ..Default::default()
        };

        assert!(config.should_include_by_match("user_test.go"));
        assert!(config.should_include_by_match("button.spec.js"));
        assert!(!config.should_include_by_match("main.go"));
    }
}
