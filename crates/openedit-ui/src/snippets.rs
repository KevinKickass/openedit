use openedit_core::cursor::Position;
use openedit_core::Document;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single snippet definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    /// Tab trigger prefix (e.g., "fn", "for", "if").
    pub trigger: String,
    /// Human-readable label shown in completions.
    pub label: String,
    /// The snippet body with placeholders ($1, $2, $0).
    pub body: String,
    /// Language this snippet applies to (e.g., "Rust", "Python").
    pub language: String,
}

/// A placeholder in an expanded snippet.
#[derive(Debug, Clone)]
pub struct SnippetPlaceholder {
    /// Placeholder index ($1, $2, etc.; $0 is final cursor position).
    pub index: usize,
    /// Default text for this placeholder.
    pub default_text: String,
    /// Line offset from snippet start.
    pub line_offset: usize,
    /// Column offset within the line.
    pub col_offset: usize,
    /// Length of the default text.
    pub length: usize,
}

/// State for active snippet expansion and tab-stop navigation.
pub struct SnippetState {
    /// Whether a snippet is currently being navigated.
    pub active: bool,
    /// Placeholders sorted by index.
    pub placeholders: Vec<SnippetPlaceholder>,
    /// Current placeholder index being edited.
    pub current_index: usize,
    /// The document position where the snippet was inserted.
    pub insert_position: Position,
}

impl Default for SnippetState {
    fn default() -> Self {
        Self {
            active: false,
            placeholders: Vec::new(),
            current_index: 1,
            insert_position: Position::zero(),
        }
    }
}

impl SnippetState {
    /// Returns positions and lengths of all placeholders for visual highlighting.
    /// Each entry is (Position, length, is_current).
    /// `is_current` is true for the placeholder that was most recently navigated to
    /// (i.e., current_index - 1, since current_index is incremented after navigation).
    pub fn placeholder_positions(&self) -> Vec<(Position, usize, bool)> {
        if !self.active {
            return Vec::new();
        }
        // The current placeholder being edited is current_index - 1
        // (since next_placeholder increments it after returning).
        let active_idx = self.current_index.saturating_sub(1);
        self.placeholders
            .iter()
            .filter(|ph| ph.index != 0) // Don't highlight $0 (final position)
            .map(|ph| {
                let line = self.insert_position.line + ph.line_offset;
                let col = if ph.line_offset == 0 {
                    self.insert_position.col + ph.col_offset
                } else {
                    ph.col_offset
                };
                let is_current = ph.index == active_idx;
                (Position::new(line, col), ph.length, is_current)
            })
            .collect()
    }

    /// Advance to the next placeholder. Returns the target position, or None if done.
    pub fn next_placeholder(&mut self) -> Option<(Position, usize)> {
        if !self.active {
            return None;
        }

        // Find placeholder with current_index
        let ph = self
            .placeholders
            .iter()
            .find(|p| p.index == self.current_index);
        if let Some(ph) = ph {
            let line = self.insert_position.line + ph.line_offset;
            let col = if ph.line_offset == 0 {
                self.insert_position.col + ph.col_offset
            } else {
                ph.col_offset
            };
            let result = (Position::new(line, col), ph.length);
            self.current_index += 1;
            Some(result)
        } else {
            // Check for $0 (final position)
            if let Some(ph) = self.placeholders.iter().find(|p| p.index == 0) {
                let line = self.insert_position.line + ph.line_offset;
                let col = if ph.line_offset == 0 {
                    self.insert_position.col + ph.col_offset
                } else {
                    ph.col_offset
                };
                self.active = false;
                Some((Position::new(line, col), 0))
            } else {
                self.active = false;
                None
            }
        }
    }

    /// Go back to the previous placeholder.
    pub fn prev_placeholder(&mut self) -> Option<(Position, usize)> {
        if !self.active || self.current_index <= 2 {
            return None;
        }
        self.current_index -= 2;
        self.next_placeholder()
    }
}

