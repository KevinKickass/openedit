//! Import themes from VS Code (.json) and Notepad++ (.xml) formats,
//! converting them to the OpenEdit TOML theme format.

use crate::theme::{SyntaxColorsFile, ThemeColorsFile, ThemeFile, ThemeRegistry};
use std::path::{Path, PathBuf};

/// Import a VS Code theme from a JSON file.
/// Returns the path to the saved TOML file on success.
pub fn import_vscode_theme(path: &Path) -> Result<PathBuf, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("read error: {}", e))?;
    let theme_file = parse_vscode_theme(&content, path)?;
    save_imported_theme(&theme_file)
}

/// Import a Notepad++ theme from an XML file.
/// Returns the path to the saved TOML file on success.
pub fn import_notepadpp_theme(path: &Path) -> Result<PathBuf, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("read error: {}", e))?;
    let theme_file = parse_notepadpp_theme(&content, path)?;
    save_imported_theme(&theme_file)
}

/// Save a ThemeFile as TOML in the user themes directory.
fn save_imported_theme(theme_file: &ThemeFile) -> Result<PathBuf, String> {
    let dir = ThemeRegistry::ensure_themes_dir()
        .ok_or_else(|| "Could not determine themes directory".to_string())?;

    let toml_content =
        toml::to_string_pretty(theme_file).map_err(|e| format!("serialize error: {}", e))?;

    let filename = theme_file
        .name
        .to_lowercase()
        .replace(' ', "_")
        .replace(|c: char| !c.is_alphanumeric() && c != '_', "");
    let path = dir.join(format!("{}.toml", filename));

    std::fs::write(&path, toml_content).map_err(|e| format!("write error: {}", e))?;

    log::info!("Imported theme '{}' to {}", theme_file.name, path.display());
    Ok(path)
}

// ── VS Code theme parsing ──

/// Parse a VS Code JSON theme string into our ThemeFile format.
pub fn parse_vscode_theme(json_str: &str, source_path: &Path) -> Result<ThemeFile, String> {
    // VS Code themes may contain comments (JSONC). Strip them before parsing.
    let clean = strip_jsonc_comments(json_str);
    let value: serde_json::Value =
        serde_json::from_str(&clean).map_err(|e| format!("JSON parse error: {}", e))?;

    let obj = value
        .as_object()
        .ok_or_else(|| "Expected JSON object at top level".to_string())?;

    // Derive name from the JSON "name" field, or from the filename.
    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            source_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Imported Theme")
                .to_string()
        });

    // Parse the "colors" section (editor colors).
    let colors_obj = obj.get("colors").and_then(|v| v.as_object());
    let colors = parse_vscode_colors(colors_obj);

    // Parse the "tokenColors" section (syntax highlighting).
    let token_colors = obj.get("tokenColors").and_then(|v| v.as_array());
    let syntax = parse_vscode_token_colors(token_colors);

    // Determine base theme from the VS Code "type" field.
    let base = obj
        .get("type")
        .and_then(|v| v.as_str())
        .map(|t| {
            if t == "light" {
                "Light".to_string()
            } else {
                "Dark".to_string()
            }
        })
        .or(Some("Dark".to_string()));

    Ok(ThemeFile {
        name,
        base,
        colors,
        syntax,
    })
}

