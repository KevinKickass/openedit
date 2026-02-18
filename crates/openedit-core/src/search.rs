use regex::Regex;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Invalid regex: {0}")]
    InvalidRegex(String),
}

/// A single search match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    /// Char offset of the match start.
    pub start: usize,
    /// Char offset of the match end (exclusive).
    pub end: usize,
    /// The matched text.
    pub text: String,
}

/// Options for search.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
    pub wrap_around: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            whole_word: false,
            use_regex: false,
            wrap_around: true,
        }
    }
}

/// Search engine for find/replace operations within a buffer.
pub struct SearchEngine {
    compiled_regex: Option<Regex>,
    pub query: String,
    pub options: SearchOptions,
    /// Cached matches (invalidated on buffer change or query change).
    pub matches: Vec<SearchMatch>,
    /// Index of the currently highlighted match.
    pub current_match: Option<usize>,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            compiled_regex: None,
            query: String::new(),
            options: SearchOptions::default(),
            matches: Vec::new(),
            current_match: None,
        }
    }

    /// Set the search query and compile the regex if needed.
    pub fn set_query(&mut self, query: &str) -> Result<(), SearchError> {
        self.query = query.to_string();
        self.compiled_regex = None;
        self.matches.clear();
        self.current_match = None;

        if query.is_empty() {
            return Ok(());
        }

        let pattern = if self.options.use_regex {
            query.to_string()
        } else {
            regex::escape(query)
        };

        let pattern = if self.options.whole_word {
            format!(r"\b{}\b", pattern)
        } else {
            pattern
        };

        let regex = if self.options.case_sensitive {
            Regex::new(&pattern)
        } else {
            Regex::new(&format!("(?i){}", pattern))
        };

        match regex {
            Ok(re) => {
                self.compiled_regex = Some(re);
                Ok(())
            }
            Err(e) => Err(SearchError::InvalidRegex(e.to_string())),
        }
    }

    /// Find all matches in the given text.
    pub fn find_all(&mut self, text: &str) {
        self.matches.clear();
        self.current_match = None;

        if let Some(ref re) = self.compiled_regex {
            // We need to track char offsets, not byte offsets.
            // Build a byte-to-char offset map for match positions.
            for mat in re.find_iter(text) {
                let start_char = text[..mat.start()].chars().count();
                let match_text = mat.as_str();
                let match_chars = match_text.chars().count();
                self.matches.push(SearchMatch {
                    start: start_char,
                    end: start_char + match_chars,
                    text: match_text.to_string(),
                });
            }
        }

        if !self.matches.is_empty() {
            self.current_match = Some(0);
        }
    }

    /// Find the next match after the given char offset.
    pub fn find_next(&mut self, from_offset: usize) -> Option<&SearchMatch> {
        if self.matches.is_empty() {
            return None;
        }

        // Find the first match after from_offset
        let idx = self
            .matches
            .iter()
            .position(|m| m.start >= from_offset);

        let idx = match idx {
            Some(i) => i,
            None if self.options.wrap_around => 0, // wrap to beginning
            None => return None,
        };

        self.current_match = Some(idx);
        Some(&self.matches[idx])
    }

    /// Find the previous match before the given char offset.
    pub fn find_prev(&mut self, from_offset: usize) -> Option<&SearchMatch> {
        if self.matches.is_empty() {
            return None;
        }

        let idx = self
            .matches
            .iter()
            .rposition(|m| m.start < from_offset);

        let idx = match idx {
            Some(i) => i,
            None if self.options.wrap_around => self.matches.len() - 1,
            None => return None,
        };

        self.current_match = Some(idx);
        Some(&self.matches[idx])
    }

    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    pub fn current_match_index(&self) -> Option<usize> {
        self.current_match
    }

    pub fn clear(&mut self) {
        self.query.clear();
        self.compiled_regex = None;
        self.matches.clear();
        self.current_match = None;
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Replace a single match in the text. Returns the new text.
pub fn replace_match(text: &str, search_match: &SearchMatch, replacement: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::new();
    for (i, ch) in chars.iter().enumerate() {
        if i == search_match.start {
            result.push_str(replacement);
        } else if i >= search_match.start && i < search_match.end {
            // skip chars being replaced
        } else {
            result.push(*ch);
        }
    }
    result
}

/// Replace all matches in the text. Returns the new text.
pub fn replace_all(text: &str, matches: &[SearchMatch], replacement: &str) -> String {
    if matches.is_empty() {
        return text.to_string();
    }

    let chars: Vec<char> = text.chars().collect();
    let mut result = String::new();
    let mut match_idx = 0;
    let mut i = 0;

    while i < chars.len() {
        if match_idx < matches.len() && i == matches[match_idx].start {
            result.push_str(replacement);
            i = matches[match_idx].end;
            match_idx += 1;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_search() {
        let mut engine = SearchEngine::new();
        engine.set_query("hello").unwrap();
        engine.find_all("hello world hello");
        assert_eq!(engine.match_count(), 2);
        assert_eq!(engine.matches[0].start, 0);
        assert_eq!(engine.matches[0].end, 5);
        assert_eq!(engine.matches[1].start, 12);
    }

    #[test]
    fn test_case_insensitive() {
        let mut engine = SearchEngine::new();
        engine.options.case_sensitive = false;
        engine.set_query("hello").unwrap();
        engine.find_all("Hello HELLO hello");
        assert_eq!(engine.match_count(), 3);
    }

    #[test]
    fn test_case_sensitive() {
        let mut engine = SearchEngine::new();
        engine.options.case_sensitive = true;
        engine.set_query("hello").unwrap();
        engine.find_all("Hello HELLO hello");
        assert_eq!(engine.match_count(), 1);
    }

    #[test]
    fn test_whole_word() {
        let mut engine = SearchEngine::new();
        engine.options.whole_word = true;
        engine.set_query("he").unwrap();
        engine.find_all("he hello he");
        assert_eq!(engine.match_count(), 2); // "he" at start and end, not "hello"
    }

    #[test]
    fn test_regex_search() {
        let mut engine = SearchEngine::new();
        engine.options.use_regex = true;
        engine.set_query(r"\d+").unwrap();
        engine.find_all("abc 123 def 456");
        assert_eq!(engine.match_count(), 2);
        assert_eq!(engine.matches[0].text, "123");
        assert_eq!(engine.matches[1].text, "456");
    }

    #[test]
    fn test_find_next_wraps() {
        let mut engine = SearchEngine::new();
        engine.set_query("x").unwrap();
        engine.find_all("axbxc");
        let m = engine.find_next(4).unwrap();
        assert_eq!(m.start, 1); // wrapped to first match
    }

    #[test]
    fn test_replace_match() {
        let result = replace_match(
            "hello world",
            &SearchMatch {
                start: 6,
                end: 11,
                text: "world".into(),
            },
            "rust",
        );
        assert_eq!(result, "hello rust");
    }

    #[test]
    fn test_replace_all() {
        let matches = vec![
            SearchMatch {
                start: 0,
                end: 3,
                text: "foo".into(),
            },
            SearchMatch {
                start: 4,
                end: 7,
                text: "foo".into(),
            },
        ];
        let result = replace_all("foo foo bar", &matches, "baz");
        assert_eq!(result, "baz baz bar");
    }
}