/// Expand a snippet body, resolving placeholders and returning the expanded text
/// and placeholder positions.
pub fn expand_snippet(body: &str) -> (String, Vec<SnippetPlaceholder>) {
    let mut result = String::new();
    let mut placeholders = Vec::new();
    let mut line_offset = 0;
    let mut col_offset = 0;
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() {
            if chars[i + 1] == '{' {
                // ${N:default} or ${N|choice1,choice2|}
                if let Some(close) = find_closing_brace(&chars, i + 2) {
                    let inner: String = chars[i + 2..close].iter().collect();
                    if let Some(colon_pos) = inner.find(':') {
                        let idx_str = &inner[..colon_pos];
                        let default = &inner[colon_pos + 1..];
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            placeholders.push(SnippetPlaceholder {
                                index: idx,
                                default_text: default.to_string(),
                                line_offset,
                                col_offset,
                                length: default.len(),
                            });
                            result.push_str(default);
                            col_offset += default.len();
                        }
                    } else if let Some(pipe_start) = inner.find('|') {
                        // Choice placeholder: ${N|choice1,choice2|}
                        let idx_str = &inner[..pipe_start];
                        let choices_str = inner[pipe_start + 1..].trim_end_matches('|');
                        let first_choice = choices_str.split(',').next().unwrap_or("");
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            placeholders.push(SnippetPlaceholder {
                                index: idx,
                                default_text: first_choice.to_string(),
                                line_offset,
                                col_offset,
                                length: first_choice.len(),
                            });
                            result.push_str(first_choice);
                            col_offset += first_choice.len();
                        }
                    } else if let Ok(idx) = inner.parse::<usize>() {
                        placeholders.push(SnippetPlaceholder {
                            index: idx,
                            default_text: String::new(),
                            line_offset,
                            col_offset,
                            length: 0,
                        });
                    }
                    i = close + 1;
                    continue;
                }
            } else if chars[i + 1].is_ascii_digit() {
                // Simple $N placeholder
                let mut end = i + 1;
                while end < chars.len() && chars[end].is_ascii_digit() {
                    end += 1;
                }
                let idx_str: String = chars[i + 1..end].iter().collect();
                if let Ok(idx) = idx_str.parse::<usize>() {
                    placeholders.push(SnippetPlaceholder {
                        index: idx,
                        default_text: String::new(),
                        line_offset,
                        col_offset,
                        length: 0,
                    });
                }
                i = end;
                continue;
            }
        }

        if chars[i] == '\n' {
            result.push('\n');
            line_offset += 1;
            col_offset = 0;
        } else {
            result.push(chars[i]);
            col_offset += 1;
        }
        i += 1;
    }

    // Sort placeholders by index
    placeholders.sort_by_key(|p| p.index);

    (result, placeholders)
}

