use std::path::Path;
use tree_sitter::{Language, Parser};

/// Languages supported for compression
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressLanguage {
    Rust,
    TypeScript,
    Tsx,
    JavaScript,
    Jsx,
    Python,
    Go,
}

/// Map a file extension to a compressible language
pub fn language_for_extension(ext: &str) -> Option<CompressLanguage> {
    match ext.to_lowercase().as_str() {
        "rs" => Some(CompressLanguage::Rust),
        "ts" => Some(CompressLanguage::TypeScript),
        "tsx" => Some(CompressLanguage::Tsx),
        "js" => Some(CompressLanguage::JavaScript),
        "jsx" => Some(CompressLanguage::Jsx),
        "py" => Some(CompressLanguage::Python),
        "go" => Some(CompressLanguage::Go),
        _ => None,
    }
}

/// Detect language from a file path's extension
pub fn language_for_path(path: &Path) -> Option<CompressLanguage> {
    path.extension()
        .and_then(|e| e.to_str())
        .and_then(language_for_extension)
}

/// Get the tree-sitter Language for a CompressLanguage
fn tree_sitter_language(lang: CompressLanguage) -> Language {
    match lang {
        CompressLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
        CompressLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        CompressLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        CompressLanguage::JavaScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        CompressLanguage::Jsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        CompressLanguage::Python => tree_sitter_python::LANGUAGE.into(),
        CompressLanguage::Go => tree_sitter_go::LANGUAGE.into(),
    }
}

/// Result of compressing a source file
#[derive(Debug)]
pub enum CompressResult {
    /// Successfully compressed
    Compressed(String),
    /// Fell back to full content (with optional reason for stderr warning)
    Fallback(String, Option<String>),
}

/// Strip UTF-8 BOM if present
fn strip_bom(source: &str) -> &str {
    source.strip_prefix('\u{FEFF}').unwrap_or(source)
}

/// Compress a source file by extracting declarations and signatures.
///
/// Returns compressed output or falls back to full content per the fallback rules:
/// - Unsupported extension → full content
/// - Parse error (NULL tree) → full content + warn
/// - ERROR nodes in parse tree → full content + warn
/// - Empty compressed output → full content + warn
/// - Compressed ≥ original → full content (no warning)
pub fn compress_source(source: &str, lang: CompressLanguage) -> CompressResult {
    let source = strip_bom(source);

    if source.is_empty() {
        return CompressResult::Compressed(String::new());
    }

    let ts_lang = tree_sitter_language(lang);

    let mut parser = Parser::new();
    if parser.set_language(&ts_lang).is_err() {
        return CompressResult::Fallback(
            source.to_string(),
            Some("failed to set parser language".to_string()),
        );
    }

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => {
            return CompressResult::Fallback(
                source.to_string(),
                Some("tree-sitter returned NULL tree".to_string()),
            );
        }
    };

    let root = tree.root_node();

    // Check for ERROR nodes
    if has_error_nodes(root) {
        return CompressResult::Fallback(
            source.to_string(),
            Some("parse tree contains ERROR nodes".to_string()),
        );
    }

    let compressed = match lang {
        CompressLanguage::Rust => compress_rust(source, root),
        CompressLanguage::TypeScript
        | CompressLanguage::Tsx
        | CompressLanguage::JavaScript
        | CompressLanguage::Jsx => compress_typescript(source, root),
        CompressLanguage::Python => compress_python(source, root),
        CompressLanguage::Go => compress_go(source, root),
    };

    if compressed.is_empty() {
        return CompressResult::Fallback(
            source.to_string(),
            Some("compressed output is empty".to_string()),
        );
    }

    if compressed.len() >= source.len() {
        return CompressResult::Compressed(source.to_string());
    }

    CompressResult::Compressed(compressed)
}

/// Recursively check if the parse tree contains any ERROR nodes
fn has_error_nodes(node: tree_sitter::Node) -> bool {
    if node.is_error() {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if has_error_nodes(child) {
            return true;
        }
    }
    false
}

/// Extract the text of a node from source
fn node_text<'a>(source: &'a str, node: tree_sitter::Node) -> &'a str {
    &source[node.byte_range()]
}

