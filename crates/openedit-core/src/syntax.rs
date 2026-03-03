use std::collections::HashMap;
use tree_sitter::Language;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

/// A symbol extracted from source code via tree-sitter AST analysis.
#[derive(Debug, Clone)]
pub struct Symbol {
    /// The name of the symbol (e.g., function name, struct name).
    pub name: String,
    /// The kind of symbol.
    pub kind: SymbolKind,
    /// 0-based line number where the symbol is defined.
    pub line: usize,
}

/// The kind of a symbol extracted from source code.
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Module,
    Constant,
    Variable,
    Other,
}

impl SymbolKind {
    /// Return a short label prefix for display in the symbol list.
    pub fn label(&self) -> &'static str {
        match self {
            SymbolKind::Function => "fn",
            SymbolKind::Method => "method",
            SymbolKind::Class => "class",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Interface => "trait",
            SymbolKind::Module => "mod",
            SymbolKind::Constant => "const",
            SymbolKind::Variable => "var",
            SymbolKind::Other => "sym",
        }
    }
}

/// Standard highlight capture names we recognize.
/// The index in this array becomes the `Highlight(usize)` value returned by tree-sitter.
pub const HIGHLIGHT_NAMES: &[&str] = &[
    "attribute",             // 0
    "boolean",               // 1
    "comment",               // 2
    "constant",              // 3
    "constant.builtin",      // 4
    "constructor",           // 5
    "embedded",              // 6
    "escape",                // 7
    "function",              // 8
    "function.builtin",      // 9
    "function.macro",        // 10
    "keyword",               // 11
    "label",                 // 12
    "module",                // 13
    "number",                // 14
    "operator",              // 15
    "property",              // 16
    "punctuation",           // 17
    "punctuation.bracket",   // 18
    "punctuation.delimiter", // 19
    "string",                // 20
    "string.escape",         // 21
    "string.special",        // 22
    "tag",                   // 23
    "type",                  // 24
    "type.builtin",          // 25
    "variable",              // 26
    "variable.builtin",      // 27
    "variable.parameter",    // 28
];

/// Custom highlight query for Kotlin (tree-sitter-kotlin-ng doesn't ship queries).
const KOTLIN_HIGHLIGHTS: &str = r#"
(line_comment) @comment
(block_comment) @comment

(string_literal) @string
(multiline_string_literal) @string
(character_literal) @string

(number_literal) @number
(float_literal) @number

(identifier) @variable

["fun" "val" "var" "class" "interface" "object" "package" "import"
 "if" "else" "when" "for" "while" "do" "return" "break" "continue"
 "throw" "try" "catch" "finally" "is" "as" "in"
 "override" "abstract" "open" "sealed" "data" "enum" "companion"
 "private" "protected" "public" "internal"
 "suspend" "inline" "constructor" "init" "typealias"
 "null" "true" "false"
] @keyword

["(" ")" "[" "]" "{" "}"] @punctuation.bracket
["." "," ";" ":"] @punctuation.delimiter
"#;

/// A span of highlighted text within a line.
#[derive(Debug, Clone)]
pub struct HighlightSpan {
    /// Start column (char index within the line).
    pub start_col: usize,
    /// End column (exclusive, char index).
    pub end_col: usize,
    /// Index into HIGHLIGHT_NAMES.
    pub highlight_idx: usize,
}

/// Manages syntax highlighting for all supported languages.
pub struct SyntaxEngine {
    configs: HashMap<String, HighlightConfiguration>,
    /// Tree-sitter Language objects for parsing (symbol extraction).
    languages: HashMap<String, Language>,
    highlighter: Highlighter,
}

