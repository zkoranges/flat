/// Parse a human-friendly number with decimal (SI) suffixes.
///
/// - `k` / `K` = ×1,000
/// - `M` = ×1,000,000
/// - `G` = ×1,000,000,000
///
/// Used for token counts and other abstract quantities.
pub fn parse_decimal_number(input: &str) -> Result<usize, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("empty input".to_string());
    }

    let (digits, multiplier) = match input.as_bytes().last() {
        Some(b'k' | b'K') => (&input[..input.len() - 1], 1_000usize),
        Some(b'M') => (&input[..input.len() - 1], 1_000_000),
        Some(b'G') => (&input[..input.len() - 1], 1_000_000_000),
        _ => (input, 1),
    };

    let base: usize = digits
        .parse()
        .map_err(|_| format!("invalid number: '{input}'"))?;

    base.checked_mul(multiplier)
        .ok_or_else(|| format!("number too large: '{input}'"))
}

/// Parse a human-friendly number with binary (IEC) suffixes.
///
/// - `k` / `K` = ×1,024
/// - `M` = ×1,048,576
/// - `G` = ×1,073,741,824
///
/// Used for byte sizes where binary multipliers are conventional.
pub fn parse_binary_number(input: &str) -> Result<u64, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("empty input".to_string());
    }

    let (digits, multiplier) = match input.as_bytes().last() {
        Some(b'k' | b'K') => (&input[..input.len() - 1], 1_024u64),
        Some(b'M') => (&input[..input.len() - 1], 1_048_576),
        Some(b'G') => (&input[..input.len() - 1], 1_073_741_824),
        _ => (input, 1),
    };

    let base: u64 = digits
        .parse()
        .map_err(|_| format!("invalid number: '{input}'"))?;

    base.checked_mul(multiplier)
        .ok_or_else(|| format!("number too large: '{input}'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Decimal parsing ──────────────────────────────────────────────

    #[test]
    fn decimal_plain_number() {
        assert_eq!(parse_decimal_number("10000").unwrap(), 10_000);
    }

    #[test]
    fn decimal_zero() {
        assert_eq!(parse_decimal_number("0").unwrap(), 0);
    }

    #[test]
    fn decimal_suffix_k_lower() {
        assert_eq!(parse_decimal_number("10k").unwrap(), 10_000);
    }

    #[test]
    fn decimal_suffix_k_upper() {
        assert_eq!(parse_decimal_number("10K").unwrap(), 10_000);
    }

    #[test]
    fn decimal_suffix_m() {
        assert_eq!(parse_decimal_number("5M").unwrap(), 5_000_000);
    }

    #[test]
    fn decimal_suffix_g() {
        assert_eq!(parse_decimal_number("2G").unwrap(), 2_000_000_000);
    }

    #[test]
    fn decimal_one_k() {
        assert_eq!(parse_decimal_number("1k").unwrap(), 1_000);
    }

    #[test]
    fn decimal_100k() {
        assert_eq!(parse_decimal_number("100K").unwrap(), 100_000);
    }

    #[test]
    fn decimal_whitespace_trimmed() {
        assert_eq!(parse_decimal_number("  8k  ").unwrap(), 8_000);
    }

    #[test]
    fn decimal_invalid_letters() {
        assert!(parse_decimal_number("abc").is_err());
    }

    #[test]
    fn decimal_invalid_decimal_point() {
        assert!(parse_decimal_number("1.5k").is_err());
    }

    #[test]
    fn decimal_empty_input() {
        assert!(parse_decimal_number("").is_err());
    }

    #[test]
    fn decimal_suffix_only() {
        assert!(parse_decimal_number("k").is_err());
    }

    #[test]
    fn decimal_negative() {
        assert!(parse_decimal_number("-1k").is_err());
    }

    #[test]
    fn decimal_overflow() {
        // usize::MAX / 1000 + 1 with k suffix should overflow
        let huge = format!("{}k", usize::MAX);
        assert!(parse_decimal_number(&huge).is_err());
    }

    // ── Binary parsing ───────────────────────────────────────────────

    #[test]
    fn binary_plain_number() {
        assert_eq!(parse_binary_number("1048576").unwrap(), 1_048_576);
    }

    #[test]
    fn binary_zero() {
        assert_eq!(parse_binary_number("0").unwrap(), 0);
    }

    #[test]
    fn binary_suffix_k_lower() {
        assert_eq!(parse_binary_number("1k").unwrap(), 1_024);
    }

    #[test]
    fn binary_suffix_k_upper() {
        assert_eq!(parse_binary_number("10K").unwrap(), 10_240);
    }

    #[test]
    fn binary_suffix_m() {
        assert_eq!(parse_binary_number("1M").unwrap(), 1_048_576);
    }

    #[test]
    fn binary_suffix_m_10() {
        assert_eq!(parse_binary_number("10M").unwrap(), 10_485_760);
    }

    #[test]
    fn binary_suffix_g() {
        assert_eq!(parse_binary_number("1G").unwrap(), 1_073_741_824);
    }

    #[test]
    fn binary_whitespace_trimmed() {
        assert_eq!(parse_binary_number("  5M  ").unwrap(), 5_242_880);
    }

    #[test]
    fn binary_invalid_letters() {
        assert!(parse_binary_number("xyz").is_err());
    }

    #[test]
    fn binary_invalid_decimal_point() {
        assert!(parse_binary_number("1.5M").is_err());
    }

    #[test]
    fn binary_empty_input() {
        assert!(parse_binary_number("").is_err());
    }

    #[test]
    fn binary_suffix_only() {
        assert!(parse_binary_number("M").is_err());
    }

    #[test]
    fn binary_overflow() {
        let huge = format!("{}G", u64::MAX);
        assert!(parse_binary_number(&huge).is_err());
    }
}
