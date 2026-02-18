use crate::buffer::Buffer;
use crate::cursor::{Cursor, Position};

/// Represents a single atomic edit operation that can be undone.
#[derive(Debug, Clone)]
pub enum EditOp {
    /// Insert text at a char offset.
    Insert {
        offset: usize,
        text: String,
    },
    /// Delete text in a char range.
    Delete {
        offset: usize,
        deleted_text: String,
    },
    /// Replace text in a char range.
    Replace {
        offset: usize,
        old_text: String,
        new_text: String,
    },
}

impl EditOp {
    /// Apply this operation to the buffer.
    pub fn apply(&self, buffer: &mut Buffer) {
        match self {
            EditOp::Insert { offset, text } => {
                buffer.insert(*offset, text);
            }
            EditOp::Delete { offset, deleted_text } => {
                buffer.remove(*offset..*offset + deleted_text.len());
            }
            EditOp::Replace { offset, old_text, new_text } => {
                buffer.replace(*offset..*offset + old_text.len(), new_text);
            }
        }
    }

    /// Return the inverse operation (for undo).
    pub fn inverse(&self) -> EditOp {
        match self {
            EditOp::Insert { offset, text } => EditOp::Delete {
                offset: *offset,
                deleted_text: text.clone(),
            },
            EditOp::Delete { offset, deleted_text } => EditOp::Insert {
                offset: *offset,
                text: deleted_text.clone(),
            },
            EditOp::Replace { offset, old_text, new_text } => EditOp::Replace {
                offset: *offset,
                old_text: new_text.clone(),
                new_text: old_text.clone(),
            },
        }
    }
}

/// Insert a character at the cursor position.
pub fn insert_char(buffer: &mut Buffer, cursor: &mut Cursor, ch: char) {
    let offset = buffer.line_col_to_char(cursor.position.line, cursor.position.col);
    let s = ch.to_string();
    buffer.insert(offset, &s);

    // Advance cursor
    if ch == '\n' {
        cursor.position.line += 1;
        cursor.position.col = 0;
    } else {
        cursor.position.col += 1;
    }
    cursor.preferred_col = None;
    cursor.clear_selection();
}

/// Insert a string at the cursor position.
pub fn insert_text(buffer: &mut Buffer, cursor: &mut Cursor, text: &str) {
    let offset = buffer.line_col_to_char(cursor.position.line, cursor.position.col);
    buffer.insert(offset, text);

    // Calculate new cursor position
    let newlines = text.chars().filter(|&c| c == '\n').count();
    if newlines > 0 {
        cursor.position.line += newlines;
        let last_line = text.rsplit('\n').next().unwrap_or("");
        cursor.position.col = last_line.len();
    } else {
        cursor.position.col += text.len();
    }
    cursor.preferred_col = None;
    cursor.clear_selection();
}

/// Delete the character before the cursor (backspace).
pub fn backspace(buffer: &mut Buffer, cursor: &mut Cursor) {
    // If there's a selection, delete it instead
    if cursor.has_selection() {
        delete_selection(buffer, cursor);
        return;
    }

    let offset = buffer.line_col_to_char(cursor.position.line, cursor.position.col);
    if offset == 0 {
        return;
    }

    // Check if we're deleting a \r\n pair
    let prev_char = buffer.char_at(offset - 1);
    let delete_count = if prev_char == '\n' && offset >= 2 && buffer.char_at(offset - 2) == '\r' {
        2
    } else {
        1
    };

    // Capture the previous line's length before deleting (needed for cross-line backspace)
    let prev_line_len = if prev_char == '\n' || prev_char == '\r' {
        buffer.line_len_chars_no_newline(cursor.position.line - 1)
    } else {
        0
    };

    buffer.remove(offset - delete_count..offset);

    // Move cursor back
    if prev_char == '\n' || prev_char == '\r' {
        cursor.position.line -= 1;
        cursor.position.col = prev_line_len;
    } else {
        cursor.position.col -= 1;
    }
    cursor.preferred_col = None;
}

/// Delete the character at the cursor (Delete key).
pub fn delete_forward(buffer: &mut Buffer, cursor: &mut Cursor) {
    if cursor.has_selection() {
        delete_selection(buffer, cursor);
        return;
    }

    let offset = buffer.line_col_to_char(cursor.position.line, cursor.position.col);
    if offset >= buffer.len_chars() {
        return;
    }

    let ch = buffer.char_at(offset);
    let delete_count = if ch == '\r' && offset + 1 < buffer.len_chars() && buffer.char_at(offset + 1) == '\n' {
        2
    } else {
        1
    };

    buffer.remove(offset..offset + delete_count);
    // Cursor stays in place
}

