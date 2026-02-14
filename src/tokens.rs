/// Estimate the number of tokens for a piece of content.
///
/// Uses pessimistic (conservative) estimation per PDR spec:
/// - Code files: bytes / 3 (~3.0 chars/token)
/// - Prose files: bytes / 4 (~4.0 chars/token)
///
/// This intentionally overestimates to stay within context windows.
pub fn estimate_tokens(content: &str, is_prose: bool) -> usize {
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
}
