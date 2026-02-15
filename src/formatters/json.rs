use super::Formatter;
use crate::output::Statistics;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

/// A file entry in JSON output
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
}

/// JSON output structure
#[derive(Serialize, Deserialize, Debug)]
pub struct JsonOutput {
    pub statistics: Statistics,
    pub files: Vec<FileEntry>,
}

/// JSON formatter that buffers files and outputs JSON
pub struct JsonFormatter {
    writer: Box<dyn Write>,
    files: Vec<FileEntry>,
    bytes_written: usize,
}

impl JsonFormatter {
    pub fn new(writer: Box<dyn Write>) -> Self {
        Self {
            writer,
            files: Vec::new(),
            bytes_written: 0,
        }
    }
}

impl Formatter for JsonFormatter {
    fn write_file(
        &mut self,
        path: &str,
        content: &str,
        mode: Option<&str>,
        extension: Option<&str>,
    ) -> io::Result<()> {
        // Buffer files - JSON needs complete structure before serialization
        self.files.push(FileEntry {
            path: path.to_string(),
            content: content.to_string(),
            mode: mode.map(|m| m.to_string()),
            extension: extension.map(|e| e.to_string()),
        });
        Ok(())
    }

    fn finalize(&mut self, stats: &Statistics) -> io::Result<()> {
        let output = JsonOutput {
            statistics: stats.clone(),
            files: std::mem::take(&mut self.files),
        };

        let json = serde_json::to_string_pretty(&output)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        self.writer.write_all(json.as_bytes())?;
        self.bytes_written += json.len();

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
    fn test_json_formatter_creates_valid_structure() {
        let mut formatter = JsonFormatter::new(Box::new(Vec::new()));

        // Write a file
        formatter
            .write_file("src/main.rs", "fn main() {}", None, Some("rs"))
            .unwrap();

        let mut stats = Statistics::new();
        stats.add_included(Some("rs"));

        formatter.finalize(&stats).unwrap();

        assert_eq!(formatter.files.len(), 0); // Files should be moved to output
        assert!(formatter.bytes_written() > 0);
    }
}