impl SyntaxEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            configs: HashMap::new(),
            languages: HashMap::new(),
            highlighter: Highlighter::new(),
        };
        engine.register_languages();
        engine
    }

    fn register_languages(&mut self) {
        // Rust
        self.register(
            "rust",
            tree_sitter_rust::LANGUAGE.into(),
            tree_sitter_rust::HIGHLIGHTS_QUERY,
            tree_sitter_rust::INJECTIONS_QUERY,
            "",
        );

        // Python
        self.register(
            "python",
            tree_sitter_python::LANGUAGE.into(),
            tree_sitter_python::HIGHLIGHTS_QUERY,
            "",
            "",
        );

        // JavaScript (uses HIGHLIGHT_QUERY, not HIGHLIGHTS_QUERY)
        self.register(
            "javascript",
            tree_sitter_javascript::LANGUAGE.into(),
            tree_sitter_javascript::HIGHLIGHT_QUERY,
            tree_sitter_javascript::INJECTIONS_QUERY,
            tree_sitter_javascript::LOCALS_QUERY,
        );

        // JSON
        self.register(
            "json",
            tree_sitter_json::LANGUAGE.into(),
            tree_sitter_json::HIGHLIGHTS_QUERY,
            "",
            "",
        );

        // C (uses HIGHLIGHT_QUERY)
        self.register(
            "c",
            tree_sitter_c::LANGUAGE.into(),
            tree_sitter_c::HIGHLIGHT_QUERY,
            "",
            "",
        );

        // Go
        self.register(
            "go",
            tree_sitter_go::LANGUAGE.into(),
            tree_sitter_go::HIGHLIGHTS_QUERY,
            "",
            "",
        );

        // TypeScript
        self.register(
            "typescript",
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            tree_sitter_typescript::HIGHLIGHTS_QUERY,
            "",
            tree_sitter_typescript::LOCALS_QUERY,
        );

        // TSX
        self.register(
            "tsx",
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            tree_sitter_typescript::HIGHLIGHTS_QUERY,
            "",
            tree_sitter_typescript::LOCALS_QUERY,
        );

        // Bash (uses HIGHLIGHT_QUERY)
        self.register(
            "bash",
            tree_sitter_bash::LANGUAGE.into(),
            tree_sitter_bash::HIGHLIGHT_QUERY,
            "",
            "",
        );

        // CSS
        self.register(
            "css",
            tree_sitter_css::LANGUAGE.into(),
            tree_sitter_css::HIGHLIGHTS_QUERY,
            "",
            "",
        );

        // HTML
        self.register(
            "html",
            tree_sitter_html::LANGUAGE.into(),
            tree_sitter_html::HIGHLIGHTS_QUERY,
            tree_sitter_html::INJECTIONS_QUERY,
            "",
        );

        // Markdown (block grammar)
        self.register(
            "markdown",
            tree_sitter_md::LANGUAGE.into(),
            tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
            tree_sitter_md::INJECTION_QUERY_BLOCK,
            "",
        );

        // C++ (query extends the C query, so concatenate both)
        {
            let cpp_query = format!(
                "{}\n{}",
                tree_sitter_c::HIGHLIGHT_QUERY,
                tree_sitter_cpp::HIGHLIGHT_QUERY
            );
            self.register("cpp", tree_sitter_cpp::LANGUAGE.into(), &cpp_query, "", "");
        }

        // Java
        self.register(
            "java",
            tree_sitter_java::LANGUAGE.into(),
            tree_sitter_java::HIGHLIGHTS_QUERY,
            "",
            "",
        );

        // PHP
        self.register(
            "php",
            tree_sitter_php::LANGUAGE_PHP.into(),
            tree_sitter_php::HIGHLIGHTS_QUERY,
            tree_sitter_php::INJECTIONS_QUERY,
            "",
        );

        // Ruby
        self.register(
            "ruby",
            tree_sitter_ruby::LANGUAGE.into(),
            tree_sitter_ruby::HIGHLIGHTS_QUERY,
            "",
            tree_sitter_ruby::LOCALS_QUERY,
        );

        // Swift
        self.register(
            "swift",
            tree_sitter_swift::LANGUAGE.into(),
            tree_sitter_swift::HIGHLIGHTS_QUERY,
            "",
            tree_sitter_swift::LOCALS_QUERY,
        );

        // Lua
        self.register(
            "lua",
            tree_sitter_lua::LANGUAGE.into(),
            tree_sitter_lua::HIGHLIGHTS_QUERY,
            tree_sitter_lua::INJECTIONS_QUERY,
            tree_sitter_lua::LOCALS_QUERY,
        );

        // YAML
        self.register(
            "yaml",
            tree_sitter_yaml::LANGUAGE.into(),
            tree_sitter_yaml::HIGHLIGHTS_QUERY,
            "",
            "",
        );

        // TOML
        self.register(
            "toml",
            tree_sitter_toml_ng::LANGUAGE.into(),
            tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
            "",
            "",
        );

        // Haskell
        self.register(
            "haskell",
            tree_sitter_haskell::LANGUAGE.into(),
            tree_sitter_haskell::HIGHLIGHTS_QUERY,
            "",
            tree_sitter_haskell::LOCALS_QUERY,
        );

        // Scala
        self.register(
            "scala",
            tree_sitter_scala::LANGUAGE.into(),
            tree_sitter_scala::HIGHLIGHTS_QUERY,
            "",
            tree_sitter_scala::LOCALS_QUERY,
        );

        // Kotlin (grammar may not be compatible with current tree-sitter ABI)
        if let Ok(lang) = std::panic::catch_unwind(|| {
            let l: Language = tree_sitter_kotlin_ng::LANGUAGE.into();
            l
        }) {
            self.register("kotlin", lang, KOTLIN_HIGHLIGHTS, "", "");
        }

        // SQL
        self.register(
            "sql",
            tree_sitter_sequel::LANGUAGE.into(),
            tree_sitter_sequel::HIGHLIGHTS_QUERY,
            "",
            "",
        );
    }

    fn register(
        &mut self,
        name: &str,
        language: Language,
        highlights_query: &str,
        injection_query: &str,
        locals_query: &str,
    ) {
        self.languages.insert(name.to_string(), language.clone());
        match HighlightConfiguration::new(
            language,
            name,
            highlights_query,
            injection_query,
            locals_query,
        ) {
            Ok(mut config) => {
                config.configure(HIGHLIGHT_NAMES);
                self.configs.insert(name.to_string(), config);
            }
            Err(e) => {
                log::warn!("Failed to register syntax for {}: {}", name, e);
            }
        }
    }

    /// Extract symbols (functions, structs, classes, etc.) from source code.
    ///
    /// Uses tree-sitter to parse the source and walk the AST, collecting
    /// relevant symbol definitions. Returns an empty Vec for unknown languages.
    pub fn extract_symbols(&mut self, source: &str, language_key: &str) -> Vec<Symbol> {
        let language = match self.languages.get(language_key) {
            Some(lang) => lang.clone(),
            None => return Vec::new(),
        };

        let mut parser = tree_sitter::Parser::new();
        if parser.set_language(&language).is_err() {
            return Vec::new();
        }

        let tree = match parser.parse(source.as_bytes(), None) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut symbols = Vec::new();
        let source_bytes = source.as_bytes();
        collect_symbols(tree.root_node(), source_bytes, language_key, &mut symbols);
        symbols
    }

    /// Map a display language name (from file detection) to our internal key.
    pub fn language_key(display_name: &str) -> Option<&'static str> {
        match display_name {
            "Rust" => Some("rust"),
            "Python" => Some("python"),
            "JavaScript" | "JSX" => Some("javascript"),
            "TypeScript" => Some("typescript"),
            "TSX" => Some("tsx"),
            "JSON" => Some("json"),
            "C" => Some("c"),
            "C++" => Some("cpp"),
            "Go" => Some("go"),
            "Java" => Some("java"),
            "PHP" => Some("php"),
            "Ruby" => Some("ruby"),
            "Swift" => Some("swift"),
            "Kotlin" => Some("kotlin"),
            "Lua" => Some("lua"),
            "YAML" => Some("yaml"),
            "TOML" => Some("toml"),
            "SQL" => Some("sql"),
            "Haskell" => Some("haskell"),
            "Scala" => Some("scala"),
            "Bash" => Some("bash"),
            "CSS" | "SCSS" => Some("css"),
            "HTML" => Some("html"),
            "Markdown" => Some("markdown"),
            _ => None,
        }
    }

    /// Check if we have a config for this language key.
    pub fn has_language(&self, language_key: &str) -> bool {
        self.configs.contains_key(language_key)
    }

    /// Highlight source text, returning per-line spans.
    /// `language_key` should come from `language_key()`, not the display name.
    pub fn highlight_lines(&mut self, source: &str, language_key: &str) -> Vec<Vec<HighlightSpan>> {
        let config = match self.configs.get(language_key) {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Pre-compute line start byte offsets
        let line_starts: Vec<usize> = std::iter::once(0)
            .chain(source.bytes().enumerate().filter_map(|(i, b)| {
                if b == b'\n' {
                    Some(i + 1)
                } else {
                    None
                }
            }))
            .collect();
        let num_lines = line_starts.len();
        let mut result: Vec<Vec<HighlightSpan>> = vec![Vec::new(); num_lines];

        let events = match self.highlighter.highlight(
            config,
            source.as_bytes(),
            None,
            |_| None, // No injection support for now
        ) {
            Ok(events) => events,
            Err(e) => {
                log::debug!("Highlighting failed for {}: {:?}", language_key, e);
                return result;
            }
        };

        let mut style_stack: Vec<usize> = Vec::new();

        for event in events {
            match event {
                Ok(HighlightEvent::HighlightStart(h)) => {
                    style_stack.push(h.0);
                }
                Ok(HighlightEvent::HighlightEnd) => {
                    style_stack.pop();
                }
                Ok(HighlightEvent::Source { start, end }) => {
                    if let Some(&highlight_idx) = style_stack.last() {
                        add_spans_for_byte_range(
                            source,
                            start,
                            end,
                            highlight_idx,
                            &line_starts,
                            &mut result,
                        );
                    }
                }
                Err(_) => break,
            }
        }

        result
    }
}

