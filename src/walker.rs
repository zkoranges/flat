use crate::compress::{compress_source, language_for_path, CompressResult};
use crate::config::Config;
use crate::filters::{
    exceeds_size_limit, is_binary_content, is_binary_extension, is_secret_file, SkipReason,
};
use crate::output::{OutputWriter, Statistics};
use crate::priority::score_file;
use crate::tokens::{estimate_tokens, is_prose_extension};
use anyhow::{Context, Result};
use ignore::WalkBuilder;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// A file candidate with its content and metadata for budget allocation
struct FileCandidate {
    path: PathBuf,
    content: String,
    score: u32,
    is_prose: bool,
}

/// Result of budget allocation for a single file
enum FileDecision {
    IncludeFull(String),
    IncludeCompressed(String),
    Excluded,
}

pub fn walk_and_flatten(config: &Config) -> Result<Statistics> {
    let mut stats = Statistics::new();

    // Build the walker with gitignore support
    let mut builder = WalkBuilder::new(&config.path);
    builder.standard_filters(true);

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

                if path.is_dir() {
                    continue;
                }

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
            }
            Err(e) => {
                eprintln!("Error walking directory: {}", e);
                stats.add_skipped(SkipReason::ReadError);
            }
        }
    }

    // Sort files by path for deterministic output
    files_to_process.sort();

    // Handle token budget mode
    if let Some(budget) = config.token_budget {
        stats.token_budget = Some(budget);
        write_with_budget(config, &files_to_process, &mut output, &mut stats, budget)?;
    } else if config.stats_only {
        for path in &files_to_process {
            let path_str = path.display().to_string();
            if config.compress {
                let file_name = path
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default();
                let is_full = config.is_full_match(&file_name);
                if !is_full {
                    if let Some(lang) = language_for_path(path) {
                        if let Ok(content) = fs::read_to_string(path) {
                            match compress_source(&content, lang) {
                                CompressResult::Compressed(compressed) => {
                                    stats.add_file_size_estimate(
                                        compressed.len() as u64,
                                        path_str.len(),
                                    );
                                    stats.add_compressed();
                                    continue;
                                }
                                CompressResult::Fallback(original, _) => {
                                    stats.add_file_size_estimate(
                                        original.len() as u64,
                                        path_str.len(),
                                    );
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
            // Non-compress mode, full-match files, or non-compressible files: use raw size
            if let Ok(metadata) = fs::metadata(path) {
                stats.add_file_size_estimate(metadata.len(), path_str.len());
            }
        }
        eprintln!("{}", stats.format_summary());
    } else if config.dry_run {
        for path in &files_to_process {
            output.write_file_path(&path.display().to_string())?;
        }
        stats.add_output_bytes(output.bytes_written());
        output.write_summary(&stats)?;
    } else {
        write_normal(config, &files_to_process, &mut output, &mut stats)?;
    }

    Ok(stats)
}

/// Write files with token budget allocation
fn write_with_budget(
    config: &Config,
    files: &[PathBuf],
    output: &mut OutputWriter,
    stats: &mut Statistics,
    budget: usize,
) -> Result<()> {
    let base_path = &config.path;

    // Read all file contents and compute scores
    let mut candidates: Vec<FileCandidate> = Vec::new();
    for path in files {
        match fs::read_to_string(path) {
            Ok(content) => {
                let score = score_file(path, base_path);
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let is_prose = is_prose_extension(ext);
                candidates.push(FileCandidate {
                    path: path.clone(),
                    content,
                    score,
                    is_prose,
                });
            }
            Err(e) => {
                eprintln!("Error reading {}: {}", path.display(), e);
            }
        }
    }

    // Sort by (score DESC, path ASC) — stable sort
    candidates.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));

    let mut remaining_budget = budget;

    // Allocate full-match files first (if --tokens + --compress + --full-match)
    let mut decisions: Vec<(&FileCandidate, FileDecision)> = Vec::new();

    for candidate in &candidates {
        let display_path = candidate.path.display().to_string();
        let file_name = candidate
            .path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        let full_tokens = estimate_tokens(&candidate.content, candidate.is_prose);

        if config.compress && config.is_full_match(&file_name) {
            // Full-match files: always use full content, never compress
            if full_tokens <= remaining_budget {
                remaining_budget -= full_tokens;
                stats.tokens_used += full_tokens;
                decisions.push((
                    candidate,
                    FileDecision::IncludeFull(candidate.content.clone()),
                ));
            } else {
                stats.excluded_by_budget.push(display_path);
                decisions.push((candidate, FileDecision::Excluded));
            }
        } else if full_tokens <= remaining_budget {
            // File fits in full
            remaining_budget -= full_tokens;
            stats.tokens_used += full_tokens;
            if config.compress {
                // Even though it fits, still compress if possible (per flag behavior)
                let content = maybe_compress(config, &candidate.path, &candidate.content, stats);
                decisions.push((candidate, content));
            } else {
                decisions.push((
                    candidate,
                    FileDecision::IncludeFull(candidate.content.clone()),
                ));
            }
        } else if config.compress {
            // Try compressed version
            if let Some(lang) = language_for_path(&candidate.path) {
                match compress_source(&candidate.content, lang) {
                    CompressResult::Compressed(compressed) => {
                        let compressed_tokens = estimate_tokens(&compressed, candidate.is_prose);
                        if compressed_tokens <= remaining_budget {
                            remaining_budget -= compressed_tokens;
                            stats.tokens_used += compressed_tokens;
                            stats.add_compressed();
                            decisions
                                .push((candidate, FileDecision::IncludeCompressed(compressed)));
                        } else {
                            stats.excluded_by_budget.push(display_path);
                            decisions.push((candidate, FileDecision::Excluded));
                        }
                    }
                    CompressResult::Fallback(original, reason) => {
                        if let Some(reason) = &reason {
                            eprintln!(
                                "Warning: compression failed for {}: {}, including full content",
                                display_path, reason
                            );
                        }
                        // Fallback is full size, which we already know doesn't fit
                        let fallback_tokens = estimate_tokens(&original, candidate.is_prose);
                        if fallback_tokens <= remaining_budget {
                            remaining_budget -= fallback_tokens;
                            stats.tokens_used += fallback_tokens;
                            decisions.push((candidate, FileDecision::IncludeFull(original)));
                        } else {
                            stats.excluded_by_budget.push(display_path);
                            decisions.push((candidate, FileDecision::Excluded));
                        }
                    }
                }
            } else {
                // Unsupported for compression, and full doesn't fit
                stats.excluded_by_budget.push(display_path);
                decisions.push((candidate, FileDecision::Excluded));
            }
        } else {
            // No compression, doesn't fit
            stats.excluded_by_budget.push(display_path);
            decisions.push((candidate, FileDecision::Excluded));
        }
    }

    // Write output
    if config.stats_only {
        for (candidate, decision) in &decisions {
            match decision {
                FileDecision::IncludeFull(content) | FileDecision::IncludeCompressed(content) => {
                    let path_str = candidate.path.display().to_string();
                    stats.add_file_size_estimate(content.len() as u64, path_str.len());
                }
                FileDecision::Excluded => {}
            }
        }
        eprintln!("{}", stats.format_summary());
    } else if config.dry_run {
        for (candidate, decision) in &decisions {
            let display_path = candidate.path.display().to_string();
            let annotation = match decision {
                FileDecision::IncludeFull(_) => "[FULL]",
                FileDecision::IncludeCompressed(_) => "[COMPRESSED]",
                FileDecision::Excluded => "[EXCLUDED]",
            };
            output.write_file_path(&format!("{} {}", display_path, annotation))?;
        }
        stats.add_output_bytes(output.bytes_written());
        output.write_summary(stats)?;
    } else {
        for (candidate, decision) in &decisions {
            let display_path = candidate.path.display().to_string();
            match decision {
                FileDecision::IncludeFull(content) => {
                    let mode = if config.compress { Some("full") } else { None };
                    output.write_file_content_with_mode(&display_path, content, mode)?;
                }
                FileDecision::IncludeCompressed(content) => {
                    output.write_file_content_with_mode(
                        &display_path,
                        content,
                        Some("compressed"),
                    )?;
                }
                FileDecision::Excluded => {}
            }
        }
        stats.add_output_bytes(output.bytes_written());
        output.write_summary(stats)?;
    }

    Ok(())
}

/// Write files without token budget (normal mode)
fn write_normal(
    config: &Config,
    files: &[PathBuf],
    output: &mut OutputWriter,
    stats: &mut Statistics,
) -> Result<()> {
    for path in files {
        match fs::read_to_string(path) {
            Ok(content) => {
                let display_path = path.display().to_string();

                if config.compress {
                    let file_name = path
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let is_full = config.is_full_match(&file_name);

                    if is_full {
                        output.write_file_content_with_mode(
                            &display_path,
                            &content,
                            Some("full"),
                        )?;
                    } else if let Some(lang) = language_for_path(path) {
                        match compress_source(&content, lang) {
                            CompressResult::Compressed(compressed) => {
                                output.write_file_content_with_mode(
                                    &display_path,
                                    &compressed,
                                    Some("compressed"),
                                )?;
                                stats.add_compressed();
                            }
                            CompressResult::Fallback(original, reason) => {
                                if let Some(reason) = reason {
                                    eprintln!(
                                        "Warning: compression failed for {}: {}, including full content",
                                        display_path, reason
                                    );
                                }
                                output.write_file_content_with_mode(
                                    &display_path,
                                    &original,
                                    Some("full"),
                                )?;
                            }
                        }
                    } else {
                        output.write_file_content_with_mode(
                            &display_path,
                            &content,
                            Some("full"),
                        )?;
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

    stats.add_output_bytes(output.bytes_written());
    output.write_summary(stats)?;
    Ok(())
}

/// Helper: Try to compress a file if applicable, returning the appropriate decision
fn maybe_compress(
    config: &Config,
    path: &Path,
    content: &str,
    stats: &mut Statistics,
) -> FileDecision {
    let file_name = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();

    if config.is_full_match(&file_name) {
        return FileDecision::IncludeFull(content.to_string());
    }

    if let Some(lang) = language_for_path(path) {
        match compress_source(content, lang) {
            CompressResult::Compressed(compressed) => {
                stats.add_compressed();
                FileDecision::IncludeCompressed(compressed)
            }
            CompressResult::Fallback(original, reason) => {
                if let Some(reason) = reason {
                    eprintln!(
                        "Warning: compression failed for {}: {}, including full content",
                        path.display(),
                        reason
                    );
                }
                FileDecision::IncludeFull(original)
            }
        }
    } else {
        FileDecision::IncludeFull(content.to_string())
    }
}

/// Check if a file should be skipped, returning the reason if so
fn should_skip(path: &Path, config: &Config) -> Option<SkipReason> {
    if let Some(file_name) = path.file_name() {
        if !config.should_include_by_match(&file_name.to_string_lossy()) {
            return Some(SkipReason::Match);
        }
    }

    if is_secret_file(path) {
        return Some(SkipReason::Secret);
    }

    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy();
        if !config.should_include_extension(&ext_str) {
            return Some(SkipReason::Extension);
        }

        if is_binary_extension(path) {
            return Some(SkipReason::Binary);
        }
    }

    if exceeds_size_limit(path, config.max_file_size) {
        return Some(SkipReason::TooLarge);
    }

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
