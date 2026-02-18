/// A single diff operation representing one line in the comparison.
#[derive(Debug, Clone, PartialEq)]
pub enum DiffOp {
    /// Line present in both documents (unchanged).
    Equal(String),
    /// Line only in the right (new) document (addition).
    Insert(String),
    /// Line only in the left (old) document (deletion).
    Delete(String),
}

/// Compute a line-based diff between two strings using the LCS (Longest Common Subsequence)
/// algorithm. Returns a sequence of `DiffOp` values describing how to transform `left` into
/// `right`.
pub fn diff_lines(left: &str, right: &str) -> Vec<DiffOp> {
    let left_lines: Vec<&str> = split_lines(left);
    let right_lines: Vec<&str> = split_lines(right);

    let m = left_lines.len();
    let n = right_lines.len();

    // Build LCS table
    // lcs[i][j] = length of LCS of left_lines[0..i] and right_lines[0..j]
    let mut lcs = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if left_lines[i - 1] == right_lines[j - 1] {
                lcs[i][j] = lcs[i - 1][j - 1] + 1;
            } else {
                lcs[i][j] = lcs[i - 1][j].max(lcs[i][j - 1]);
            }
        }
    }

    // Backtrack to produce diff ops
    let mut ops = Vec::new();
    let mut i = m;
    let mut j = n;

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && left_lines[i - 1] == right_lines[j - 1] {
            ops.push(DiffOp::Equal(left_lines[i - 1].to_string()));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || lcs[i][j - 1] >= lcs[i - 1][j]) {
            ops.push(DiffOp::Insert(right_lines[j - 1].to_string()));
            j -= 1;
        } else {
            ops.push(DiffOp::Delete(left_lines[i - 1].to_string()));
            i -= 1;
        }
    }

    ops.reverse();
    ops
}

/// Split a string into lines, preserving the content of each line without the trailing newline.
/// An empty string produces an empty vec. A trailing newline does NOT produce an extra empty
/// element (matching intuitive line-by-line diffing behavior).
fn split_lines(s: &str) -> Vec<&str> {
    if s.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<&str> = s.split('\n').collect();
    // If the string ends with a newline, the split produces a trailing empty string; remove it
    if s.ends_with('\n') {
        lines.pop();
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_inputs() {
        let ops = diff_lines("", "");
        assert!(ops.is_empty());
    }

    #[test]
    fn test_left_empty() {
        let ops = diff_lines("", "hello\nworld\n");
        assert_eq!(
            ops,
            vec![
                DiffOp::Insert("hello".to_string()),
                DiffOp::Insert("world".to_string()),
            ]
        );
    }

    #[test]
    fn test_right_empty() {
        let ops = diff_lines("hello\nworld\n", "");
        assert_eq!(
            ops,
            vec![
                DiffOp::Delete("hello".to_string()),
                DiffOp::Delete("world".to_string()),
            ]
        );
    }

    #[test]
    fn test_identical_inputs() {
        let text = "line one\nline two\nline three\n";
        let ops = diff_lines(text, text);
        assert_eq!(
            ops,
            vec![
                DiffOp::Equal("line one".to_string()),
                DiffOp::Equal("line two".to_string()),
                DiffOp::Equal("line three".to_string()),
            ]
        );
    }

    #[test]
    fn test_all_different() {
        let left = "alpha\nbeta\n";
        let right = "gamma\ndelta\n";
        let ops = diff_lines(left, right);
        // No common lines, so all deletes then all inserts (or interleaved)
        // Verify that we get the right counts
        let deletes: Vec<_> = ops.iter().filter(|o| matches!(o, DiffOp::Delete(_))).collect();
        let inserts: Vec<_> = ops.iter().filter(|o| matches!(o, DiffOp::Insert(_))).collect();
        let equals: Vec<_> = ops.iter().filter(|o| matches!(o, DiffOp::Equal(_))).collect();
        assert_eq!(deletes.len(), 2);
        assert_eq!(inserts.len(), 2);
        assert_eq!(equals.len(), 0);
    }

    #[test]
    fn test_mixed_additions_deletions_unchanged() {
        let left = "a\nb\nc\nd\ne\n";
        let right = "a\nc\nd\nf\ne\n";
        let ops = diff_lines(left, right);

        // Expected: Equal(a), Delete(b), Equal(c), Equal(d), Insert(f), Equal(e)
        assert_eq!(
            ops,
            vec![
                DiffOp::Equal("a".to_string()),
                DiffOp::Delete("b".to_string()),
                DiffOp::Equal("c".to_string()),
                DiffOp::Equal("d".to_string()),
                DiffOp::Insert("f".to_string()),
                DiffOp::Equal("e".to_string()),
            ]
        );
    }

    #[test]
    fn test_single_line_change() {
        let left = "hello\n";
        let right = "world\n";
        let ops = diff_lines(left, right);
        assert_eq!(
            ops,
            vec![
                DiffOp::Delete("hello".to_string()),
                DiffOp::Insert("world".to_string()),
            ]
        );
    }

    #[test]
    fn test_multiline_interleaved_changes() {
        let left = "1\n2\n3\n4\n5\n";
        let right = "1\nX\n3\nY\n5\n";
        let ops = diff_lines(left, right);
        assert_eq!(
            ops,
            vec![
                DiffOp::Equal("1".to_string()),
                DiffOp::Delete("2".to_string()),
                DiffOp::Insert("X".to_string()),
                DiffOp::Equal("3".to_string()),
                DiffOp::Delete("4".to_string()),
                DiffOp::Insert("Y".to_string()),
                DiffOp::Equal("5".to_string()),
            ]
        );
    }

    #[test]
    fn test_addition_at_end() {
        let left = "a\nb\n";
        let right = "a\nb\nc\n";
        let ops = diff_lines(left, right);
        assert_eq!(
            ops,
            vec![
                DiffOp::Equal("a".to_string()),
                DiffOp::Equal("b".to_string()),
                DiffOp::Insert("c".to_string()),
            ]
        );
    }

    #[test]
    fn test_no_trailing_newline() {
        let left = "foo\nbar";
        let right = "foo\nbaz";
        let ops = diff_lines(left, right);
        assert_eq!(
            ops,
            vec![
                DiffOp::Equal("foo".to_string()),
                DiffOp::Delete("bar".to_string()),
                DiffOp::Insert("baz".to_string()),
            ]
        );
    }
}
