use crate::filters::SkipReason;
use std::collections::HashMap;
use std::io::Write;

#[derive(Debug, Default)]
pub struct Statistics {
    pub total_files: usize,
    pub included_files: usize,
    pub skipped_by_reason: HashMap<String, usize>,
}

impl Statistics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_included(&mut self) {
        self.total_files += 1;
        self.included_files += 1;
    }

    pub fn add_skipped(&mut self, reason: SkipReason) {
        self.total_files += 1;
        *self.skipped_by_reason.entry(reason.to_string()).or_insert(0) += 1;
    }

    pub fn total_skipped(&self) -> usize {
        self.skipped_by_reason.values().sum()
    }

    pub fn format_summary(&self) -> String {
        let mut summary = format!(
            "<summary>\nTotal files: {}\nIncluded: {}\n",
            self.total_files, self.included_files
        );

        if self.total_skipped() > 0 {
            summary.push_str(&format!("Skipped: {}", self.total_skipped()));

            let mut reasons: Vec<_> = self.skipped_by_reason.iter().collect();
            reasons.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

            let reason_str = reasons
                .iter()
                .map(|(reason, count)| format!("{} {}", count, reason))
                .collect::<Vec<_>>()
                .join(", ");

            summary.push_str(&format!(" ({})", reason_str));
        }

        summary.push_str("\n</summary>\n");
        summary
    }
}

pub struct OutputWriter {
    writer: Box<dyn Write>,
}

impl OutputWriter {
    pub fn new(writer: Box<dyn Write>) -> Self {
        Self { writer }
    }

    pub fn write_file_content(&mut self, path: &str, content: &str) -> std::io::Result<()> {
        writeln!(self.writer, "<file path=\"{}\">", escape_xml(path))?;
        write!(self.writer, "{}", content)?;
        if !content.ends_with('\n') {
            writeln!(self.writer)?;
        }
        writeln!(self.writer, "</file>")?;
        writeln!(self.writer)?;
        Ok(())
    }

    pub fn write_summary(&mut self, stats: &Statistics) -> std::io::Result<()> {
        write!(self.writer, "{}", stats.format_summary())?;
        writeln!(self.writer)?;
        Ok(())
    }

    pub fn write_file_path(&mut self, path: &str) -> std::io::Result<()> {
        writeln!(self.writer, "{}", path)?;
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
        stats.add_included();
        stats.add_included();
        stats.add_skipped(SkipReason::Binary);
        stats.add_skipped(SkipReason::Secret);
        stats.add_skipped(SkipReason::Binary);

        assert_eq!(stats.total_files, 5);
        assert_eq!(stats.included_files, 2);
        assert_eq!(stats.total_skipped(), 3);
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("hello"), "hello");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml("\"quoted\""), "&quot;quoted&quot;");
    }
}