// ============================================================================
// Rust Compressor
// ============================================================================

fn compress_rust(source: &str, root: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            // Keep these entirely
            "use_declaration" | "extern_crate_declaration" | "mod_item"
            | "type_item" | "const_item" | "static_item" | "attribute_item"
            | "inner_attribute_item" | "macro_definition" | "macro_invocation" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            // Keep line/block comments (doc comments and regular)
            "line_comment" | "block_comment" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            // Function: keep signature, replace body
            "function_item" => {
                output.push_str(&compress_rust_function(source, child));
                output.push('\n');
            }
            // Struct: keep with field names/types
            "struct_item" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            // Enum: keep with variants
            "enum_item" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            // Trait: keep signatures, strip method bodies
            "trait_item" => {
                output.push_str(&compress_rust_trait(source, child));
                output.push('\n');
            }
            // Impl block: keep signatures, strip method bodies
            "impl_item" => {
                output.push_str(&compress_rust_impl(source, child));
                output.push('\n');
            }
            _ => {}
        }
    }

    output.trim_end().to_string()
}

fn compress_rust_function(source: &str, node: tree_sitter::Node) -> String {
    // Find the block (body) and replace it with { ... }
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "block" {
            // Everything before the block is the signature
            return format!("{} {{ ... }}", source[node.start_byte()..child.start_byte()].trim_end());
        }
    }

    // No block found (declaration without body), keep as-is
    node_text(source, node).to_string()
}

fn compress_rust_trait(source: &str, node: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "declaration_list" {
            // Output everything before the declaration list
            output.push_str(source[node.start_byte()..child.start_byte()].trim_end());
            output.push_str(" {\n");

            // Process items inside the trait
            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                match item.kind() {
                    "function_item" => {
                        output.push_str("    ");
                        output.push_str(&compress_rust_function(source, item));
                        output.push('\n');
                    }
                    "function_signature_item" => {
                        output.push_str("    ");
                        output.push_str(node_text(source, item));
                        output.push('\n');
                    }
                    "type_item" | "const_item" | "attribute_item" => {
                        output.push_str("    ");
                        output.push_str(node_text(source, item));
                        output.push('\n');
                    }
                    "line_comment" | "block_comment" => {
                        output.push_str("    ");
                        output.push_str(node_text(source, item));
                        output.push('\n');
                    }
                    _ => {}
                }
            }
            output.push('}');
            return output;
        }
    }

    node_text(source, node).to_string()
}

fn compress_rust_impl(source: &str, node: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "declaration_list" {
            output.push_str(source[node.start_byte()..child.start_byte()].trim_end());
            output.push_str(" {\n");

            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                match item.kind() {
                    "function_item" => {
                        output.push_str("    ");
                        output.push_str(&compress_rust_function(source, item));
                        output.push('\n');
                    }
                    "type_item" | "const_item" | "attribute_item" => {
                        output.push_str("    ");
                        output.push_str(node_text(source, item));
                        output.push('\n');
                    }
                    "line_comment" | "block_comment" => {
                        output.push_str("    ");
                        output.push_str(node_text(source, item));
                        output.push('\n');
                    }
                    _ => {}
                }
            }
            output.push('}');
            return output;
        }
    }

    node_text(source, node).to_string()
}

// ============================================================================
// TypeScript/JavaScript Compressor
// ============================================================================

fn compress_typescript(source: &str, root: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            // Imports/exports
            "import_statement" | "export_statement" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            // Comments
            "comment" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            // Function declarations
            "function_declaration" => {
                output.push_str(&compress_ts_function(source, child));
                output.push('\n');
            }
            // Class declarations
            "class_declaration" => {
                output.push_str(&compress_ts_class(source, child));
                output.push('\n');
            }
            // Interface declarations
            "interface_declaration" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            // Type aliases
            "type_alias_declaration" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            // Enum declarations
            "enum_declaration" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            // Variable declarations (const/let/var at top level - often exports)
            "lexical_declaration" | "variable_declaration" => {
                output.push_str(&compress_ts_variable(source, child));
                output.push('\n');
            }
            // Export default
            "export_default_declaration" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            // Module declarations
            "module" | "ambient_declaration" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            _ => {}
        }
    }

    output.trim_end().to_string()
}