/// Delete the selected text and collapse cursor to start of selection.
pub fn delete_selection(buffer: &mut Buffer, cursor: &mut Cursor) {
    if let Some((start, end)) = cursor.selection_range() {
        let start_offset = buffer.line_col_to_char(start.line, start.col);
        let end_offset = buffer.line_col_to_char(end.line, end.col);
        buffer.remove(start_offset..end_offset);
        cursor.position = start;
        cursor.clear_selection();
        cursor.preferred_col = None;
    }
}

/// Duplicate the current line.
pub fn duplicate_line(buffer: &mut Buffer, cursor: &Cursor) {
    let line_idx = cursor.position.line;
    let line_text = buffer.line(line_idx).to_string();
    let line_end = buffer.line_to_char(line_idx) + buffer.line_len_chars(line_idx);

    if line_text.ends_with('\n') || line_text.ends_with('\r') {
        buffer.insert(line_end, &line_text);
    } else {
        // Last line without newline
        buffer.insert(line_end, &format!("\n{}", line_text));
    }
}

/// Delete the entire current line.
pub fn delete_line(buffer: &mut Buffer, cursor: &mut Cursor) {
    let line_idx = cursor.position.line;
    let line_start = buffer.line_to_char(line_idx);
    let line_end = line_start + buffer.line_len_chars(line_idx);
    buffer.remove(line_start..line_end);

    // Adjust cursor
    if cursor.position.line >= buffer.len_lines() {
        cursor.position.line = buffer.len_lines().saturating_sub(1);
    }
    let max_col = buffer.line_len_chars_no_newline(cursor.position.line);
    cursor.position.col = cursor.position.col.min(max_col);
    cursor.preferred_col = None;
}

/// Swap the current line with the line above.
pub fn move_line_up(buffer: &mut Buffer, cursor: &mut Cursor) {
    if cursor.position.line == 0 {
        return;
    }
    swap_lines(buffer, cursor.position.line - 1, cursor.position.line);
    cursor.position.line -= 1;
}

/// Swap the current line with the line below.
pub fn move_line_down(buffer: &mut Buffer, cursor: &mut Cursor) {
    if cursor.position.line + 1 >= buffer.len_lines() {
        return;
    }
    swap_lines(buffer, cursor.position.line, cursor.position.line + 1);
    cursor.position.line += 1;
}

fn swap_lines(buffer: &mut Buffer, line_a: usize, line_b: usize) {
    let a_text = buffer.line(line_a).to_string();
    let b_text = buffer.line(line_b).to_string();

    // Remove both lines starting from line_b (which is after line_a)
    let a_start = buffer.line_to_char(line_a);
    let b_end = buffer.line_to_char(line_b) + buffer.line_len_chars(line_b);

    let combined_len = b_end - a_start;
    buffer.remove(a_start..a_start + combined_len);
    // Insert b then a
    buffer.insert(a_start, &format!("{}{}", b_text, a_text));
}

/// Cursor movement helpers.
pub fn move_cursor_left(buffer: &Buffer, cursor: &mut Cursor, extend_selection: bool) {
    let new_pos = if cursor.position.col > 0 {
        Position::new(cursor.position.line, cursor.position.col - 1)
    } else if cursor.position.line > 0 {
        let prev_line = cursor.position.line - 1;
        Position::new(prev_line, buffer.line_len_chars_no_newline(prev_line))
    } else {
        return;
    };
    cursor.move_to(new_pos, extend_selection);
    cursor.preferred_col = None;
}

pub fn move_cursor_right(buffer: &Buffer, cursor: &mut Cursor, extend_selection: bool) {
    let line_len = buffer.line_len_chars_no_newline(cursor.position.line);
    let new_pos = if cursor.position.col < line_len {
        Position::new(cursor.position.line, cursor.position.col + 1)
    } else if cursor.position.line + 1 < buffer.len_lines() {
        Position::new(cursor.position.line + 1, 0)
    } else {
        return;
    };
    cursor.move_to(new_pos, extend_selection);
    cursor.preferred_col = None;
}

