use crate::filters::SkipReason;
use std::collections::HashMap;
use std::io::Write;

#[derive(Debug, Default)]
pub struct Statistics {
    pub total_files: usize,
    pub included_files: usize,
    pub skipped_by_reason: HashMap<String, usize>,
    pub included_by_extension: HashMap<String, usize>,
    pub output_size: usize,
    pub compressed_files: usize,
    pub token_budget: Option<usize>,
    pub tokens_used: usize,
    pub excluded_by_budget: Vec<String>,
}

impl Statistics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_included(&mut self, extension: Option<&str>) {
        self.total_files += 1;
        self.included_files += 1;
        let ext = extension.unwrap_or("no extension").to_string();
        *self.included_by_extension.entry(ext).or_insert(0) += 1;
    }

    pub fn add_file_size_estimate(&mut self, file_size: u64, path_length: usize) {
        // Estimate XML overhead:
        // - Opening tag: <file path="..."> + newline = ~15 + path_length bytes
        // - Closing tag: </file>\n\n = 9 bytes
        // - Potential newline after content = 1 byte
        let overhead = 25 + path_length;
        self.output_size += file_size as usize + overhead;
    }

    pub fn add_compressed(&mut self) {
        self.compressed_files += 1;
    }

    pub fn add_skipped(&mut self, reason: SkipReason) {
        self.total_files += 1;
        *self
            .skipped_by_reason
            .entry(reason.to_string())
            .or_insert(0) += 1;
    }

    pub fn add_output_bytes(&mut self, bytes: usize) {
        self.output_size += bytes;
    }

    pub fn total_skipped(&self) -> usize {
        self.skipped_by_reason.values().sum()
    }

    pub fn estimated_tokens(&self) -> usize {
        // Rough estimate: ~4 characters per token
        self.output_size / 4
    }

    fn format_bytes(bytes: usize) -> String {
        const KB: usize = 1024;
        const MB: usize = KB * 1024;

        if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} bytes", bytes)
        }
    }

    fn format_tokens(tokens: usize) -> String {
        if tokens >= 10_000 {
            // Use k suffix for 10k and above
            if tokens >= 1_000_000 {
                format!("{:.1}M", tokens as f64 / 1_000_000.0)
            } else {
                format!("{:.1}k", tokens as f64 / 1_000.0)
            }
        } else if tokens >= 1_000 {
            // Use commas for thousands (manual formatting)
            let s = tokens.to_string();
            let mut result = String::new();
            for (i, c) in s.chars().rev().enumerate() {
                if i > 0 && i % 3 == 0 {
                    result.push(',');
                }
                result.push(c);
            }
            result.chars().rev().collect()
        } else {
            // No formatting for small numbers
            tokens.to_string()
        }
    }

    pub fn format_summary(&self) -> String {
        let mut summary = format!(
            "<summary>\nTotal files: {}\nIncluded: {}",
            self.total_files, self.included_files
        );

        // Add extension breakdown for included files
        if !self.included_by_extension.is_empty() {
            let mut extensions: Vec<_> = self.included_by_extension.iter().collect();
            extensions.sort_by(|(a_ext, a_count), (b_ext, b_count)| {
                b_count.cmp(a_count).then_with(|| a_ext.cmp(b_ext))
            });

            let ext_str = extensions
                .iter()
                .map(|(ext, count)| {
                    if *ext == "no extension" {
                        format!("{} without extension", count)
                    } else {
                        format!("{} .{}", count, ext)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");

            summary.push_str(&format!(" ({})", ext_str));
        }

        summary.push('\n');

        if self.compressed_files > 0 {
            summary.push_str(&format!("Compressed: {} files\n", self.compressed_files));
        }

        if self.total_skipped() > 0 {
            summary.push_str(&format!("Skipped: {}", self.total_skipped()));

            let mut reasons: Vec<_> = self.skipped_by_reason.iter().collect();
            reasons.sort_by(|(a_reason, a_count), (b_reason, b_count)| {
                b_count.cmp(a_count).then_with(|| a_reason.cmp(b_reason))
            });

            let reason_str = reasons
                .iter()
                .map(|(reason, count)| format!("{} {}", count, reason))
                .collect::<Vec<_>>()
                .join(", ");

            summary.push_str(&format!(" ({})", reason_str));
            summary.push('\n');
        }

        // Add token budget info
        if let Some(budget) = self.token_budget {
            summary.push_str(&format!(
                "Token budget: {} / {} used\n",
                Self::format_tokens(self.tokens_used),
                Self::format_tokens(budget)
            ));
            if !self.excluded_by_budget.is_empty() {
                summary.push_str(&format!(
                    "Excluded by budget: {} files\n",
                    self.excluded_by_budget.len()
                ));
            }
        }

        // Add output size and token estimate if there's output
        if self.output_size > 0 {
            summary.push_str(&format!(
                "Output size: {} (~{} tokens)\n",
                Self::format_bytes(self.output_size),
                Self::format_tokens(self.estimated_tokens())
            ));
        }

        summary.push_str("</summary>\n");
        summary
    }
}

pub struct OutputWriter {
    writer: Box<dyn Write>,
    bytes_written: usize,
}

impl OutputWriter {
    pub fn new(writer: Box<dyn Write>) -> Self {
        Self {
            writer,
            bytes_written: 0,
        }
    }

    pub fn bytes_written(&self) -> usize {
        self.bytes_written
    }

    pub fn write_file_content(&mut self, path: &str, content: &str) -> std::io::Result<()> {
        self.write_file_content_with_mode(path, content, None)
    }

    pub fn write_file_content_with_mode(
        &mut self,
        path: &str,
        content: &str,
        mode: Option<&str>,
    ) -> std::io::Result<()> {
        let escaped_path = escape_xml(path);
        let opening_tag = match mode {
            Some(m) => format!("<file path=\"{}\" mode=\"{}\">\n", escaped_path, m),
            None => format!("<file path=\"{}\">\n", escaped_path),
        };
        self.writer.write_all(opening_tag.as_bytes())?;
        self.bytes_written += opening_tag.len();

        self.writer.write_all(content.as_bytes())?;
        self.bytes_written += content.len();

        if !content.ends_with('\n') {
            self.writer.write_all(b"\n")?;
            self.bytes_written += 1;
        }

        self.writer.write_all(b"</file>\n\n")?;
        self.bytes_written += 9; // "</file>\n\n"

        Ok(())
    }

    pub fn write_summary(&mut self, stats: &Statistics) -> std::io::Result<()> {
        let summary = stats.format_summary();
        self.writer.write_all(summary.as_bytes())?;
        self.bytes_written += summary.len();

        self.writer.write_all(b"\n")?;
        self.bytes_written += 1;

        Ok(())
    }

    pub fn write_file_path(&mut self, path: &str) -> std::io::Result<()> {
        let line = format!("{}\n", path);
        self.writer.write_all(line.as_bytes())?;
        self.bytes_written += line.len();
        Ok(())
    }
}

/// Escape XML special characters in strings
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_statistics() {
        let mut stats = Statistics::new();
        stats.add_included(Some("rs"));
        stats.add_included(Some("toml"));
        stats.add_skipped(SkipReason::Binary);
        stats.add_skipped(SkipReason::Secret);
        stats.add_skipped(SkipReason::Binary);

        assert_eq!(stats.total_files, 5);
        assert_eq!(stats.included_files, 2);
        assert_eq!(stats.total_skipped(), 3);
        assert_eq!(stats.included_by_extension.get("rs"), Some(&1));
        assert_eq!(stats.included_by_extension.get("toml"), Some(&1));
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("hello"), "hello");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml("\"quoted\""), "&quot;quoted&quot;");
    }
}