fn find_closing_brace(chars: &[char], start: usize) -> Option<usize> {
    let mut depth = 1;
    let mut i = start;
    while i < chars.len() {
        if chars[i] == '{' {
            depth += 1;
        } else if chars[i] == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Get all built-in snippets.
pub fn builtin_snippets() -> Vec<Snippet> {
    let mut snippets = Vec::new();

    // Rust snippets
    snippets.extend(vec![
        Snippet {
            trigger: "fn".into(),
            label: "Function definition".into(),
            body: "fn ${1:name}(${2:params}) ${3:-> ${4:ReturnType} }{\n    $0\n}".into(),
            language: "Rust".into(),
        },
        Snippet {
            trigger: "pfn".into(),
            label: "Public function".into(),
            body: "pub fn ${1:name}(${2:params}) ${3:-> ${4:ReturnType} }{\n    $0\n}".into(),
            language: "Rust".into(),
        },
        Snippet {
            trigger: "struct".into(),
            label: "Struct definition".into(),
            body: "struct ${1:Name} {\n    ${2:field}: ${3:Type},\n}".into(),
            language: "Rust".into(),
        },
        Snippet {
            trigger: "impl".into(),
            label: "Impl block".into(),
            body: "impl ${1:Type} {\n    $0\n}".into(),
            language: "Rust".into(),
        },
        Snippet {
            trigger: "enum".into(),
            label: "Enum definition".into(),
            body: "enum ${1:Name} {\n    ${2:Variant},\n}".into(),
            language: "Rust".into(),
        },
        Snippet {
            trigger: "match".into(),
            label: "Match expression".into(),
            body: "match ${1:expr} {\n    ${2:pattern} => ${3:result},\n}".into(),
            language: "Rust".into(),
        },
        Snippet {
            trigger: "test".into(),
            label: "Test function".into(),
            body: "#[test]\nfn ${1:test_name}() {\n    $0\n}".into(),
            language: "Rust".into(),
        },
        Snippet {
            trigger: "for".into(),
            label: "For loop".into(),
            body: "for ${1:item} in ${2:iter} {\n    $0\n}".into(),
            language: "Rust".into(),
        },
        Snippet {
            trigger: "if".into(),
            label: "If statement".into(),
            body: "if ${1:condition} {\n    $0\n}".into(),
            language: "Rust".into(),
        },
        Snippet {
            trigger: "iflet".into(),
            label: "If let".into(),
            body: "if let ${1:Some(val)} = ${2:expr} {\n    $0\n}".into(),
            language: "Rust".into(),
        },
    ]);

    // Python snippets
    snippets.extend(vec![
        Snippet {
            trigger: "def".into(),
            label: "Function definition".into(),
            body: "def ${1:name}(${2:params}):\n    ${0:pass}".into(),
            language: "Python".into(),
        },
        Snippet {
            trigger: "class".into(),
            label: "Class definition".into(),
            body: "class ${1:Name}:\n    def __init__(self${2:, params}):\n        ${0:pass}"
                .into(),
            language: "Python".into(),
        },
        Snippet {
            trigger: "for".into(),
            label: "For loop".into(),
            body: "for ${1:item} in ${2:iterable}:\n    $0".into(),
            language: "Python".into(),
        },
        Snippet {
            trigger: "if".into(),
            label: "If statement".into(),
            body: "if ${1:condition}:\n    $0".into(),
            language: "Python".into(),
        },
        Snippet {
            trigger: "with".into(),
            label: "With statement".into(),
            body: "with ${1:expr} as ${2:var}:\n    $0".into(),
            language: "Python".into(),
        },
        Snippet {
            trigger: "try".into(),
            label: "Try/except".into(),
            body: "try:\n    ${1:pass}\nexcept ${2:Exception} as ${3:e}:\n    $0".into(),
            language: "Python".into(),
        },
        Snippet {
            trigger: "main".into(),
            label: "Main guard".into(),
            body: "if __name__ == \"__main__\":\n    ${0:main()}".into(),
            language: "Python".into(),
        },
        Snippet {
            trigger: "lam".into(),
            label: "Lambda".into(),
            body: "lambda ${1:x}: ${0:x}".into(),
            language: "Python".into(),
        },
    ]);

    // TypeScript snippets
    snippets.extend(vec![
        Snippet {
            trigger: "fn".into(),
            label: "Function".into(),
            body: "function ${1:name}(${2:params}): ${3:void} {\n    $0\n}".into(),
            language: "TypeScript".into(),
        },
        Snippet {
            trigger: "afn".into(),
            label: "Arrow function".into(),
            body: "const ${1:name} = (${2:params}): ${3:void} => {\n    $0\n};".into(),
            language: "TypeScript".into(),
        },
        Snippet {
            trigger: "class".into(),
            label: "Class".into(),
            body: "class ${1:Name} {\n    constructor(${2:params}) {\n        $0\n    }\n}".into(),
            language: "TypeScript".into(),
        },
        Snippet {
            trigger: "interface".into(),
            label: "Interface".into(),
            body: "interface ${1:Name} {\n    ${2:prop}: ${3:type};\n}".into(),
            language: "TypeScript".into(),
        },
        Snippet {
            trigger: "for".into(),
            label: "For loop".into(),
            body: "for (let ${1:i} = 0; ${1:i} < ${2:length}; ${1:i}++) {\n    $0\n}".into(),
            language: "TypeScript".into(),
        },
        Snippet {
            trigger: "forof".into(),
            label: "For...of loop".into(),
            body: "for (const ${1:item} of ${2:iterable}) {\n    $0\n}".into(),
            language: "TypeScript".into(),
        },
        Snippet {
            trigger: "if".into(),
            label: "If statement".into(),
            body: "if (${1:condition}) {\n    $0\n}".into(),
            language: "TypeScript".into(),
        },
        Snippet {
            trigger: "try".into(),
            label: "Try/catch".into(),
            body: "try {\n    ${1}\n} catch (${2:error}) {\n    $0\n}".into(),
            language: "TypeScript".into(),
        },
    ]);

    // Go snippets
    snippets.extend(vec![
        Snippet {
            trigger: "func".into(),
            label: "Function".into(),
            body: "func ${1:name}(${2:params}) ${3:error} {\n\t$0\n}".into(),
            language: "Go".into(),
        },
        Snippet {
            trigger: "main".into(),
            label: "Main function".into(),
            body: "func main() {\n\t$0\n}".into(),
            language: "Go".into(),
        },
        Snippet {
            trigger: "struct".into(),
            label: "Struct".into(),
            body: "type ${1:Name} struct {\n\t${2:Field} ${3:Type}\n}".into(),
            language: "Go".into(),
        },
        Snippet {
            trigger: "interface".into(),
            label: "Interface".into(),
            body: "type ${1:Name} interface {\n\t${2:Method}(${3:params}) ${4:error}\n}".into(),
            language: "Go".into(),
        },
        Snippet {
            trigger: "for".into(),
            label: "For loop".into(),
            body: "for ${1:i} := 0; ${1:i} < ${2:n}; ${1:i}++ {\n\t$0\n}".into(),
            language: "Go".into(),
        },
        Snippet {
            trigger: "forr".into(),
            label: "For range".into(),
            body: "for ${1:i}, ${2:v} := range ${3:collection} {\n\t$0\n}".into(),
            language: "Go".into(),
        },
        Snippet {
            trigger: "if".into(),
            label: "If statement".into(),
            body: "if ${1:condition} {\n\t$0\n}".into(),
            language: "Go".into(),
        },
        Snippet {
            trigger: "iferr".into(),
            label: "If error".into(),
            body: "if err != nil {\n\t${0:return err}\n}".into(),
            language: "Go".into(),
        },
    ]);

    snippets
}

/// Find matching snippets for the current word being typed.
pub fn find_matching_snippets(trigger: &str, language: &str) -> Vec<Snippet> {
    builtin_snippets()
        .into_iter()
        .filter(|s| {
            s.trigger.starts_with(trigger)
                && (s.language.eq_ignore_ascii_case(language) || language.is_empty())
        })
        .collect()
}

/// Returns the path to the user snippets JSON file in the platform config directory.
pub fn user_snippets_path() -> Option<PathBuf> {
    let config_dir = if cfg!(target_os = "windows") {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
    } else {
        // Linux and other Unix
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".config"))
            })
    };
    config_dir.map(|d| d.join("openedit").join("snippets.json"))
}

