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
    Java,
    CSharp,
    C,
    Cpp,
    Ruby,
    Php,
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
        "java" => Some(CompressLanguage::Java),
        "cs" => Some(CompressLanguage::CSharp),
        "c" | "h" => Some(CompressLanguage::C),
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => Some(CompressLanguage::Cpp),
        "rb" => Some(CompressLanguage::Ruby),
        "php" => Some(CompressLanguage::Php),
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
        CompressLanguage::Java => tree_sitter_java::LANGUAGE.into(),
        CompressLanguage::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
        CompressLanguage::C => tree_sitter_c::LANGUAGE.into(),
        CompressLanguage::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        CompressLanguage::Ruby => tree_sitter_ruby::LANGUAGE.into(),
        CompressLanguage::Php => tree_sitter_php::LANGUAGE_PHP.into(),
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
/// - tree-sitter panic → full content + warn (catch_unwind)
pub fn compress_source(source: &str, lang: CompressLanguage) -> CompressResult {
    let source = strip_bom(source);

    if source.is_empty() {
        return CompressResult::Compressed(String::new());
    }

    // Wrap tree-sitter calls in catch_unwind to prevent panics from crashing the process
    let source_owned = source.to_string();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        compress_source_inner(&source_owned, lang)
    }));

    match result {
        Ok(compress_result) => compress_result,
        Err(_) => CompressResult::Fallback(
            source.to_string(),
            Some("tree-sitter panic caught".to_string()),
        ),
    }
}

/// Inner compression logic, separated so catch_unwind can wrap it
fn compress_source_inner(source: &str, lang: CompressLanguage) -> CompressResult {
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
        CompressLanguage::Java => compress_java(source, root),
        CompressLanguage::CSharp => compress_csharp(source, root),
        CompressLanguage::C => compress_c(source, root),
        CompressLanguage::Cpp => compress_cpp(source, root),
        CompressLanguage::Ruby => compress_ruby(source, root),
        CompressLanguage::Php => compress_php(source, root),
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

/// Replace a function/method body with `{ ... }`, keeping the signature.
///
/// Searches for the first child matching any of `body_kinds` and replaces it.
/// Falls back to the full node text if no matching body child is found.
fn compress_body(source: &str, node: tree_sitter::Node, body_kinds: &[&str]) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if body_kinds.contains(&child.kind()) {
            return format!(
                "{} {{ ... }}",
                source[node.start_byte()..child.start_byte()].trim_end()
            );
        }
    }
    node_text(source, node).to_string()
}

/// Append a single line with indentation to an output string.
fn push_indented(output: &mut String, indent: &str, text: &str) {
    output.push_str(indent);
    output.push_str(text);
    output.push('\n');
}

/// Append a multi-line block with indentation to an output string.
fn push_indented_block(output: &mut String, indent: &str, block: &str) {
    for line in block.lines() {
        output.push_str(indent);
        output.push_str(line);
        output.push('\n');
    }
}

// ============================================================================
// Rust Compressor
// ============================================================================

fn compress_rust(source: &str, root: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            "function_item" => {
                output.push_str(&compress_rust_function(source, child));
                output.push('\n');
            }
            "trait_item" => {
                output.push_str(&compress_rust_trait(source, child));
                output.push('\n');
            }
            "impl_item" => {
                output.push_str(&compress_rust_impl(source, child));
                output.push('\n');
            }
            "use_declaration"
            | "extern_crate_declaration"
            | "mod_item"
            | "type_item"
            | "const_item"
            | "static_item"
            | "attribute_item"
            | "inner_attribute_item"
            | "macro_definition"
            | "macro_invocation"
            | "line_comment"
            | "block_comment"
            | "struct_item"
            | "enum_item" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            _ => {}
        }
    }

    output.trim_end().to_string()
}

fn compress_rust_function(source: &str, node: tree_sitter::Node) -> String {
    compress_body(source, node, &["block"])
}

fn compress_rust_trait(source: &str, node: tree_sitter::Node) -> String {
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
                        push_indented(&mut output, "    ", &compress_rust_function(source, item));
                    }
                    "function_signature_item"
                    | "type_item"
                    | "const_item"
                    | "attribute_item"
                    | "line_comment"
                    | "block_comment" => {
                        push_indented(&mut output, "    ", node_text(source, item));
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
                        push_indented(&mut output, "    ", &compress_rust_function(source, item));
                    }
                    "type_item" | "const_item" | "attribute_item" | "line_comment"
                    | "block_comment" => {
                        push_indented(&mut output, "    ", node_text(source, item));
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
            "export_statement" => {
                output.push_str(&compress_ts_export(source, child));
                output.push('\n');
            }
            "function_declaration" => {
                output.push_str(&compress_ts_function(source, child));
                output.push('\n');
            }
            "class_declaration" => {
                output.push_str(&compress_ts_class(source, child));
                output.push('\n');
            }
            "lexical_declaration" | "variable_declaration" => {
                output.push_str(&compress_ts_variable(source, child));
                output.push('\n');
            }
            "import_statement"
            | "comment"
            | "interface_declaration"
            | "type_alias_declaration"
            | "enum_declaration"
            | "export_default_declaration"
            | "module"
            | "ambient_declaration" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            _ => {}
        }
    }

    output.trim_end().to_string()
}

