use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub path: PathBuf,
    pub include_extensions: Option<Vec<String>>,
    pub exclude_extensions: Option<Vec<String>>,
    pub output_file: Option<PathBuf>,
    pub dry_run: bool,
    pub stats_only: bool,
    pub gitignore_path: Option<PathBuf>,
    pub max_file_size: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            include_extensions: None,
            exclude_extensions: None,
            output_file: None,
            dry_run: false,
            stats_only: false,
            gitignore_path: None,
            max_file_size: 1024 * 1024, // 1MB
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
