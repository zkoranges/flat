use super::Formatter;
use crate::output::Statistics;
use std::io::{self, Write};

/// XML formatter that outputs files in XML format
pub struct XmlFormatter {
    writer: Box<dyn Write>,
    bytes_written: usize,
}

impl XmlFormatter {
    pub fn new(writer: Box<dyn Write>) -> Self {
        Self {
            writer,
            bytes_written: 0,
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
}

impl Formatter for XmlFormatter {
    fn write_file(
        &mut self,
        path: &str,
        content: &str,
        mode: Option<&str>,
        _extension: Option<&str>,
    ) -> io::Result<()> {
        let escaped_path = Self::escape_xml(path);
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

    fn finalize(&mut self, stats: &Statistics) -> io::Result<()> {
        let summary = stats.format_summary();
        self.writer.write_all(summary.as_bytes())?;
        self.bytes_written += summary.len();

        self.writer.write_all(b"\n")?;
        self.bytes_written += 1;

        Ok(())
    }

    fn bytes_written(&self) -> usize {
        self.bytes_written
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_xml() {
        assert_eq!(XmlFormatter::escape_xml("hello"), "hello");
        assert_eq!(XmlFormatter::escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(XmlFormatter::escape_xml("a & b"), "a &amp; b");
        assert_eq!(XmlFormatter::escape_xml("\"quoted\""), "&quot;quoted&quot;");
    }
}