/// Load user-defined snippets from the JSON file.
/// Returns an empty Vec if the file does not exist or cannot be parsed.
pub fn load_user_snippets() -> Vec<Snippet> {
    let Some(path) = user_snippets_path() else {
        return Vec::new();
    };
    match std::fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<Vec<Snippet>>(&content) {
            Ok(snippets) => {
                log::info!(
                    "Loaded {} user snippets from {}",
                    snippets.len(),
                    path.display()
                );
                snippets
            }
            Err(e) => {
                log::warn!("Failed to parse user snippets at {}: {}", path.display(), e);
                Vec::new()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => {
            log::warn!("Failed to read user snippets at {}: {}", path.display(), e);
            Vec::new()
        }
    }
}

/// Ensure the user snippets file exists, creating it with example content if needed.
/// Returns the path to the file.
pub fn ensure_user_snippets_file() -> Option<PathBuf> {
    let path = user_snippets_path()?;
    if !path.exists() {
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                log::error!(
                    "Failed to create config directory {}: {}",
                    parent.display(),
                    e
                );
                return Some(path);
            }
        }
        let example_snippets = vec![
            Snippet {
                trigger: "todo".into(),
                label: "TODO comment".into(),
                body: "// TODO: ${1:description}$0".into(),
                language: "Rust".into(),
            },
            Snippet {
                trigger: "dbg".into(),
                label: "Debug print".into(),
                body: "println!(\"DEBUG: {} = {:?}\", stringify!(${1:var}), ${1:var});$0".into(),
                language: "Rust".into(),
            },
        ];
        match serde_json::to_string_pretty(&example_snippets) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&path, content) {
                    log::error!(
                        "Failed to write example snippets to {}: {}",
                        path.display(),
                        e
                    );
                }
            }
            Err(e) => {
                log::error!("Failed to serialize example snippets: {}", e);
            }
        }
    }
    Some(path)
}