pub fn move_cursor_up(buffer: &Buffer, cursor: &mut Cursor, extend_selection: bool) {
    if cursor.position.line == 0 {
        return;
    }
    let target_col = cursor.preferred_col.unwrap_or(cursor.position.col);
    let new_line = cursor.position.line - 1;
    let max_col = buffer.line_len_chars_no_newline(new_line);
    let new_col = target_col.min(max_col);
    cursor.move_to(Position::new(new_line, new_col), extend_selection);
    cursor.preferred_col = Some(target_col);
}

pub fn move_cursor_down(buffer: &Buffer, cursor: &mut Cursor, extend_selection: bool) {
    if cursor.position.line + 1 >= buffer.len_lines() {
        return;
    }
    let target_col = cursor.preferred_col.unwrap_or(cursor.position.col);
    let new_line = cursor.position.line + 1;
    let max_col = buffer.line_len_chars_no_newline(new_line);
    let new_col = target_col.min(max_col);
    cursor.move_to(Position::new(new_line, new_col), extend_selection);
    cursor.preferred_col = Some(target_col);
}

pub fn move_cursor_page_up(buffer: &Buffer, cursor: &mut Cursor, page_size: usize, extend_selection: bool) {
    let target_col = cursor.preferred_col.unwrap_or(cursor.position.col);
    let new_line = cursor.position.line.saturating_sub(page_size);
    let max_col = buffer.line_len_chars_no_newline(new_line);
    let new_col = target_col.min(max_col);
    cursor.move_to(Position::new(new_line, new_col), extend_selection);
    cursor.preferred_col = Some(target_col);
}

pub fn move_cursor_page_down(buffer: &Buffer, cursor: &mut Cursor, page_size: usize, extend_selection: bool) {
    let target_col = cursor.preferred_col.unwrap_or(cursor.position.col);
    let new_line = (cursor.position.line + page_size).min(buffer.len_lines().saturating_sub(1));
    let max_col = buffer.line_len_chars_no_newline(new_line);
    let new_col = target_col.min(max_col);
    cursor.move_to(Position::new(new_line, new_col), extend_selection);
    cursor.preferred_col = Some(target_col);
}

pub fn move_cursor_home(_buffer: &Buffer, cursor: &mut Cursor, extend_selection: bool) {
    cursor.move_to(Position::new(cursor.position.line, 0), extend_selection);
    cursor.preferred_col = None;
}

pub fn move_cursor_end(buffer: &Buffer, cursor: &mut Cursor, extend_selection: bool) {
    let col = buffer.line_len_chars_no_newline(cursor.position.line);
    cursor.move_to(Position::new(cursor.position.line, col), extend_selection);
    cursor.preferred_col = None;
}

/// Move cursor to the start of the next word.
pub fn move_cursor_word_right(buffer: &Buffer, cursor: &mut Cursor, extend_selection: bool) {
    let offset = buffer.line_col_to_char(cursor.position.line, cursor.position.col);
    let total = buffer.len_chars();
    if offset >= total {
        return;
    }

    let mut i = offset;
    // Skip current word characters
    while i < total && is_word_char(buffer.char_at(i)) {
        i += 1;
    }
    // Skip whitespace/punctuation
    while i < total && !is_word_char(buffer.char_at(i)) {
        i += 1;
    }

    let (line, col) = buffer.char_to_line_col(i.min(total));
    cursor.move_to(Position::new(line, col), extend_selection);
    cursor.preferred_col = None;
}

