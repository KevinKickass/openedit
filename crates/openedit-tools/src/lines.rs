/// Remove duplicate lines (preserves first occurrence, maintains order).
pub fn remove_duplicates(text: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    text.lines()
        .filter(|line| seen.insert(*line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Remove empty lines.
pub fn remove_empty_lines(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Join all lines into a single line with a space separator.
pub fn join_lines(text: &str) -> String {
    text.lines().collect::<Vec<_>>().join(" ")
}

/// Reverse the order of lines.
pub fn reverse_lines(text: &str) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    lines.reverse();
    lines.join("\n")
}

/// Trim trailing whitespace from each line.
pub fn trim_trailing(text: &str) -> String {
    text.lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_duplicates() {
        assert_eq!(remove_duplicates("a\nb\na\nc\nb"), "a\nb\nc");
    }

    #[test]
    fn test_remove_empty_lines() {
        assert_eq!(remove_empty_lines("a\n\nb\n  \nc"), "a\nb\nc");
    }

    #[test]
    fn test_join_lines() {
        assert_eq!(join_lines("a\nb\nc"), "a b c");
    }

    #[test]
    fn test_reverse_lines() {
        assert_eq!(reverse_lines("a\nb\nc"), "c\nb\na");
    }

    #[test]
    fn test_trim_trailing() {
        assert_eq!(trim_trailing("a  \nb\t\nc"), "a\nb\nc");
    }
}