/// Merge built-in and user snippets. User snippets with the same trigger+language
/// take precedence over built-in ones.
fn merge_snippets(builtin: Vec<Snippet>, user: Vec<Snippet>) -> Vec<Snippet> {
    let mut result = builtin;
    for user_snippet in user {
        // Remove any built-in snippet with the same trigger+language (case insensitive)
        result.retain(|s| {
            !(s.trigger == user_snippet.trigger
                && s.language.eq_ignore_ascii_case(&user_snippet.language))
        });
        result.push(user_snippet);
    }
    result
}

/// Snippet engine that manages snippet lookup and expansion.
pub struct SnippetEngine {
    snippets: Vec<Snippet>,
    pub state: SnippetState,
}

impl Default for SnippetEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SnippetEngine {
    pub fn new() -> Self {
        let builtin = builtin_snippets();
        let user = load_user_snippets();
        Self {
            snippets: merge_snippets(builtin, user),
            state: SnippetState::default(),
        }
    }

    /// Reload user snippets from disk and re-merge with built-in snippets.
    pub fn reload_user_snippets(&mut self) {
        let builtin = builtin_snippets();
        let user = load_user_snippets();
        self.snippets = merge_snippets(builtin, user);
    }

    /// Try to expand a snippet at the current cursor position.
    /// Returns true if a snippet was expanded.
    pub fn try_expand(&mut self, doc: &mut Document) -> bool {
        let pos = doc.cursors.primary().position;
        let language = doc.language.as_deref().unwrap_or("");

        // Get the word before cursor
        let line = doc.buffer.line(pos.line).to_string();
        let line_chars: Vec<char> = line.chars().collect();
        let mut word_start = pos.col;
        while word_start > 0
            && line_chars
                .get(word_start - 1)
                .is_some_and(|c| c.is_alphanumeric() || *c == '_')
        {
            word_start -= 1;
        }
        let word: String = line_chars[word_start..pos.col].iter().collect();

        if word.is_empty() {
            return false;
        }

        // Find matching snippet
        let snippet = self.snippets.iter().find(|s| {
            s.trigger == word
                && (s.language == language || s.language.eq_ignore_ascii_case(language))
        });

        let snippet = match snippet {
            Some(s) => s.clone(),
            None => return false,
        };

        // Delete the trigger word
        doc.cursors.primary_mut().anchor = Some(Position::new(pos.line, word_start));
        doc.delete_selection_public();

        // Expand the snippet body
        let (expanded, placeholders) = expand_snippet(&snippet.body);
        let insert_pos = doc.cursors.primary().position;

        doc.insert_text(&expanded);

        // Set up placeholder navigation
        if !placeholders.is_empty() {
            self.state = SnippetState {
                active: true,
                placeholders,
                current_index: 1,
                insert_position: insert_pos,
            };

            // Jump to first placeholder
            if let Some((target, len)) = self.state.next_placeholder() {
                doc.cursors.primary_mut().move_to(target, false);
                if len > 0 {
                    doc.cursors.primary_mut().anchor = Some(target);
                    doc.cursors.primary_mut().position =
                        Position::new(target.line, target.col + len);
                }
            }
        }

        true
    }

    /// Navigate to the next snippet placeholder.
    pub fn next_placeholder(&mut self, doc: &mut Document) -> bool {
        if !self.state.active {
            return false;
        }
        if let Some((target, len)) = self.state.next_placeholder() {
            doc.cursors.primary_mut().move_to(target, false);
            if len > 0 {
                doc.cursors.primary_mut().anchor = Some(target);
                doc.cursors.primary_mut().position = Position::new(target.line, target.col + len);
            }
            true
        } else {
            false
        }
    }

    /// Navigate to the previous snippet placeholder.
    pub fn prev_placeholder(&mut self, doc: &mut Document) -> bool {
        if !self.state.active {
            return false;
        }
        if let Some((target, len)) = self.state.prev_placeholder() {
            doc.cursors.primary_mut().move_to(target, false);
            if len > 0 {
                doc.cursors.primary_mut().anchor = Some(target);
                doc.cursors.primary_mut().position = Position::new(target.line, target.col + len);
            }
            true
        } else {
            false
        }
    }