impl Default for SyntaxEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Recursively collect symbols from a tree-sitter AST node.
fn collect_symbols(
    node: tree_sitter::Node,
    source: &[u8],
    language_key: &str,
    symbols: &mut Vec<Symbol>,
) {
    let kind = node.kind();

    if let Some(symbol_kind) = classify_node(kind, language_key) {
        if let Some(name) = extract_symbol_name(&node, source, kind, language_key) {
            symbols.push(Symbol {
                name,
                kind: symbol_kind,
                line: node.start_position().row,
            });
        }
    }

    // Recurse into children
    let child_count = node.child_count();
    for i in 0..child_count {
        if let Some(child) = node.child(i) {
            collect_symbols(child, source, language_key, symbols);
        }
    }
}

/// Map a tree-sitter node kind to a SymbolKind, based on the language.
fn classify_node(kind: &str, language_key: &str) -> Option<SymbolKind> {
    match language_key {
        "rust" => match kind {
            "function_item" => Some(SymbolKind::Function),
            "struct_item" => Some(SymbolKind::Struct),
            "enum_item" => Some(SymbolKind::Enum),
            "trait_item" => Some(SymbolKind::Interface),
            "impl_item" => Some(SymbolKind::Class),
            "const_item" => Some(SymbolKind::Constant),
            "static_item" => Some(SymbolKind::Constant),
            "type_item" => Some(SymbolKind::Other),
            "mod_item" => Some(SymbolKind::Module),
            _ => None,
        },
        "python" => match kind {
            "function_definition" => Some(SymbolKind::Function),
            "class_definition" => Some(SymbolKind::Class),
            _ => None,
        },
        "javascript" | "tsx" => match kind {
            "function_declaration" => Some(SymbolKind::Function),
            "class_declaration" => Some(SymbolKind::Class),
            "method_definition" => Some(SymbolKind::Method),
            _ => None,
        },
        "typescript" => match kind {
            "function_declaration" => Some(SymbolKind::Function),
            "class_declaration" => Some(SymbolKind::Class),
            "method_definition" => Some(SymbolKind::Method),
            "interface_declaration" => Some(SymbolKind::Interface),
            "enum_declaration" => Some(SymbolKind::Enum),
            _ => None,
        },
        "go" => match kind {
            "function_declaration" => Some(SymbolKind::Function),
            "method_declaration" => Some(SymbolKind::Method),
            "type_declaration" => Some(SymbolKind::Other),
            _ => None,
        },
        "c" => match kind {
            "function_definition" => Some(SymbolKind::Function),
            "struct_specifier" => Some(SymbolKind::Struct),
            "enum_specifier" => Some(SymbolKind::Enum),
            _ => None,
        },
        "cpp" => match kind {
            "function_definition" => Some(SymbolKind::Function),
            "class_specifier" => Some(SymbolKind::Class),
            "struct_specifier" => Some(SymbolKind::Struct),
            "enum_specifier" => Some(SymbolKind::Enum),
            "namespace_definition" => Some(SymbolKind::Module),
            _ => None,
        },
        "java" => match kind {
            "method_declaration" => Some(SymbolKind::Method),
            "class_declaration" => Some(SymbolKind::Class),
            "interface_declaration" => Some(SymbolKind::Interface),
            "enum_declaration" => Some(SymbolKind::Enum),
            _ => None,
        },
        "php" => match kind {
            "function_definition" => Some(SymbolKind::Function),
            "method_declaration" => Some(SymbolKind::Method),
            "class_declaration" => Some(SymbolKind::Class),
            "interface_declaration" => Some(SymbolKind::Interface),
            "enum_declaration" => Some(SymbolKind::Enum),
            _ => None,
        },
        "ruby" => match kind {
            "method" => Some(SymbolKind::Function),
            "singleton_method" => Some(SymbolKind::Function),
            "class" => Some(SymbolKind::Class),
            "module" => Some(SymbolKind::Module),
            _ => None,
        },
        "swift" => match kind {
            "function_declaration" => Some(SymbolKind::Function),
            "class_declaration" => Some(SymbolKind::Class),
            "struct_declaration" => Some(SymbolKind::Struct),
            "enum_declaration" => Some(SymbolKind::Enum),
            "protocol_declaration" => Some(SymbolKind::Interface),
            _ => None,
        },
        "kotlin" => match kind {
            "function_declaration" => Some(SymbolKind::Function),
            "class_declaration" => Some(SymbolKind::Class),
            "object_declaration" => Some(SymbolKind::Class),
            "interface_declaration" => Some(SymbolKind::Interface),
            _ => None,
        },
        "lua" => match kind {
            "function_declaration" => Some(SymbolKind::Function),
            "function_definition" => Some(SymbolKind::Function),
            _ => None,
        },
        "haskell" => match kind {
            "function" => Some(SymbolKind::Function),
            "data_type" => Some(SymbolKind::Other),
            "newtype" => Some(SymbolKind::Other),
            "class_declaration" => Some(SymbolKind::Interface),
            "type_alias" => Some(SymbolKind::Other),
            _ => None,
        },
        "scala" => match kind {
            "function_definition" => Some(SymbolKind::Function),
            "class_definition" => Some(SymbolKind::Class),
            "object_definition" => Some(SymbolKind::Class),
            "trait_definition" => Some(SymbolKind::Interface),
            "val_definition" => Some(SymbolKind::Constant),
            _ => None,
        },
        _ => None,
    }
}

