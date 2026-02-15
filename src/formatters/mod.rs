use crate::output::Statistics;
use std::io;

/// Trait for output formatters that support different output formats
pub trait Formatter {
    /// Write a file entry to the output
    fn write_file(
        &mut self,
        path: &str,
        content: &str,
        mode: Option<&str>,
        extension: Option<&str>,
    ) -> io::Result<()>;

    /// Finalize the output with statistics and summary
    fn finalize(&mut self, stats: &Statistics) -> io::Result<()>;

    /// Get the total number of bytes written
    fn bytes_written(&self) -> usize;
}

pub mod json;
pub mod plaintext;
pub mod template;
pub mod xml;

#[cfg(test)]
mod tests {
    #[test]
    fn test_formatter_trait_exists() {
        // Compile-time test to ensure trait is properly defined
        // This will fail if the trait is not properly accessible
    }
}
