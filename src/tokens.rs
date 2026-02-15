use std::fmt;
use tiktoken_rs::CoreBPE;

/// Which tokenizer to use for token counting.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum TokenizerKind {
    /// Heuristic estimation: bytes/3 for code, bytes/4 for prose (default)
    #[default]
    Heuristic,
    /// Claude tokenizer (uses cl100k_base as approximation)
    Claude,
    /// GPT-4 tokenizer (cl100k_base)
    Gpt4,
    /// GPT-3.5 tokenizer (cl100k_base)
    Gpt35,
}

impl fmt::Display for TokenizerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenizerKind::Heuristic => write!(f, "heuristic"),
            TokenizerKind::Claude => write!(f, "claude"),
            TokenizerKind::Gpt4 => write!(f, "gpt-4"),
            TokenizerKind::Gpt35 => write!(f, "gpt-3.5"),
        }
    }
}

impl TokenizerKind {
    /// Parse a tokenizer name from CLI input.
    pub fn parse_name(s: &str) -> Option<Self> {
        match s {
            "heuristic" => Some(TokenizerKind::Heuristic),
            "claude" => Some(TokenizerKind::Claude),
            "gpt-4" | "gpt4" => Some(TokenizerKind::Gpt4),
            "gpt-3.5" | "gpt3.5" | "gpt-35" => Some(TokenizerKind::Gpt35),
            _ => None,
        }
    }

    /// List valid tokenizer names for help text.
    pub fn valid_names() -> &'static str {
        "heuristic, claude, gpt-4, gpt-3.5"
    }
}

/// A tokenizer that can count tokens in text.
pub struct Tokenizer {
    kind: TokenizerKind,
    bpe: Option<CoreBPE>,
}

impl Tokenizer {
    /// Create a new tokenizer of the given kind.
    /// Falls back to heuristic if the BPE model fails to load.
    pub fn new(kind: TokenizerKind) -> Self {
        let bpe = match &kind {
            TokenizerKind::Heuristic => None,
            TokenizerKind::Claude | TokenizerKind::Gpt4 | TokenizerKind::Gpt35 => {
                match tiktoken_rs::cl100k_base() {
                    Ok(bpe) => Some(bpe),
                    Err(e) => {
                        eprintln!(
                            "Warning: failed to load {} tokenizer: {}, falling back to heuristic",
                            kind, e
                        );
                        None
                    }
                }
            }
        };
        Self { kind, bpe }
    }

    /// Count tokens in the given text.
    pub fn count_tokens(&self, content: &str, is_prose: bool) -> usize {
        match &self.bpe {
            Some(bpe) => bpe.encode_with_special_tokens(content).len(),
            None => estimate_tokens_heuristic(content, is_prose),
        }
    }

    /// Return the kind of this tokenizer.
    pub fn kind(&self) -> &TokenizerKind {
        &self.kind
    }

    /// Whether this tokenizer uses real BPE encoding (not heuristic).
    pub fn is_real(&self) -> bool {
        self.bpe.is_some()
    }
}

/// Estimate the number of tokens for a piece of content using heuristic.
///
/// Uses pessimistic (conservative) estimation per PDR spec:
/// - Code files: bytes / 3 (~3.0 chars/token)
/// - Prose files: bytes / 4 (~4.0 chars/token)
///
/// This intentionally overestimates to stay within context windows.
pub fn estimate_tokens(content: &str, is_prose: bool) -> usize {
    estimate_tokens_heuristic(content, is_prose)
}

fn estimate_tokens_heuristic(content: &str, is_prose: bool) -> usize {
    let byte_count = content.len();
    if is_prose {
        byte_count / 4
    } else {
        byte_count / 3
    }
}

