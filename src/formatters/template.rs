use super::Formatter;
use crate::output::Statistics;
use handlebars::Handlebars;
use serde::Serialize;
use std::io::{self, Write};

/// Template context for rendering
#[derive(Serialize)]
struct TemplateContext {
    repo_name: String,
    timestamp: String,
    statistics: Statistics,
    files: Vec<TemplateFile>,
}

/// File entry for template rendering
#[derive(Serialize)]
struct TemplateFile {
    path: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extension: Option<String>,
}

/// Template formatter that renders using Handlebars
pub struct TemplateFormatter {
    writer: Box<dyn Write>,
    handlebars: Handlebars<'static>,
    template_name: String,
    files: Vec<TemplateFile>,
    bytes_written: usize,
    repo_name: String,
}

impl TemplateFormatter {
    pub fn new(template_path: &str, writer: Box<dyn Write>, repo_name: String) -> io::Result<Self> {
        let mut hb = Handlebars::new();

        // Register built-in templates
        hb.register_template_string(
            "minimal",
            include_str!("../../templates/minimal.hbs"),
        )
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        hb.register_template_string(
            "claude-review",
            include_str!("../../templates/claude-review.hbs"),
        )
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        hb.register_template_string(
            "openai-docs",
            include_str!("../../templates/openai-docs.hbs"),
        )
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        // Determine which template to use
        let template_name = match template_path {
            "minimal" | "claude-review" | "openai-docs" => template_path.to_string(),
            other => {
                // Try to load user template from ~/.config/flat/templates/<name>.hbs
                let config_dir = dirs::config_dir()
                    .map(|d| d.join("flat").join("templates").join(format!("{}.hbs", other)))
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::NotFound,
                            "Cannot determine config directory",
                        )
                    })?;

                if config_dir.exists() {
                    let content = std::fs::read_to_string(&config_dir)?;
                    hb.register_template_string("custom", &content)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
                    "custom".to_string()
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!(
                            "Template '{}' not found. Built-in templates: minimal, claude-review, openai-docs",
                            other
                        ),
                    ));
                }
            }
        };

        Ok(Self {
            writer,
            handlebars: hb,
            template_name,
            files: Vec::new(),
            bytes_written: 0,
            repo_name,
        })
    }
}

impl Formatter for TemplateFormatter {
    fn write_file(
        &mut self,
        path: &str,
        content: &str,
        mode: Option<&str>,
        extension: Option<&str>,
    ) -> io::Result<()> {
        // Buffer files for rendering at finalize time
        self.files.push(TemplateFile {
            path: path.to_string(),
            content: content.to_string(),
            mode: mode.map(|m| m.to_string()),
            extension: extension.map(|e| e.to_string()),
        });
        Ok(())
    }

    fn finalize(&mut self, stats: &Statistics) -> io::Result<()> {
        // Create template context
        let context = TemplateContext {
            repo_name: self.repo_name.clone(),
            timestamp: chrono::Local::now().to_rfc3339(),
            statistics: stats.clone(),
            files: std::mem::take(&mut self.files),
        };

        // Render template
        let rendered = self
            .handlebars
            .render(&self.template_name, &context)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        self.writer.write_all(rendered.as_bytes())?;
        self.bytes_written += rendered.len();

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
    fn test_template_formatter_minimal() {
        let buffer = Vec::new();
        let mut formatter =
            TemplateFormatter::new("minimal", Box::new(buffer), "test_repo".to_string()).unwrap();

        formatter
            .write_file("src/main.rs", "fn main() {}", None, Some("rs"))
            .unwrap();

        let mut stats = Statistics::new();
        stats.add_included(Some("rs"));

        formatter.finalize(&stats).unwrap();

        assert!(formatter.bytes_written() > 0);
    }

    #[test]
    fn test_template_not_found() {
        let result = TemplateFormatter::new("nonexistent", Box::new(Vec::new()), "repo".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_claude_review_template_exists() {
        let buffer = Vec::new();
        let result =
            TemplateFormatter::new("claude-review", Box::new(buffer), "test_repo".to_string());
        assert!(result.is_ok());
    }
}
