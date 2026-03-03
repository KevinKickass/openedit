use crate::buffer::Buffer;
use crate::cursor::{Cursor, MultiCursorState, Position};
use crate::edit::{self, EditOp};
use crate::encoding::Encoding;
use crate::folding::FoldingState;
use crate::line_ending::LineEnding;
use crate::search::SearchEngine;
use crate::undo::UndoManager;
use std::path::PathBuf;

/// Unique document ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocId(pub u64);

static NEXT_DOC_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl DocId {
    pub fn next() -> Self {
        Self(NEXT_DOC_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

/// A document is an open file (or untitled buffer) with all associated state.
pub struct Document {
    pub id: DocId,
    pub buffer: Buffer,
    pub cursors: MultiCursorState,
    pub undo_manager: UndoManager,
    pub search: SearchEngine,
    pub path: Option<PathBuf>,
    pub encoding: Encoding,
    pub line_ending: LineEnding,
    pub language: Option<String>,
    pub modified: bool,
    pub read_only: bool,
    /// Scroll offset in lines for the viewport.
    pub scroll_line: usize,
    /// Horizontal scroll offset in columns.
    pub scroll_col: usize,
    /// Code folding state.
    pub folding: FoldingState,
    /// Bookmarked line indices (sorted).
    pub bookmarks: Vec<usize>,
}

impl Document {
    pub fn new() -> Self {
        Self {
            id: DocId::next(),
            buffer: Buffer::new(),
            cursors: MultiCursorState::new(),
            undo_manager: UndoManager::new(),
            search: SearchEngine::new(),
            path: None,
            encoding: Encoding::default(),
            line_ending: LineEnding::default(),
            language: None,
            modified: false,
            read_only: false,
            scroll_line: 0,
            scroll_col: 0,
            folding: FoldingState::new(),
            bookmarks: Vec::new(),
        }
    }

    pub fn from_str(text: &str) -> Self {
        let line_ending = LineEnding::detect(text);
        // Normalize to LF internally — we'll convert back on save
        let normalized = LineEnding::normalize(text, LineEnding::LF);
        Self {
            id: DocId::next(),
            buffer: Buffer::from_str(&normalized),
            cursors: MultiCursorState::new(),
            undo_manager: UndoManager::new(),
            search: SearchEngine::new(),
            path: None,
            encoding: Encoding::default(),
            line_ending,
            language: None,
            modified: false,
            read_only: false,
            scroll_line: 0,
            scroll_col: 0,
            folding: FoldingState::new(),
            bookmarks: Vec::new(),
        }
    }

    /// Display name for the tab (file name or "Untitled").
    pub fn display_name(&self) -> String {
        match &self.path {
            Some(p) => p
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Untitled".into()),
            None => "Untitled".into(),
        }
    }

    /// Get the content to save (with correct line endings).
    pub fn content_for_save(&self) -> String {
        let text = self.buffer.to_string();
        LineEnding::normalize(&text, self.line_ending)
    }

    /// Get the raw bytes to write to disk.
    pub fn bytes_for_save(&self) -> Vec<u8> {
        let text = self.content_for_save();
        self.encoding.encode(&text)
    }

    // --- Edit operations that record undo ---

    pub fn insert_char(&mut self, ch: char) {
        if self.read_only {
            return;
        }
        let cursor = *self.cursors.primary();

        // Delete selection first if any
        if cursor.has_selection() {
            self.delete_selection();
        }

        let cursor = *self.cursors.primary();
        let offset = self
            .buffer
            .line_col_to_char(cursor.position.line, cursor.position.col);
        let text = ch.to_string();

        self.undo_manager.record(
            EditOp::Insert {
                offset,
                text: text.clone(),
            },
            cursor,
        );

        edit::insert_char(&mut self.buffer, self.cursors.primary_mut(), ch);
        self.modified = true;
    }

    pub fn insert_text(&mut self, text: &str) {
        if self.read_only || text.is_empty() {
            return;
        }
        let cursor = *self.cursors.primary();

        if cursor.has_selection() {
            self.delete_selection();
        }

        let cursor = *self.cursors.primary();
        let offset = self
            .buffer
            .line_col_to_char(cursor.position.line, cursor.position.col);

        self.undo_manager.record(
            EditOp::Insert {
                offset,
                text: text.to_string(),
            },
            cursor,
        );

        edit::insert_text(&mut self.buffer, self.cursors.primary_mut(), text);
        self.modified = true;
    }

    pub fn backspace(&mut self) {
        if self.read_only {
            return;
        }
        let cursor = *self.cursors.primary();

        if cursor.has_selection() {
            self.delete_selection();
            return;
        }

        let offset = self
            .buffer
            .line_col_to_char(cursor.position.line, cursor.position.col);
        if offset == 0 {
            return;
        }

        let prev_char = self.buffer.char_at(offset - 1);
        let delete_start =
            if prev_char == '\n' && offset >= 2 && self.buffer.char_at(offset - 2) == '\r' {
                offset - 2
            } else {
                offset - 1
            };
        let deleted = self.buffer.slice_to_string(delete_start..offset);

        self.undo_manager.record(
            EditOp::Delete {
                offset: delete_start,
                deleted_text: deleted,
            },
            cursor,
        );

        edit::backspace(&mut self.buffer, self.cursors.primary_mut());
        self.modified = true;
    }

    pub fn delete_forward(&mut self) {
        if self.read_only {
            return;
        }
        let cursor = *self.cursors.primary();

        if cursor.has_selection() {
            self.delete_selection();
            return;
        }

        let offset = self
            .buffer
            .line_col_to_char(cursor.position.line, cursor.position.col);
        if offset >= self.buffer.len_chars() {
            return;
        }

        let ch = self.buffer.char_at(offset);
        let delete_end = if ch == '\r'
            && offset + 1 < self.buffer.len_chars()
            && self.buffer.char_at(offset + 1) == '\n'
        {
            offset + 2
        } else {
            offset + 1
        };
        let deleted = self.buffer.slice_to_string(offset..delete_end);

        self.undo_manager.record(
            EditOp::Delete {
                offset,
                deleted_text: deleted,
            },
            cursor,
        );

        edit::delete_forward(&mut self.buffer, self.cursors.primary_mut());
        self.modified = true;
    }

    fn delete_selection(&mut self) {
        let cursor = *self.cursors.primary();
        if let Some((start, end)) = cursor.selection_range() {
            let start_offset = self.buffer.line_col_to_char(start.line, start.col);
            let end_offset = self.buffer.line_col_to_char(end.line, end.col);
            let deleted = self.buffer.slice_to_string(start_offset..end_offset);

            self.undo_manager.record(
                EditOp::Delete {
                    offset: start_offset,
                    deleted_text: deleted,
                },
                cursor,
            );

            edit::delete_selection(&mut self.buffer, self.cursors.primary_mut());
            self.modified = true;
        }
    }

    pub fn delete_selection_public(&mut self) {
        if self.read_only {
            return;
        }
        self.delete_selection();
    }

    /// Insert a newline and auto-indent to match the previous line.
    pub fn insert_newline_with_indent(&mut self) {
        if self.read_only {
            return;
        }
        // Delete selection first if any
        let cursor = *self.cursors.primary();
        if cursor.has_selection() {
            self.delete_selection();
        }

        let cursor = *self.cursors.primary();
        let line_text = self.buffer.line(cursor.position.line).to_string();
        let indent: String = line_text
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        let insert = format!("\n{}", indent);

        let offset = self
            .buffer
            .line_col_to_char(cursor.position.line, cursor.position.col);
        self.undo_manager.record(
            EditOp::Insert {
                offset,
                text: insert.clone(),
            },
            cursor,
        );

        edit::insert_text(&mut self.buffer, self.cursors.primary_mut(), &insert);
        self.modified = true;
    }

    /// Remove one level of indentation from the current line (Shift+Tab).
    pub fn unindent(&mut self) {
        if self.read_only {
            return;
        }
        let cursor = *self.cursors.primary();
        let line_idx = cursor.position.line;
        let line_text = self.buffer.line(line_idx).to_string();

        // Determine how many spaces to remove (up to 4 spaces or 1 tab)
        let remove_count = if line_text.starts_with('\t') {
            1
        } else {
            let spaces = line_text.chars().take_while(|c| *c == ' ').count();
            spaces.min(4)
        };

        if remove_count == 0 {
            return;
        }

        let line_start = self.buffer.line_to_char(line_idx);
        let deleted = self
            .buffer
            .slice_to_string(line_start..line_start + remove_count);

        self.undo_manager.record(
            EditOp::Delete {
                offset: line_start,
                deleted_text: deleted,
            },
            cursor,
        );

        self.buffer.remove(line_start..line_start + remove_count);
        self.cursors.primary_mut().position.col = cursor.position.col.saturating_sub(remove_count);
        self.cursors.primary_mut().preferred_col = None;
        self.modified = true;
    }

    /// Delete from cursor to the start of the previous word (Ctrl+Backspace).
    pub fn delete_word_left(&mut self) {
        if self.read_only {
            return;
        }
        let cursor = *self.cursors.primary();
        if cursor.has_selection() {
            self.delete_selection();
            return;
        }
        let offset = self
            .buffer
            .line_col_to_char(cursor.position.line, cursor.position.col);
        if offset == 0 {
            return;
        }

        // Find word boundary (same logic as move_cursor_word_left)
        let mut i = offset;
        while i > 0
            && !self.buffer.char_at(i - 1).is_alphanumeric()
            && self.buffer.char_at(i - 1) != '_'
        {
            i -= 1;
        }
        while i > 0
            && (self.buffer.char_at(i - 1).is_alphanumeric() || self.buffer.char_at(i - 1) == '_')
        {
            i -= 1;
        }

        let deleted = self.buffer.slice_to_string(i..offset);
        self.undo_manager.record(
            EditOp::Delete {
                offset: i,
                deleted_text: deleted,
            },
            cursor,
        );

        self.buffer.remove(i..offset);
        let (line, col) = self.buffer.char_to_line_col(i);
        self.cursors.primary_mut().position = Position::new(line, col);
        self.cursors.primary_mut().preferred_col = None;
        self.modified = true;
    }

    /// Delete from cursor to the end of the next word (Ctrl+Delete).
    pub fn delete_word_right(&mut self) {
        if self.read_only {
            return;
        }
        let cursor = *self.cursors.primary();
        if cursor.has_selection() {
            self.delete_selection();
            return;
        }
        let offset = self
            .buffer
            .line_col_to_char(cursor.position.line, cursor.position.col);
        let total = self.buffer.len_chars();
        if offset >= total {
            return;
        }

        let mut i = offset;
        while i < total
            && (self.buffer.char_at(i).is_alphanumeric() || self.buffer.char_at(i) == '_')
        {
            i += 1;
        }
        while i < total
            && !self.buffer.char_at(i).is_alphanumeric()
            && self.buffer.char_at(i) != '_'
        {
            i += 1;
        }

        let deleted = self.buffer.slice_to_string(offset..i);
        self.undo_manager.record(
            EditOp::Delete {
                offset,
                deleted_text: deleted,
            },
            cursor,
        );

        self.buffer.remove(offset..i);
        self.modified = true;
    }

    pub fn duplicate_line(&mut self) {
        if self.read_only {
            return;
        }
        edit::duplicate_line(&mut self.buffer, self.cursors.primary());
        self.modified = true;
    }

    pub fn delete_line(&mut self) {
        if self.read_only {
            return;
        }
        edit::delete_line(&mut self.buffer, self.cursors.primary_mut());
        self.modified = true;
    }

    pub fn move_line_up(&mut self) {
        if self.read_only {
            return;
        }
        edit::move_line_up(&mut self.buffer, self.cursors.primary_mut());
        self.modified = true;
    }

    pub fn move_line_down(&mut self) {
        if self.read_only {
            return;
        }
        edit::move_line_down(&mut self.buffer, self.cursors.primary_mut());
        self.modified = true;
    }

    // --- Toggle Comment ---

    /// Get the line comment prefix for the current language.
    fn comment_prefix(&self) -> &'static str {
        match self.language.as_deref() {
            Some("rust") | Some("c") | Some("cpp") | Some("go") | Some("java")
            | Some("javascript") | Some("typescript") | Some("swift") | Some("kotlin")
            | Some("scala") | Some("dart") | Some("css") | Some("scss") | Some("less")
            | Some("c_sharp") | Some("json") | Some("jsonc") => "//",
            Some("python") | Some("ruby") | Some("bash") | Some("shell") | Some("sh")
            | Some("yaml") | Some("toml") | Some("perl") | Some("r") | Some("dockerfile")
            | Some("makefile") | Some("cmake") | Some("powershell") => "#",
            Some("lua") | Some("sql") | Some("haskell") | Some("ada") => "--",
            Some("lisp") | Some("clojure") | Some("scheme") | Some("ini") | Some("assembly")
            | Some("asm") => ";",
            Some("html") | Some("xml") => "//", // simplified; real HTML uses <!-- -->
            _ => "//",
        }
    }

    /// Insert text at a specific column across a range of lines.
    ///
    /// For each line in `start_line..=end_line`, inserts `text` at column `col`.
    /// If a line is shorter than `col`, it is padded with spaces.
    /// This is recorded as a single undo transaction.
    pub fn column_insert_text(
        &mut self,
        start_line: usize,
        end_line: usize,
        col: usize,
        text: &str,
    ) {
        if self.read_only || text.is_empty() {
            return;
        }
        let end_line = end_line.min(self.buffer.len_lines().saturating_sub(1));
        if start_line > end_line {
            return;
        }

        let cursor = *self.cursors.primary();
        self.undo_manager.begin_transaction(cursor);

        // Process lines from bottom to top so offsets stay valid
        for line in (start_line..=end_line).rev() {
            let line_len = self.buffer.line_len_chars_no_newline(line);
            let offset = self.buffer.line_col_to_char(line, line_len);

            // Pad with spaces if line is shorter than target column
            if line_len < col {
                let padding = " ".repeat(col - line_len);
                self.undo_manager.record(
                    EditOp::Insert {
                        offset,
                        text: padding.clone(),
                    },
                    cursor,
                );
                self.buffer.insert(offset, &padding);
            }

            // Insert the text at the target column
            let insert_offset = self.buffer.line_col_to_char(line, col);
            self.undo_manager.record(
                EditOp::Insert {
                    offset: insert_offset,
                    text: text.to_string(),
                },
                cursor,
            );
            self.buffer.insert(insert_offset, text);
        }

        self.undo_manager.commit_transaction(cursor);
        self.modified = true;
    }

    /// Insert incrementing numbers at a specific column across a range of lines.
    ///
    /// Starting from `initial`, increments by `step` for each line.
    /// Numbers are formatted with optional zero-padding to `pad_width` digits.
    pub fn column_insert_numbers(
        &mut self,
        start_line: usize,
        end_line: usize,
        col: usize,
        initial: i64,
        step: i64,
        pad_width: usize,
    ) {
        if self.read_only {
            return;
        }
        let end_line = end_line.min(self.buffer.len_lines().saturating_sub(1));
        if start_line > end_line {
            return;
        }

        let cursor = *self.cursors.primary();
        self.undo_manager.begin_transaction(cursor);

        // Process lines from bottom to top so offsets stay valid
        let count = (end_line - start_line + 1) as i64;
        for i in (0..count).rev() {
            let line = start_line + i as usize;
            let value = initial + i * step;
            let text = if pad_width > 0 {
                format!("{:0>width$}", value, width = pad_width)
            } else {
                value.to_string()
            };

            let line_len = self.buffer.line_len_chars_no_newline(line);
            let offset = self.buffer.line_col_to_char(line, line_len);

            // Pad with spaces if line is shorter than target column
            if line_len < col {
                let padding = " ".repeat(col - line_len);
                self.undo_manager.record(
                    EditOp::Insert {
                        offset,
                        text: padding.clone(),
                    },
                    cursor,
                );
                self.buffer.insert(offset, &padding);
            }

            // Insert the number at the target column
            let insert_offset = self.buffer.line_col_to_char(line, col);
            self.undo_manager.record(
                EditOp::Insert {
                    offset: insert_offset,
                    text: text.clone(),
                },
                cursor,
            );
            self.buffer.insert(insert_offset, &text);
        }

        self.undo_manager.commit_transaction(cursor);
        self.modified = true;
    }

    /// Toggle line comments on the current line or all lines in the selection.
    ///
    /// If all affected lines are commented, uncomments them. Otherwise, comments all lines.
    /// This is an undoable operation grouped as a single transaction.
    pub fn toggle_comment(&mut self) {
        if self.read_only {
            return;
        }

        let prefix = self.comment_prefix();
        let cursor = *self.cursors.primary();

        // Determine the range of lines to affect
        let (first_line, last_line) = if let Some((start, end)) = cursor.selection_range() {
            (start.line, end.line)
        } else {
            (cursor.position.line, cursor.position.line)
        };

        // Collect the line texts (without trailing newline)
        let mut line_texts: Vec<String> = Vec::new();
        for line_idx in first_line..=last_line {
            let line = self.buffer.line(line_idx).to_string();
            let trimmed = line.trim_end_matches(&['\n', '\r'][..]);
            line_texts.push(trimmed.to_string());
        }

        // Check if all lines are commented (after leading whitespace)
        let all_commented = line_texts.iter().all(|line| {
            let stripped = line.trim_start();
            stripped.is_empty() || stripped.starts_with(prefix)
        });

        // Build the new line texts
        let new_line_texts: Vec<String> = if all_commented {
            // Uncomment: remove the comment prefix (and one optional trailing space)
            line_texts
                .iter()
                .map(|line| {
                    let stripped = line.trim_start();
                    if stripped.is_empty() {
                        return line.clone();
                    }
                    let leading_ws: String =
                        line.chars().take_while(|c| c.is_whitespace()).collect();
                    let after_prefix = &stripped[prefix.len()..];
                    // Remove one space after prefix if present
                    let after_prefix = after_prefix.strip_prefix(' ').unwrap_or(after_prefix);
                    format!("{}{}", leading_ws, after_prefix)
                })
                .collect()
        } else {
            // Comment: find minimum indentation among non-empty lines
            let min_indent = line_texts
                .iter()
                .filter(|line| !line.trim().is_empty())
                .map(|line| line.chars().take_while(|c| c.is_whitespace()).count())
                .min()
                .unwrap_or(0);

            line_texts
                .iter()
                .map(|line| {
                    if line.trim().is_empty() {
                        line.clone()
                    } else {
                        let indent: String = line.chars().take(min_indent).collect();
                        let rest: String = line.chars().skip(min_indent).collect();
                        format!("{}{} {}", indent, prefix, rest)
                    }
                })
                .collect()
        };

        // Build the old and new region strings
        // The region spans from the start of first_line to end of last_line (excluding newline)
        let region_start = self.buffer.line_to_char(first_line);
        let region_end_line_len = self.buffer.line_len_chars_no_newline(last_line);
        let region_end = self.buffer.line_to_char(last_line) + region_end_line_len;

        let old_text = self.buffer.slice_to_string(region_start..region_end);
        let new_text = new_line_texts.join("\n");

        if old_text == new_text {
            return;
        }

        // Record as a transaction
        self.undo_manager.begin_transaction(cursor);
        self.undo_manager.record(
            EditOp::Replace {
                offset: region_start,
                old_text: old_text.clone(),
                new_text: new_text.clone(),
            },
            cursor,
        );

        // Apply the replacement
        self.buffer
            .remove(region_start..region_start + old_text.chars().count());
        self.buffer.insert(region_start, &new_text);

        // Update cursor position
        let new_cursor = *self.cursors.primary();
        self.undo_manager.commit_transaction(new_cursor);

        // Adjust cursor column based on what happened
        let primary = self.cursors.primary_mut();
        let cursor_line_idx = primary.position.line;
        if cursor_line_idx >= first_line && cursor_line_idx <= last_line {
            let line_in_range = cursor_line_idx - first_line;
            let old_line = &line_texts[line_in_range];
            let new_line = &new_line_texts[line_in_range];
            let old_len = old_line.chars().count();
            let new_len = new_line.chars().count();
            if new_len > old_len {
                // Comment was added
                let diff = new_len - old_len;
                primary.position.col = primary.position.col.saturating_add(diff);
            } else if old_len > new_len {
                // Comment was removed
                let diff = old_len - new_len;
                primary.position.col = primary.position.col.saturating_sub(diff);
            }
        }
        primary.preferred_col = None;

        self.modified = true;
    }

    // --- Bookmarks ---

    /// Toggle a bookmark on the given line. Adds if not present, removes if present.
    pub fn toggle_bookmark(&mut self, line: usize) {
        match self.bookmarks.binary_search(&line) {
            Ok(idx) => {
                self.bookmarks.remove(idx);
            }
            Err(idx) => {
                self.bookmarks.insert(idx, line);
            }
        }
    }

    /// Find the next bookmarked line after `current_line`. Wraps around to the beginning.
    pub fn next_bookmark(&self, current_line: usize) -> Option<usize> {
        if self.bookmarks.is_empty() {
            return None;
        }
        // Find the first bookmark with line > current_line
        for &bm in &self.bookmarks {
            if bm > current_line {
                return Some(bm);
            }
        }
        // Wrap around to the first bookmark
        Some(self.bookmarks[0])
    }

    /// Find the previous bookmarked line before `current_line`. Wraps around to the end.
    pub fn prev_bookmark(&self, current_line: usize) -> Option<usize> {
        if self.bookmarks.is_empty() {
            return None;
        }
        // Find the last bookmark with line < current_line
        for &bm in self.bookmarks.iter().rev() {
            if bm < current_line {
                return Some(bm);
            }
        }
        // Wrap around to the last bookmark
        Some(*self.bookmarks.last().unwrap())
    }

    /// Remove all bookmarks.
    pub fn clear_bookmarks(&mut self) {
        self.bookmarks.clear();
    }

    // --- Code folding ---

    /// Recompute fold ranges from current buffer content.
    pub fn update_fold_ranges(&mut self) {
        let lines: Vec<String> = (0..self.buffer.len_lines())
            .map(|i| self.buffer.line(i).to_string())
            .collect();
        self.folding.compute_fold_ranges(&lines);
    }

    /// Toggle fold at a line.
    pub fn toggle_fold(&mut self, line: usize) -> bool {
        self.folding.toggle_fold(line)
    }

    // --- Cursor movement (no undo needed) ---

    pub fn move_cursor_left(&mut self, extend_selection: bool) {
        edit::move_cursor_left(&self.buffer, self.cursors.primary_mut(), extend_selection);
    }

    pub fn move_cursor_right(&mut self, extend_selection: bool) {
        edit::move_cursor_right(&self.buffer, self.cursors.primary_mut(), extend_selection);
    }

    pub fn move_cursor_up(&mut self, extend_selection: bool) {
        edit::move_cursor_up(&self.buffer, self.cursors.primary_mut(), extend_selection);
    }

    pub fn move_cursor_down(&mut self, extend_selection: bool) {
        edit::move_cursor_down(&self.buffer, self.cursors.primary_mut(), extend_selection);
    }

    pub fn move_cursor_page_up(&mut self, page_size: usize, extend_selection: bool) {
        edit::move_cursor_page_up(
            &self.buffer,
            self.cursors.primary_mut(),
            page_size,
            extend_selection,
        );
    }

    pub fn move_cursor_page_down(&mut self, page_size: usize, extend_selection: bool) {
        edit::move_cursor_page_down(
            &self.buffer,
            self.cursors.primary_mut(),
            page_size,
            extend_selection,
        );
    }

    pub fn move_cursor_home(&mut self, extend_selection: bool) {
        edit::move_cursor_home(&self.buffer, self.cursors.primary_mut(), extend_selection);
    }

    pub fn move_cursor_end(&mut self, extend_selection: bool) {
        edit::move_cursor_end(&self.buffer, self.cursors.primary_mut(), extend_selection);
    }

    pub fn move_cursor_word_left(&mut self, extend_selection: bool) {
        edit::move_cursor_word_left(&self.buffer, self.cursors.primary_mut(), extend_selection);
    }

    pub fn move_cursor_word_right(&mut self, extend_selection: bool) {
        edit::move_cursor_word_right(&self.buffer, self.cursors.primary_mut(), extend_selection);
    }

    pub fn move_cursor_doc_start(&mut self, extend_selection: bool) {
        self.cursors
            .primary_mut()
            .move_to(Position::zero(), extend_selection);
        self.cursors.primary_mut().preferred_col = None;
    }

    pub fn move_cursor_doc_end(&mut self, extend_selection: bool) {
        let last_line = self.buffer.len_lines() - 1;
        let last_col = self.buffer.line_len_chars_no_newline(last_line);
        self.cursors
            .primary_mut()
            .move_to(Position::new(last_line, last_col), extend_selection);
        self.cursors.primary_mut().preferred_col = None;
    }

    pub fn select_all(&mut self) {
        let last_line = self.buffer.len_lines() - 1;
        let last_col = self.buffer.line_len_chars_no_newline(last_line);
        let cursor = self.cursors.primary_mut();
        cursor.anchor = Some(Position::zero());
        cursor.position = Position::new(last_line, last_col);
    }

    pub fn go_to_line(&mut self, line: usize) {
        let line = line.min(self.buffer.len_lines().saturating_sub(1));
        self.cursors
            .primary_mut()
            .move_to(Position::new(line, 0), false);
        self.scroll_line = line.saturating_sub(5); // scroll with some context
    }

    /// Get the selected text (or empty string if no selection).
    pub fn selected_text(&self) -> String {
        let cursor = self.cursors.primary();
        if let Some((start, end)) = cursor.selection_range() {
            let start_offset = self.buffer.line_col_to_char(start.line, start.col);
            let end_offset = self.buffer.line_col_to_char(end.line, end.col);
            self.buffer.slice_to_string(start_offset..end_offset)
        } else {
            String::new()
        }
    }

    /// Select the next occurrence of the current selection (Ctrl+D).
    /// If nothing is selected, select the word under/around the cursor.
    pub fn select_next_occurrence(&mut self) {
        let primary = self.cursors.primary();

        if !primary.has_selection() {
            // No selection — select the word under cursor
            let pos = primary.position;
            let offset = self.buffer.line_col_to_char(pos.line, pos.col);
            let total = self.buffer.len_chars();

            if total == 0 {
                return;
            }

            // Find word boundaries
            let offset = offset.min(total.saturating_sub(1));
            let ch = self.buffer.char_at(offset);
            if !ch.is_alphanumeric() && ch != '_' {
                return;
            }

            let mut start = offset;
            while start > 0 {
                let c = self.buffer.char_at(start - 1);
                if c.is_alphanumeric() || c == '_' {
                    start -= 1;
                } else {
                    break;
                }
            }

            let mut end = offset;
            while end < total {
                let c = self.buffer.char_at(end);
                if c.is_alphanumeric() || c == '_' {
                    end += 1;
                } else {
                    break;
                }
            }

            if start == end {
                return;
            }

            let (start_line, start_col) = self.buffer.char_to_line_col(start);
            let (end_line, end_col) = self.buffer.char_to_line_col(end);

            let cursor = self.cursors.primary_mut();
            cursor.anchor = Some(Position::new(start_line, start_col));
            cursor.position = Position::new(end_line, end_col);
            return;
        }

        // Has selection — find next occurrence
        let selected = self.selected_text();
        if selected.is_empty() {
            return;
        }

        let text = self.buffer.to_string();

        // Find the furthest cursor position to search from
        let last_cursor = self
            .cursors
            .cursors()
            .iter()
            .filter_map(|c| c.selection_range())
            .map(|(_, end)| self.buffer.line_col_to_char(end.line, end.col))
            .max()
            .unwrap_or(0);

        // Search forward from after the last cursor
        let search_start = last_cursor;
        let found = text[search_start..]
            .find(&selected)
            .map(|i| search_start + i);

        // If not found, wrap around from beginning
        let found = found.or_else(|| text[..search_start.min(text.len())].find(&selected));

        if let Some(byte_offset) = found {
            // Convert byte offset to char offset
            let char_start = text[..byte_offset].chars().count();
            let char_end = char_start + selected.chars().count();

            let (start_line, start_col) = self.buffer.char_to_line_col(char_start);
            let (end_line, end_col) = self.buffer.char_to_line_col(char_end);

            // Check this isn't already selected by an existing cursor
            let already_selected = self.cursors.cursors().iter().any(|c| {
                if let Some((s, e)) = c.selection_range() {
                    s == Position::new(start_line, start_col)
                        && e == Position::new(end_line, end_col)
                } else {
                    false
                }
            });

            if !already_selected {
                let mut new_cursor = Cursor::new(end_line, end_col);
                new_cursor.anchor = Some(Position::new(start_line, start_col));
                self.cursors.add_cursor(new_cursor);
            }
        }
    }

    /// Select all occurrences of the current selection (Ctrl+Shift+L).
    pub fn select_all_occurrences(&mut self) {
        let primary = self.cursors.primary();
        if !primary.has_selection() {
            // Select word under cursor first
            self.select_next_occurrence();
            if !self.cursors.primary().has_selection() {
                return;
            }
        }

        let selected = self.selected_text();
        if selected.is_empty() {
            return;
        }

        let text = self.buffer.to_string();
        let mut search_start = 0;

        // Clear extra cursors, keep primary
        self.cursors.clear_extra_cursors();

        // Find all occurrences
        while let Some(byte_offset) = text[search_start..].find(&selected) {
            let abs_byte_offset = search_start + byte_offset;
            let char_start = text[..abs_byte_offset].chars().count();
            let char_end = char_start + selected.chars().count();

            let (start_line, start_col) = self.buffer.char_to_line_col(char_start);
            let (end_line, end_col) = self.buffer.char_to_line_col(char_end);

            // Check if this is already the primary cursor's selection
            let primary = self.cursors.primary();
            let is_primary = if let Some((s, e)) = primary.selection_range() {
                s == Position::new(start_line, start_col) && e == Position::new(end_line, end_col)
            } else {
                false
            };

            if !is_primary {
                let mut new_cursor = Cursor::new(end_line, end_col);
                new_cursor.anchor = Some(Position::new(start_line, start_col));
                self.cursors.add_cursor(new_cursor);
            }

            search_start = abs_byte_offset + selected.len();
        }
    }

    // --- Undo/Redo ---

    pub fn undo(&mut self) {
        if let Some(txn) = self.undo_manager.undo() {
            // Apply inverse operations in reverse order
            for op in txn.ops.iter().rev() {
                op.inverse().apply(&mut self.buffer);
            }
            self.cursors.set_primary(txn.cursor_before);
            self.modified = true;
        }
    }

    pub fn redo(&mut self) {
        if let Some(txn) = self.undo_manager.redo() {
            for op in &txn.ops {
                op.apply(&mut self.buffer);
            }
            self.cursors.set_primary(txn.cursor_after);
            self.modified = true;
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_document() {
        let doc = Document::new();
        assert!(doc.buffer.is_empty());
        assert!(!doc.modified);
        assert_eq!(doc.display_name(), "Untitled");
    }

    #[test]
    fn test_from_str() {
        let doc = Document::from_str("hello\nworld");
        assert_eq!(doc.buffer.len_lines(), 2);
        assert!(!doc.modified);
    }

    #[test]
    fn test_insert_and_undo() {
        let mut doc = Document::new();
        doc.insert_char('h');
        doc.insert_char('i');
        assert_eq!(doc.buffer.to_string(), "hi");
        assert!(doc.modified);

        doc.undo();
        assert_eq!(doc.buffer.to_string(), "h");
        doc.undo();
        assert_eq!(doc.buffer.to_string(), "");
    }

    #[test]
    fn test_display_name_with_path() {
        let mut doc = Document::new();
        doc.path = Some(PathBuf::from("/home/user/test.rs"));
        assert_eq!(doc.display_name(), "test.rs");
    }

    #[test]
    fn test_content_for_save_crlf() {
        let mut doc = Document::from_str("hello\nworld");
        doc.line_ending = LineEnding::CRLF;
        assert_eq!(doc.content_for_save(), "hello\r\nworld");
    }

    #[test]
    fn test_select_all() {
        let mut doc = Document::from_str("abc\ndef");
        doc.select_all();
        assert_eq!(doc.selected_text(), "abc\ndef");
    }

    #[test]
    fn test_go_to_line() {
        let mut doc = Document::from_str("line0\nline1\nline2\nline3");
        doc.go_to_line(2);
        assert_eq!(doc.cursors.primary().position, Position::new(2, 0));
    }

    #[test]
    fn test_insert_newline_with_indent() {
        let mut doc = Document::from_str("    hello");
        // Place cursor at end of line
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 9), false);
        doc.insert_newline_with_indent();
        assert_eq!(doc.buffer.to_string(), "    hello\n    ");
        assert_eq!(doc.cursors.primary().position, Position::new(1, 4));
    }

    #[test]
    fn test_unindent() {
        let mut doc = Document::from_str("        indented");
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 8), false);
        doc.unindent();
        assert_eq!(doc.buffer.to_string(), "    indented");
        assert_eq!(doc.cursors.primary().position, Position::new(0, 4));
    }

    #[test]
    fn test_unindent_partial() {
        let mut doc = Document::from_str("  small");
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 2), false);
        doc.unindent();
        assert_eq!(doc.buffer.to_string(), "small");
        assert_eq!(doc.cursors.primary().position, Position::new(0, 0));
    }

    #[test]
    fn test_delete_word_left() {
        let mut doc = Document::from_str("hello world");
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 11), false);
        doc.delete_word_left();
        assert_eq!(doc.buffer.to_string(), "hello ");
    }

    #[test]
    fn test_delete_word_right() {
        let mut doc = Document::from_str("hello world");
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 0), false);
        doc.delete_word_right();
        assert_eq!(doc.buffer.to_string(), "world");
    }

    #[test]
    fn test_move_cursor_doc_start_end() {
        let mut doc = Document::from_str("abc\ndef\nghi");
        doc.cursors
            .primary_mut()
            .move_to(Position::new(1, 1), false);
        doc.move_cursor_doc_start(false);
        assert_eq!(doc.cursors.primary().position, Position::new(0, 0));
        doc.move_cursor_doc_end(false);
        assert_eq!(doc.cursors.primary().position, Position::new(2, 3));
    }

    #[test]
    fn test_select_next_occurrence_selects_word() {
        let mut doc = Document::from_str("hello world hello");
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 1), false); // inside "hello"
        doc.select_next_occurrence();
        // Should select "hello" (first one)
        assert_eq!(doc.selected_text(), "hello");
        assert_eq!(doc.cursors.primary().anchor, Some(Position::new(0, 0)));
        assert_eq!(doc.cursors.primary().position, Position::new(0, 5));
    }

    #[test]
    fn test_select_next_occurrence_adds_cursor() {
        let mut doc = Document::from_str("hello world hello");
        // Select first "hello" manually
        doc.cursors.primary_mut().anchor = Some(Position::new(0, 0));
        doc.cursors.primary_mut().position = Position::new(0, 5);

        doc.select_next_occurrence();
        assert_eq!(doc.cursors.cursor_count(), 2);
    }

    #[test]
    fn test_select_next_occurrence_no_duplicate() {
        let mut doc = Document::from_str("hello world hello");
        // Select first "hello" manually
        doc.cursors.primary_mut().anchor = Some(Position::new(0, 0));
        doc.cursors.primary_mut().position = Position::new(0, 5);

        doc.select_next_occurrence(); // adds second "hello"
        assert_eq!(doc.cursors.cursor_count(), 2);

        doc.select_next_occurrence(); // should not add duplicate of first "hello"
        assert_eq!(doc.cursors.cursor_count(), 2);
    }

    #[test]
    fn test_select_next_occurrence_wraps_around() {
        let mut doc = Document::from_str("hello world hello");
        // Select second "hello" manually (position 12..17)
        doc.cursors.primary_mut().anchor = Some(Position::new(0, 12));
        doc.cursors.primary_mut().position = Position::new(0, 17);

        doc.select_next_occurrence(); // should wrap around and find first "hello"
        assert_eq!(doc.cursors.cursor_count(), 2);
    }

    #[test]
    fn test_select_next_occurrence_empty_doc() {
        let mut doc = Document::from_str("");
        doc.select_next_occurrence(); // should not panic
        assert_eq!(doc.cursors.cursor_count(), 1);
    }

    #[test]
    fn test_select_next_occurrence_non_word_char() {
        let mut doc = Document::from_str("hello world");
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 5), false); // on space
        doc.select_next_occurrence(); // should do nothing
        assert!(!doc.cursors.primary().has_selection());
    }

    // --- Bookmark tests ---

    #[test]
    fn test_toggle_bookmark() {
        let mut doc = Document::from_str("line0\nline1\nline2\nline3");
        assert!(doc.bookmarks.is_empty());

        doc.toggle_bookmark(1);
        assert_eq!(doc.bookmarks, vec![1]);

        doc.toggle_bookmark(3);
        assert_eq!(doc.bookmarks, vec![1, 3]);

        // Toggle off
        doc.toggle_bookmark(1);
        assert_eq!(doc.bookmarks, vec![3]);
    }

    #[test]
    fn test_toggle_bookmark_sorted() {
        let mut doc = Document::from_str("a\nb\nc\nd\ne");
        doc.toggle_bookmark(3);
        doc.toggle_bookmark(1);
        doc.toggle_bookmark(4);
        assert_eq!(doc.bookmarks, vec![1, 3, 4]);
    }

    #[test]
    fn test_next_bookmark() {
        let mut doc = Document::from_str("a\nb\nc\nd\ne");
        doc.toggle_bookmark(1);
        doc.toggle_bookmark(3);

        assert_eq!(doc.next_bookmark(0), Some(1));
        assert_eq!(doc.next_bookmark(1), Some(3));
        assert_eq!(doc.next_bookmark(2), Some(3));
    }

    #[test]
    fn test_next_bookmark_wraps() {
        let mut doc = Document::from_str("a\nb\nc\nd\ne");
        doc.toggle_bookmark(1);
        doc.toggle_bookmark(3);

        // Past all bookmarks, should wrap to first
        assert_eq!(doc.next_bookmark(3), Some(1));
        assert_eq!(doc.next_bookmark(4), Some(1));
    }

    #[test]
    fn test_prev_bookmark() {
        let mut doc = Document::from_str("a\nb\nc\nd\ne");
        doc.toggle_bookmark(1);
        doc.toggle_bookmark(3);

        assert_eq!(doc.prev_bookmark(4), Some(3));
        assert_eq!(doc.prev_bookmark(3), Some(1));
        assert_eq!(doc.prev_bookmark(2), Some(1));
    }

    #[test]
    fn test_prev_bookmark_wraps() {
        let mut doc = Document::from_str("a\nb\nc\nd\ne");
        doc.toggle_bookmark(1);
        doc.toggle_bookmark(3);

        // Before all bookmarks, should wrap to last
        assert_eq!(doc.prev_bookmark(1), Some(3));
        assert_eq!(doc.prev_bookmark(0), Some(3));
    }

    #[test]
    fn test_bookmark_empty() {
        let doc = Document::from_str("a\nb\nc");
        assert_eq!(doc.next_bookmark(0), None);
        assert_eq!(doc.prev_bookmark(0), None);
    }

    #[test]
    fn test_clear_bookmarks() {
        let mut doc = Document::from_str("a\nb\nc\nd");
        doc.toggle_bookmark(0);
        doc.toggle_bookmark(2);
        doc.toggle_bookmark(3);
        assert_eq!(doc.bookmarks.len(), 3);

        doc.clear_bookmarks();
        assert!(doc.bookmarks.is_empty());
    }

    #[test]
    fn test_single_bookmark_navigation() {
        let mut doc = Document::from_str("a\nb\nc");
        doc.toggle_bookmark(1);

        // Next from any position wraps to the single bookmark
        assert_eq!(doc.next_bookmark(0), Some(1));
        assert_eq!(doc.next_bookmark(1), Some(1)); // wraps to itself
        assert_eq!(doc.next_bookmark(2), Some(1));

        // Prev from any position wraps to the single bookmark
        assert_eq!(doc.prev_bookmark(0), Some(1)); // wraps to itself
        assert_eq!(doc.prev_bookmark(1), Some(1)); // wraps to itself
        assert_eq!(doc.prev_bookmark(2), Some(1));
    }

    // --- Column Insert tests ---

    #[test]
    fn test_column_insert_text() {
        let mut doc = Document::from_str("aaa\nbbb\nccc\n");
        doc.column_insert_text(0, 2, 1, "X");
        assert_eq!(doc.buffer.to_string(), "aXaa\nbXbb\ncXcc\n");
    }

    #[test]
    fn test_column_insert_text_pads_short_lines() {
        let mut doc = Document::from_str("ab\nc\ndefg\n");
        doc.column_insert_text(0, 2, 3, "|");
        // "ab" (len 2) -> pad to col 3 then insert = "ab |"
        // "c"  (len 1) -> pad to col 3 then insert = "c  |"
        // "defg" (len 4) -> insert at col 3 = "def|g"
        assert_eq!(doc.buffer.to_string(), "ab |\nc  |\ndef|g\n");
    }

    #[test]
    fn test_column_insert_numbers() {
        let mut doc = Document::from_str("aaa\nbbb\nccc\n");
        doc.column_insert_numbers(0, 2, 0, 1, 1, 0);
        assert_eq!(doc.buffer.to_string(), "1aaa\n2bbb\n3ccc\n");
    }

    #[test]
    fn test_column_insert_numbers_padded() {
        let mut doc = Document::from_str("aaa\nbbb\nccc\n");
        doc.column_insert_numbers(0, 2, 0, 1, 1, 3);
        assert_eq!(doc.buffer.to_string(), "001aaa\n002bbb\n003ccc\n");
    }

    #[test]
    fn test_column_insert_numbers_step() {
        let mut doc = Document::from_str("x\nx\nx\nx\n");
        doc.column_insert_numbers(0, 3, 1, 10, 5, 0);
        assert_eq!(doc.buffer.to_string(), "x10\nx15\nx20\nx25\n");
    }

    #[test]
    fn test_column_insert_text_undo() {
        let mut doc = Document::from_str("aaa\nbbb\n");
        doc.column_insert_text(0, 1, 1, "X");
        assert_eq!(doc.buffer.to_string(), "aXaa\nbXbb\n");
        doc.undo();
        assert_eq!(doc.buffer.to_string(), "aaa\nbbb\n");
    }

    #[test]
    fn test_column_insert_on_empty_line_range() {
        let mut doc = Document::from_str("abc\ndef\n");
        // start > end should be a no-op
        doc.column_insert_text(2, 0, 0, "X");
        assert_eq!(doc.buffer.to_string(), "abc\ndef\n");
    }

    // --- Toggle Comment tests ---

    #[test]
    fn test_toggle_comment_single_line_add() {
        let mut doc = Document::from_str("hello world");
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 5), false);
        doc.toggle_comment();
        assert_eq!(doc.buffer.to_string(), "// hello world");
        assert!(doc.modified);
    }

    #[test]
    fn test_toggle_comment_single_line_remove() {
        let mut doc = Document::from_str("// hello world");
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 5), false);
        doc.toggle_comment();
        assert_eq!(doc.buffer.to_string(), "hello world");
        assert!(doc.modified);
    }

    #[test]
    fn test_toggle_comment_preserves_indent() {
        let mut doc = Document::from_str("    hello world");
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 8), false);
        doc.toggle_comment();
        assert_eq!(doc.buffer.to_string(), "    // hello world");
    }

    #[test]
    fn test_toggle_comment_remove_with_indent() {
        let mut doc = Document::from_str("    // hello world");
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 8), false);
        doc.toggle_comment();
        assert_eq!(doc.buffer.to_string(), "    hello world");
    }

    #[test]
    fn test_toggle_comment_multiline_add() {
        let mut doc = Document::from_str("line one\nline two\nline three");
        // Select from line 0 to line 2
        doc.cursors.primary_mut().anchor = Some(Position::new(0, 0));
        doc.cursors.primary_mut().position = Position::new(2, 5);
        doc.toggle_comment();
        assert_eq!(
            doc.buffer.to_string(),
            "// line one\n// line two\n// line three"
        );
    }

    #[test]
    fn test_toggle_comment_multiline_remove() {
        let mut doc = Document::from_str("// line one\n// line two\n// line three");
        doc.cursors.primary_mut().anchor = Some(Position::new(0, 0));
        doc.cursors.primary_mut().position = Position::new(2, 5);
        doc.toggle_comment();
        assert_eq!(doc.buffer.to_string(), "line one\nline two\nline three");
    }

    #[test]
    fn test_toggle_comment_mixed_lines_comments_all() {
        // When some lines are commented and some aren't, all should be commented
        let mut doc = Document::from_str("// already\nnot commented\n// also here");
        doc.cursors.primary_mut().anchor = Some(Position::new(0, 0));
        doc.cursors.primary_mut().position = Position::new(2, 5);
        doc.toggle_comment();
        assert_eq!(
            doc.buffer.to_string(),
            "// // already\n// not commented\n// // also here"
        );
    }

    #[test]
    fn test_toggle_comment_python_language() {
        let mut doc = Document::from_str("print('hello')");
        doc.language = Some("python".to_string());
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 5), false);
        doc.toggle_comment();
        assert_eq!(doc.buffer.to_string(), "# print('hello')");
    }

    #[test]
    fn test_toggle_comment_python_uncomment() {
        let mut doc = Document::from_str("# print('hello')");
        doc.language = Some("python".to_string());
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 5), false);
        doc.toggle_comment();
        assert_eq!(doc.buffer.to_string(), "print('hello')");
    }

    #[test]
    fn test_toggle_comment_lua_language() {
        let mut doc = Document::from_str("print('hello')");
        doc.language = Some("lua".to_string());
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 5), false);
        doc.toggle_comment();
        assert_eq!(doc.buffer.to_string(), "-- print('hello')");
    }

    #[test]
    fn test_toggle_comment_empty_lines_skipped() {
        let mut doc = Document::from_str("hello\n\nworld");
        doc.cursors.primary_mut().anchor = Some(Position::new(0, 0));
        doc.cursors.primary_mut().position = Position::new(2, 3);
        doc.toggle_comment();
        // Empty line should be left alone
        assert_eq!(doc.buffer.to_string(), "// hello\n\n// world");
    }

    #[test]
    fn test_toggle_comment_undo() {
        let mut doc = Document::from_str("hello world");
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 5), false);
        doc.toggle_comment();
        assert_eq!(doc.buffer.to_string(), "// hello world");
        doc.undo();
        assert_eq!(doc.buffer.to_string(), "hello world");
    }

    #[test]
    fn test_toggle_comment_undo_uncomment() {
        let mut doc = Document::from_str("// hello world");
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 5), false);
        doc.toggle_comment();
        assert_eq!(doc.buffer.to_string(), "hello world");
        doc.undo();
        assert_eq!(doc.buffer.to_string(), "// hello world");
    }

    #[test]
    fn test_toggle_comment_read_only_noop() {
        let mut doc = Document::from_str("hello world");
        doc.read_only = true;
        doc.cursors
            .primary_mut()
            .move_to(Position::new(0, 5), false);
        doc.toggle_comment();
        assert_eq!(doc.buffer.to_string(), "hello world");
        assert!(!doc.modified);
    }

    #[test]
    fn test_toggle_comment_minimum_indent_preserved() {
        let mut doc = Document::from_str("    line1\n        line2\n    line3");
        doc.cursors.primary_mut().anchor = Some(Position::new(0, 0));
        doc.cursors.primary_mut().position = Position::new(2, 5);
        doc.toggle_comment();
        // Comment at minimum indent (4 spaces)
        assert_eq!(
            doc.buffer.to_string(),
            "    // line1\n    //     line2\n    // line3"
        );
    }

    #[test]
    fn test_toggle_comment_default_prefix_for_unknown_language() {
        let mut doc = Document::from_str("hello");
        doc.language = Some("unknown_language".to_string());
        doc.toggle_comment();
        assert_eq!(doc.buffer.to_string(), "// hello");
    }

    #[test]
    fn test_toggle_comment_no_language() {
        let mut doc = Document::from_str("hello");
        doc.language = None;
        doc.toggle_comment();
        assert_eq!(doc.buffer.to_string(), "// hello");
    }
}
