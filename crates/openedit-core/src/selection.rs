use crate::cursor::Position;

/// A selection is a range from `start` to `end` where start <= end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub start: Position,
    pub end: Position,
}

impl Selection {
    pub fn new(start: Position, end: Position) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn contains(&self, pos: Position) -> bool {
        pos >= self.start && pos <= self.end
    }

    pub fn overlaps(&self, other: &Selection) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_normalizes() {
        let sel = Selection::new(Position::new(5, 0), Position::new(2, 0));
        assert_eq!(sel.start, Position::new(2, 0));
        assert_eq!(sel.end, Position::new(5, 0));
    }

    #[test]
    fn test_selection_contains() {
        let sel = Selection::new(Position::new(1, 0), Position::new(3, 0));
        assert!(sel.contains(Position::new(2, 0)));
        assert!(!sel.contains(Position::new(4, 0)));
    }

    #[test]
    fn test_selection_overlaps() {
        let a = Selection::new(Position::new(1, 0), Position::new(3, 0));
        let b = Selection::new(Position::new(2, 0), Position::new(5, 0));
        assert!(a.overlaps(&b));
    }
}
