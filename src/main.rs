use anyhow::{bail, Context, Result};
use clap::Parser;
use flat::config::OutputFormat;
use flat::github;
use flat::parse::{parse_binary_number, parse_decimal_number};
use flat::tokens::TokenizerKind;
use flat::{walk_and_flatten, Config};
use globset::Glob;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "flat")]
#[command(version)]
#[command(about = "Flatten a codebase into AI-friendly format")]
#[command(long_about = "\
Flatten a codebase into AI-friendly format. Outputs XML/JSON with source content, \
respecting .gitignore and skipping binaries and secrets automatically.

Examples:
  flat                                      Flatten current directory
  flat src/ | pbcopy                        Copy to clipboard (macOS)
  flat --github owner/repo                  Clone and analyze a GitHub repo
  flat -I node_modules,dist                 Exclude directories (short alias)
  flat -i rs,toml -c                        Include Rust+TOML, compress
  flat -m '*_test.go'                       Match pattern (short alias)
  flat --no-ignore --stats                  Include gitignored files
  flat --format json                        Output as structured JSON
  flat --template claude-review             Code review format
  flat --compress --tokens 8k               Fit into 8k token budget
  flat --compress --full-match 'main.rs'    Keep main.rs full, compress rest
  flat --stats                              Show file count and size
  flat --dry-run                            List files without content")]
#[command(after_help = "\
FILTERING PRECEDENCE:
  1. Directory exclusion (--exclude-dir/-I)
  2. .gitignore files (unless --no-ignore)
  3. Pattern matching (--match/-m)
  4. Secret detection (always)
  5. Extension filters (--include/-i, --exclude)
  6. Binary detection
  7. File size (--max-size)

COMPRESSION: --compress extracts signatures and strips bodies, reducing tokens 30-80%. \
Supports: Rust, TypeScript, JavaScript, Python, Go, C, C++, Java, C#, PHP, Ruby, Solidity, Elixir, SQL, Bash.

GITHUB: --github owner/repo clones and analyzes in one command. Works with all other flags. \
Supports SSH URLs, branches, subpaths, and private repos (with GITHUB_TOKEN).

TEMPLATES: --template {minimal,claude-review,openai-docs} or path to custom .hbs file

TOKEN BUDGETING: --compress --tokens 100k fits code into 100k tokens, prioritizing important files.

Exit codes: 0 = success, 3 = no files matched")]
struct FlattenArgs {
    /// Run in MCP server mode (JSON-RPC 2.0 interface for Claude Desktop integration)
    #[arg(long, global = true)]
    serve: bool,
    /// Directory to process
    #[arg(default_value = ".", value_name = "DIR")]
    path: PathBuf,

    /// Include only these extensions [e.g. -i rs,toml,md or --include rs,toml,md]
    #[arg(short, long, value_delimiter = ',', value_name = "EXT")]
    include: Option<Vec<String>>,

    /// Exclude these extensions [e.g. --exclude json,lock]
    #[arg(long, value_delimiter = ',', value_name = "EXT")]
    exclude: Option<Vec<String>>,

    /// Only files matching a glob pattern [e.g. -m '*_test.go' or --match '*_test.go']
    #[arg(short, long, alias = "regex", value_name = "GLOB")]
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

    /// Extract signatures and strip function bodies (16 languages: Rust, TS/JS, Python, Go, Java, C/C++, C#, Ruby, PHP, Solidity, Elixir, SQL, Bash)
    #[arg(short, long)]
    compress: bool,

    /// Exclude directories by pattern [e.g. -I node_modules,dist or --exclude-dir node_modules]
    #[arg(short = 'I', long, value_delimiter = ',', value_name = "PATTERN")]
    exclude_dir: Option<Vec<String>>,

    /// Disable standard gitignore and .git/info/exclude filtering
    #[arg(long)]
    no_ignore: bool,

    /// Path to custom ignore file (replaces --gitignore)
    #[arg(long, value_name = "FILE")]
    ignore_file: Option<PathBuf>,

    /// Keep full content for files matching these globs (use with --compress)
    #[arg(long, value_delimiter = ',', value_name = "GLOB")]
    full_match: Option<Vec<String>>,

    /// Cap output to an estimated token budget (supports k/M/G suffixes, e.g., 10k)
    #[arg(long, value_parser = parse_decimal_number, value_name = "N")]
    tokens: Option<usize>,

    /// Tokenizer for token counting [heuristic, claude, gpt-4, gpt-3.5]
    #[arg(long, default_value = "heuristic", value_name = "NAME")]
    tokenizer: String,

    /// Output format [xml, json]
    #[arg(long, default_value = "xml", value_name = "FORMAT")]
    format: String,

    /// Use a template for output (overrides --format)
    /// [minimal, claude-review, openai-docs, or path to custom template]
    #[arg(long, value_name = "TEMPLATE")]
    template: Option<String>,

    /// Treat path as GitHub URL (format: owner/repo or full URL)
    #[arg(long)]
    github: bool,

    /// GitHub personal access token for private repos (can also set GITHUB_TOKEN env var)
    #[arg(long, value_name = "TOKEN")]
    github_token: Option<String>,
}

fn main() -> Result<()> {
    let cli = FlattenArgs::parse();

    if cli.serve {
        flat::mcp::run()
    } else {
        run_flatten(cli)
    }
}

fn run_flatten(cli: FlattenArgs) -> Result<()> {
    // Check for deprecated --gitignore flag
    if cli.gitignore.is_some() {
        eprintln!("Warning: --gitignore is deprecated, use --ignore-file instead");
    }

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

    let tokenizer = match TokenizerKind::parse_name(&cli.tokenizer) {
        Some(kind) => kind,
        None => bail!(
            "Unknown tokenizer '{}'. Valid options: {}",
            cli.tokenizer,
            TokenizerKind::valid_names()
        ),
    };

    let output_format = match OutputFormat::from_string(&cli.format) {
        Some(fmt) => fmt,
        None => bail!(
            "Unknown format '{}'. Valid options: xml, json",
            cli.format
        ),
    };

    // Handle GitHub URL if --github flag is set
    let (actual_path, _temp_dir_holder) = if cli.github {
        let github_url = github::parse_github_url(&cli.path.to_string_lossy())
            .context("Failed to parse GitHub URL")?;

        // Get GitHub token from CLI arg or environment variable
        let token = cli
            .github_token
            .clone()
            .or_else(|| std::env::var("GITHUB_TOKEN").ok());

        let clone_result = github::clone_repo(&github_url, token.as_deref())
            .context("Failed to clone repository from GitHub")?;

        let (mut path, temp_dir) = clone_result.into_path_and_temp();

        // If subpath is specified, use it
        if let Some(ref subpath) = github_url.subpath {
            path = path.join(subpath);
        }

        (path, Some(temp_dir))
    } else {
        (cli.path.clone(), None)
    };

    // Resolve ignore file: --ignore-file takes precedence, fall back to --gitignore
    let ignore_file = cli.ignore_file.or(cli.gitignore);

    let config = Config {
        path: actual_path,
        include_extensions: cli.include,
        exclude_extensions: cli.exclude,
        match_patterns,
        output_file: cli.output,
        dry_run: cli.dry_run,
        stats_only: cli.stats,
        gitignore_path: ignore_file,
        max_file_size: cli.max_size,
        compress: cli.compress,
        full_match_patterns,
        token_budget: cli.tokens,
        tokenizer,
        output_format,
        template: cli.template,
        github: cli.github,
        github_token: cli.github_token,
        exclude_dirs: cli.exclude_dir,
        no_ignore: cli.no_ignore,
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
