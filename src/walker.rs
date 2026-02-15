use crate::cache::{self, Cache, CacheEntry};
use crate::compress::{compress_source, language_for_path, CompressResult};
use crate::config::{Config, OutputFormat};
use crate::filters::{
    exceeds_size_limit, is_binary_content, is_binary_extension, is_secret_file, SkipReason,
};
use crate::output::{OutputWriter, Statistics};
use crate::priority::score_file;
use crate::tokens::{is_prose_extension, Tokenizer};
use anyhow::{Context, Result};
use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use rayon::prelude::*;
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

/// Result of processing a single file (for parallel processing)
enum FileProcessResult {
    Success {
        path: PathBuf,
        display_path: String,
        content: String,
        mode: Option<String>, // "full", "compressed", or None
        is_compressed: bool,
        cache_hit: bool,
    },
    Error {
        display_path: String,
        error: String,
    },
}

pub fn walk_and_flatten(config: &Config) -> Result<Statistics> {
    let mut stats = Statistics::new();
    let tokenizer = Tokenizer::new(config.tokenizer.clone());

    // Build the walker with gitignore support
    let mut builder = WalkBuilder::new(&config.path);

    // Enable standard filters (.gitignore, .git/info/exclude, etc.) unless --no-ignore
    builder.standard_filters(!config.no_ignore);

    // Add custom ignore file if specified and gitignore is not disabled
    if let Some(ref gitignore_path) = config.gitignore_path {
        if !config.no_ignore {
            builder.add_custom_ignore_filename(gitignore_path);
        }
    }

    // Apply directory exclusions via overrides
    if let Some(ref exclude_dirs) = config.exclude_dirs {
        let mut override_builder = OverrideBuilder::new(&config.path);
        for dir_pattern in exclude_dirs {
            // Convert to gitignore-style exclusion pattern: !dir/**
            let pattern = format!("!{}/**", dir_pattern);
            override_builder
                .add(&pattern)
                .with_context(|| format!("Invalid exclude-dir pattern: {}", dir_pattern))?;
        }
        builder.overrides(override_builder.build()?);
    }

    // Create output writer
    let writer: Box<dyn Write> = match &config.output_file {
        Some(path) => Box::new(
            fs::File::create(path)
                .with_context(|| format!("Failed to create output file: {}", path.display()))?,
        ),
        None => Box::new(std::io::stdout()),
    };

    // Determine output format - template takes precedence over --format
    let output_format = if config.dry_run {
        OutputFormat::PlainText
    } else if config.template.is_some() {
        // Use template mode (format doesn't matter)
        config.output_format.clone()
    } else {
        config.output_format.clone()
    };

    // Get repository name for templates
    let repo_name = config
        .path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "repository".to_string());

    let mut output = if let Some(ref template_name) = config.template {
        OutputWriter::new_template(template_name, writer, repo_name).with_context(|| {
            format!(
                "Failed to load template '{}'. \
                 Built-in templates: minimal, claude-review, openai-docs. \
                 Or provide a custom template at ~/.config/flat/templates/{{name}}.hbs",
                template_name
            )
        })?
    } else {
        match output_format {
            OutputFormat::Xml => OutputWriter::new_xml(writer),
            OutputFormat::Json => OutputWriter::new_json(writer),
            OutputFormat::PlainText => OutputWriter::new_plaintext(writer),
            OutputFormat::Template(_) => {
                // This shouldn't happen but fallback to XML
                OutputWriter::new_xml(writer)
            }
        }
    };

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
        write_with_budget(config, &files_to_process, &mut output, &mut stats, budget, &tokenizer)?;
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
                                        path.extension().and_then(|e| e.to_str()),
                                    );
                                    stats.add_compressed();
                                    continue;
                                }
                                CompressResult::Fallback(original, _) => {
                                    stats.add_file_size_estimate(
                                        original.len() as u64,
                                        path_str.len(),
                                        path.extension().and_then(|e| e.to_str()),
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
                stats.add_file_size_estimate(
                    metadata.len(),
                    path_str.len(),
                    path.extension().and_then(|e| e.to_str()),
                );
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
        // Dispatch to parallel or sequential writer based on config
        if config.parallel {
            write_normal_parallel(config, &files_to_process, &mut output, &mut stats)?;
        } else {
            write_normal(config, &files_to_process, &mut output, &mut stats)?;
        }
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
    tokenizer: &Tokenizer,
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
        let full_tokens = tokenizer.count_tokens(&candidate.content, candidate.is_prose);

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
                        let compressed_tokens = tokenizer.count_tokens(&compressed, candidate.is_prose);
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
                        let fallback_tokens = tokenizer.count_tokens(&original, candidate.is_prose);
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
                    stats.add_file_size_estimate(
                        content.len() as u64,
                        path_str.len(),
                        candidate.path.extension().and_then(|e| e.to_str()),
                    );
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

/// Write files without token budget using parallel processing with optional caching
fn write_normal_parallel(
    config: &Config,
    files: &[PathBuf],
    output: &mut OutputWriter,
    stats: &mut Statistics,
) -> Result<()> {
    // Load cache if enabled
    let mut cache = if config.cache {
        let cache_dir = cache::get_cache_dir(&config.path)?;
        let cache_path = cache_dir.join("cache.bin");
        Cache::load(&cache_path)?
    } else {
        Cache::new()
    };

    // Generate config hash for cache validation
    let config_hash = cache::generate_config_hash(
        config.compress,
        config.max_file_size,
        &config.include_extensions,
        &config.exclude_extensions,
    );

    // Process files in parallel, collecting results in order
    let results: Vec<FileProcessResult> = files
        .par_iter()
        .map(|path| {
            let display_path = path.display().to_string();

            // Try cache first (if enabled)
            if config.cache {
                if let Ok((mtime, size)) = cache::get_file_metadata(path) {
                    if let Some(entry) = cache.get(&display_path, mtime, size, &config_hash) {
                        let is_compressed = entry.mode.as_deref() == Some("compressed");
                        return FileProcessResult::Success {
                            path: path.clone(),
                            display_path,
                            content: entry.content,
                            mode: entry.mode,
                            is_compressed,
                            cache_hit: true,
                        };
                    }
                }
            }

            // Cache miss - process file
            match fs::read_to_string(path) {
                Ok(content) => {
                    if config.compress {
                        let file_name = path
                            .file_name()
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let is_full = config.is_full_match(&file_name);

                        if is_full {
                            FileProcessResult::Success {
                                path: path.clone(),
                                display_path,
                                content,
                                mode: Some("full".to_string()),
                                is_compressed: false,
                                cache_hit: false,
                            }
                        } else if let Some(lang) = language_for_path(path) {
                            match compress_source(&content, lang) {
                                CompressResult::Compressed(compressed) => {
                                    FileProcessResult::Success {
                                        path: path.clone(),
                                        display_path,
                                        content: compressed,
                                        mode: Some("compressed".to_string()),
                                        is_compressed: true,
                                        cache_hit: false,
                                    }
                                }
                                CompressResult::Fallback(original, _reason) => {
                                    FileProcessResult::Success {
                                        path: path.clone(),
                                        display_path,
                                        content: original,
                                        mode: Some("full".to_string()),
                                        is_compressed: false,
                                        cache_hit: false,
                                    }
                                }
                            }
                        } else {
                            FileProcessResult::Success {
                                path: path.clone(),
                                display_path,
                                content,
                                mode: Some("full".to_string()),
                                is_compressed: false,
                                cache_hit: false,
                            }
                        }
                    } else {
                        FileProcessResult::Success {
                            path: path.clone(),
                            display_path,
                            content,
                            mode: None,
                            is_compressed: false,
                            cache_hit: false,
                        }
                    }
                }
                Err(e) => FileProcessResult::Error {
                    display_path,
                    error: e.to_string(),
                },
            }
        })
        .collect();

    // Update cache with new entries (sequential to avoid data races)
    if config.cache {
        let cache_dir = cache::get_cache_dir(&config.path)?;
        let cache_path = cache_dir.join("cache.bin");

        for result in &results {
            if let FileProcessResult::Success {
                path,
                display_path,
                content,
                mode,
                cache_hit: false,
                ..
            } = result
            {
                if let Ok((mtime, size)) = cache::get_file_metadata(path) {
                    let entry = CacheEntry {
                        mtime,
                        size,
                        content: content.clone(),
                        mode: mode.clone(),
                        config_hash: config_hash.clone(),
                    };
                    cache.insert(display_path.clone(), entry);
                }
            }
        }

        // Prune stale entries
        let paths_to_keep: Vec<&str> = results
            .iter()
            .filter_map(|r| match r {
                FileProcessResult::Success { display_path, .. } => Some(display_path.as_str()),
                _ => None,
            })
            .collect();
        cache.prune_stale(&paths_to_keep);

        // Save cache
        cache.save(&cache_path)?;
    }

    // Write results sequentially (preserves deterministic order)
    let mut cache_hits = 0;
    let mut cache_misses = 0;

    for result in results {
        match result {
            FileProcessResult::Success {
                display_path,
                content,
                mode,
                is_compressed,
                cache_hit,
                ..
            } => {
                if cache_hit {
                    cache_hits += 1;
                } else {
                    cache_misses += 1;
                }

                if let Some(ref m) = mode {
                    output.write_file_content_with_mode(&display_path, &content, Some(m.as_str()))?;
                } else {
                    output.write_file_content(&display_path, &content)?;
                }
                if is_compressed {
                    stats.add_compressed();
                }
            }
            FileProcessResult::Error {
                display_path,
                error,
            } => {
                eprintln!("Error reading {}: {}", display_path, error);
            }
        }
    }

    // Record cache statistics
    if config.cache {
        stats.cache_hits = cache_hits;
        stats.cache_misses = cache_misses;
    }

    stats.add_output_bytes(output.bytes_written());
    output.write_summary(stats)?;
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
    use globset::Glob;

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

    // ========================================================================
    // NEW: Compression Integration Tests (TIER 1)
    // ========================================================================

    #[test]
    fn test_maybe_compress_respects_full_match() {
        // IMPORTANT: Full-match patterns should bypass compression
        // This is critical for preserving exact code when needed
        let config = Config {
            full_match_patterns: Some(vec![Glob::new("main.rs").unwrap().compile_matcher()]),
            compress: true,
            ..Default::default()
        };

        let content = "fn main() {\n    println!(\"hello\");\n}";
        let result = maybe_compress(&config, Path::new("main.rs"), content, &mut Statistics::new());

        match result {
            FileDecision::IncludeFull(c) => assert!(c.contains("println!")),
            _ => panic!("Expected IncludeFull for full-match file"),
        }
    }

    #[test]
    fn test_maybe_compress_handles_unsupported_language() {
        // IMPORTANT: Unsupported languages should not be corrupted
        // We must return full content, not try to compress unknown formats
        let config = Config {
            compress: true,
            ..Default::default()
        };

        let content = "[package]\nname = \"test\"";
        let result = maybe_compress(&config, Path::new("config.toml"), content, &mut Statistics::new());

        match result {
            FileDecision::IncludeFull(c) => assert_eq!(c, content),
            _ => panic!("Expected IncludeFull for unsupported language"),
        }
    }

    // ========================================================================
    // NEW: Skip Logic Tests (TIER 1)
    // ========================================================================

    #[test]
    fn test_should_skip_respects_extension_filtering() {
        // IMPORTANT: Extension filtering must work correctly
        // This prevents analysis of unwanted file types
        let config = Config {
            include_extensions: Some(vec!["rs".to_string()]),
            ..Default::default()
        };

        // Should skip: wrong extension
        assert_eq!(
            should_skip(Path::new("config.json"), &config),
            Some(SkipReason::Extension)
        );

        // Should not skip: correct extension
        assert_eq!(should_skip(Path::new("main.rs"), &config), None);
    }

    #[test]
    fn test_should_skip_priority_secret_over_extension() {
        // IMPORTANT: Secrets must always be skipped, even if extension matches
        // This is a security feature that cannot be overridden
        let config = Config {
            include_extensions: Some(vec!["env".to_string()]), // Hypothetically include .env
            ..Default::default()
        };

        // Secret check should have priority
        assert_eq!(
            should_skip(Path::new(".env"), &config),
            Some(SkipReason::Secret)
        );
    }
}