fn compress_ts_function(source: &str, node: tree_sitter::Node) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "statement_block" {
            return format!("{} {{ ... }}", source[node.start_byte()..child.start_byte()].trim_end());
        }
    }
    node_text(source, node).to_string()
}

fn compress_ts_class(source: &str, node: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "class_body" {
            output.push_str(source[node.start_byte()..child.start_byte()].trim_end());
            output.push_str(" {\n");

            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                match item.kind() {
                    "method_definition" | "public_field_definition" | "property_definition" => {
                        output.push_str("    ");
                        output.push_str(&compress_ts_method(source, item));
                        output.push('\n');
                    }
                    "comment" => {
                        output.push_str("    ");
                        output.push_str(node_text(source, item));
                        output.push('\n');
                    }
                    _ => {}
                }
            }
            output.push('}');
            return output;
        }
    }

    node_text(source, node).to_string()
}

fn compress_ts_method(source: &str, node: tree_sitter::Node) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "statement_block" {
            return format!("{} {{ ... }}", source[node.start_byte()..child.start_byte()].trim_end());
        }
    }
    node_text(source, node).to_string()
}

fn compress_ts_variable(source: &str, node: tree_sitter::Node) -> String {
    // For arrow functions and complex initializers, try to compress
    let text = node_text(source, node);
    if text.contains("=>") && text.len() > 80 {
        // Try to find arrow function body and compress it
        let mut cursor = node.walk();
        if let Some(compressed) = compress_ts_var_inner(source, node, &mut cursor) {
            return compressed;
        }
    }
    text.to_string()
}

fn compress_ts_var_inner(source: &str, node: tree_sitter::Node, _cursor: &mut tree_sitter::TreeCursor) -> Option<String> {
    // Walk to find arrow_function children with statement_block bodies
    fn find_arrow_body(node: tree_sitter::Node) -> Option<(usize, usize)> {
        if node.kind() == "arrow_function" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "statement_block" {
                    return Some((child.start_byte(), child.end_byte()));
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(range) = find_arrow_body(child) {
                return Some(range);
            }
        }
        None
    }

    if let Some((body_start, body_end)) = find_arrow_body(node) {
        let before = &source[node.start_byte()..body_start];
        let after = &source[body_end..node.end_byte()];
        Some(format!("{}{{ ... }}{}", before.trim_end(), after))
    } else {
        None
    }
}

// ============================================================================
// Python Compressor
// ============================================================================

fn compress_python(source: &str, root: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            // Imports
            "import_statement" | "import_from_statement" | "future_import_statement" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            // Comments
            "comment" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            // Expression statements (often docstrings at module level)
            "expression_statement" => {
                let text = node_text(source, child);
                // Keep module-level docstrings
                if text.starts_with("\"\"\"") || text.starts_with("'''") {
                    output.push_str(text);
                    output.push('\n');
                }
            }
            // Function definitions
            "function_definition" | "decorated_definition" => {
                output.push_str(&compress_python_function(source, child));
                output.push('\n');
            }
            // Class definitions
            "class_definition" => {
                output.push_str(&compress_python_class(source, child));
                output.push('\n');
            }
            // Global variable assignments at module level
            "assignment" => {
                let text = node_text(source, child);
                // Keep type-annotated assignments and simple constants
                if text.len() <= 120 {
                    output.push_str(text);
                    output.push('\n');
                }
            }
            _ => {}
        }
    }

    output.trim_end().to_string()
}

fn compress_python_function(source: &str, node: tree_sitter::Node) -> String {
    let mut cursor = node.walk();

    // Handle decorated functions
    if node.kind() == "decorated_definition" {
        let mut decorators = String::new();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "decorator" => {
                    decorators.push_str(node_text(source, child));
                    decorators.push('\n');
                }
                "function_definition" => {
                    decorators.push_str(&compress_python_function_inner(source, child));
                    return decorators;
                }
                "class_definition" => {
                    decorators.push_str(&compress_python_class(source, child));
                    return decorators;
                }
                _ => {}
            }
        }
        return decorators;
    }

    compress_python_function_inner(source, node)
}