/// Strip single-line (//) and block (/* */) comments from JSONC content.
fn strip_jsonc_comments(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_string = false;

    while i < len {
        if in_string {
            result.push(chars[i]);
            if chars[i] == '\\' && i + 1 < len {
                i += 1;
                result.push(chars[i]);
            } else if chars[i] == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if chars[i] == '"' {
            in_string = true;
            result.push(chars[i]);
            i += 1;
            continue;
        }

        if chars[i] == '/' && i + 1 < len {
            if chars[i + 1] == '/' {
                // Single-line comment: skip until end of line.
                i += 2;
                while i < len && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            } else if chars[i + 1] == '*' {
                // Block comment: skip until */.
                i += 2;
                while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                if i + 1 < len {
                    i += 2; // skip */
                }
                continue;
            }
        }

        // Also handle trailing commas before } or ] (common in VS Code themes).
        result.push(chars[i]);
        i += 1;
    }

    // Remove trailing commas before } or ] (a common JSONC pattern).
    strip_trailing_commas(&result)
}

/// Remove trailing commas before closing braces/brackets.
fn strip_trailing_commas(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_string = false;

    while i < len {
        if in_string {
            result.push(chars[i]);
            if chars[i] == '\\' && i + 1 < len {
                i += 1;
                result.push(chars[i]);
            } else if chars[i] == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if chars[i] == '"' {
            in_string = true;
            result.push(chars[i]);
            i += 1;
            continue;
        }

        if chars[i] == ',' {
            // Check if the next non-whitespace character is } or ].
            let mut j = i + 1;
            while j < len && chars[j].is_whitespace() {
                j += 1;
            }
            if j < len && (chars[j] == '}' || chars[j] == ']') {
                // Skip the trailing comma.
                i += 1;
                continue;
            }
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Extract editor colors from VS Code "colors" object.
fn parse_vscode_colors(
    colors: Option<&serde_json::Map<String, serde_json::Value>>,
) -> ThemeColorsFile {
    let get = |key: &str| -> Option<String> {
        colors
            .and_then(|c| c.get(key))
            .and_then(|v| v.as_str())
            .map(|s| normalize_color(s))
    };

    ThemeColorsFile {
        background: get("editor.background"),
        foreground: get("editor.foreground"),
        gutter_bg: get("editorGutter.background")
            .or_else(|| get("editorLineNumber.background"))
            .or_else(|| get("editor.background")),
        gutter_fg: get("editorLineNumber.foreground"),
        current_line_bg: get("editor.lineHighlightBackground"),
        selection_bg: get("editor.selectionBackground"),
        cursor: get("editorCursor.foreground"),
        search_match_bg: get("editor.findMatchHighlightBackground"),
        search_current_match_bg: get("editor.findMatchBackground"),
        tab_active_bg: get("tab.activeBackground"),
        tab_inactive_bg: get("tab.inactiveBackground"),
        tab_text: get("tab.activeForeground"),
        status_bar_bg: get("statusBar.background"),
        status_bar_fg: get("statusBar.foreground"),
        modified_indicator: get("editorGutter.modifiedBackground")
            .or_else(|| get("tab.activeForeground")),
    }
}

/// Normalize a color string: VS Code sometimes uses #RRGGBBAA format with
/// shorter hex (e.g., #FFF), or colors may have extra whitespace.
fn normalize_color(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('#') {
        let hex = &s[1..];
        match hex.len() {
            // #RGB -> #RRGGBB
            3 => {
                let chars: Vec<char> = hex.chars().collect();
                format!("#{0}{0}{1}{1}{2}{2}", chars[0], chars[1], chars[2])
            }
            // #RGBA -> #RRGGBBAA
            4 => {
                let chars: Vec<char> = hex.chars().collect();
                format!(
                    "#{0}{0}{1}{1}{2}{2}{3}{3}",
                    chars[0], chars[1], chars[2], chars[3]
                )
            }
            // #RRGGBB or #RRGGBBAA - already valid
            6 | 8 => s.to_string(),
            _ => s.to_string(),
        }
    } else {
        s.to_string()
    }
}

/// Extract syntax colors from VS Code "tokenColors" array.
fn parse_vscode_token_colors(token_colors: Option<&Vec<serde_json::Value>>) -> SyntaxColorsFile {
    let mut syntax = SyntaxColorsFile::default();

    let tokens = match token_colors {
        Some(t) => t,
        None => return syntax,
    };

    for entry in tokens {
        let obj = match entry.as_object() {
            Some(o) => o,
            None => continue,
        };

        let settings = match obj.get("settings").and_then(|v| v.as_object()) {
            Some(s) => s,
            None => continue,
        };

        let foreground = settings
            .get("foreground")
            .and_then(|v| v.as_str())
            .map(|s| normalize_color(s));

        let foreground = match foreground {
            Some(f) => f,
            None => continue,
        };

        // Get the scope(s) for this token color rule.
        let scopes = match obj.get("scope") {
            Some(serde_json::Value::String(s)) => s
                .split(',')
                .map(|part| part.trim().to_string())
                .collect::<Vec<_>>(),
            Some(serde_json::Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                .collect(),
            _ => continue,
        };

        for scope in &scopes {
            let scope = scope.as_str();
            // Map VS Code scopes to our syntax color fields.
            // More specific scopes should be checked first.
            if scope_matches(scope, &["keyword", "storage.type", "storage.modifier"]) {
                if syntax.keyword.is_none() {
                    syntax.keyword = Some(foreground.clone());
                }
            }
            if scope_matches(scope, &["comment", "punctuation.definition.comment"]) {
                if syntax.comment.is_none() {
                    syntax.comment = Some(foreground.clone());
                }
            }
            if scope_matches(scope, &["string", "string.quoted"]) {
                if syntax.string.is_none() {
                    syntax.string = Some(foreground.clone());
                }
            }
            if scope_matches(scope, &["constant.numeric"]) {
                if syntax.number.is_none() {
                    syntax.number = Some(foreground.clone());
                }
            }
            if scope_matches(
                scope,
                &[
                    "entity.name.function",
                    "support.function",
                    "meta.function-call",
                ],
            ) {
                if syntax.function.is_none() {
                    syntax.function = Some(foreground.clone());
                }
            }
            if scope_matches(
                scope,
                &[
                    "entity.name.type",
                    "support.type",
                    "support.class",
                    "entity.name.class",
                ],
            ) {
                if syntax.r#type.is_none() {
                    syntax.r#type = Some(foreground.clone());
                }
            }
            if scope_matches(scope, &["support.type.builtin"]) {
                if syntax.type_builtin.is_none() {
                    syntax.type_builtin = Some(foreground.clone());
                }
            }
            if scope_matches(scope, &["variable", "variable.other"]) {
                if syntax.variable.is_none() {
                    syntax.variable = Some(foreground.clone());
                }
            }
            if scope_matches(
                scope,
                &["variable.language", "variable.other.readwrite.global"],
            ) {
                if syntax.variable_builtin.is_none() {
                    syntax.variable_builtin = Some(foreground.clone());
                }
            }
            if scope_matches(
                scope,
                &["variable.other.property", "meta.object-literal.key"],
            ) {
                if syntax.property.is_none() {
                    syntax.property = Some(foreground.clone());
                }
            }
            if scope_matches(scope, &["keyword.operator", "punctuation.accessor"]) {
                if syntax.operator.is_none() {
                    syntax.operator = Some(foreground.clone());
                }
            }
            if scope_matches(
                scope,
                &["punctuation", "punctuation.definition", "meta.brace"],
            ) {
                if syntax.punctuation.is_none() {
                    syntax.punctuation = Some(foreground.clone());
                }
            }
            if scope_matches(scope, &["constant", "constant.language"]) {
                if syntax.constant.is_none() {
                    syntax.constant = Some(foreground.clone());
                }
            }
            if scope_matches(scope, &["constant.language"]) {
                if syntax.constant_builtin.is_none() {
                    syntax.constant_builtin = Some(foreground.clone());
                }
            }
            if scope_matches(scope, &["entity.other.attribute-name", "meta.attribute"]) {
                if syntax.attribute.is_none() {
                    syntax.attribute = Some(foreground.clone());
                }
            }
            if scope_matches(scope, &["entity.name.tag", "meta.tag"]) {
                if syntax.tag.is_none() {
                    syntax.tag = Some(foreground.clone());
                }
            }
            if scope_matches(scope, &["constant.character.escape", "string.escape"]) {
                if syntax.escape.is_none() {
                    syntax.escape = Some(foreground.clone());
                }
            }
            if scope_matches(scope, &["entity.name.function.macro", "support.macro"]) {
                if syntax.function_macro.is_none() {
                    syntax.function_macro = Some(foreground.clone());
                }
            }
        }
    }

    syntax
}

/// Check if a scope string matches any of the given patterns.
/// A scope matches if it equals the pattern or starts with the pattern followed by a dot.
fn scope_matches(scope: &str, patterns: &[&str]) -> bool {
    for pattern in patterns {
        if scope == *pattern || scope.starts_with(&format!("{}.", pattern)) {
            return true;
        }
    }
    false
}

// ── Notepad++ theme parsing ──

/// Parse a Notepad++ XML theme string into our ThemeFile format.
pub fn parse_notepadpp_theme(xml_str: &str, source_path: &Path) -> Result<ThemeFile, String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml_str);

    // Derive name from the filename.
    let name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported Theme")
        .to_string();

    let mut colors = ThemeColorsFile::default();
    let mut syntax = SyntaxColorsFile::default();

    // Track if we are inside <GlobalStyles> or <LexerStyles>.
    let mut in_global_styles = false;
    let mut in_lexer_styles = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match tag_name.as_str() {
                    "GlobalStyles" => in_global_styles = true,
                    "LexerStyles" => in_lexer_styles = true,
                    "WidgetStyle" if in_global_styles => {
                        parse_npp_global_widget_style(e, &mut colors);
                    }
                    "WordsStyle" if in_lexer_styles => {
                        parse_npp_lexer_words_style(e, &mut syntax);
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match tag_name.as_str() {
                    "GlobalStyles" => in_global_styles = false,
                    "LexerStyles" => in_lexer_styles = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
    }

    // Determine base from background lightness.
    let base = if let Some(ref bg) = colors.background {
        if is_light_color(bg) {
            Some("Light".to_string())
        } else {
            Some("Dark".to_string())
        }
    } else {
        Some("Dark".to_string())
    };

    Ok(ThemeFile {
        name,
        base,
        colors,
        syntax,
    })
}

/// Parse a Notepad++ `<WidgetStyle>` element from GlobalStyles.
fn parse_npp_global_widget_style(
    e: &quick_xml::events::BytesStart<'_>,
    colors: &mut ThemeColorsFile,
) {
    let name = get_xml_attr(e, "name").unwrap_or_default();
    let fg = get_xml_attr(e, "fgColor").and_then(|c| npp_color_to_hex(&c));
    let bg = get_xml_attr(e, "bgColor").and_then(|c| npp_color_to_hex(&c));

    match name.as_str() {
        "Default Style" => {
            if colors.foreground.is_none() {
                colors.foreground = fg;
            }
            if colors.background.is_none() {
                colors.background = bg;
            }
        }
        "Caret colour" => {
            if colors.cursor.is_none() {
                colors.cursor = fg;
            }
        }
        "Current line background colour" => {
            if colors.current_line_bg.is_none() {
                colors.current_line_bg = bg;
            }
        }
        "Selected text colour" => {
            if colors.selection_bg.is_none() {
                colors.selection_bg = bg;
            }
        }
        "Line number" => {
            if colors.gutter_fg.is_none() {
                colors.gutter_fg = fg;
            }
            if colors.gutter_bg.is_none() {
                colors.gutter_bg = bg;
            }
        }
        "Find Mark Style" | "Mark Style 1" => {
            if colors.search_match_bg.is_none() {
                colors.search_match_bg = bg;
            }
        }
        "Smart Highlighting" | "Incremental highlight all" => {
            if colors.search_current_match_bg.is_none() {
                colors.search_current_match_bg = bg.or(fg);
            }
        }
        _ => {}
    }
}

/// Parse a Notepad++ `<WordsStyle>` element from LexerStyles.
fn parse_npp_lexer_words_style(
    e: &quick_xml::events::BytesStart<'_>,
    syntax: &mut SyntaxColorsFile,
) {
    let name = get_xml_attr(e, "name").unwrap_or_default();
    let fg = get_xml_attr(e, "fgColor").and_then(|c| npp_color_to_hex(&c));

    let fg = match fg {
        Some(f) => f,
        None => return,
    };

    let name_lower = name.to_lowercase();

    // Map Notepad++ style names to our syntax fields.
    if name_lower.contains("keyword") || name_lower == "instruction word" || name_lower == "word1" {
        if syntax.keyword.is_none() {
            syntax.keyword = Some(fg.clone());
        }
    }
    if name_lower.contains("comment") || name_lower == "commentline" || name_lower == "commentdoc" {
        if syntax.comment.is_none() {
            syntax.comment = Some(fg.clone());
        }
    }
    if name_lower.contains("string") || name_lower == "character" || name_lower == "verbatim" {
        if syntax.string.is_none() {
            syntax.string = Some(fg.clone());
        }
    }
    if name_lower == "number" || name_lower.contains("numeric") {
        if syntax.number.is_none() {
            syntax.number = Some(fg.clone());
        }
    }
    if name_lower.contains("function") || name_lower == "word2" || name_lower == "globalclass" {
        if syntax.function.is_none() {
            syntax.function = Some(fg.clone());
        }
    }
    if name_lower == "type" || name_lower.contains("class name") || name_lower == "word4" {
        if syntax.r#type.is_none() {
            syntax.r#type = Some(fg.clone());
        }
    }
    if name_lower == "variable" || name_lower.contains("variable") {
        if syntax.variable.is_none() {
            syntax.variable = Some(fg.clone());
        }
    }
    if name_lower.contains("operator") {
        if syntax.operator.is_none() {
            syntax.operator = Some(fg.clone());
        }
    }
    if name_lower.contains("preprocessor") || name_lower.contains("attribute") {
        if syntax.attribute.is_none() {
            syntax.attribute = Some(fg.clone());
        }
    }
    if name_lower.contains("tag") || name_lower == "html tag" {
        if syntax.tag.is_none() {
            syntax.tag = Some(fg.clone());
        }
    }
    if name_lower.contains("escape") {
        if syntax.escape.is_none() {
            syntax.escape = Some(fg.clone());
        }
    }
    if name_lower.contains("delimiter")
        || name_lower.contains("brace")
        || name_lower.contains("bracket")
    {
        if syntax.punctuation.is_none() {
            syntax.punctuation = Some(fg.clone());
        }
    }
    if name_lower.contains("constant") {
        if syntax.constant.is_none() {
            syntax.constant = Some(fg.clone());
        }
    }
}

/// Get an XML attribute value by name from a start/empty element.
fn get_xml_attr(e: &quick_xml::events::BytesStart<'_>, attr_name: &str) -> Option<String> {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == attr_name.as_bytes() {
            return String::from_utf8(attr.value.to_vec()).ok();
        }
    }
    None
}

/// Convert a Notepad++ color string (6-digit hex without #, e.g., "FF0000") to "#rrggbb".
/// Notepad++ stores colors in BGR order in some cases, but the XML theme files
/// typically use RGB order in their fgColor/bgColor attributes.
fn npp_color_to_hex(color: &str) -> Option<String> {
    let color = color.trim();
    if color.is_empty() || color == "0" {
        return None;
    }

    // Pad to 6 digits if shorter.
    let padded = format!("{:0>6}", color);
    if padded.len() != 6 {
        return None;
    }

    // Validate that it's valid hex.
    if u32::from_str_radix(&padded, 16).is_err() {
        return None;
    }

    Some(format!("#{}", padded.to_lowercase()))
}

/// Determine if a hex color string represents a "light" color (for base theme selection).
fn is_light_color(hex: &str) -> bool {
    let hex = hex.trim().strip_prefix('#').unwrap_or(hex);
    if hex.len() < 6 {
        return false;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32;
    // Use relative luminance formula.
    let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
    luminance > 128.0
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── JSONC stripping tests ──

    #[test]
    fn test_strip_jsonc_single_line_comment() {
        let input = r#"{
  "name": "test" // this is a comment
}"#;
        let cleaned = strip_jsonc_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&cleaned).unwrap();
        assert_eq!(parsed["name"], "test");
    }

    #[test]
    fn test_strip_jsonc_block_comment() {
        let input = r#"{
  /* block comment */
  "name": "test"
}"#;
        let cleaned = strip_jsonc_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&cleaned).unwrap();
        assert_eq!(parsed["name"], "test");
    }

    #[test]
    fn test_strip_jsonc_trailing_comma() {
        let input = r#"{
  "a": 1,
  "b": 2,
}"#;
        let cleaned = strip_jsonc_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&cleaned).unwrap();
        assert_eq!(parsed["a"], 1);
        assert_eq!(parsed["b"], 2);
    }

    #[test]
    fn test_strip_jsonc_preserves_strings_with_slashes() {
        let input = r#"{"url": "https://example.com"}"#;
        let cleaned = strip_jsonc_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&cleaned).unwrap();
        assert_eq!(parsed["url"], "https://example.com");
    }

    // ── Color normalization ──

    #[test]
    fn test_normalize_color_6_digit() {
        assert_eq!(normalize_color("#aabbcc"), "#aabbcc");
    }

    #[test]
    fn test_normalize_color_8_digit() {
        assert_eq!(normalize_color("#aabbcc80"), "#aabbcc80");
    }

    #[test]
    fn test_normalize_color_3_digit() {
        assert_eq!(normalize_color("#abc"), "#aabbcc");
    }

    #[test]
    fn test_normalize_color_4_digit() {
        assert_eq!(normalize_color("#abcd"), "#aabbccdd");
    }

    // ── Scope matching ──

    #[test]
    fn test_scope_matches_exact() {
        assert!(scope_matches("keyword", &["keyword"]));
        assert!(!scope_matches("keywords", &["keyword"]));
    }

    #[test]
    fn test_scope_matches_prefix() {
        assert!(scope_matches("keyword.control", &["keyword"]));
        assert!(scope_matches(
            "keyword.operator.assignment",
            &["keyword.operator"]
        ));
    }

    #[test]
    fn test_scope_no_match() {
        assert!(!scope_matches("string", &["keyword"]));
        assert!(!scope_matches("comment", &["keyword", "string"]));
    }

    // ── VS Code theme parsing ──

    #[test]
    fn test_parse_vscode_theme_basic() {
        let json = r##"{
            "name": "My Test Theme",
            "type": "dark",
            "colors": {
                "editor.background": "#1e1e1e",
                "editor.foreground": "#d4d4d4",
                "editorLineNumber.foreground": "#858585",
                "editor.selectionBackground": "#264f78",
                "editorCursor.foreground": "#aeafad",
                "editor.lineHighlightBackground": "#2a2d2e",
                "editor.findMatchBackground": "#515c6a",
                "editor.findMatchHighlightBackground": "#ea5c0055",
                "tab.activeBackground": "#1e1e1e",
                "tab.inactiveBackground": "#2d2d2d",
                "tab.activeForeground": "#ffffff",
                "statusBar.background": "#007acc",
                "statusBar.foreground": "#ffffff"
            },
            "tokenColors": [
                {
                    "scope": "keyword",
                    "settings": {
                        "foreground": "#569cd6"
                    }
                },
                {
                    "scope": ["comment", "punctuation.definition.comment"],
                    "settings": {
                        "foreground": "#6a9955"
                    }
                },
                {
                    "scope": "string",
                    "settings": {
                        "foreground": "#ce9178"
                    }
                },
                {
                    "scope": "constant.numeric",
                    "settings": {
                        "foreground": "#b5cea8"
                    }
                },
                {
                    "scope": "entity.name.function",
                    "settings": {
                        "foreground": "#dcdcaa"
                    }
                },
                {
                    "scope": "entity.name.type",
                    "settings": {
                        "foreground": "#4ec9b0"
                    }
                },
                {
                    "scope": "variable",
                    "settings": {
                        "foreground": "#9cdcfe"
                    }
                },
                {
                    "scope": "keyword.operator",
                    "settings": {
                        "foreground": "#d4d4d4"
                    }
                }
            ]
        }"##;

        let path = Path::new("test_theme.json");
        let theme = parse_vscode_theme(json, path).unwrap();

        assert_eq!(theme.name, "My Test Theme");
        assert_eq!(theme.base, Some("Dark".to_string()));
        assert_eq!(theme.colors.background, Some("#1e1e1e".to_string()));
        assert_eq!(theme.colors.foreground, Some("#d4d4d4".to_string()));
        assert_eq!(theme.colors.gutter_fg, Some("#858585".to_string()));
        assert_eq!(theme.colors.selection_bg, Some("#264f78".to_string()));
        assert_eq!(theme.colors.cursor, Some("#aeafad".to_string()));
        assert_eq!(theme.colors.current_line_bg, Some("#2a2d2e".to_string()));
        assert_eq!(theme.colors.tab_active_bg, Some("#1e1e1e".to_string()));
        assert_eq!(theme.colors.status_bar_bg, Some("#007acc".to_string()));

        assert_eq!(theme.syntax.keyword, Some("#569cd6".to_string()));
        assert_eq!(theme.syntax.comment, Some("#6a9955".to_string()));
        assert_eq!(theme.syntax.string, Some("#ce9178".to_string()));
        assert_eq!(theme.syntax.number, Some("#b5cea8".to_string()));
        assert_eq!(theme.syntax.function, Some("#dcdcaa".to_string()));
        assert_eq!(theme.syntax.r#type, Some("#4ec9b0".to_string()));
        assert_eq!(theme.syntax.variable, Some("#9cdcfe".to_string()));
        assert_eq!(theme.syntax.operator, Some("#d4d4d4".to_string()));
    }

    #[test]
    fn test_parse_vscode_theme_light_type() {
        let json = r##"{
            "name": "Light Theme",
            "type": "light",
            "colors": {},
            "tokenColors": []
        }"##;

        let path = Path::new("light.json");
        let theme = parse_vscode_theme(json, path).unwrap();
        assert_eq!(theme.base, Some("Light".to_string()));
    }

    #[test]
    fn test_parse_vscode_theme_no_name_uses_filename() {
        let json = r##"{
            "colors": {},
            "tokenColors": []
        }"##;

        let path = Path::new("/themes/Cobalt2.json");
        let theme = parse_vscode_theme(json, path).unwrap();
        assert_eq!(theme.name, "Cobalt2");
    }

    #[test]
    fn test_parse_vscode_theme_comma_separated_scopes() {
        let json = r##"{
            "name": "Test",
            "colors": {},
            "tokenColors": [
                {
                    "scope": "keyword, storage.type, storage.modifier",
                    "settings": {
                        "foreground": "#ff0000"
                    }
                }
            ]
        }"##;

        let path = Path::new("test.json");
        let theme = parse_vscode_theme(json, path).unwrap();
        assert_eq!(theme.syntax.keyword, Some("#ff0000".to_string()));
    }

    #[test]
    fn test_parse_vscode_theme_with_comments() {
        let json = r##"{
            // Theme name
            "name": "Commented Theme",
            /* Colors section */
            "colors": {
                "editor.background": "#282c34"
            },
            "tokenColors": []
        }"##;

        let path = Path::new("test.json");
        let theme = parse_vscode_theme(json, path).unwrap();
        assert_eq!(theme.name, "Commented Theme");
        assert_eq!(theme.colors.background, Some("#282c34".to_string()));
    }

    // ── Notepad++ theme parsing ──

    #[test]
    fn test_parse_notepadpp_theme_basic() {
        let xml = r##"<?xml version="1.0" encoding="UTF-8" ?>
<NotepadPlus>
    <GlobalStyles>
        <WidgetStyle name="Default Style" styleID="0" fgColor="D4D4D4" bgColor="1E1E1E" fontName="Consolas" fontStyle="0" fontSize="10" />
        <WidgetStyle name="Caret colour" styleID="2069" fgColor="AEAFAD" bgColor="" />
        <WidgetStyle name="Current line background colour" styleID="0" fgColor="" bgColor="2A2D2E" />
        <WidgetStyle name="Selected text colour" styleID="0" fgColor="" bgColor="264F78" />
        <WidgetStyle name="Line number" styleID="33" fgColor="858585" bgColor="1E1E1E" />
    </GlobalStyles>
    <LexerStyles>
        <LexerType name="cpp" desc="C/C++">
            <WordsStyle name="KEYWORD" styleID="5" fgColor="569CD6" bgColor="" fontStyle="1" />
            <WordsStyle name="COMMENT" styleID="1" fgColor="6A9955" bgColor="" fontStyle="0" />
            <WordsStyle name="STRING" styleID="6" fgColor="CE9178" bgColor="" fontStyle="0" />
            <WordsStyle name="NUMBER" styleID="4" fgColor="B5CEA8" bgColor="" fontStyle="0" />
            <WordsStyle name="OPERATOR" styleID="10" fgColor="D4D4D4" bgColor="" fontStyle="0" />
        </LexerType>
    </LexerStyles>
</NotepadPlus>"##;

        let path = Path::new("DarkTheme.xml");
        let theme = parse_notepadpp_theme(xml, path).unwrap();

        assert_eq!(theme.name, "DarkTheme");
        assert_eq!(theme.base, Some("Dark".to_string()));
        assert_eq!(theme.colors.background, Some("#1e1e1e".to_string()));
        assert_eq!(theme.colors.foreground, Some("#d4d4d4".to_string()));
        assert_eq!(theme.colors.cursor, Some("#aeafad".to_string()));
        assert_eq!(theme.colors.current_line_bg, Some("#2a2d2e".to_string()));
        assert_eq!(theme.colors.selection_bg, Some("#264f78".to_string()));
        assert_eq!(theme.colors.gutter_fg, Some("#858585".to_string()));
        assert_eq!(theme.colors.gutter_bg, Some("#1e1e1e".to_string()));

        assert_eq!(theme.syntax.keyword, Some("#569cd6".to_string()));
        assert_eq!(theme.syntax.comment, Some("#6a9955".to_string()));
        assert_eq!(theme.syntax.string, Some("#ce9178".to_string()));
        assert_eq!(theme.syntax.number, Some("#b5cea8".to_string()));
        assert_eq!(theme.syntax.operator, Some("#d4d4d4".to_string()));
    }

    #[test]
    fn test_parse_notepadpp_theme_light() {
        let xml = r##"<?xml version="1.0" encoding="UTF-8" ?>
<NotepadPlus>
    <GlobalStyles>
        <WidgetStyle name="Default Style" styleID="0" fgColor="000000" bgColor="FFFFFF" />
    </GlobalStyles>
    <LexerStyles>
    </LexerStyles>
</NotepadPlus>"##;

        let path = Path::new("LightTheme.xml");
        let theme = parse_notepadpp_theme(xml, path).unwrap();
        assert_eq!(theme.base, Some("Light".to_string()));
    }

    // ── npp_color_to_hex ──

    #[test]
    fn test_npp_color_to_hex_normal() {
        assert_eq!(npp_color_to_hex("FF0000"), Some("#ff0000".to_string()));
        assert_eq!(npp_color_to_hex("1E1E1E"), Some("#1e1e1e".to_string()));
    }

    #[test]
    fn test_npp_color_to_hex_short() {
        // Short values should be zero-padded.
        assert_eq!(npp_color_to_hex("FF"), Some("#0000ff".to_string()));
    }

    #[test]
    fn test_npp_color_to_hex_empty() {
        assert_eq!(npp_color_to_hex(""), None);
        assert_eq!(npp_color_to_hex("0"), None);
    }

    // ── is_light_color ──

    #[test]
    fn test_is_light_color() {
        assert!(is_light_color("#ffffff"));
        assert!(is_light_color("#f0f0f0"));
        assert!(!is_light_color("#1e1e1e"));
        assert!(!is_light_color("#000000"));
    }

    // ── Full conversion roundtrip ──

    #[test]
    fn test_vscode_to_toml_roundtrip() {
        let json = r##"{
            "name": "Roundtrip Test",
            "type": "dark",
            "colors": {
                "editor.background": "#282c34",
                "editor.foreground": "#abb2bf"
            },
            "tokenColors": [
                {
                    "scope": "keyword",
                    "settings": { "foreground": "#c678dd" }
                },
                {
                    "scope": "comment",
                    "settings": { "foreground": "#5c6370" }
                }
            ]
        }"##;

        let path = Path::new("roundtrip.json");
        let theme_file = parse_vscode_theme(json, path).unwrap();

        // Serialize to TOML.
        let toml_str = toml::to_string_pretty(&theme_file).unwrap();

        // Parse back.
        let parsed: ThemeFile = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.name, "Roundtrip Test");
        assert_eq!(parsed.colors.background, Some("#282c34".to_string()));
        assert_eq!(parsed.syntax.keyword, Some("#c678dd".to_string()));
        assert_eq!(parsed.syntax.comment, Some("#5c6370".to_string()));
    }

    #[test]
    fn test_notepadpp_to_toml_roundtrip() {
        let xml = r##"<?xml version="1.0" encoding="UTF-8" ?>
<NotepadPlus>
    <GlobalStyles>
        <WidgetStyle name="Default Style" styleID="0" fgColor="ABB2BF" bgColor="282C34" />
    </GlobalStyles>
    <LexerStyles>
        <LexerType name="cpp" desc="C/C++">
            <WordsStyle name="KEYWORD" styleID="5" fgColor="C678DD" bgColor="" />
        </LexerType>
    </LexerStyles>
</NotepadPlus>"##;

        let path = Path::new("roundtrip.xml");
        let theme_file = parse_notepadpp_theme(xml, path).unwrap();

        let toml_str = toml::to_string_pretty(&theme_file).unwrap();
        let parsed: ThemeFile = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.colors.background, Some("#282c34".to_string()));
        assert_eq!(parsed.syntax.keyword, Some("#c678dd".to_string()));
    }

    #[test]
    fn test_parse_vscode_theme_missing_sections() {
        // Theme with no tokenColors and no colors should still parse.
        let json = r##"{"name": "Minimal"}"##;
        let path = Path::new("minimal.json");
        let theme = parse_vscode_theme(json, path).unwrap();
        assert_eq!(theme.name, "Minimal");
        assert!(theme.syntax.keyword.is_none());
        assert!(theme.colors.background.is_none());
    }

    #[test]
    fn test_parse_vscode_token_no_foreground() {
        // Token color entries without a foreground should be skipped.
        let json = r##"{
            "name": "Test",
            "colors": {},
            "tokenColors": [
                {
                    "scope": "keyword",
                    "settings": {
                        "fontStyle": "bold"
                    }
                }
            ]
        }"##;

        let path = Path::new("test.json");
        let theme = parse_vscode_theme(json, path).unwrap();
        assert!(theme.syntax.keyword.is_none());
    }

    #[test]
    fn test_parse_notepadpp_empty_lexer_styles() {
        let xml = r##"<?xml version="1.0" encoding="UTF-8" ?>
<NotepadPlus>
    <GlobalStyles>
        <WidgetStyle name="Default Style" styleID="0" fgColor="FFFFFF" bgColor="000000" />
    </GlobalStyles>
    <LexerStyles>
    </LexerStyles>
</NotepadPlus>"##;

        let path = Path::new("empty.xml");
        let theme = parse_notepadpp_theme(xml, path).unwrap();
        assert_eq!(theme.colors.foreground, Some("#ffffff".to_string()));
        assert!(theme.syntax.keyword.is_none());
    }
}
