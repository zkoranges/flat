use crate::compress::{compress_source, language_for_path, CompressResult};
use crate::config::Config;
use crate::filters::{
    exceeds_size_limit, is_binary_content, is_binary_extension, is_secret_file, SkipReason,
};
use crate::output::{OutputWriter, Statistics};
use anyhow::{Context, Result};
use ignore::WalkBuilder;
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn walk_and_flatten(config: &Config) -> Result<Statistics> {
    let mut stats = Statistics::new();

    // Build the walker with gitignore support
    let mut builder = WalkBuilder::new(&config.path);
    builder.standard_filters(true); // Enable .gitignore, .ignore, etc.

    if let Some(ref gitignore_path) = config.gitignore_path {
        builder.add_custom_ignore_filename(gitignore_path);
    }

    // Create output writer
    let writer: Box<dyn Write> = match &config.output_file {
        Some(path) => Box::new(
            fs::File::create(path)
                .with_context(|| format!("Failed to create output file: {}", path.display()))?,
        ),
        None => Box::new(std::io::stdout()),
    };

    let mut output = OutputWriter::new(writer);

    // First pass: collect all files
    let mut files_to_process = Vec::new();

    for result in builder.build() {
        match result {
            Ok(entry) => {
                let path = entry.path();

                // Skip directories
                if path.is_dir() {
                    continue;
                }

                // Check filters
                if let Some(reason) = should_skip(path, config) {
                    stats.add_skipped(reason.clone());
                    if !config.stats_only {
                        eprintln!("Skipping {}: {}", path.display(), reason);
                    }
                    continue;
                }

                files_to_process.push(path.to_path_buf());
                let extension = path.extension().and_then(|e| e.to_str());
                stats.add_included(extension);

                // Add file size estimate for output calculation
                if let Ok(metadata) = fs::metadata(path) {
                    let file_size = metadata.len();
                    let path_str = path.display().to_string();
                    stats.add_file_size_estimate(file_size, path_str.len());
                }
            }
            Err(e) => {
                eprintln!("Error walking directory: {}", e);
                stats.add_skipped(SkipReason::ReadError);
            }
        }
    }

    // Sort files by path for deterministic output
    files_to_process.sort();

    // Write output based on mode
    if config.stats_only {
        // Stats only mode - just print statistics
        // Add overhead for summary block (approximately 200 bytes)
        stats.add_output_bytes(200);
        eprintln!("{}", stats.format_summary());
    } else if config.dry_run {
        // Dry run mode - list files that would be included
        for path in &files_to_process {
            output.write_file_path(&path.display().to_string())?;
        }
        // Update stats with output size and write summary
        stats.add_output_bytes(output.bytes_written());
        output.write_summary(&stats)?;
    } else {
        // Normal mode - write file contents
        for path in &files_to_process {
            match fs::read_to_string(path) {
                Ok(content) => {
                    let display_path = path.display().to_string();

                    if config.compress {
                        let file_name = path.file_name()
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let is_full = config.is_full_match(&file_name);

                        if is_full {
                            output.write_file_content_with_mode(&display_path, &content, Some("full"))?;
                        } else if let Some(lang) = language_for_path(path) {
                            match compress_source(&content, lang) {
                                CompressResult::Compressed(compressed) => {
                                    output.write_file_content_with_mode(&display_path, &compressed, Some("compressed"))?;
                                    stats.add_compressed();
                                }
                                CompressResult::Fallback(original, reason) => {
                                    if let Some(reason) = reason {
                                        eprintln!("Warning: compression failed for {}: {}, including full content", display_path, reason);
                                    }
                                    output.write_file_content_with_mode(&display_path, &original, Some("full"))?;
                                }
                            }
                        } else {
                            // Unsupported extension - full content
                            output.write_file_content_with_mode(&display_path, &content, Some("full"))?;
                        }
                    } else {
                        output.write_file_content(&display_path, &content)?;
                    }
                }
                Err(e) => {
                    eprintln!("Error reading {}: {}", path.display(), e);
                }
            }
        }
        // Update stats with output size and write summary at the end
        stats.add_output_bytes(output.bytes_written());
        output.write_summary(&stats)?;
    }

    Ok(stats)
}

/// Check if a file should be skipped, returning the reason if so
fn should_skip(path: &Path, config: &Config) -> Option<SkipReason> {
    // Check match pattern filter
    if let Some(file_name) = path.file_name() {
        if !config.should_include_by_match(&file_name.to_string_lossy()) {
            return Some(SkipReason::Match);
        }
    }

    // Check if it's a secret file
    if is_secret_file(path) {
        return Some(SkipReason::Secret);
    }

    // Check extension filter
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy();
        if !config.should_include_extension(&ext_str) {
            return Some(SkipReason::Extension);
        }

        // Check if it's a known binary extension
        if is_binary_extension(path) {
            return Some(SkipReason::Binary);
        }
    }

    // Check file size
    if exceeds_size_limit(path, config.max_file_size) {
        return Some(SkipReason::TooLarge);
    }

    // Check if it's binary content
    if is_binary_content(path) {
        return Some(SkipReason::Binary);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_skip_secret() {
        let config = Config::default();
        assert_eq!(
            should_skip(Path::new(".env"), &config),
            Some(SkipReason::Secret)
        );
        assert_eq!(
            should_skip(Path::new("credentials.json"), &config),
            Some(SkipReason::Secret)
        );
    }

    #[test]
    fn test_should_skip_binary_extension() {
        let config = Config::default();
        assert_eq!(
            should_skip(Path::new("image.png"), &config),
            Some(SkipReason::Binary)
        );
        assert_eq!(
            should_skip(Path::new("binary.exe"), &config),
            Some(SkipReason::Binary)
        );
    }

    #[test]
    fn test_should_skip_extension_filter() {
        let config = Config {
            include_extensions: Some(vec!["rs".to_string()]),
            ..Default::default()
        };

        assert_eq!(
            should_skip(Path::new("file.json"), &config),
            Some(SkipReason::Extension)
        );
        assert_eq!(should_skip(Path::new("file.rs"), &config), None);
    }

    #[test]
    fn test_should_skip_match_filter() {
        let config = Config {
            match_patterns: Some(vec![globset::Glob::new("*_test.go")
                .unwrap()
                .compile_matcher()]),
            ..Default::default()
        };

        assert_eq!(
            should_skip(Path::new("main.go"), &config),
            Some(SkipReason::Match)
        );
        assert_eq!(should_skip(Path::new("user_test.go"), &config), None);
    }
}
