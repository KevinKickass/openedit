use std::collections::HashSet;

/// Manages code folding state for a document.
#[derive(Debug, Clone, Default)]
pub struct FoldingState {
    /// Set of line indices where a fold starts (the line with the opening brace/indent).
    /// When folded, all lines from fold_start+1 to fold_end are hidden.
    pub folded_lines: HashSet<usize>,
    /// Cached fold ranges: (start_line, end_line) inclusive.
    /// end_line is the last line that's part of the fold body.
    pub fold_ranges: Vec<FoldRange>,
}

#[derive(Debug, Clone)]
pub struct FoldRange {
    /// The line where the fold marker is shown (the "header" line).
    pub start_line: usize,
    /// The last line hidden by this fold (inclusive).
    pub end_line: usize,
    /// Indentation level of the start line.
    pub indent_level: usize,
}

impl FoldingState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute all foldable regions based on indentation.
    /// A foldable region starts at a line followed by lines with greater indentation.
    pub fn compute_fold_ranges(&mut self, lines: &[String]) {
        self.fold_ranges.clear();
        let indents: Vec<usize> = lines.iter().map(|l| indent_level(l)).collect();
        let n = lines.len();

        for i in 0..n.saturating_sub(1) {
            // Skip blank lines
            if lines[i].trim().is_empty() {
                continue;
            }

            let current_indent = indents[i];
            // Check if next non-blank line has greater indentation
            let mut j = i + 1;
            while j < n && lines[j].trim().is_empty() {
                j += 1;
            }
            if j >= n {
                continue;
            }
            if indents[j] > current_indent {
                // Find the end of this fold: last line before indentation drops back
                let mut end = j;
                for k in (j + 1)..n {
                    if lines[k].trim().is_empty() {
                        continue;
                    }
                    if indents[k] > current_indent {
                        end = k;
                    } else {
                        break;
                    }
                }
                self.fold_ranges.push(FoldRange {
                    start_line: i,
                    end_line: end,
                    indent_level: current_indent,
                });
            }
        }
    }

    /// Toggle fold at the given line. Returns true if the line was a fold point.
    pub fn toggle_fold(&mut self, line: usize) -> bool {
        // Check if this line is a fold start
        let is_fold_start = self.fold_ranges.iter().any(|r| r.start_line == line);
        if !is_fold_start {
            return false;
        }
        if self.folded_lines.contains(&line) {
            self.folded_lines.remove(&line);
        } else {
            self.folded_lines.insert(line);
        }
        true
    }

    /// Check if a line is hidden due to folding.
    pub fn is_line_hidden(&self, line: usize) -> bool {
        for fold_start in &self.folded_lines {
            if let Some(range) = self
                .fold_ranges
                .iter()
                .find(|r| r.start_line == *fold_start)
            {
                if line > range.start_line && line <= range.end_line {
                    return true;
                }
            }
        }
        false
    }

    /// Check if a line is a fold start point (has a fold marker).
    pub fn is_fold_start(&self, line: usize) -> bool {
        self.fold_ranges.iter().any(|r| r.start_line == line)
    }

    /// Check if a fold at this line is currently collapsed.
    pub fn is_folded(&self, line: usize) -> bool {
        self.folded_lines.contains(&line)
    }

    /// Get the fold range starting at this line.
    pub fn fold_range_at(&self, line: usize) -> Option<&FoldRange> {
        self.fold_ranges.iter().find(|r| r.start_line == line)
    }

    /// Fold all regions.
    pub fn fold_all(&mut self) {
        self.folded_lines = self.fold_ranges.iter().map(|r| r.start_line).collect();
    }

    /// Unfold all regions.
    pub fn unfold_all(&mut self) {
        self.folded_lines.clear();
    }

    /// Map a visible line index to the actual document line index,
    /// accounting for folded (hidden) lines.
    pub fn visible_to_actual(&self, visible_line: usize, total_lines: usize) -> usize {
        let mut actual = 0;
        let mut visible_count = 0;
        while actual < total_lines && visible_count < visible_line {
            actual += 1;
            if !self.is_line_hidden(actual) {
                visible_count += 1;
            }
        }
        actual
    }

    /// Count the total number of visible lines.
    pub fn visible_line_count(&self, total_lines: usize) -> usize {
        (0..total_lines)
            .filter(|&l| !self.is_line_hidden(l))
            .count()
    }
}

/// Count leading whitespace as indent level (tabs count as 4 spaces).
fn indent_level(line: &str) -> usize {
    let mut level = 0;
    for ch in line.chars() {
        match ch {
            ' ' => level += 1,
            '\t' => level += 4,
            _ => break,
        }
    }
    level
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_fold_ranges() {
        let lines = vec![
            "fn main() {".to_string(),
            "    let x = 1;".to_string(),
            "    if x > 0 {".to_string(),
            "        println!(\"hi\");".to_string(),
            "    }".to_string(),
            "}".to_string(),
        ];
        let mut state = FoldingState::new();
        state.compute_fold_ranges(&lines);

        // Line 0 (fn main) should be foldable
        assert!(state.is_fold_start(0));
        // Line 2 (if x > 0) should be foldable
        assert!(state.is_fold_start(2));
        // Line 1 should not be a fold start
        assert!(!state.is_fold_start(1));
    }

    #[test]
    fn test_toggle_fold() {
        let lines = vec![
            "fn main() {".to_string(),
            "    let x = 1;".to_string(),
            "    let y = 2;".to_string(),
            "}".to_string(),
        ];
        let mut state = FoldingState::new();
        state.compute_fold_ranges(&lines);

        // Toggle fold at line 0
        assert!(state.toggle_fold(0));
        assert!(state.is_folded(0));

        // Lines 1 and 2 should be hidden
        assert!(state.is_line_hidden(1));
        assert!(state.is_line_hidden(2));
        assert!(!state.is_line_hidden(0));
        assert!(!state.is_line_hidden(3));

        // Toggle again to unfold
        state.toggle_fold(0);
        assert!(!state.is_folded(0));
        assert!(!state.is_line_hidden(1));
    }

    #[test]
    fn test_fold_unfold_all() {
        let lines = vec![
            "fn a() {".to_string(),
            "    body_a".to_string(),
            "}".to_string(),
            "fn b() {".to_string(),
            "    body_b".to_string(),
            "}".to_string(),
        ];
        let mut state = FoldingState::new();
        state.compute_fold_ranges(&lines);

        state.fold_all();
        assert!(state.is_folded(0));
        assert!(state.is_folded(3));

        state.unfold_all();
        assert!(!state.is_folded(0));
        assert!(!state.is_folded(3));
    }

    #[test]
    fn test_visible_line_count() {
        let lines = vec![
            "fn main() {".to_string(),
            "    let x = 1;".to_string(),
            "    let y = 2;".to_string(),
            "}".to_string(),
        ];
        let mut state = FoldingState::new();
        state.compute_fold_ranges(&lines);

        assert_eq!(state.visible_line_count(4), 4);

        state.toggle_fold(0);
        // Lines 1, 2 hidden -> 2 visible (0 and 3)
        assert_eq!(state.visible_line_count(4), 2);
    }

    #[test]
    fn test_non_fold_line_toggle() {
        let lines = vec!["let x = 1;".to_string(), "let y = 2;".to_string()];
        let mut state = FoldingState::new();
        state.compute_fold_ranges(&lines);
        assert!(!state.toggle_fold(0));
    }
}