fn compress_ts_function(source: &str, node: tree_sitter::Node) -> String {
    compress_body(source, node, &["statement_block"])
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
                        push_indented(
                            &mut output,
                            "    ",
                            &compress_body(source, item, &["statement_block"]),
                        );
                    }
                    "comment" => {
                        push_indented(&mut output, "    ", node_text(source, item));
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

fn compress_ts_var_inner(
    source: &str,
    node: tree_sitter::Node,
    _cursor: &mut tree_sitter::TreeCursor,
) -> Option<String> {
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

fn compress_ts_export(source: &str, node: tree_sitter::Node) -> String {
    let mut cursor = node.walk();
    for inner in node.children(&mut cursor) {
        match inner.kind() {
            "function_declaration" => {
                // Find the statement_block in the function
                let mut fcursor = inner.walk();
                for fchild in inner.children(&mut fcursor) {
                    if fchild.kind() == "statement_block" {
                        // Everything from export start to the body start is the signature
                        let sig = source[node.start_byte()..fchild.start_byte()].trim_end();
                        return format!("{} {{ ... }}", sig);
                    }
                }
                // No body found, keep as-is
                return node_text(source, node).to_string();
            }
            "class_declaration" => {
                let prefix = &source[node.start_byte()..inner.start_byte()];
                return format!("{}{}", prefix, compress_ts_class(source, inner));
            }
            _ => {}
        }
    }
    // No compressible child found, keep verbatim
    node_text(source, node).to_string()
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
            // Expression statements (docstrings and assignments at module level)
            "expression_statement" => {
                let text = node_text(source, child);
                // Keep module-level docstrings
                if text.starts_with("\"\"\"") || text.starts_with("'''") {
                    output.push_str(text);
                    output.push('\n');
                } else {
                    // Keep simple assignments (e.g., MAX_RETRIES = 3)
                    let mut inner_cursor = child.walk();
                    for inner_child in child.children(&mut inner_cursor) {
                        if inner_child.kind() == "assignment" && text.len() <= 120 {
                            output.push_str(text);
                            output.push('\n');
                            break;
                        }
                    }
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
                        if text.starts_with("\"\"\"")
                            || text.starts_with("'''")
                            || text.contains('=')
                        {
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
            "function_declaration" | "method_declaration" => {
                output.push_str(&compress_body(source, child, &["block"]));
                output.push('\n');
            }
            "package_clause" | "import_declaration" | "comment" | "type_declaration"
            | "const_declaration" | "var_declaration" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            _ => {}
        }
    }

    output.trim_end().to_string()
}

// ============================================================================
// Java Compressor
// ============================================================================

fn compress_java(source: &str, root: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "annotation_type_declaration" => {
                output.push_str(&compress_java_class(source, child));
                output.push('\n');
            }
            "package_declaration" | "import_declaration" | "line_comment" | "block_comment" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            _ => {}
        }
    }

    output.trim_end().to_string()
}

fn compress_java_class(source: &str, node: tree_sitter::Node) -> String {
    let body_kind = match node.kind() {
        "enum_declaration" => "enum_body",
        "interface_declaration" => "interface_body",
        "annotation_type_declaration" => "annotation_type_body",
        _ => "class_body",
    };

    let mut output = String::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == body_kind {
            output.push_str(source[node.start_byte()..child.start_byte()].trim_end());
            output.push_str(" {\n");

            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                match item.kind() {
                    "method_declaration" | "constructor_declaration" => {
                        push_indented(
                            &mut output,
                            "    ",
                            &compress_body(source, item, &["block", "constructor_body"]),
                        );
                    }
                    "enum_constant"
                    | "field_declaration"
                    | "constant_declaration"
                    | "line_comment"
                    | "block_comment" => {
                        push_indented(&mut output, "    ", node_text(source, item));
                    }
                    "enum_body_declarations" => {
                        // In Java enums, fields/methods are wrapped in this node
                        let mut decl_cursor = item.walk();
                        for decl in item.children(&mut decl_cursor) {
                            match decl.kind() {
                                "method_declaration" | "constructor_declaration" => {
                                    push_indented(
                                        &mut output,
                                        "    ",
                                        &compress_body(
                                            source,
                                            decl,
                                            &["block", "constructor_body"],
                                        ),
                                    );
                                }
                                "field_declaration"
                                | "constant_declaration"
                                | "line_comment"
                                | "block_comment" => {
                                    push_indented(&mut output, "    ", node_text(source, decl));
                                }
                                _ => {}
                            }
                        }
                    }
                    "class_declaration"
                    | "interface_declaration"
                    | "enum_declaration"
                    | "record_declaration" => {
                        push_indented_block(
                            &mut output,
                            "    ",
                            &compress_java_class(source, item),
                        );
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
// C# Compressor
// ============================================================================

fn compress_csharp(source: &str, root: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            "namespace_declaration" | "file_scoped_namespace_declaration" => {
                output.push_str(&compress_csharp_namespace(source, child));
                output.push('\n');
            }
            "class_declaration"
            | "interface_declaration"
            | "struct_declaration"
            | "enum_declaration"
            | "record_declaration" => {
                output.push_str(&compress_csharp_class(source, child));
                output.push('\n');
            }
            "using_directive" | "comment" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            _ => {}
        }
    }

    output.trim_end().to_string()
}

fn compress_csharp_namespace(source: &str, node: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "declaration_list" {
            output.push_str(source[node.start_byte()..child.start_byte()].trim_end());
            output.push_str(" {\n");

            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                match item.kind() {
                    "class_declaration"
                    | "interface_declaration"
                    | "struct_declaration"
                    | "enum_declaration"
                    | "record_declaration" => {
                        push_indented_block(
                            &mut output,
                            "    ",
                            &compress_csharp_class(source, item),
                        );
                    }
                    "using_directive" | "comment" => {
                        push_indented(&mut output, "    ", node_text(source, item));
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

fn compress_csharp_class(source: &str, node: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "declaration_list" {
            output.push_str(source[node.start_byte()..child.start_byte()].trim_end());
            output.push_str(" {\n");

            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                match item.kind() {
                    "method_declaration" | "constructor_declaration" => {
                        push_indented(
                            &mut output,
                            "    ",
                            &compress_body(source, item, &["block"]),
                        );
                    }
                    "property_declaration" => {
                        push_indented(
                            &mut output,
                            "    ",
                            &compress_body(source, item, &["accessor_list"]),
                        );
                    }
                    "field_declaration"
                    | "event_declaration"
                    | "event_field_declaration"
                    | "comment" => {
                        push_indented(&mut output, "    ", node_text(source, item));
                    }
                    "class_declaration"
                    | "interface_declaration"
                    | "struct_declaration"
                    | "enum_declaration"
                    | "record_declaration" => {
                        push_indented_block(
                            &mut output,
                            "    ",
                            &compress_csharp_class(source, item),
                        );
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
// C Compressor
// ============================================================================

fn compress_c(source: &str, root: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                output.push_str(&compress_body(source, child, &["compound_statement"]));
                output.push('\n');
            }
            "preproc_include"
            | "preproc_def"
            | "preproc_ifdef"
            | "preproc_if"
            | "preproc_ifndef"
            | "preproc_function_def"
            | "preproc_call"
            | "comment"
            | "declaration"
            | "type_definition"
            | "struct_specifier"
            | "enum_specifier"
            | "union_specifier" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            _ => {}
        }
    }

    output.trim_end().to_string()
}

// ============================================================================
// C++ Compressor
// ============================================================================

fn compress_cpp(source: &str, root: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                output.push_str(&compress_body(source, child, &["compound_statement"]));
                output.push('\n');
            }
            "class_specifier" => {
                output.push_str(&compress_cpp_class(source, child));
                output.push('\n');
            }
            "namespace_definition" => {
                output.push_str(&compress_cpp_namespace(source, child));
                output.push('\n');
            }
            "template_declaration" => {
                output.push_str(&compress_cpp_template(source, child));
                output.push('\n');
            }
            "linkage_specification" => {
                output.push_str(&compress_cpp_linkage(source, child));
                output.push('\n');
            }
            "preproc_include"
            | "preproc_def"
            | "preproc_ifdef"
            | "preproc_if"
            | "preproc_ifndef"
            | "preproc_function_def"
            | "preproc_call"
            | "comment"
            | "declaration"
            | "type_definition"
            | "using_declaration"
            | "alias_declaration"
            | "struct_specifier"
            | "enum_specifier"
            | "union_specifier" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            _ => {}
        }
    }

    output.trim_end().to_string()
}

fn compress_cpp_class(source: &str, node: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "field_declaration_list" {
            output.push_str(source[node.start_byte()..child.start_byte()].trim_end());
            output.push_str(" {\n");

            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                match item.kind() {
                    "function_definition" => {
                        push_indented(
                            &mut output,
                            "    ",
                            &compress_body(source, item, &["compound_statement"]),
                        );
                    }
                    "template_declaration" => {
                        push_indented_block(
                            &mut output,
                            "    ",
                            &compress_cpp_template(source, item),
                        );
                    }
                    "field_declaration" | "declaration" | "using_declaration"
                    | "alias_declaration" | "type_definition" | "access_specifier"
                    | "friend_declaration" | "preproc_ifdef" | "preproc_if" | "preproc_ifndef"
                    | "preproc_def" | "preproc_call" | "comment" => {
                        push_indented(&mut output, "    ", node_text(source, item));
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

fn compress_cpp_namespace(source: &str, node: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "declaration_list" {
            output.push_str(source[node.start_byte()..child.start_byte()].trim_end());
            output.push_str(" {\n");

            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                match item.kind() {
                    "function_definition" => {
                        push_indented(
                            &mut output,
                            "    ",
                            &compress_body(source, item, &["compound_statement"]),
                        );
                    }
                    "class_specifier" => {
                        push_indented_block(&mut output, "    ", &compress_cpp_class(source, item));
                    }
                    "template_declaration" => {
                        push_indented_block(
                            &mut output,
                            "    ",
                            &compress_cpp_template(source, item),
                        );
                    }
                    "namespace_definition" => {
                        push_indented_block(
                            &mut output,
                            "    ",
                            &compress_cpp_namespace(source, item),
                        );
                    }
                    "struct_specifier" | "enum_specifier" | "union_specifier" | "declaration"
                    | "type_definition" | "using_declaration" | "alias_declaration"
                    | "preproc_ifdef" | "preproc_if" | "preproc_ifndef" | "preproc_def"
                    | "preproc_call" | "comment" => {
                        push_indented(&mut output, "    ", node_text(source, item));
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

fn compress_cpp_template(source: &str, node: tree_sitter::Node) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let prefix = source[node.start_byte()..child.start_byte()].trim_end();
        match child.kind() {
            "function_definition" => {
                return format!(
                    "{}\n{}",
                    prefix,
                    compress_body(source, child, &["compound_statement"])
                );
            }
            "class_specifier" => {
                return format!("{}\n{}", prefix, compress_cpp_class(source, child));
            }
            "declaration" => {
                return format!("{}\n{}", prefix, node_text(source, child));
            }
            _ => {}
        }
    }
    node_text(source, node).to_string()
}

fn compress_cpp_linkage(source: &str, node: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "declaration_list" {
            output.push_str(source[node.start_byte()..child.start_byte()].trim_end());
            output.push_str(" {\n");

            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                match item.kind() {
                    "function_definition" => {
                        push_indented(
                            &mut output,
                            "    ",
                            &compress_body(source, item, &["compound_statement"]),
                        );
                    }
                    "declaration" | "comment" => {
                        push_indented(&mut output, "    ", node_text(source, item));
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
// Ruby Compressor
// ============================================================================

fn compress_ruby(source: &str, root: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            "comment" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            "call" => {
                let text = node_text(source, child);
                if text.starts_with("require") {
                    output.push_str(text);
                    output.push('\n');
                }
            }
            "method" | "singleton_method" => {
                output.push_str(&compress_ruby_method(source, child));
                output.push('\n');
            }
            "class" | "module" => {
                output.push_str(&compress_ruby_class(source, child));
                output.push('\n');
            }
            "assignment" => {
                let text = node_text(source, child);
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

fn compress_ruby_method(source: &str, node: tree_sitter::Node) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "body_statement" {
            let sig = source[node.start_byte()..child.start_byte()].trim_end();
            return format!("{}\n  ...\nend", sig);
        }
    }
    node_text(source, node).to_string()
}

fn compress_ruby_class(source: &str, node: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "body_statement" {
            let header = source[node.start_byte()..child.start_byte()].trim_end();
            output.push_str(header);
            output.push('\n');

            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                match item.kind() {
                    "method" | "singleton_method" => {
                        push_indented_block(&mut output, "  ", &compress_ruby_method(source, item));
                    }
                    "class" | "module" => {
                        push_indented_block(&mut output, "  ", &compress_ruby_class(source, item));
                    }
                    "comment" => {
                        push_indented(&mut output, "  ", node_text(source, item));
                    }
                    "call" | "assignment" => {
                        let text = node_text(source, item);
                        if text.len() <= 120 {
                            push_indented(&mut output, "  ", text);
                        }
                    }
                    _ => {}
                }
            }

            output.push_str("end");
            return output;
        }
    }

    node_text(source, node).to_string()
}

// ============================================================================
// PHP Compressor
// ============================================================================

fn compress_php(source: &str, root: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                output.push_str(&compress_body(source, child, &["compound_statement"]));
                output.push('\n');
            }
            "namespace_definition" => {
                output.push_str(&compress_php_namespace(source, child));
                output.push('\n');
            }
            "class_declaration"
            | "interface_declaration"
            | "trait_declaration"
            | "enum_declaration" => {
                output.push_str(&compress_php_class(source, child));
                output.push('\n');
            }
            "php_tag" | "namespace_use_declaration" | "const_declaration" | "comment" => {
                output.push_str(node_text(source, child));
                output.push('\n');
            }
            _ => {}
        }
    }

    output.trim_end().to_string()
}

fn compress_php_namespace(source: &str, node: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "compound_statement" || child.kind() == "declaration_list" {
            output.push_str(source[node.start_byte()..child.start_byte()].trim_end());
            output.push_str(" {\n");

            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                match item.kind() {
                    "class_declaration"
                    | "interface_declaration"
                    | "trait_declaration"
                    | "enum_declaration" => {
                        push_indented_block(&mut output, "    ", &compress_php_class(source, item));
                    }
                    "function_definition" => {
                        push_indented(
                            &mut output,
                            "    ",
                            &compress_body(source, item, &["compound_statement"]),
                        );
                    }
                    "namespace_use_declaration" | "const_declaration" | "comment" => {
                        push_indented(&mut output, "    ", node_text(source, item));
                    }
                    _ => {}
                }
            }
            output.push('}');
            return output;
        }
    }

    // Statement form: namespace Foo;
    node_text(source, node).to_string()
}

fn compress_php_class(source: &str, node: tree_sitter::Node) -> String {
    let mut output = String::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "declaration_list" || child.kind() == "enum_declaration_list" {
            output.push_str(source[node.start_byte()..child.start_byte()].trim_end());
            output.push_str(" {\n");

            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                match item.kind() {
                    "method_declaration" => {
                        push_indented(
                            &mut output,
                            "    ",
                            &compress_body(source, item, &["compound_statement"]),
                        );
                    }
                    "property_declaration"
                    | "const_declaration"
                    | "use_declaration"
                    | "enum_case"
                    | "comment" => {
                        push_indented(&mut output, "    ", node_text(source, item));
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
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Language detection tests
    #[test]
    fn test_language_for_extension() {
        assert_eq!(language_for_extension("rs"), Some(CompressLanguage::Rust));
        assert_eq!(
            language_for_extension("ts"),
            Some(CompressLanguage::TypeScript)
        );
        assert_eq!(language_for_extension("tsx"), Some(CompressLanguage::Tsx));
        assert_eq!(
            language_for_extension("js"),
            Some(CompressLanguage::JavaScript)
        );
        assert_eq!(language_for_extension("jsx"), Some(CompressLanguage::Jsx));
        assert_eq!(language_for_extension("py"), Some(CompressLanguage::Python));
        assert_eq!(language_for_extension("go"), Some(CompressLanguage::Go));
        assert_eq!(language_for_extension("md"), None);
        assert_eq!(language_for_extension("toml"), None);
    }

    #[test]
    fn test_language_for_path() {
        assert_eq!(
            language_for_path(Path::new("main.rs")),
            Some(CompressLanguage::Rust)
        );
        assert_eq!(
            language_for_path(Path::new("foo.test.ts")),
            Some(CompressLanguage::TypeScript)
        );
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
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
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
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
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
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
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
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
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
            CompressResult::Fallback(_, _) => {
                panic!("Comments-only should compress (keeping comments)")
            }
        }
    }

    #[test]
    fn test_compress_typescript_export_function() {
        let source = r#"import { Config } from './config';

export function processData(data: string[]): number {
    const filtered = data.filter(x => x.length > 0);
    return filtered.length;
}"#;
        match compress_source(source, CompressLanguage::TypeScript) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("import { Config }"));
                assert!(
                    output.contains("export function processData(data: string[]): number { ... }"),
                    "export function should be compressed, got: {}",
                    output
                );
                assert!(
                    !output.contains("filtered"),
                    "export function body should be stripped"
                );
            }
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
        }
    }

    #[test]
    fn test_compress_typescript_export_class() {
        let source = r#"export class UserService {
    private db: Database;

    constructor(db: Database) {
        this.db = db;
    }

    async getUser(id: string): Promise<User> {
        const user = await this.db.find(id);
        return user;
    }
}"#;
        match compress_source(source, CompressLanguage::TypeScript) {
            CompressResult::Compressed(output) => {
                assert!(
                    output.contains("export class UserService"),
                    "export class should be preserved"
                );
                assert!(
                    output.contains("{ ... }"),
                    "method bodies should be compressed"
                );
                assert!(
                    !output.contains("await this.db.find"),
                    "method body should be stripped"
                );
            }
            CompressResult::Fallback(_, _) => panic!("Expected compression"),
        }
    }

    #[test]
    fn test_compress_python_module_constant() {
        let source = "MAX_RETRIES = 3\nDEBUG = True\n\ndef run():\n    print('running')\n";
        match compress_source(source, CompressLanguage::Python) {
            CompressResult::Compressed(output) => {
                assert!(
                    output.contains("MAX_RETRIES = 3"),
                    "Module-level constant should be preserved, got: {}",
                    output
                );
                assert!(
                    output.contains("DEBUG = True"),
                    "Module-level boolean constant should be preserved"
                );
                assert!(output.contains("def run():"));
                assert!(
                    !output.contains("print('running')"),
                    "Function body should be stripped"
                );
            }
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
        }
    }

    // Java compression tests
    #[test]
    fn test_compress_java_class_with_methods() {
        let source = r#"package com.example;

import java.util.List;

public class UserService {
    private final Database db;

    public UserService(Database db) {
        this.db = db;
    }

    public User getUser(String id) {
        User user = db.find(id);
        if (user == null) {
            throw new RuntimeException("Not found");
        }
        return user;
    }

    public List<User> listUsers() {
        return db.findAll();
    }
}"#;
        match compress_source(source, CompressLanguage::Java) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("package com.example;"));
                assert!(output.contains("import java.util.List;"));
                assert!(output.contains("public class UserService"));
                assert!(output.contains("private final Database db;"));
                assert!(output.contains("public UserService(Database db) { ... }"));
                assert!(output.contains("public User getUser(String id) { ... }"));
                assert!(output.contains("public List<User> listUsers() { ... }"));
                assert!(!output.contains("throw new RuntimeException"));
            }
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
        }
    }

    #[test]
    fn test_compress_java_interface() {
        let source = r#"public interface Repository<T> {
    T findById(String id);
    List<T> findAll();
    void save(T entity);
}"#;
        match compress_source(source, CompressLanguage::Java) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("public interface Repository<T>"));
                assert!(output.contains("T findById(String id);"));
                assert!(output.contains("void save(T entity);"));
            }
            CompressResult::Fallback(_, _) => panic!("Expected compression"),
        }
    }

    // C# compression tests
    #[test]
    fn test_compress_csharp_class_with_methods() {
        let source = r#"using System;
using System.Collections.Generic;

namespace MyApp.Services
{
    public class UserService
    {
        private readonly IDatabase _db;

        public UserService(IDatabase db)
        {
            _db = db;
        }

        public User GetUser(string id)
        {
            var user = _db.Find(id);
            if (user == null)
                throw new Exception("Not found");
            return user;
        }
    }
}"#;
        match compress_source(source, CompressLanguage::CSharp) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("using System;"));
                assert!(output.contains("namespace MyApp.Services"));
                assert!(output.contains("public class UserService"));
                assert!(output.contains("public UserService(IDatabase db) { ... }"));
                assert!(output.contains("public User GetUser(string id) { ... }"));
                assert!(!output.contains("throw new Exception"));
            }
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
        }
    }

    #[test]
    fn test_compress_csharp_interface() {
        let source = r#"public interface IRepository<T>
{
    T FindById(string id);
    IList<T> FindAll();
    void Save(T entity);
}"#;
        match compress_source(source, CompressLanguage::CSharp) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("public interface IRepository<T>"));
                assert!(output.contains("T FindById(string id);"));
            }
            CompressResult::Fallback(_, _) => panic!("Expected compression"),
        }
    }

    // C compression tests
    #[test]
    fn test_compress_c_function() {
        let source = r#"#include <stdio.h>
#include <stdlib.h>

#define MAX_SIZE 1024

typedef struct {
    int x;
    int y;
} Point;

int process_data(const char *input, int length) {
    char *buffer = malloc(length);
    if (!buffer) return -1;
    memcpy(buffer, input, length);
    int result = compute(buffer, length);
    free(buffer);
    return result;
}"#;
        match compress_source(source, CompressLanguage::C) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("#include <stdio.h>"));
                assert!(output.contains("#define MAX_SIZE 1024"));
                assert!(output.contains("typedef struct"));
                assert!(output.contains("int process_data(const char *input, int length) { ... }"));
                assert!(!output.contains("malloc"));
            }
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
        }
    }

    #[test]
    fn test_compress_c_header() {
        let source = r#"#ifndef MYLIB_H
#define MYLIB_H

typedef struct Node {
    int value;
    struct Node *next;
} Node;

int process(const char *input);
void cleanup(Node *head);

#endif"#;
        match compress_source(source, CompressLanguage::C) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("#ifndef MYLIB_H"));
                assert!(output.contains("typedef struct Node"));
                assert!(output.contains("int process(const char *input);"));
            }
            CompressResult::Fallback(_, _) => panic!("Expected compression"),
        }
    }

    // C++ compression tests
    #[test]
    fn test_compress_cpp_class() {
        let source = r#"#include <string>
#include <vector>

namespace mylib {

class UserService {
public:
    UserService(Database& db) : db_(db) {
        initialized_ = true;
    }

    User getUser(const std::string& id) {
        auto user = db_.find(id);
        if (!user) throw std::runtime_error("not found");
        return *user;
    }

private:
    Database& db_;
    bool initialized_;
};

}"#;
        match compress_source(source, CompressLanguage::Cpp) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("#include <string>"));
                assert!(output.contains("namespace mylib"));
                assert!(output.contains("class UserService"));
                assert!(output.contains("{ ... }"));
                assert!(!output.contains("throw std::runtime_error"));
            }
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
        }
    }

    #[test]
    fn test_compress_cpp_template_function() {
        let source = r#"template<typename T>
T max_value(T a, T b) {
    return (a > b) ? a : b;
}"#;
        match compress_source(source, CompressLanguage::Cpp) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("template<typename T>"));
                assert!(output.contains("T max_value(T a, T b) { ... }"));
                assert!(!output.contains("return"));
            }
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
        }
    }

    // Ruby compression tests
    #[test]
    fn test_compress_ruby_class() {
        let source = r#"require 'json'

class UserService
  attr_reader :db

  def initialize(db)
    @db = db
    @cache = {}
  end

  def find_user(id)
    return @cache[id] if @cache.key?(id)
    user = @db.find(id)
    @cache[id] = user
    user
  end
end"#;
        match compress_source(source, CompressLanguage::Ruby) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("require 'json'"));
                assert!(output.contains("class UserService"));
                assert!(output.contains("attr_reader :db"));
                assert!(output.contains("def initialize(db)"));
                assert!(output.contains("..."));
                assert!(output.contains("def find_user(id)"));
                assert!(!output.contains("@cache[id] = user"));
                assert!(output.contains("end"));
            }
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
        }
    }

    #[test]
    fn test_compress_ruby_module() {
        let source = r#"module Validators
  def self.validate_email(email)
    email.match?(/\A[\w+\-.]+@[a-z\d\-]+(\.[a-z]+)*\.[a-z]+\z/i)
  end

  def self.validate_name(name)
    name.length >= 2 && name.length <= 100
  end
end"#;
        match compress_source(source, CompressLanguage::Ruby) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("module Validators"));
                assert!(output.contains("def self.validate_email(email)"));
                assert!(output.contains("def self.validate_name(name)"));
                assert!(!output.contains("match?"));
            }
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
        }
    }

    // PHP compression tests
    #[test]
    fn test_compress_php_class() {
        let source = r#"<?php

namespace App\Services;

use App\Models\User;

class UserService
{
    private $db;

    public function __construct(Database $db)
    {
        $this->db = $db;
    }

    public function getUser(string $id): User
    {
        $user = $this->db->find($id);
        if (!$user) {
            throw new \Exception('Not found');
        }
        return $user;
    }
}"#;
        match compress_source(source, CompressLanguage::Php) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("<?php"));
                assert!(output.contains("namespace App\\Services;"));
                assert!(output.contains("use App\\Models\\User;"));
                assert!(output.contains("class UserService"));
                assert!(output.contains("public function __construct(Database $db) { ... }"));
                assert!(output.contains("public function getUser(string $id): User { ... }"));
                assert!(!output.contains("throw new"));
            }
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
        }
    }

    #[test]
    fn test_compress_php_function() {
        let source = r#"<?php

function processData(array $items): int
{
    $count = 0;
    foreach ($items as $item) {
        if ($item->isValid()) {
            $count++;
        }
    }
    return $count;
}"#;
        match compress_source(source, CompressLanguage::Php) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("<?php"));
                assert!(output.contains("function processData(array $items): int { ... }"));
                assert!(!output.contains("foreach"));
            }
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
        }
    }

    // Extension mapping tests for new languages
    #[test]
    fn test_language_for_extension_new_languages() {
        assert_eq!(language_for_extension("java"), Some(CompressLanguage::Java));
        assert_eq!(language_for_extension("cs"), Some(CompressLanguage::CSharp));
        assert_eq!(language_for_extension("c"), Some(CompressLanguage::C));
        assert_eq!(language_for_extension("h"), Some(CompressLanguage::C));
        assert_eq!(language_for_extension("cpp"), Some(CompressLanguage::Cpp));
        assert_eq!(language_for_extension("cc"), Some(CompressLanguage::Cpp));
        assert_eq!(language_for_extension("cxx"), Some(CompressLanguage::Cpp));
        assert_eq!(language_for_extension("hpp"), Some(CompressLanguage::Cpp));
        assert_eq!(language_for_extension("hh"), Some(CompressLanguage::Cpp));
        assert_eq!(language_for_extension("hxx"), Some(CompressLanguage::Cpp));
        assert_eq!(language_for_extension("rb"), Some(CompressLanguage::Ruby));
        assert_eq!(language_for_extension("php"), Some(CompressLanguage::Php));
    }

    // Edge case tests found during QA review
    #[test]
    fn test_compress_java_enum_with_constants() {
        let source = r#"public enum Color {
    RED("red"),
    GREEN("green"),
    BLUE("blue");

    private final String code;

    Color(String code) {
        this.code = code;
    }

    public String getCode() {
        return this.code;
    }
}"#;
        match compress_source(source, CompressLanguage::Java) {
            CompressResult::Compressed(output) => {
                assert!(
                    output.contains("RED(\"red\")"),
                    "Enum constant RED should be preserved, got: {}",
                    output
                );
                assert!(
                    output.contains("GREEN(\"green\")"),
                    "Enum constant GREEN should be preserved"
                );
                assert!(
                    output.contains("BLUE(\"blue\")"),
                    "Enum constant BLUE should be preserved"
                );
                assert!(
                    output.contains("private final String code;"),
                    "Enum field should be preserved"
                );
                assert!(
                    output.contains("Color(String code) { ... }"),
                    "Enum constructor should be compressed, got: {}",
                    output
                );
                assert!(
                    output.contains("public String getCode() { ... }"),
                    "Enum method should be compressed, got: {}",
                    output
                );
                assert!(
                    !output.contains("return this.code"),
                    "Method body should be stripped"
                );
            }
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
        }
    }

    #[test]
    fn test_compress_php_enum_with_cases() {
        let source = r#"<?php

enum Suit: string
{
    case Hearts = 'H';
    case Diamonds = 'D';
    case Clubs = 'C';
    case Spades = 'S';

    public function color(): string
    {
        return match($this) {
            self::Hearts, self::Diamonds => 'red',
            self::Clubs, self::Spades => 'black',
        };
    }
}"#;
        match compress_source(source, CompressLanguage::Php) {
            CompressResult::Compressed(output) => {
                assert!(
                    output.contains("case Hearts = 'H';"),
                    "Enum case should be preserved, got: {}",
                    output
                );
                assert!(
                    output.contains("case Spades = 'S';"),
                    "Enum case should be preserved"
                );
                assert!(
                    output.contains("public function color(): string { ... }"),
                    "Enum method should be compressed, got: {}",
                    output
                );
                assert!(!output.contains("match("), "Method body should be stripped");
            }
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
        }
    }

    #[test]
    fn test_compress_cpp_class_with_preproc() {
        let source = r#"class Config {
public:
    Config() {}

    std::string getName() const {
        return name_;
    }

#ifdef DEBUG
    void debugPrint() {
        std::cout << name_ << std::endl;
    }
#endif

private:
    std::string name_;
};"#;
        match compress_source(source, CompressLanguage::Cpp) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("class Config"));
                assert!(
                    output.contains("#ifdef DEBUG"),
                    "Preprocessor directive inside class should be preserved, got: {}",
                    output
                );
                assert!(
                    output.contains("#endif"),
                    "Preprocessor endif should be preserved"
                );
                assert!(output.contains("std::string name_;"));
            }
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
        }
    }

    #[test]
    fn test_compress_csharp_property() {
        let source = r#"public class Person
{
    public string Name { get; set; }
    public int Age { get; set; }

    public string GetGreeting()
    {
        return $"Hello, {Name}!";
    }
}"#;
        match compress_source(source, CompressLanguage::CSharp) {
            CompressResult::Compressed(output) => {
                assert!(output.contains("public class Person"));
                assert!(
                    output.contains("Name"),
                    "Property name should be preserved, got: {}",
                    output
                );
                assert!(output.contains("Age"), "Property name should be preserved");
                assert!(output.contains("public string GetGreeting() { ... }"));
                assert!(!output.contains("Hello, {Name}"));
            }
            CompressResult::Fallback(_, reason) => {
                panic!("Expected compression, got fallback: {:?}", reason)
            }
        }
    }

    #[test]
    fn test_compress_rust_syntax_error_fallback() {
        // Source with syntax errors should fall back to full content
        let source = "fn broken( {\n    this is not valid rust\n}\n";
        match compress_source(source, CompressLanguage::Rust) {
            CompressResult::Compressed(_) => {
                panic!("Syntax error should produce fallback, not compressed")
            }
            CompressResult::Fallback(content, reason) => {
                assert_eq!(content, source, "Fallback should return original content");
                assert!(reason.is_some(), "Fallback should include a warning reason");
                assert!(
                    reason.unwrap().contains("ERROR"),
                    "Reason should mention ERROR nodes"
                );
            }
        }
    }
}