/// Extract the name of a symbol from its AST node.
fn extract_symbol_name(
    node: &tree_sitter::Node,
    source: &[u8],
    kind: &str,
    language_key: &str,
) -> Option<String> {
    // Most languages store the name in a "name" field
    if let Some(name_node) = node.child_by_field_name("name") {
        return name_node.utf8_text(source).ok().map(|s| s.to_string());
    }

    // Rust impl_item: look for the type being implemented
    if language_key == "rust" && kind == "impl_item" {
        // Try to find the "type" field (the type being implemented)
        if let Some(type_node) = node.child_by_field_name("type") {
            return type_node
                .utf8_text(source)
                .ok()
                .map(|s| format!("impl {}", s));
        }
        // Fallback: look for "trait" field for trait implementations
        if let Some(trait_node) = node.child_by_field_name("trait") {
            if let Some(type_node) = node.child_by_field_name("type") {
                let trait_name = trait_node.utf8_text(source).unwrap_or("?");
                let type_name = type_node.utf8_text(source).unwrap_or("?");
                return Some(format!("impl {} for {}", trait_name, type_name));
            }
        }
    }

    // C struct_specifier / enum_specifier: look for first identifier child
    if language_key == "c" && (kind == "struct_specifier" || kind == "enum_specifier") {
        let child_count = node.child_count();
        for i in 0..child_count {
            if let Some(child) = node.child(i) {
                if child.kind() == "type_identifier" {
                    return child.utf8_text(source).ok().map(|s| s.to_string());
                }
            }
        }
    }

    // Go type_declaration: the actual name is in the type_spec child
    if language_key == "go" && kind == "type_declaration" {
        let child_count = node.child_count();
        for i in 0..child_count {
            if let Some(child) = node.child(i) {
                if child.kind() == "type_spec" {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        return name_node.utf8_text(source).ok().map(|s| s.to_string());
                    }
                }
            }
        }
    }

    // Fallback: look for first "identifier" child node
    let child_count = node.child_count();
    for i in 0..child_count {
        if let Some(child) = node.child(i) {
            if child.kind() == "identifier" {
                return child.utf8_text(source).ok().map(|s| s.to_string());
            }
        }
    }

    None
}