fn compress_python_function_inner(source: &str, node: tree_sitter::Node) -> String {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "block" {
            let sig = source[node.start_byte()..child.start_byte()].trim_end();
            // Check for docstring (first statement only)
            let mut block_cursor = child.walk();
            if let Some(block_child) = child.children(&mut block_cursor).next() {
                if block_child.kind() == "expression_statement" {
                    let text = node_text(source, block_child);
                    if text.starts_with("\"\"\"") || text.starts_with("'''") {
                        return format!("{}\n    {}\n    ...", sig, text);
                    }
                }
            }
            return format!("{}\n    ...", sig);
        }
    }

    node_text(source, node).to_string()
}

fn compress_python_class(source: &str, node: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "block" {
            let header = source[node.start_byte()..child.start_byte()].trim_end();
            output.push_str(header);
            output.push('\n');

            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                match item.kind() {
                    "function_definition" | "decorated_definition" => {
                        // Indent the compressed function
                        let compressed = compress_python_function(source, item);
                        for line in compressed.lines() {
                            output.push_str("    ");
                            output.push_str(line);
                            output.push('\n');
                        }
                    }
                    "expression_statement" => {
                        let text = node_text(source, item);
                        // Keep docstrings and assignments (class-level vars)
                        if text.starts_with("\"\"\"") || text.starts_with("'''") || text.contains('=') {
                            output.push_str("    ");
                            output.push_str(text);
                            output.push('\n');
                        }
                    }
                    "comment" => {
                        output.push_str("    ");
                        output.push_str(node_text(source, item));
                        output.push('\n');
                    }
                    _ => {}
                }
            }

            return output.trim_end().to_string();
        }
    }

    node_text(source, node).to_string()
}

// ============================================================================
// Go Compressor
// ============================================================================

fn compress_go(source: &str, root: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            // Package declaration
            "package_clause" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            // Import declarations
            "import_declaration" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            // Comments
            "comment" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            // Function declarations
            "function_declaration" => {
                output.push_str(&compress_go_function(source, child));
                output.push('\n');
            }
            // Method declarations
            "method_declaration" => {
                output.push_str(&compress_go_function(source, child));
                output.push('\n');
            }
            // Type declarations (struct, interface, etc.)
            "type_declaration" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            // Const/var declarations
            "const_declaration" | "var_declaration" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            _ => {}
        }
    }

    output.trim_end().to_string()
}

