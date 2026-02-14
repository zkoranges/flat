use anyhow::{bail, Result};
use clap::Parser;
use flat::parse::{parse_binary_number, parse_decimal_number};
use flat::{walk_and_flatten, Config};
use globset::Glob;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "flat")]
#[command(version)]
#[command(about = "Flatten a codebase into AI-friendly format")]
#[command(long_about = "\
Flatten a codebase into AI-friendly XML format. Outputs <file> tags with source \
content, respecting .gitignore and skipping binaries and secrets automatically.

Examples:
  flat                                  Flatten current directory to stdout
  flat src/ | pbcopy                    Copy to clipboard (macOS)
  flat --include rs,toml                Only Rust and TOML files
  flat --compress                       Signatures only — strip function bodies
  flat --compress --tokens 8k            Fit into a token budget (8k = 8,000 tokens)
  flat --compress --full-match 'main.rs'  Keep main.rs full, compress the rest
  flat --stats                          Preview file count and size
  flat --dry-run                        List files without content")]
#[command(after_help = "\
Compression (--compress) extracts signatures and strips function/method bodies, \
reducing token usage by 30-60%. Supported languages: Rust, TypeScript, JavaScript, \
Python, Go. Unsupported files pass through in full.

Combine --compress with --tokens to fit a codebase into a context window. \
High-priority files (README, entry points, configs) are included first; \
low-priority files (tests, fixtures) are excluded first.

Exit codes: 0 = success, 3 = no files matched")]
struct Cli {
    /// Directory to process
    #[arg(default_value = ".", value_name = "DIR")]
    path: PathBuf,

    /// Include only these extensions [e.g. --include rs,toml,md]
    #[arg(long, value_delimiter = ',', value_name = "EXT")]
    include: Option<Vec<String>>,

    /// Exclude these extensions [e.g. --exclude json,lock]
    #[arg(long, value_delimiter = ',', value_name = "EXT")]
    exclude: Option<Vec<String>>,

    /// Only files matching a glob pattern [e.g. --match '*_test.go']
    #[arg(long, alias = "regex", value_name = "GLOB")]
    r#match: Option<Vec<String>>,

    /// Write output to a file instead of stdout
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// List files that would be included, without content
    #[arg(long)]
    dry_run: bool,

    /// Show statistics only — no file listing or content
    #[arg(long)]
    stats: bool,

    /// Path to a custom .gitignore file
    #[arg(long, value_name = "FILE")]
    gitignore: Option<PathBuf>,

    /// Maximum file size in bytes (supports k/M/G suffixes, e.g., 10M)
    #[arg(long, default_value = "1048576", value_parser = parse_binary_number, value_name = "BYTES")]
    max_size: u64,

    /// Extract signatures and strip function bodies (Rust, TS, JS, Python, Go)
    #[arg(long)]
    compress: bool,

    /// Keep full content for files matching these globs (use with --compress)
    #[arg(long, value_delimiter = ',', value_name = "GLOB")]
    full_match: Option<Vec<String>>,

    /// Cap output to an estimated token budget (supports k/M/G suffixes, e.g., 10k)
    #[arg(long, value_parser = parse_decimal_number, value_name = "N")]
    tokens: Option<usize>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let match_patterns = match cli.r#match {
        Some(patterns) => {
            let mut compiled = Vec::new();
            for pattern in &patterns {
                match Glob::new(pattern) {
                    Ok(glob) => compiled.push(glob.compile_matcher()),
                    Err(e) => bail!("Invalid match pattern '{}': {}", pattern, e),
                }
            }
            Some(compiled)
        }
        None => None,
    };

    let full_match_patterns = match cli.full_match {
        Some(patterns) => {
            if !cli.compress {
                eprintln!("Warning: --full-match has no effect without --compress");
            }
            let mut compiled = Vec::new();
            for pattern in &patterns {
                match Glob::new(pattern) {
                    Ok(glob) => compiled.push(glob.compile_matcher()),
                    Err(e) => bail!("Invalid full-match pattern '{}': {}", pattern, e),
                }
            }
            Some(compiled)
        }
        None => None,
    };

    let config = Config {
        path: cli.path,
        include_extensions: cli.include,
        exclude_extensions: cli.exclude,
        match_patterns,
        output_file: cli.output,
        dry_run: cli.dry_run,
        stats_only: cli.stats,
        gitignore_path: cli.gitignore,
        max_file_size: cli.max_size,
        compress: cli.compress,
        full_match_patterns,
        token_budget: cli.tokens,
    };

    let stats = walk_and_flatten(&config)?;

    // Exit with error if no files appear in the output
    let output_files = if stats.token_budget.is_some() {
        stats
            .included_files
            .saturating_sub(stats.excluded_by_budget.len())
    } else {
        stats.included_files
    };
    if output_files == 0 {
        eprintln!("Error: No files matched the criteria");
        std::process::exit(3);
    }

    Ok(())
}
