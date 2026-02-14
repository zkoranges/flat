use anyhow::{bail, Result};
use clap::Parser;
use flat::{walk_and_flatten, Config};
use globset::Glob;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "flat")]
#[command(about = "Flatten a codebase into AI-friendly format", long_about = None)]
#[command(version)]
struct Cli {
    /// Directory to process (default: current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Include only these file extensions (comma-separated, e.g., rs,toml,md)
    #[arg(long, value_delimiter = ',')]
    include: Option<Vec<String>>,

    /// Exclude these file extensions (comma-separated, e.g., test,json)
    #[arg(long, value_delimiter = ',')]
    exclude: Option<Vec<String>>,

    /// Include only files whose name matches a glob pattern (can be repeated, e.g., *_test.go)
    #[arg(long, alias = "regex")]
    r#match: Option<Vec<String>>,

    /// Write output to file instead of stdout
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// List files that would be included without processing them
    #[arg(long)]
    dry_run: bool,

    /// Show statistics only (no file listing or content)
    #[arg(long)]
    stats: bool,

    /// Use a custom .gitignore file
    #[arg(long)]
    gitignore: Option<PathBuf>,

    /// Maximum file size in bytes (default: 1MB)
    #[arg(long, default_value = "1048576")]
    max_size: u64,
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
    };

    let stats = walk_and_flatten(&config)?;

    // Exit with error if no files were processed
    if stats.included_files == 0 {
        eprintln!("Error: No files matched the criteria");
        std::process::exit(3);
    }

    Ok(())
}