/// Move cursor to the start of the previous word.
pub fn move_cursor_word_left(buffer: &Buffer, cursor: &mut Cursor, extend_selection: bool) {
    let offset = buffer.line_col_to_char(cursor.position.line, cursor.position.col);
    if offset == 0 {
        return;
    }

    let mut i = offset;
    // Skip whitespace/punctuation before
    while i > 0 && !is_word_char(buffer.char_at(i - 1)) {
        i -= 1;
    }
    // Skip word characters
    while i > 0 && is_word_char(buffer.char_at(i - 1)) {
        i -= 1;
    }

    let (line, col) = buffer.char_to_line_col(i);
    cursor.move_to(Position::new(line, col), extend_selection);
    cursor.preferred_col = None;
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_char() {
        let mut buf = Buffer::from_str("hllo");
        let mut cur = Cursor::new(0, 1);
        insert_char(&mut buf, &mut cur, 'e');
        assert_eq!(buf.to_string(), "hello");
        assert_eq!(cur.position, Position::new(0, 2));
    }

    #[test]
    fn test_insert_newline() {
        let mut buf = Buffer::from_str("ab");
        let mut cur = Cursor::new(0, 1);
        insert_char(&mut buf, &mut cur, '\n');
        assert_eq!(buf.to_string(), "a\nb");
        assert_eq!(cur.position, Position::new(1, 0));
    }

    #[test]
    fn test_backspace() {
        let mut buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 3);
        backspace(&mut buf, &mut cur);
        assert_eq!(buf.to_string(), "helo");
        assert_eq!(cur.position, Position::new(0, 2));
    }

    #[test]
    fn test_backspace_at_start() {
        let mut buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 0);
        backspace(&mut buf, &mut cur);
        assert_eq!(buf.to_string(), "hello"); // no change
    }

    #[test]
    fn test_backspace_across_line() {
        let mut buf = Buffer::from_str("ab\ncd");
        let mut cur = Cursor::new(1, 0);
        backspace(&mut buf, &mut cur);
        assert_eq!(buf.to_string(), "abcd");
        assert_eq!(cur.position, Position::new(0, 2));
    }

    #[test]
    fn test_delete_forward() {
        let mut buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 2);
        delete_forward(&mut buf, &mut cur);
        assert_eq!(buf.to_string(), "helo");
        assert_eq!(cur.position, Position::new(0, 2)); // stays
    }

    #[test]
    fn test_duplicate_line() {
        let mut buf = Buffer::from_str("hello\nworld");
        let cur = Cursor::new(0, 0);
        duplicate_line(&mut buf, &cur);
        assert_eq!(buf.to_string(), "hello\nhello\nworld");
    }

    #[test]
    fn test_delete_line() {
        let mut buf = Buffer::from_str("aaa\nbbb\nccc");
        let mut cur = Cursor::new(1, 1);
        delete_line(&mut buf, &mut cur);
        assert_eq!(buf.to_string(), "aaa\nccc");
    }

    #[test]
    fn test_move_line_down() {
        let mut buf = Buffer::from_str("aaa\nbbb\nccc");
        let mut cur = Cursor::new(0, 0);
        move_line_down(&mut buf, &mut cur);
        assert_eq!(buf.to_string(), "bbb\naaa\nccc");
        assert_eq!(cur.position.line, 1);
    }

    #[test]
    fn test_move_cursor_left_right() {
        let buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 2);
        move_cursor_left(&buf, &mut cur, false);
        assert_eq!(cur.position.col, 1);
        move_cursor_right(&buf, &mut cur, false);
        assert_eq!(cur.position.col, 2);
    }

    #[test]
    fn test_move_cursor_up_down() {
        let buf = Buffer::from_str("abc\ndef\nghi");
        let mut cur = Cursor::new(1, 1);
        move_cursor_up(&buf, &mut cur, false);
        assert_eq!(cur.position, Position::new(0, 1));
        move_cursor_down(&buf, &mut cur, false);
        assert_eq!(cur.position, Position::new(1, 1));
    }

    #[test]
    fn test_move_cursor_preferred_col() {
        let buf = Buffer::from_str("abcde\nab\nabcde");
        let mut cur = Cursor::new(0, 4);
        move_cursor_down(&buf, &mut cur, false);
        // Line 1 only has 2 chars, so col is clamped
        assert_eq!(cur.position, Position::new(1, 2));
        // But preferred_col remembers 4
        move_cursor_down(&buf, &mut cur, false);
        assert_eq!(cur.position, Position::new(2, 4));
    }

    #[test]
    fn test_delete_selection() {
        let mut buf = Buffer::from_str("hello world");
        let mut cur = Cursor::new(0, 0);
        cur.move_to(Position::new(0, 5), true); // select "hello"
        delete_selection(&mut buf, &mut cur);
        assert_eq!(buf.to_string(), " world");
        assert_eq!(cur.position, Position::new(0, 0));
    }

    #[test]
    fn test_move_cursor_word_right() {
        let buf = Buffer::from_str("hello world foo");
        let mut cur = Cursor::new(0, 0);
        move_cursor_word_right(&buf, &mut cur, false);
        assert_eq!(cur.position, Position::new(0, 6)); // "world"
    }

    #[test]
    fn test_move_cursor_word_left() {
        let buf = Buffer::from_str("hello world");
        let mut cur = Cursor::new(0, 8);
        move_cursor_word_left(&buf, &mut cur, false);
        assert_eq!(cur.position, Position::new(0, 6)); // start of "world"
    }
}
