/// A position in the document as (line, column), both 0-indexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

impl Position {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }

    pub fn zero() -> Self {
        Self { line: 0, col: 0 }
    }
}

/// A single cursor with an optional selection anchor.
///
/// The cursor is always at `position`. If `anchor` is Some, the selection
/// spans from `anchor` to `position` (anchor may be before or after position).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub position: Position,
    /// If set, selection is from anchor to position.
    pub anchor: Option<Position>,
    /// Desired column for vertical movement (remembers column when moving through short lines).
    pub preferred_col: Option<usize>,
}

impl Cursor {
    pub fn new(line: usize, col: usize) -> Self {
        Self {
            position: Position::new(line, col),
            anchor: None,
            preferred_col: None,
        }
    }

    pub fn at(pos: Position) -> Self {
        Self {
            position: pos,
            anchor: None,
            preferred_col: None,
        }
    }

    pub fn has_selection(&self) -> bool {
        self.anchor.is_some() && self.anchor != Some(self.position)
    }

    /// Returns the selection range as (start, end) where start <= end.
    pub fn selection_range(&self) -> Option<(Position, Position)> {
        self.anchor.map(|anchor| {
            if anchor <= self.position {
                (anchor, self.position)
            } else {
                (self.position, anchor)
            }
        })
    }

    /// Start selecting from the current position (set anchor = position).
    pub fn start_selection(&mut self) {
        if self.anchor.is_none() {
            self.anchor = Some(self.position);
        }
    }

    /// Clear the selection.
    pub fn clear_selection(&mut self) {
        self.anchor = None;
    }

    /// Move to a new position, optionally extending the selection.
    pub fn move_to(&mut self, pos: Position, extend_selection: bool) {
        if extend_selection {
            self.start_selection();
        } else {
            self.clear_selection();
        }
        self.position = pos;
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Multi-cursor state. There is always at least one cursor (the primary).
#[derive(Debug, Clone)]
pub struct MultiCursorState {
    cursors: Vec<Cursor>,
}

impl MultiCursorState {
    pub fn new() -> Self {
        Self {
            cursors: vec![Cursor::default()],
        }
    }

    pub fn primary(&self) -> &Cursor {
        &self.cursors[0]
    }

    pub fn primary_mut(&mut self) -> &mut Cursor {
        &mut self.cursors[0]
    }

    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    pub fn cursors_mut(&mut self) -> &mut Vec<Cursor> {
        &mut self.cursors
    }

    pub fn set_primary(&mut self, cursor: Cursor) {
        self.cursors[0] = cursor;
    }

    pub fn add_cursor(&mut self, cursor: Cursor) {
        self.cursors.push(cursor);
        self.merge_overlapping();
    }

    pub fn clear_extra_cursors(&mut self) {
        self.cursors.truncate(1);
    }

    pub fn cursor_count(&self) -> usize {
        self.cursors.len()
    }

    /// Merge overlapping cursors (same position = keep one).
    fn merge_overlapping(&mut self) {
        self.cursors
            .sort_by_key(|c| (c.position.line, c.position.col));
        self.cursors.dedup_by(|a, b| a.position == b.position);
    }
}

impl Default for MultiCursorState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_default() {
        let c = Cursor::default();
        assert_eq!(c.position, Position::zero());
        assert!(!c.has_selection());
    }

    #[test]
    fn test_cursor_selection() {
        let mut c = Cursor::new(0, 0);
        c.move_to(Position::new(0, 5), true);
        assert!(c.has_selection());
        let (start, end) = c.selection_range().unwrap();
        assert_eq!(start, Position::new(0, 0));
        assert_eq!(end, Position::new(0, 5));
    }

    #[test]
    fn test_cursor_clear_selection() {
        let mut c = Cursor::new(0, 0);
        c.move_to(Position::new(0, 5), true);
        assert!(c.has_selection());
        c.move_to(Position::new(0, 3), false);
        assert!(!c.has_selection());
    }

    #[test]
    fn test_multi_cursor() {
        let mut mc = MultiCursorState::new();
        assert_eq!(mc.cursor_count(), 1);
        mc.add_cursor(Cursor::new(5, 0));
        assert_eq!(mc.cursor_count(), 2);
        mc.clear_extra_cursors();
        assert_eq!(mc.cursor_count(), 1);
    }

    #[test]
    fn test_multi_cursor_dedup() {
        let mut mc = MultiCursorState::new();
        mc.add_cursor(Cursor::new(0, 0)); // duplicate of primary
        assert_eq!(mc.cursor_count(), 1);
    }
}