    /// Cancel active snippet navigation.
    pub fn cancel(&mut self) {
        self.state.active = false;
    }

    /// Whether a snippet is currently being navigated.
    pub fn is_active(&self) -> bool {
        self.state.active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_simple_placeholder() {
        let (text, phs) = expand_snippet("fn ${1:name}() {\n    $0\n}");
        assert_eq!(text, "fn name() {\n    \n}");
        assert_eq!(phs.len(), 2);
        assert_eq!(phs[0].index, 0);
        assert_eq!(phs[1].index, 1);
    }

    #[test]
    fn test_expand_numbered_placeholders() {
        let (text, phs) = expand_snippet("for ${1:i} in ${2:iter} {\n    $0\n}");
        assert_eq!(text, "for i in iter {\n    \n}");
        assert!(phs.len() >= 3);
    }

    #[test]
    fn test_snippet_trigger_match() {
        let snippets = builtin_snippets();
        let rust_fn = snippets
            .iter()
            .find(|s| s.trigger == "fn" && s.language == "Rust");
        assert!(rust_fn.is_some());
    }

    #[test]
    fn test_builtin_snippet_counts() {
        let snippets = builtin_snippets();
        let rust_count = snippets.iter().filter(|s| s.language == "Rust").count();
        let python_count = snippets.iter().filter(|s| s.language == "Python").count();
        let ts_count = snippets
            .iter()
            .filter(|s| s.language == "TypeScript")
            .count();
        let go_count = snippets.iter().filter(|s| s.language == "Go").count();
        assert!(rust_count >= 8);
        assert!(python_count >= 6);
        assert!(ts_count >= 6);
        assert!(go_count >= 6);
    }

    #[test]
    fn test_choice_placeholder() {
        let (text, phs) = expand_snippet("${1|public,private|}");
        assert_eq!(text, "public");
        assert_eq!(phs.len(), 1);
        assert_eq!(phs[0].default_text, "public");
    }

    #[test]
    fn test_snippet_engine_expand() {
        let mut engine = SnippetEngine::new();
        let mut doc = Document::from_str("fn");
        doc.language = Some("Rust".to_string());
        // Move cursor to end of "fn"
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 2), false);

