use serde::{Deserialize, Serialize};

/// Line ending style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineEnding {
    /// Unix-style: \n
    LF,
    /// Windows-style: \r\n
    CRLF,
    /// Old Mac-style: \r
    CR,
}

impl LineEnding {
    pub fn as_str(&self) -> &'static str {
        match self {
            LineEnding::LF => "\n",
            LineEnding::CRLF => "\r\n",
            LineEnding::CR => "\r",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            LineEnding::LF => "LF",
            LineEnding::CRLF => "CRLF",
            LineEnding::CR => "CR",
        }
    }

    /// Detect the dominant line ending in the given text.
    /// Returns LF as default if no line endings found.
    pub fn detect(text: &str) -> Self {
        let mut lf_count = 0usize;
        let mut crlf_count = 0usize;
        let mut cr_count = 0usize;

        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'\r' {
                if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    crlf_count += 1;
                    i += 2;
                } else {
                    cr_count += 1;
                    i += 1;
                }
            } else if bytes[i] == b'\n' {
                lf_count += 1;
                i += 1;
            } else {
                i += 1;
            }
        }

        if crlf_count >= lf_count && crlf_count >= cr_count && crlf_count > 0 {
            LineEnding::CRLF
        } else if cr_count > lf_count && cr_count > 0 {
            LineEnding::CR
        } else {
            LineEnding::LF
        }
    }

    /// Convert all line endings in `text` to this style.
    pub fn normalize(text: &str, target: LineEnding) -> String {
        // First normalize everything to LF
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        match target {
            LineEnding::LF => normalized,
            LineEnding::CRLF => normalized.replace('\n', "\r\n"),
            LineEnding::CR => normalized.replace('\n', "\r"),
        }
    }
}

impl Default for LineEnding {
    fn default() -> Self {
        if cfg!(windows) {
            LineEnding::CRLF
        } else {
            LineEnding::LF
        }
    }
}

impl std::fmt::Display for LineEnding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_lf() {
        assert_eq!(LineEnding::detect("hello\nworld\n"), LineEnding::LF);
    }

    #[test]
    fn test_detect_crlf() {
        assert_eq!(LineEnding::detect("hello\r\nworld\r\n"), LineEnding::CRLF);
    }

    #[test]
    fn test_detect_cr() {
        assert_eq!(LineEnding::detect("hello\rworld\r"), LineEnding::CR);
    }

    #[test]
    fn test_detect_mixed_prefers_majority() {
        // 2 CRLF vs 1 LF → CRLF
        assert_eq!(
            LineEnding::detect("a\r\nb\r\nc\n"),
            LineEnding::CRLF
        );
    }

    #[test]
    fn test_detect_no_newlines() {
        assert_eq!(LineEnding::detect("hello"), LineEnding::LF);
    }

    #[test]
    fn test_normalize_to_crlf() {
        let result = LineEnding::normalize("a\nb\nc", LineEnding::CRLF);
        assert_eq!(result, "a\r\nb\r\nc");
    }

    #[test]
    fn test_normalize_from_mixed() {
        let result = LineEnding::normalize("a\r\nb\nc\rd", LineEnding::LF);
        assert_eq!(result, "a\nb\nc\nd");
    }
}