/// Convert a byte range in the source into per-line HighlightSpans with char-based columns.
fn add_spans_for_byte_range(
    source: &str,
    start_byte: usize,
    end_byte: usize,
    highlight_idx: usize,
    line_starts: &[usize],
    result: &mut [Vec<HighlightSpan>],
) {
    if start_byte >= end_byte {
        return;
    }

    // Binary search for the line containing start_byte
    let start_line = match line_starts.binary_search(&start_byte) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };

    // Binary search for the line containing end_byte
    let end_line = match line_starts.binary_search(&end_byte) {
        Ok(i) => {
            // end_byte is exactly at a line start — span ends at end of previous line
            if i > 0 {
                i - 1
            } else {
                0
            }
        }
        Err(i) => i.saturating_sub(1),
    };

    for line_idx in start_line..=end_line.min(result.len().saturating_sub(1)) {
        let line_start_byte = line_starts[line_idx];
        let line_end_byte = if line_idx + 1 < line_starts.len() {
            line_starts[line_idx + 1]
        } else {
            source.len()
        };

        let span_start_byte = if line_idx == start_line {
            start_byte
        } else {
            line_start_byte
        };
        let span_end_byte = if line_idx == end_line {
            end_byte.min(line_end_byte)
        } else {
            line_end_byte
        };

        if span_start_byte >= span_end_byte {
            continue;
        }

        // Convert byte offsets within the line to char columns
        let line_bytes = &source.as_bytes()[line_start_byte..line_end_byte];
        let start_col = byte_offset_to_char_col(line_bytes, span_start_byte - line_start_byte);
        let end_col = byte_offset_to_char_col(line_bytes, span_end_byte - line_start_byte);

        if start_col < end_col {
            result[line_idx].push(HighlightSpan {
                start_col,
                end_col,
                highlight_idx,
            });
        }
    }
}