        let expanded = engine.try_expand(&mut doc);
        assert!(expanded);
        assert!(engine.is_active());
    }

    #[test]
    fn test_snippet_placeholder_navigation() {
        let mut state = SnippetState {
            active: true,
            placeholders: vec![
                SnippetPlaceholder {
                    index: 1,
                    default_text: "name".into(),
                    line_offset: 0,
                    col_offset: 3,
                    length: 4,
                },
                SnippetPlaceholder {
                    index: 2,
                    default_text: "params".into(),
                    line_offset: 0,
                    col_offset: 8,
                    length: 6,
                },
                SnippetPlaceholder {
                    index: 0,
                    default_text: String::new(),
                    line_offset: 1,
                    col_offset: 4,
                    length: 0,
                },
            ],
            current_index: 1,
            insert_position: Position::new(0, 0),
        };

        let first = state.next_placeholder();
        assert!(first.is_some());
        let (pos, len) = first.unwrap();
        assert_eq!(pos, Position::new(0, 3));
        assert_eq!(len, 4);

        let second = state.next_placeholder();
        assert!(second.is_some());

        let final_pos = state.next_placeholder();
        assert!(final_pos.is_some());
        assert!(!state.active); // Should deactivate after $0
    }

    #[test]
    fn test_placeholder_positions_inactive() {
        let state = SnippetState::default();
        let positions = state.placeholder_positions();
        assert!(positions.is_empty());
    }

    #[test]
    fn test_placeholder_positions_active() {
        let state = SnippetState {
            active: true,
            placeholders: vec![
                SnippetPlaceholder {
                    index: 1,
                    default_text: "name".into(),
                    line_offset: 0,
                    col_offset: 3,
                    length: 4,
                },
                SnippetPlaceholder {
                    index: 2,
                    default_text: "params".into(),
                    line_offset: 0,
                    col_offset: 8,
                    length: 6,
                },
                SnippetPlaceholder {
                    index: 0,
                    default_text: String::new(),
                    line_offset: 1,
                    col_offset: 4,
                    length: 0,
                },
            ],
            current_index: 2, // After navigating to placeholder 1, current_index becomes 2
            insert_position: Position::new(5, 10),
        };

        let positions = state.placeholder_positions();
        // Should have 2 entries (indices 1 and 2; $0 is filtered out)
        assert_eq!(positions.len(), 2);

        // First placeholder (index 1) should be current (active_idx = 2-1 = 1)
        let (pos1, len1, is_current1) = &positions[0];
        assert_eq!(*pos1, Position::new(5, 13)); // line 5, col 10+3
        assert_eq!(*len1, 4);
        assert!(is_current1);

        // Second placeholder (index 2) should not be current
        let (pos2, len2, is_current2) = &positions[1];
        assert_eq!(*pos2, Position::new(5, 18)); // line 5, col 10+8
        assert_eq!(*len2, 6);
        assert!(!is_current2);
    }

    #[test]
    fn test_placeholder_positions_multiline() {
        let state = SnippetState {
            active: true,
            placeholders: vec![SnippetPlaceholder {
                index: 1,
                default_text: "body".into(),
                line_offset: 1,
                col_offset: 4,
                length: 4,
            }],
            current_index: 2, // After navigating past placeholder 1
            insert_position: Position::new(0, 0),
        };

        let positions = state.placeholder_positions();
        assert_eq!(positions.len(), 1);
        let (pos, len, is_current) = &positions[0];
        // line_offset=1 means second line, col_offset=4 (not added to insert col)
        assert_eq!(*pos, Position::new(1, 4));
        assert_eq!(*len, 4);
        assert!(is_current);
    }

    #[test]
    fn test_merge_snippets_user_override() {
        let builtin = vec![
            Snippet {
                trigger: "fn".into(),
                label: "Built-in function".into(),
                body: "fn $1() {}".into(),
                language: "Rust".into(),
            },
            Snippet {
                trigger: "for".into(),
                label: "Built-in for".into(),
                body: "for $1 in $2 {}".into(),
                language: "Rust".into(),
            },
        ];
        let user = vec![Snippet {
            trigger: "fn".into(),
            label: "My custom function".into(),
            body: "pub fn $1() { $0 }".into(),
            language: "Rust".into(),
        }];

        let merged = merge_snippets(builtin, user);
        // Should have 2 snippets: user "fn" replaced built-in "fn", plus built-in "for"
        assert_eq!(merged.len(), 2);
        let fn_snippet = merged.iter().find(|s| s.trigger == "fn").unwrap();
        assert_eq!(fn_snippet.label, "My custom function");
        let for_snippet = merged.iter().find(|s| s.trigger == "for").unwrap();
        assert_eq!(for_snippet.label, "Built-in for");
    }

    #[test]
    fn test_merge_snippets_different_language() {
        let builtin = vec![Snippet {
            trigger: "fn".into(),
            label: "Rust fn".into(),
            body: "fn $1() {}".into(),
            language: "Rust".into(),
        }];
        let user = vec![Snippet {
            trigger: "fn".into(),
            label: "Python fn".into(),
            body: "def $1(): pass".into(),
            language: "Python".into(),
        }];

        let merged = merge_snippets(builtin, user);
        // Different languages, both should be kept
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn test_snippet_serialization_roundtrip() {
        let snippets = vec![Snippet {
            trigger: "test".into(),
            label: "Test snippet".into(),
            body: "fn ${1:test_name}() {\n    $0\n}".into(),
            language: "Rust".into(),
        }];
        let json = serde_json::to_string_pretty(&snippets).unwrap();
        let deserialized: Vec<Snippet> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized[0].trigger, "test");
        assert_eq!(deserialized[0].body, "fn ${1:test_name}() {\n    $0\n}");
    }

    #[test]
    fn test_user_snippets_path_not_none() {
        // Should return a path on any supported platform
        let path = user_snippets_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("openedit"));
        assert!(path.to_string_lossy().contains("snippets.json"));
    }
}
