/// Sort lines ascending (case-sensitive).
pub fn sort_lines_asc(text: &str) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    lines.sort();
    lines.join("\n")
}

/// Sort lines descending (case-sensitive).
pub fn sort_lines_desc(text: &str) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    lines.sort();
    lines.reverse();
    lines.join("\n")
}

/// Sort lines case-insensitive.
pub fn sort_lines_case_insensitive(text: &str) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    lines.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    lines.join("\n")
}

/// Sort lines numerically (by leading number).
pub fn sort_lines_numeric(text: &str) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    lines.sort_by(|a, b| {
        let num_a: f64 = a.trim().split_whitespace().next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(f64::MAX);
        let num_b: f64 = b.trim().split_whitespace().next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(f64::MAX);
        num_a.partial_cmp(&num_b).unwrap_or(std::cmp::Ordering::Equal)
    });
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_asc() {
        assert_eq!(sort_lines_asc("cherry\napple\nbanana"), "apple\nbanana\ncherry");
    }

    #[test]
    fn test_sort_desc() {
        assert_eq!(sort_lines_desc("apple\nbanana\ncherry"), "cherry\nbanana\napple");
    }

    #[test]
    fn test_sort_numeric() {
        assert_eq!(sort_lines_numeric("10 ten\n2 two\n1 one"), "1 one\n2 two\n10 ten");
    }
}