/// Check if a file extension indicates prose content
pub fn is_prose_extension(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        "md" | "txt" | "rst" | "adoc" | "textile" | "org" | "wiki"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_code() {
        // 300 bytes of code = 100 tokens (300/3)
        let code = "x".repeat(300);
        assert_eq!(estimate_tokens(&code, false), 100);
    }

    #[test]
    fn test_estimate_tokens_prose() {
        // 400 bytes of prose = 100 tokens (400/4)
        let prose = "x".repeat(400);
        assert_eq!(estimate_tokens(&prose, true), 100);
    }

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(estimate_tokens("", false), 0);
        assert_eq!(estimate_tokens("", true), 0);
    }

    #[test]
    fn test_is_prose_extension() {
        assert!(is_prose_extension("md"));
        assert!(is_prose_extension("txt"));
        assert!(is_prose_extension("rst"));
        assert!(!is_prose_extension("rs"));
        assert!(!is_prose_extension("py"));
        assert!(!is_prose_extension("ts"));
    }

    #[test]
    fn test_tokenizer_kind_parse() {
        assert_eq!(TokenizerKind::parse_name("heuristic"), Some(TokenizerKind::Heuristic));
        assert_eq!(TokenizerKind::parse_name("claude"), Some(TokenizerKind::Claude));
        assert_eq!(TokenizerKind::parse_name("gpt-4"), Some(TokenizerKind::Gpt4));
        assert_eq!(TokenizerKind::parse_name("gpt4"), Some(TokenizerKind::Gpt4));
        assert_eq!(TokenizerKind::parse_name("gpt-3.5"), Some(TokenizerKind::Gpt35));
        assert_eq!(TokenizerKind::parse_name("gpt3.5"), Some(TokenizerKind::Gpt35));
        assert_eq!(TokenizerKind::parse_name("invalid"), None);
    }

    #[test]
    fn test_tokenizer_kind_default() {
        assert_eq!(TokenizerKind::default(), TokenizerKind::Heuristic);
    }

    #[test]
    fn test_heuristic_tokenizer() {
        let tok = Tokenizer::new(TokenizerKind::Heuristic);
        assert!(!tok.is_real());
        let code = "x".repeat(300);
        assert_eq!(tok.count_tokens(&code, false), 100);
    }

    #[test]
    fn test_real_tokenizer_loads() {
        let tok = Tokenizer::new(TokenizerKind::Gpt4);
        assert!(tok.is_real());
    }

    #[test]
    fn test_real_tokenizer_counts() {
        let tok = Tokenizer::new(TokenizerKind::Gpt4);
        let count = tok.count_tokens("Hello, world!", false);
        assert!(count > 0 && count < 10, "Expected 1-9 tokens, got {}", count);
    }

    #[test]
    fn test_real_tokenizer_known_value() {
        let tok = Tokenizer::new(TokenizerKind::Gpt4);
        let count = tok.count_tokens("Hello world", false);
        assert_eq!(count, 2, "Expected 2 tokens for 'Hello world', got {}", count);
    }

    #[test]
    fn test_real_vs_heuristic_comparison() {
        let real = Tokenizer::new(TokenizerKind::Gpt4);
        let heuristic = Tokenizer::new(TokenizerKind::Heuristic);

        let code = "fn main() {\n    println!(\"Hello, world!\");\n}\n";
        let real_count = real.count_tokens(code, false);
        let heuristic_count = heuristic.count_tokens(code, false);

        assert!(real_count > 0);
        assert!(heuristic_count > 0);
        assert!(real_count < code.len(), "Real count should be less than byte length");
    }

    #[test]
    fn test_heuristic_overestimates_code() {
        let real = Tokenizer::new(TokenizerKind::Gpt4);
        let heuristic = Tokenizer::new(TokenizerKind::Heuristic);

        let code = r#"
use std::collections::HashMap;

pub struct Config {
    pub name: String,
    pub values: HashMap<String, Vec<u32>>,
}

impl Config {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            values: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: &str, val: u32) {
        self.values.entry(key.to_string()).or_default().push(val);
    }
}
"#;
        let real_count = real.count_tokens(code, false);
        let heuristic_count = heuristic.count_tokens(code, false);

        assert!(
            heuristic_count >= real_count,
            "Heuristic ({}) should overestimate vs real ({}) for code",
            heuristic_count,
            real_count
        );
    }

    #[test]
    fn test_heuristic_overestimates_prose() {
        let real = Tokenizer::new(TokenizerKind::Gpt4);
        let heuristic = Tokenizer::new(TokenizerKind::Heuristic);

        let prose = "The quick brown fox jumps over the lazy dog. \
            This is a longer piece of prose text that should be tokenized \
            differently from code. Natural language tends to have longer tokens \
            on average compared to source code with its punctuation and symbols.";

        let real_count = real.count_tokens(prose, true);
        let heuristic_count = heuristic.count_tokens(prose, true);

        assert!(real_count > 0, "Real tokenizer should produce tokens for prose");
        assert!(heuristic_count > 0, "Heuristic should produce tokens for prose");
        let ratio = heuristic_count as f64 / real_count as f64;
        assert!(
            ratio > 0.5 && ratio < 3.0,
            "Heuristic ({}) and real ({}) should be within 3x of each other for prose (ratio: {:.2})",
            heuristic_count,
            real_count,
            ratio
        );
    }

    #[test]
    fn test_all_real_tokenizers_produce_same_counts() {
        let claude = Tokenizer::new(TokenizerKind::Claude);
        let gpt4 = Tokenizer::new(TokenizerKind::Gpt4);
        let gpt35 = Tokenizer::new(TokenizerKind::Gpt35);

        let text = "fn main() { println!(\"Hello, world!\"); }";

        let claude_count = claude.count_tokens(text, false);
        let gpt4_count = gpt4.count_tokens(text, false);
        let gpt35_count = gpt35.count_tokens(text, false);

        assert_eq!(claude_count, gpt4_count, "Claude and GPT-4 should match");
        assert_eq!(gpt4_count, gpt35_count, "GPT-4 and GPT-3.5 should match");
    }

    #[test]
    fn test_real_tokenizer_empty_string() {
        let tok = Tokenizer::new(TokenizerKind::Gpt4);
        assert_eq!(tok.count_tokens("", false), 0);
        assert_eq!(tok.count_tokens("", true), 0);
    }

    #[test]
    fn test_real_tokenizer_whitespace_only() {
        let tok = Tokenizer::new(TokenizerKind::Gpt4);
        let count = tok.count_tokens("   \n\n\t  ", false);
        assert!(count > 0, "Whitespace should produce at least 1 token, got {}", count);
    }

    #[test]
    fn test_tokenizer_kind_display() {
        assert_eq!(format!("{}", TokenizerKind::Heuristic), "heuristic");
        assert_eq!(format!("{}", TokenizerKind::Claude), "claude");
        assert_eq!(format!("{}", TokenizerKind::Gpt4), "gpt-4");
        assert_eq!(format!("{}", TokenizerKind::Gpt35), "gpt-3.5");
    }

    #[test]
    fn test_tokenizer_kind_roundtrip() {
        for kind in [
            TokenizerKind::Heuristic,
            TokenizerKind::Claude,
            TokenizerKind::Gpt4,
            TokenizerKind::Gpt35,
        ] {
            let display = format!("{}", kind);
            let parsed = TokenizerKind::parse_name(&display);
            assert_eq!(
                parsed,
                Some(kind.clone()),
                "Roundtrip failed for {}",
                display
            );
        }
    }

    #[test]
    fn test_tokenizer_kind_accessor() {
        let tok = Tokenizer::new(TokenizerKind::Claude);
        assert_eq!(*tok.kind(), TokenizerKind::Claude);
    }
}
