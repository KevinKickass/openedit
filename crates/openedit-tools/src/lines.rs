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

/// Shuffle lines randomly.
pub fn shuffle_lines(text: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut lines: Vec<&str> = text.lines().collect();
    if lines.len() <= 1 {
        return lines.join("\n");
    }

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    for i in (1..lines.len()).rev() {
        let mut hasher = DefaultHasher::new();
        hasher.write_u64(seed.wrapping_mul(i as u64));
        let j = (hasher.finish() as usize) % (i + 1);
        lines.swap(i, j);
    }

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

    #[test]
    fn test_shuffle_lines() {
        let input = "a\nb\nc\nd\ne";
        let result = shuffle_lines(input);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 5);
        assert!(lines.contains(&"a"));
        assert!(lines.contains(&"b"));
        assert!(lines.contains(&"c"));
        assert!(lines.contains(&"d"));
        assert!(lines.contains(&"e"));
    }

    #[test]
    fn test_shuffle_lines_single() {
        assert_eq!(shuffle_lines("a"), "a");
    }
}
