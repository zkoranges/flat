use crate::filters::SkipReason;
use crate::tokens::is_prose_extension;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub total_files: usize,
    pub included_files: usize,
    pub skipped_by_reason: HashMap<String, usize>,
    pub included_by_extension: HashMap<String, usize>,
    pub output_size: usize,
    pub prose_bytes: usize,   // bytes from prose files (md, txt, rst, etc.)
    pub code_bytes: usize,    // bytes from code files
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

    pub fn add_file_size_estimate(&mut self, file_size: u64, path_length: usize, extension: Option<&str>) {
        // Estimate XML overhead:
        // - Opening tag: <file path="..."> + newline = ~15 + path_length bytes
        // - Closing tag: </file>\n\n = 9 bytes
        // - Potential newline after content = 1 byte
        let overhead = 25 + path_length;
        let total_bytes = file_size as usize + overhead;
        self.output_size += total_bytes;

        // Track by content type
        let ext_str = extension.unwrap_or("");
        if is_prose_extension(ext_str) {
            self.prose_bytes += total_bytes;
        } else {
            self.code_bytes += total_bytes;
        }
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
        // Conservative: treat all as code (higher token estimate) when type unknown
        self.code_bytes += bytes;
    }

    pub fn total_skipped(&self) -> usize {
        self.skipped_by_reason.values().sum()
    }

    pub fn estimated_tokens(&self) -> usize {
        // Conservative estimation per PDR spec:
        // - Code files: bytes / 3 (~3.0 chars/token)
        // - Prose files: bytes / 4 (~4.0 chars/token)
        let code_tokens = self.code_bytes / 3;
        let prose_tokens = self.prose_bytes / 4;
        code_tokens + prose_tokens
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

        // Add output size (skip token estimate when budget is active to avoid confusion)
        if self.output_size > 0 {
            if self.token_budget.is_some() {
                summary.push_str(&format!(
                    "Output size: {}\n",
                    Self::format_bytes(self.output_size),
                ));
            } else {
                summary.push_str(&format!(
                    "Output size: {} (~{} tokens)\n",
                    Self::format_bytes(self.output_size),
                    Self::format_tokens(self.estimated_tokens())
                ));
            }
        }

        summary.push_str("</summary>\n");
        summary
    }
}

pub struct OutputWriter {
    formatter: Box<dyn crate::formatters::Formatter>,
}

impl OutputWriter {
    pub fn new(writer: Box<dyn Write>) -> Self {
        Self {
            formatter: Box::new(crate::formatters::xml::XmlFormatter::new(writer)),
        }
    }

    pub fn new_xml(writer: Box<dyn Write>) -> Self {
        Self::new(writer)
    }

    pub fn new_plaintext(writer: Box<dyn Write>) -> Self {
        Self {
            formatter: Box::new(crate::formatters::plaintext::PlainTextFormatter::new(writer)),
        }
    }

    pub fn new_json(writer: Box<dyn Write>) -> Self {
        Self {
            formatter: Box::new(crate::formatters::json::JsonFormatter::new(writer)),
        }
    }

    pub fn new_template(
        template_path: &str,
        writer: Box<dyn Write>,
        repo_name: String,
    ) -> std::io::Result<Self> {
        let formatter =
            crate::formatters::template::TemplateFormatter::new(template_path, writer, repo_name)?;
        Ok(Self {
            formatter: Box::new(formatter),
        })
    }

    pub fn bytes_written(&self) -> usize {
        self.formatter.bytes_written()
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
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str());
        self.formatter.write_file(path, content, mode, extension)
    }

    pub fn write_summary(&mut self, stats: &Statistics) -> std::io::Result<()> {
        self.formatter.finalize(stats)
    }

    pub fn write_file_path(&mut self, path: &str) -> std::io::Result<()> {
        // In dry-run mode, write a minimal file entry with just the path
        // The formatter will handle the actual output format
        self.formatter
            .write_file(path, "", None, None)
    }
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
}
