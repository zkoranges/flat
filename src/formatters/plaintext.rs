use super::Formatter;
use crate::output::Statistics;
use std::io::{self, Write};

/// Plain text formatter for dry-run mode - just lists file paths
pub struct PlainTextFormatter {
    writer: Box<dyn Write>,
    bytes_written: usize,
}

impl PlainTextFormatter {
    pub fn new(writer: Box<dyn Write>) -> Self {
        Self {
            writer,
            bytes_written: 0,
        }
    }
}

impl Formatter for PlainTextFormatter {
    fn write_file(
        &mut self,
        path: &str,
        _content: &str,
        _mode: Option<&str>,
        _extension: Option<&str>,
    ) -> io::Result<()> {
        let line = format!("{}\n", path);
        self.writer.write_all(line.as_bytes())?;
        self.bytes_written += line.len();
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