fn compress_go_function(source: &str, node: tree_sitter::Node) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block" {
            return format!("{} {{ ... }}", source[node.start_byte()..child.start_byte()].trim_end());
        }
    }
    node_text(source, node).to_string()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Language detection tests
    #[test]
    fn test_language_for_extension() {
        assert_eq!(language_for_extension("rs"), Some(CompressLanguage::Rust));
        assert_eq!(language_for_extension("ts"), Some(CompressLanguage::TypeScript));
        assert_eq!(language_for_extension("tsx"), Some(CompressLanguage::Tsx));
        assert_eq!(language_for_extension("js"), Some(CompressLanguage::JavaScript));
        assert_eq!(language_for_extension("jsx"), Some(CompressLanguage::Jsx));
        assert_eq!(language_for_extension("py"), Some(CompressLanguage::Python));
        assert_eq!(language_for_extension("go"), Some(CompressLanguage::Go));
        assert_eq!(language_for_extension("md"), None);
        assert_eq!(language_for_extension("toml"), None);
    }

    #[test]
    fn test_language_for_path() {
        assert_eq!(language_for_path(Path::new("main.rs")), Some(CompressLanguage::Rust));
        assert_eq!(language_for_path(Path::new("foo.test.ts")), Some(CompressLanguage::TypeScript));
        assert_eq!(language_for_path(Path::new("Makefile")), None);
        assert_eq!(language_for_path(Path::new("README.md")), None);
    }

    // Rust compression tests
    #[test]
    fn test_compress_rust_function() {
        let source = r#"fn hello(name: &str) -> String {
    let greeting = format!("Hello, {}!", name);
    println!("{}", greeting);
    greeting
}"#;
        match compress_source(source, CompressLanguage::Rust) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("fn hello(name: &str) -> String"));
                assert!(output.contains("{ ... }"));
                assert!(!output.contains("let greeting"));
            }
            CompressResult::Fallback(_, reason) => panic!("Expected compression, got fallback: {:?}", reason),
        }
    }

    #[test]
    fn test_compress_rust_struct() {
        let source = r#"pub struct Config {
    pub path: String,
    pub verbose: bool,
}"#;
        match compress_source(source, CompressLanguage::Rust) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("pub struct Config"));
                assert!(output.contains("pub path: String"));
                assert!(output.contains("pub verbose: bool"));
            }
            CompressResult::Fallback(_, _) => panic!("Expected compression"),
        }
    }

    #[test]
    fn test_compress_rust_impl() {
        let source = r#"impl Config {
    pub fn new() -> Self {
        Self { path: String::new(), verbose: false }
    }

    pub fn validate(&self) -> bool {
        !self.path.is_empty()
    }
}"#;
        match compress_source(source, CompressLanguage::Rust) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("impl Config"));
                assert!(output.contains("pub fn new() -> Self { ... }"));
                assert!(output.contains("pub fn validate(&self) -> bool { ... }"));
                assert!(!output.contains("is_empty"));
            }
            CompressResult::Fallback(_, _) => panic!("Expected compression"),
        }
    }

    #[test]
    fn test_compress_rust_use_and_const() {
        let source = r#"use std::path::Path;
use std::collections::HashMap;

const MAX_SIZE: usize = 1024;

fn process() {
    // complex logic
    println!("processing");
}"#;
        match compress_source(source, CompressLanguage::Rust) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("use std::path::Path;"));
                assert!(output.contains("use std::collections::HashMap;"));
                assert!(output.contains("const MAX_SIZE: usize = 1024;"));
                assert!(output.contains("fn process() { ... }"));
            }
            CompressResult::Fallback(_, _) => panic!("Expected compression"),
        }
    }

    #[test]
    fn test_compress_rust_trait() {
        let source = r#"pub trait Compressor {
    fn name(&self) -> &str;
    fn compress(&self, source: &str) -> String {
        source.to_string()
    }
}"#;
        match compress_source(source, CompressLanguage::Rust) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("pub trait Compressor"));
                assert!(output.contains("fn name(&self) -> &str;"));
                assert!(output.contains("fn compress(&self, source: &str) -> String { ... }"));
            }
            CompressResult::Fallback(_, _) => panic!("Expected compression"),
        }
    }

    // TypeScript compression tests
    #[test]
    fn test_compress_typescript_function() {
        let source = r#"import { Config } from './config';

function processData(data: string[]): number {
    const filtered = data.filter(x => x.length > 0);
    return filtered.length;
}

export default processData;"#;
        match compress_source(source, CompressLanguage::TypeScript) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("import { Config }"));
                assert!(output.contains("function processData(data: string[]): number { ... }"));
                assert!(output.contains("export default processData;"));
                assert!(!output.contains("filtered"));
            }
            CompressResult::Fallback(_, reason) => panic!("Expected compression, got fallback: {:?}", reason),
        }
    }

    #[test]
    fn test_compress_typescript_class() {
        let source = r#"class UserService {
    private db: Database;

    constructor(db: Database) {
        this.db = db;
    }

    async getUser(id: string): Promise<User> {
        const user = await this.db.find(id);
        if (!user) throw new Error('Not found');
        return user;
    }
}"#;
        match compress_source(source, CompressLanguage::TypeScript) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("class UserService"));
                assert!(output.contains("{ ... }"));
                assert!(!output.contains("throw new Error"));
            }
            CompressResult::Fallback(_, _) => panic!("Expected compression"),
        }
    }

    #[test]
    fn test_compress_typescript_interface() {
        let source = r#"interface User {
    id: string;
    name: string;
    email: string;
}"#;
        match compress_source(source, CompressLanguage::TypeScript) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("interface User"));
                assert!(output.contains("id: string"));
            }
            CompressResult::Fallback(_, _) => panic!("Expected compression"),
        }
    }

    // Python compression tests
    #[test]
    fn test_compress_python_function() {
        let source = r#"import os
from pathlib import Path

def process_file(path: str) -> bool:
    """Process a single file."""
    content = Path(path).read_text()
    lines = content.splitlines()
    return len(lines) > 0"#;
        match compress_source(source, CompressLanguage::Python) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("import os"));
                assert!(output.contains("from pathlib import Path"));
                assert!(output.contains("def process_file(path: str) -> bool:"));
                assert!(output.contains("\"\"\"Process a single file.\"\"\""));
                assert!(output.contains("..."));
                assert!(!output.contains("splitlines"));
            }
            CompressResult::Fallback(_, reason) => panic!("Expected compression, got fallback: {:?}", reason),
        }
    }

    #[test]
    fn test_compress_python_class() {
        let source = r#"class Config:
    """Configuration container."""
    DEFAULT_SIZE = 1024

    def __init__(self, path: str):
        self.path = path
        self.size = self.DEFAULT_SIZE

    def validate(self) -> bool:
        return os.path.exists(self.path)"#;
        match compress_source(source, CompressLanguage::Python) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("class Config:"));
                assert!(output.contains("\"\"\"Configuration container.\"\"\""));
                assert!(output.contains("DEFAULT_SIZE = 1024"));
                assert!(output.contains("def __init__(self, path: str):"));
                assert!(output.contains("def validate(self) -> bool:"));
                assert!(!output.contains("os.path.exists"));
            }
            CompressResult::Fallback(_, _) => panic!("Expected compression"),
        }
    }

    // Go compression tests
    #[test]
    fn test_compress_go_function() {
        let source = r#"package main

import "fmt"

// ProcessData handles incoming data
func ProcessData(data []string) int {
	filtered := make([]string, 0)
	for _, d := range data {
		if len(d) > 0 {
			filtered = append(filtered, d)
		}
	}
	return len(filtered)
}"#;
        match compress_source(source, CompressLanguage::Go) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("package main"));
                assert!(output.contains("import \"fmt\""));
                assert!(output.contains("// ProcessData handles incoming data"));
                assert!(output.contains("func ProcessData(data []string) int { ... }"));
                assert!(!output.contains("filtered"));
            }
            CompressResult::Fallback(_, reason) => panic!("Expected compression, got fallback: {:?}", reason),
        }
    }

    #[test]
    fn test_compress_go_struct_and_method() {
        let source = r#"package main

type Config struct {
	Path    string
	Verbose bool
}

func (c *Config) Validate() bool {
	return c.Path != ""
}"#;
        match compress_source(source, CompressLanguage::Go) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("type Config struct"));
                assert!(output.contains("Path    string"));
                assert!(output.contains("func (c *Config) Validate() bool { ... }"));
            }
            CompressResult::Fallback(_, _) => panic!("Expected compression"),
        }
    }

    // Fallback tests
    #[test]
    fn test_compress_empty_source() {
        match compress_source("", CompressLanguage::Rust) {
            CompressResult::Compressed(output) => assert!(output.is_empty()),
            CompressResult::Fallback(_, _) => panic!("Empty source should return empty compressed"),
        }
    }

    #[test]
    fn test_compress_bom_stripped() {
        let source = "\u{FEFF}fn main() {\n    println!(\"hello\");\n}";
        match compress_source(source, CompressLanguage::Rust) {
            CompressResult::Compressed(output) => {
                assert!(!output.starts_with('\u{FEFF}'));
                assert!(output.contains("fn main()"));
            }
            CompressResult::Fallback(_, _) => panic!("Expected compression"),
        }
    }

    #[test]
    fn test_compress_only_comments() {
        let source = "// This is a comment\n// Another comment\n";
        match compress_source(source, CompressLanguage::Rust) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("// This is a comment"));
                assert!(output.contains("// Another comment"));
            }
            CompressResult::Fallback(_, _) => panic!("Comments-only should compress (keeping comments)"),
        }
    }
}