/// Convert a byte offset within a UTF-8 byte slice to a character count.
fn byte_offset_to_char_col(line_bytes: &[u8], byte_offset: usize) -> usize {
    let clamped = byte_offset.min(line_bytes.len());
    // Safe: line_bytes came from a &str so the prefix up to clamped is valid UTF-8
    // at character boundaries (tree-sitter only produces byte offsets at char boundaries).
    std::str::from_utf8(&line_bytes[..clamped])
        .map(|s| s.chars().count())
        .unwrap_or(clamped) // fallback: assume ASCII
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_languages_registered() {
        let engine = SyntaxEngine::new();
        // Original 12 languages (11 + tsx)
        assert!(engine.has_language("rust"));
        assert!(engine.has_language("python"));
        assert!(engine.has_language("javascript"));
        assert!(engine.has_language("json"));
        assert!(engine.has_language("c"));
        assert!(engine.has_language("go"));
        assert!(engine.has_language("typescript"));
        assert!(engine.has_language("tsx"));
        assert!(engine.has_language("bash"));
        assert!(engine.has_language("css"));
        assert!(engine.has_language("html"));
        assert!(engine.has_language("markdown"));
        // New languages
        assert!(engine.has_language("cpp"));
        assert!(engine.has_language("java"));
        assert!(engine.has_language("php"));
        assert!(engine.has_language("ruby"));
        assert!(engine.has_language("swift"));
        // kotlin grammar may not be compatible with current tree-sitter ABI
        // assert!(engine.has_language("kotlin"));
        assert!(engine.has_language("lua"));
        assert!(engine.has_language("yaml"));
        assert!(engine.has_language("toml"));
        assert!(engine.has_language("haskell"));
        assert!(engine.has_language("scala"));
        assert!(engine.has_language("sql"));
        // Nonexistent
        assert!(!engine.has_language("nonexistent"));
    }

    #[test]
    fn test_language_key_mapping() {
        // Original mappings
        assert_eq!(SyntaxEngine::language_key("Rust"), Some("rust"));
        assert_eq!(SyntaxEngine::language_key("Python"), Some("python"));
        assert_eq!(SyntaxEngine::language_key("JavaScript"), Some("javascript"));
        assert_eq!(SyntaxEngine::language_key("JSX"), Some("javascript"));
        assert_eq!(SyntaxEngine::language_key("TypeScript"), Some("typescript"));
        assert_eq!(SyntaxEngine::language_key("TSX"), Some("tsx"));
        assert_eq!(SyntaxEngine::language_key("C"), Some("c"));
        assert_eq!(SyntaxEngine::language_key("Go"), Some("go"));
        assert_eq!(SyntaxEngine::language_key("Bash"), Some("bash"));
        assert_eq!(SyntaxEngine::language_key("CSS"), Some("css"));
        assert_eq!(SyntaxEngine::language_key("SCSS"), Some("css"));
        assert_eq!(SyntaxEngine::language_key("HTML"), Some("html"));
        assert_eq!(SyntaxEngine::language_key("Markdown"), Some("markdown"));
        // New mappings
        assert_eq!(SyntaxEngine::language_key("C++"), Some("cpp"));
        assert_eq!(SyntaxEngine::language_key("Java"), Some("java"));
        assert_eq!(SyntaxEngine::language_key("PHP"), Some("php"));
        assert_eq!(SyntaxEngine::language_key("Ruby"), Some("ruby"));
        assert_eq!(SyntaxEngine::language_key("Swift"), Some("swift"));
        assert_eq!(SyntaxEngine::language_key("Kotlin"), Some("kotlin"));
        assert_eq!(SyntaxEngine::language_key("Lua"), Some("lua"));
        assert_eq!(SyntaxEngine::language_key("YAML"), Some("yaml"));
        assert_eq!(SyntaxEngine::language_key("TOML"), Some("toml"));
        assert_eq!(SyntaxEngine::language_key("SQL"), Some("sql"));
        assert_eq!(SyntaxEngine::language_key("Haskell"), Some("haskell"));
        assert_eq!(SyntaxEngine::language_key("Scala"), Some("scala"));
        // Not mapped
        assert_eq!(SyntaxEngine::language_key("Plain Text"), None);
    }

    #[test]
    fn test_highlight_rust_keywords() {
        let mut engine = SyntaxEngine::new();
        let source = "fn main() {\n    let x = 42;\n}\n";
        let lines = engine.highlight_lines(source, "rust");

        assert!(lines.len() >= 3);
        // Line 0: "fn main() {"
        let line0 = &lines[0];
        assert!(!line0.is_empty(), "Line 0 should have highlights");

        // "fn" at col 0..2 should be keyword (index 11)
        let keyword_idx = HIGHLIGHT_NAMES
            .iter()
            .position(|&n| n == "keyword")
            .unwrap();
        let has_fn_keyword = line0
            .iter()
            .any(|s| s.highlight_idx == keyword_idx && s.start_col == 0 && s.end_col == 2);
        assert!(
            has_fn_keyword,
            "Should find 'fn' as keyword, got: {:?}",
            line0
        );
    }

    #[test]
    fn test_highlight_rust_strings_and_comments() {
        let mut engine = SyntaxEngine::new();
        let source = "// comment\nlet s = \"hello\";\n";
        let lines = engine.highlight_lines(source, "rust");

        assert!(lines.len() >= 2);

        let comment_idx = HIGHLIGHT_NAMES
            .iter()
            .position(|&n| n == "comment")
            .unwrap();
        let has_comment = lines[0].iter().any(|s| s.highlight_idx == comment_idx);
        assert!(has_comment, "Line 0 should have comment highlight");

        let string_idx = HIGHLIGHT_NAMES.iter().position(|&n| n == "string").unwrap();
        let has_string = lines[1].iter().any(|s| s.highlight_idx == string_idx);
        assert!(has_string, "Line 1 should have string highlight");
    }

    #[test]
    fn test_highlight_unknown_language() {
        let mut engine = SyntaxEngine::new();
        let lines = engine.highlight_lines("hello", "nonexistent");
        assert!(lines.is_empty());
    }

    #[test]
    fn test_highlight_json() {
        let mut engine = SyntaxEngine::new();
        let source = "{\"key\": 42, \"bool\": true}\n";
        let lines = engine.highlight_lines(source, "json");
        assert!(!lines.is_empty());
        assert!(!lines[0].is_empty(), "JSON should have highlights");
    }

    #[test]
    fn test_byte_to_char_col() {
        assert_eq!(byte_offset_to_char_col(b"hello", 0), 0);
        assert_eq!(byte_offset_to_char_col(b"hello", 3), 3);
        assert_eq!(byte_offset_to_char_col(b"hello", 5), 5);
    }

    #[test]
    fn test_extract_symbols_rust() {
        let mut engine = SyntaxEngine::new();
        let source = r#"
fn main() {
    println!("hello");
}

struct Buffer {
    data: Vec<u8>,
}

enum Color {
    Red,
    Green,
    Blue,
}

impl Buffer {
    fn new() -> Self {
        Buffer { data: Vec::new() }
    }

    fn len(&self) -> usize {
        self.data.len()
    }
}

trait Drawable {
    fn draw(&self);
}

const MAX_SIZE: usize = 1024;

mod utils {
    pub fn helper() {}
}
"#;
        let symbols = engine.extract_symbols(source, "rust");
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

        assert!(
            names.contains(&"main"),
            "Should find 'main', got: {:?}",
            names
        );
        assert!(
            names.contains(&"Buffer"),
            "Should find struct 'Buffer', got: {:?}",
            names
        );
        assert!(
            names.contains(&"Color"),
            "Should find enum 'Color', got: {:?}",
            names
        );
        assert!(
            names.contains(&"Drawable"),
            "Should find trait 'Drawable', got: {:?}",
            names
        );
        assert!(
            names.contains(&"MAX_SIZE"),
            "Should find const 'MAX_SIZE', got: {:?}",
            names
        );
        assert!(
            names.contains(&"utils"),
            "Should find mod 'utils', got: {:?}",
            names
        );

        // Check that functions inside impl are found
        assert!(
            names.contains(&"new"),
            "Should find 'new' in impl, got: {:?}",
            names
        );
        assert!(
            names.contains(&"len"),
            "Should find 'len' in impl, got: {:?}",
            names
        );

        // Check impl item itself
        assert!(
            symbols
                .iter()
                .any(|s| s.name.starts_with("impl") && s.kind == SymbolKind::Class),
            "Should find impl block, got: {:?}",
            names
        );

        // Verify line numbers are sensible (0-based)
        let main_sym = symbols.iter().find(|s| s.name == "main").unwrap();
        assert_eq!(main_sym.line, 1, "main should be on line 1 (0-based)");
        assert_eq!(main_sym.kind, SymbolKind::Function);

        let buffer_sym = symbols
            .iter()
            .find(|s| s.name == "Buffer" && s.kind == SymbolKind::Struct)
            .unwrap();
        assert_eq!(
            buffer_sym.line, 5,
            "Buffer struct should be on line 5 (0-based)"
        );
    }

    #[test]
    fn test_extract_symbols_python() {
        let mut engine = SyntaxEngine::new();
        let source = r#"
def hello():
    print("hello")

class MyClass:
    def method(self):
        pass

def another_function(x, y):
    return x + y
"#;
        let symbols = engine.extract_symbols(source, "python");
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

        assert!(
            names.contains(&"hello"),
            "Should find 'hello', got: {:?}",
            names
        );
        assert!(
            names.contains(&"MyClass"),
            "Should find 'MyClass', got: {:?}",
            names
        );
        assert!(
            names.contains(&"method"),
            "Should find 'method', got: {:?}",
            names
        );
        assert!(
            names.contains(&"another_function"),
            "Should find 'another_function', got: {:?}",
            names
        );

        let class_sym = symbols.iter().find(|s| s.name == "MyClass").unwrap();
        assert_eq!(class_sym.kind, SymbolKind::Class);

        let fn_sym = symbols.iter().find(|s| s.name == "hello").unwrap();
        assert_eq!(fn_sym.kind, SymbolKind::Function);
    }

    #[test]
    fn test_extract_symbols_unknown_language() {
        let mut engine = SyntaxEngine::new();
        let symbols = engine.extract_symbols("some text", "nonexistent");
        assert!(
            symbols.is_empty(),
            "Unknown language should return empty symbols"
        );
    }

    #[test]
    fn test_extract_symbols_empty_source() {
        let mut engine = SyntaxEngine::new();
        let symbols = engine.extract_symbols("", "rust");
        assert!(
            symbols.is_empty(),
            "Empty source should return empty symbols"
        );
    }

    #[test]
    fn test_highlight_cpp() {
        let mut engine = SyntaxEngine::new();
        let source = "int main() {\n    int x = 42;\n    return 0;\n}\n";
        let lines = engine.highlight_lines(source, "cpp");
        assert!(!lines.is_empty());
        let total_spans: usize = lines.iter().map(|l| l.len()).sum();
        assert!(total_spans > 0, "C++ should have some highlights");
    }

    #[test]
    fn test_highlight_java() {
        let mut engine = SyntaxEngine::new();
        let source =
            "public class Hello {\n    public static void main(String[] args) {\n    }\n}\n";
        let lines = engine.highlight_lines(source, "java");
        assert!(!lines.is_empty());
        let keyword_idx = HIGHLIGHT_NAMES
            .iter()
            .position(|&n| n == "keyword")
            .unwrap();
        let has_keyword = lines[0].iter().any(|s| s.highlight_idx == keyword_idx);
        assert!(has_keyword, "Java should highlight 'public' as keyword");
    }

    #[test]
    fn test_highlight_php() {
        let mut engine = SyntaxEngine::new();
        let source = "<?php\nfunction hello() {\n    echo \"world\";\n}\n";
        let lines = engine.highlight_lines(source, "php");
        assert!(!lines.is_empty());
        // PHP should produce some highlights
        let total_spans: usize = lines.iter().map(|l| l.len()).sum();
        assert!(total_spans > 0, "PHP should have some highlights");
    }

    #[test]
    fn test_highlight_ruby() {
        let mut engine = SyntaxEngine::new();
        let source = "def hello\n  puts \"world\"\nend\n";
        let lines = engine.highlight_lines(source, "ruby");
        assert!(!lines.is_empty());
        let keyword_idx = HIGHLIGHT_NAMES
            .iter()
            .position(|&n| n == "keyword")
            .unwrap();
        let has_keyword = lines[0].iter().any(|s| s.highlight_idx == keyword_idx);
        assert!(has_keyword, "Ruby should highlight 'def' as keyword");
    }

    #[test]
    fn test_highlight_swift() {
        let mut engine = SyntaxEngine::new();
        let source = "func greet() {\n    print(\"hello\")\n}\n";
        let lines = engine.highlight_lines(source, "swift");
        assert!(!lines.is_empty());
        let total_spans: usize = lines.iter().map(|l| l.len()).sum();
        assert!(total_spans > 0, "Swift should have some highlights");
    }

    #[test]
    fn test_highlight_kotlin() {
        let mut engine = SyntaxEngine::new();
        if !engine.has_language("kotlin") {
            // kotlin grammar may not be compatible with current tree-sitter ABI
            return;
        }
        let source = "fun main() {\n    val x = 42\n    println(x)\n}\n";
        let lines = engine.highlight_lines(source, "kotlin");
        assert!(!lines.is_empty());
        let total_spans: usize = lines.iter().map(|l| l.len()).sum();
        assert!(total_spans > 0, "Kotlin should have some highlights");
    }

    #[test]
    fn test_highlight_lua() {
        let mut engine = SyntaxEngine::new();
        let source = "-- comment\nlocal x = 42\nfunction hello()\n    print(x)\nend\n";
        let lines = engine.highlight_lines(source, "lua");
        assert!(!lines.is_empty());
        let comment_idx = HIGHLIGHT_NAMES
            .iter()
            .position(|&n| n == "comment")
            .unwrap();
        let has_comment = lines[0].iter().any(|s| s.highlight_idx == comment_idx);
        assert!(has_comment, "Lua should highlight comment on line 0");
    }

    #[test]
    fn test_highlight_yaml() {
        let mut engine = SyntaxEngine::new();
        let source = "name: hello\nversion: 1.0\nitems:\n  - one\n  - two\n";
        let lines = engine.highlight_lines(source, "yaml");
        assert!(!lines.is_empty());
        let total_spans: usize = lines.iter().map(|l| l.len()).sum();
        assert!(total_spans > 0, "YAML should have some highlights");
    }

    #[test]
    fn test_highlight_toml() {
        let mut engine = SyntaxEngine::new();
        let source = "[package]\nname = \"hello\"\nversion = \"0.1.0\"\n";
        let lines = engine.highlight_lines(source, "toml");
        assert!(!lines.is_empty());
        let string_idx = HIGHLIGHT_NAMES.iter().position(|&n| n == "string").unwrap();
        let has_string = lines[1].iter().any(|s| s.highlight_idx == string_idx);
        assert!(has_string, "TOML should highlight string on line 1");
    }

    #[test]
    fn test_highlight_haskell() {
        let mut engine = SyntaxEngine::new();
        let source = "module Main where\n\nmain :: IO ()\nmain = putStrLn \"Hello\"\n";
        let lines = engine.highlight_lines(source, "haskell");
        assert!(!lines.is_empty());
        let total_spans: usize = lines.iter().map(|l| l.len()).sum();
        assert!(total_spans > 0, "Haskell should have some highlights");
    }

    #[test]
    fn test_highlight_scala() {
        let mut engine = SyntaxEngine::new();
        let source = "object Main {\n  def main(args: Array[String]): Unit = {\n    println(\"hello\")\n  }\n}\n";
        let lines = engine.highlight_lines(source, "scala");
        assert!(!lines.is_empty());
        let total_spans: usize = lines.iter().map(|l| l.len()).sum();
        assert!(total_spans > 0, "Scala should have some highlights");
    }

    #[test]
    fn test_highlight_sql() {
        let mut engine = SyntaxEngine::new();
        let source = "SELECT name, age FROM users WHERE age > 18;\n";
        let lines = engine.highlight_lines(source, "sql");
        assert!(!lines.is_empty());
        let total_spans: usize = lines.iter().map(|l| l.len()).sum();
        assert!(total_spans > 0, "SQL should have some highlights");
    }
}
