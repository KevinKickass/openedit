#[allow(dead_code)]
use openedit_core::cursor::Position;
use openedit_core::Document;

/// Vim editing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMode {
    Normal,
    Insert,
    Visual,
    VisualLine,
    Command,
}

impl std::fmt::Display for VimMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VimMode::Normal => write!(f, "NORMAL"),
            VimMode::Insert => write!(f, "INSERT"),
            VimMode::Visual => write!(f, "VISUAL"),
            VimMode::VisualLine => write!(f, "V-LINE"),
            VimMode::Command => write!(f, "COMMAND"),
        }
    }
}

/// A recorded vim action for the `.` repeat command.
#[derive(Debug, Clone)]
enum VimAction {
    #[allow(dead_code)]
    InsertText(String),
    #[allow(dead_code)]
    Delete {
        motion: VimMotion,
        count: usize,
    },
    #[allow(dead_code)]
    Change {
        motion: VimMotion,
        count: usize,
    },
    Yank {
        motion: VimMotion,
        count: usize,
    },
    Put,
    DeleteLine(usize),
    YankLine(usize),
    ReplaceChar(char),
    JoinLines,
    OpenLineBelow,
    OpenLineAbove,
    Indent,
    Unindent,
    ToggleComment,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum VimMotion {
    Word,
    WordEnd,
    BackWord,
    InnerWord,
    Line,
    ToEnd,
    ToStart,
    CharRight,
    CharLeft,
    FindChar(char),
    TilChar(char),
}

/// Macro slot for `q`/`@` recording.
#[derive(Debug, Clone)]
struct VimMacro {
    keys: Vec<String>,
}

/// State for vim mode editing.
pub struct VimState {
    pub enabled: bool,
    pub mode: VimMode,
    /// Command-line input when in Command mode.
    pub command_line: String,
    /// Pending operator (d, c, y, etc.) waiting for a motion.
    pending_op: Option<char>,
    /// Numeric count prefix.
    count: Option<usize>,
    /// Register for yank/delete (default unnamed).
    #[allow(dead_code)]
    register: char,
    /// Yank register contents.
    pub yank_register: String,
    /// Whether yanked text is line-wise.
    yank_linewise: bool,
    /// Last repeatable action for `.`.
    last_action: Option<VimAction>,
    /// Text inserted during last insert mode session (for `.` repeat).
    insert_text_buffer: String,
    /// Last insert action's entry method for `.` repeat.
    #[allow(dead_code)]
    last_insert_entry: Option<VimAction>,
    /// Macro recording state.
    recording_macro: Option<char>,
    macro_buffer: Vec<String>,
    macros: std::collections::HashMap<char, VimMacro>,
    /// Visual mode anchor position.
    visual_anchor: Option<Position>,
}

impl Default for VimState {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: VimMode::Normal,
            command_line: String::new(),
            pending_op: None,
            count: None,
            #[allow(dead_code)]
            register: '"',
            yank_register: String::new(),
            yank_linewise: false,
            last_action: None,
            insert_text_buffer: String::new(),
            #[allow(dead_code)]
            last_insert_entry: None,
            recording_macro: None,
            macro_buffer: Vec::new(),
            macros: std::collections::HashMap::new(),
            visual_anchor: None,
        }
    }
}

impl VimState {
    pub fn new() -> Self {
        Self::default()
    }

    fn get_count(&mut self) -> usize {
        self.count.take().unwrap_or(1)
    }

    fn reset_pending(&mut self) {
        self.pending_op = None;
        self.count = None;
    }

    /// Handle a key event in vim mode. Returns true if the key was consumed.
    /// `modified` output parameter indicates if the document was modified.
    pub fn handle_key(&mut self, key_str: &str, doc: &mut Document, modified: &mut bool) -> bool {
        if !self.enabled {
            return false;
        }

        // Record key for macro
        if self.recording_macro.is_some() && key_str != "q" {
            self.macro_buffer.push(key_str.to_string());
        }

        match self.mode {
            VimMode::Normal => self.handle_normal(key_str, doc, modified),
            VimMode::Insert => self.handle_insert(key_str, doc, modified),
            VimMode::Visual | VimMode::VisualLine => self.handle_visual(key_str, doc, modified),
            VimMode::Command => self.handle_command(key_str, doc, modified),
        }
    }

    fn handle_normal(&mut self, key: &str, doc: &mut Document, modified: &mut bool) -> bool {
        // Handle count prefix
        if key.len() == 1 {
            let ch = key.chars().next().unwrap();
            if ch.is_ascii_digit() && (ch != '0' || self.count.is_some()) {
                let digit = ch.to_digit(10).unwrap() as usize;
                self.count = Some(self.count.unwrap_or(0) * 10 + digit);
                return true;
            }
        }

        // Handle pending operator + motion
        if let Some(op) = self.pending_op {
            // Save the pending_op before calling handler, since handler may set a new pending
            let consumed = self.handle_operator_motion(op, key, doc, modified);
            if consumed {
                // Only clear pending_op if the handler didn't set a new one
                // (e.g., 'c' + 'i' sets pending to 'C' for inner text object)
                if self.pending_op == Some(op) {
                    self.pending_op = None;
                }
                return true;
            }
        }

        match key {
            // Mode switching
            "i" => {
                self.mode = VimMode::Insert;
                self.insert_text_buffer.clear();
                self.reset_pending();
                return true;
            }
            "I" => {
                doc.move_cursor_home(false);
                self.mode = VimMode::Insert;
                self.insert_text_buffer.clear();
                self.reset_pending();
                return true;
            }
            "a" => {
                doc.move_cursor_right(false);
                self.mode = VimMode::Insert;
                self.insert_text_buffer.clear();
                self.reset_pending();
                return true;
            }
            "A" => {
                doc.move_cursor_end(false);
                self.mode = VimMode::Insert;
                self.insert_text_buffer.clear();
                self.reset_pending();
                return true;
            }
            "o" => {
                doc.move_cursor_end(false);
                doc.insert_newline_with_indent();
                self.mode = VimMode::Insert;
                self.insert_text_buffer.clear();
                *modified = true;
                self.last_action = Some(VimAction::OpenLineBelow);
                self.reset_pending();
                return true;
            }
            "O" => {
                doc.move_cursor_home(false);
                doc.insert_newline_with_indent();
                doc.move_cursor_up(false);
                self.mode = VimMode::Insert;
                self.insert_text_buffer.clear();
                *modified = true;
                self.last_action = Some(VimAction::OpenLineAbove);
                self.reset_pending();
                return true;
            }
            "v" => {
                self.mode = VimMode::Visual;
                self.visual_anchor = Some(doc.cursors.primary().position);
                self.reset_pending();
                return true;
            }
            "V" => {
                self.mode = VimMode::VisualLine;
                self.visual_anchor = Some(doc.cursors.primary().position);
                // Select current line
                let line = doc.cursors.primary().position.line;
                let line_len = doc.buffer.line_len_chars_no_newline(line);
                doc.cursors.primary_mut().anchor = Some(Position::new(line, 0));
                doc.cursors.primary_mut().position = Position::new(line, line_len);
                self.reset_pending();
                return true;
            }
            ":" => {
                self.mode = VimMode::Command;
                self.command_line.clear();
                self.reset_pending();
                return true;
            }
            // Movement
            "h" => {
                let count = self.get_count();
                for _ in 0..count {
                    doc.move_cursor_left(false);
                }
                return true;
            }
            "j" => {
                let count = self.get_count();
                for _ in 0..count {
                    doc.move_cursor_down(false);
                }
                return true;
            }
            "k" => {
                let count = self.get_count();
                for _ in 0..count {
                    doc.move_cursor_up(false);
                }
                return true;
            }
            "l" => {
                let count = self.get_count();
                for _ in 0..count {
                    doc.move_cursor_right(false);
                }
                return true;
            }
            "w" => {
                let count = self.get_count();
                for _ in 0..count {
                    doc.move_cursor_word_right(false);
                }
                return true;
            }
            "b" => {
                let count = self.get_count();
                for _ in 0..count {
                    doc.move_cursor_word_left(false);
                }
                return true;
            }
            "e" => {
                let count = self.get_count();
                for _ in 0..count {
                    move_to_word_end(doc);
                }
                return true;
            }
            "0" => {
                let pos = doc.cursors.primary().position;
                doc.cursors
                    .primary_mut()
                    .move_to(Position::new(pos.line, 0), false);
                return true;
            }
            "^" => {
                doc.move_cursor_home(false);
                return true;
            }
            "$" => {
                doc.move_cursor_end(false);
                return true;
            }
            "G" => {
                let count = self.count.take();
                if let Some(n) = count {
                    doc.go_to_line(n.saturating_sub(1));
                } else {
                    doc.move_cursor_doc_end(false);
                }
                return true;
            }
            "g" => {
                // Wait for next key (gg)
                self.pending_op = Some('g');
                return true;
            }
            // Editing
            "x" => {
                let count = self.get_count();
                for _ in 0..count {
                    doc.delete_forward();
                }
                *modified = true;
                return true;
            }
            "X" => {
                let count = self.get_count();
                for _ in 0..count {
                    doc.backspace();
                }
                *modified = true;
                return true;
            }
            "r" => {
                // Replace char — wait for next char via pending
                self.pending_op = Some('r');
                return true;
            }
            "J" => {
                // Join lines
                let pos = doc.cursors.primary().position;
                let line_len = doc.buffer.line_len_chars_no_newline(pos.line);
                doc.cursors
                    .primary_mut()
                    .move_to(Position::new(pos.line, line_len), false);
                doc.delete_forward(); // delete newline
                doc.insert_text(" ");
                *modified = true;
                self.last_action = Some(VimAction::JoinLines);
                self.reset_pending();
                return true;
            }
            // Operators (wait for motion)
            "d" => {
                self.pending_op = Some('d');
                return true;
            }
            "c" => {
                self.pending_op = Some('c');
                return true;
            }
            "y" => {
                self.pending_op = Some('y');
                return true;
            }
            ">" => {
                self.pending_op = Some('>');
                return true;
            }
            "<" => {
                self.pending_op = Some('<');
                return true;
            }
            // Put
            "p" => {
                let count = self.get_count();
                for _ in 0..count {
                    self.put_after(doc);
                }
                *modified = true;
                self.last_action = Some(VimAction::Put);
                return true;
            }
            "P" => {
                let count = self.get_count();
                for _ in 0..count {
                    self.put_before(doc);
                }
                *modified = true;
                return true;
            }
            // Repeat
            "." => {
                self.repeat_last_action(doc, modified);
                return true;
            }
            // Undo/redo
            "u" => {
                doc.undo();
                return true;
            }
            // Ctrl+R for redo is handled as special key
            // Macro recording
            "q" => {
                if let Some(reg) = self.recording_macro.take() {
                    // Stop recording
                    let keys = std::mem::take(&mut self.macro_buffer);
                    self.macros.insert(reg, VimMacro { keys });
                } else {
                    // Will need next key for register - use pending
                    self.pending_op = Some('Q'); // Q = start macro recording
                }
                return true;
            }
            "@" => {
                self.pending_op = Some('@');
                return true;
            }
            // Search
            "/" => {
                self.mode = VimMode::Command;
                self.command_line = "/".to_string();
                return true;
            }
            "?" => {
                self.mode = VimMode::Command;
                self.command_line = "?".to_string();
                return true;
            }
            "n" => {
                // Next search match
                if let Some(idx) = doc.search.current_match {
                    let next = (idx + 1) % doc.search.matches.len().max(1);
                    doc.search.current_match = Some(next);
                    if let Some(m) = doc.search.matches.get(next) {
                        let (line, col) = doc.buffer.char_to_line_col(m.start);
                        doc.cursors
                            .primary_mut()
                            .move_to(Position::new(line, col), false);
                    }
                }
                return true;
            }
            "N" => {
                // Previous search match
                if let Some(idx) = doc.search.current_match {
                    let prev = if idx == 0 {
                        doc.search.matches.len().saturating_sub(1)
                    } else {
                        idx - 1
                    };
                    doc.search.current_match = Some(prev);
                    if let Some(m) = doc.search.matches.get(prev) {
                        let (line, col) = doc.buffer.char_to_line_col(m.start);
                        doc.cursors
                            .primary_mut()
                            .move_to(Position::new(line, col), false);
                    }
                }
                return true;
            }
            "Escape" => {
                self.reset_pending();
                if doc.cursors.cursor_count() > 1 {
                    doc.cursors.clear_extra_cursors();
                }
                return true;
            }
            _ => {}
        }

        // Special key handling
        match key {
            "Ctrl+r" | "Ctrl+R" => {
                doc.redo();
                return true;
            }
            "Ctrl+f" => {
                doc.move_cursor_page_down(30, false);
                return true;
            }
            "Ctrl+b" => {
                doc.move_cursor_page_up(30, false);
                return true;
            }
            "Ctrl+d" => {
                for _ in 0..15 {
                    doc.move_cursor_down(false);
                }
                return true;
            }
            "Ctrl+u" => {
                for _ in 0..15 {
                    doc.move_cursor_up(false);
                }
                return true;
            }
            _ => {}
        }

        false
    }

    fn handle_operator_motion(
        &mut self,
        op: char,
        key: &str,
        doc: &mut Document,
        modified: &mut bool,
    ) -> bool {
        // Handle special pending ops
        match op {
            'r' => {
                // Replace single char
                if key.len() == 1 {
                    let ch = key.chars().next().unwrap();
                    doc.delete_forward();
                    doc.insert_text(&ch.to_string());
                    doc.move_cursor_left(false);
                    *modified = true;
                    self.last_action = Some(VimAction::ReplaceChar(ch));
                    self.reset_pending();
                    return true;
                }
                self.reset_pending();
                return true;
            }
            'g' => match key {
                "g" => {
                    let count = self.count.take();
                    if let Some(n) = count {
                        doc.go_to_line(n.saturating_sub(1));
                    } else {
                        doc.move_cursor_doc_start(false);
                    }
                    return true;
                }
                _ => {
                    self.reset_pending();
                    return false;
                }
            },
            'Q' => {
                // Start recording macro to register `key`
                if key.len() == 1 {
                    let reg = key.chars().next().unwrap();
                    self.recording_macro = Some(reg);
                    self.macro_buffer.clear();
                }
                self.reset_pending();
                return true;
            }
            '@' => {
                // Replay macro
                if key.len() == 1 {
                    let reg = key.chars().next().unwrap();
                    if let Some(mac) = self.macros.get(&reg).cloned() {
                        let count = self.get_count();
                        for _ in 0..count {
                            for mkey in &mac.keys {
                                self.handle_key(mkey, doc, modified);
                            }
                        }
                    }
                }
                self.reset_pending();
                return true;
            }
            _ => {}
        }

        let count = self.get_count();

        // dd, cc, yy — operate on whole lines
        if key.len() == 1 && key.chars().next().unwrap() == op {
            match op {
                'd' => {
                    for _ in 0..count {
                        let line_text = doc
                            .buffer
                            .line(doc.cursors.primary().position.line)
                            .to_string();
                        self.yank_register = line_text;
                        self.yank_linewise = true;
                        doc.delete_line();
                    }
                    *modified = true;
                    self.last_action = Some(VimAction::DeleteLine(count));
                    return true;
                }
                'c' => {
                    for _ in 0..count {
                        doc.delete_line();
                    }
                    doc.insert_newline_with_indent();
                    doc.move_cursor_up(false);
                    self.mode = VimMode::Insert;
                    self.insert_text_buffer.clear();
                    *modified = true;
                    return true;
                }
                'y' => {
                    let line = doc.cursors.primary().position.line;
                    let mut text = String::new();
                    for i in 0..count {
                        if line + i < doc.buffer.len_lines() {
                            text.push_str(&doc.buffer.line(line + i).to_string());
                        }
                    }
                    self.yank_register = text;
                    self.yank_linewise = true;
                    self.last_action = Some(VimAction::YankLine(count));
                    return true;
                }
                '>' => {
                    for _ in 0..count {
                        doc.insert_text("    ");
                        doc.move_cursor_home(false);
                    }
                    *modified = true;
                    return true;
                }
                '<' => {
                    for _ in 0..count {
                        doc.unindent();
                    }
                    *modified = true;
                    return true;
                }
                _ => {}
            }
        }

        // Handle inner text objects (second key after `i`) — must be before motion matching
        // so that e.g. 'C' + "w" dispatches to InnerWord, not Word motion.
        match op {
            'D' | 'C' | 'Y' => {
                let real_op = match op {
                    'D' => 'd',
                    'C' => 'c',
                    'Y' => 'y',
                    _ => unreachable!(),
                };
                match key {
                    "w" => {
                        self.execute_operator_with_motion(
                            real_op,
                            doc,
                            count,
                            VimMotion::InnerWord,
                            modified,
                        );
                        return true;
                    }
                    _ => {
                        self.reset_pending();
                        return false;
                    }
                }
            }
            _ => {}
        }

        // diw, ciw, yiw etc.
        if key == "i" {
            // Set pending to wait for text object
            // We need a second level pending — encode as combined op
            self.pending_op = Some(match op {
                'd' => 'D', // D = d + i (inner text object pending)
                'c' => 'C',
                'y' => 'Y',
                _ => {
                    self.reset_pending();
                    return false;
                }
            });
            return true;
        }

        // Operator + motion
        match key {
            "w" => {
                self.execute_operator_with_motion(op, doc, count, VimMotion::Word, modified);
                return true;
            }
            "e" => {
                self.execute_operator_with_motion(op, doc, count, VimMotion::WordEnd, modified);
                return true;
            }
            "b" => {
                self.execute_operator_with_motion(op, doc, count, VimMotion::BackWord, modified);
                return true;
            }
            "$" => {
                self.execute_operator_with_motion(op, doc, count, VimMotion::ToEnd, modified);
                return true;
            }
            "0" | "^" => {
                self.execute_operator_with_motion(op, doc, count, VimMotion::ToStart, modified);
                return true;
            }
            "l" => {
                self.execute_operator_with_motion(op, doc, count, VimMotion::CharRight, modified);
                return true;
            }
            "h" => {
                self.execute_operator_with_motion(op, doc, count, VimMotion::CharLeft, modified);
                return true;
            }
            _ => {}
        }

        self.reset_pending();
        false
    }

    fn execute_operator_with_motion(
        &mut self,
        op: char,
        doc: &mut Document,
        count: usize,
        motion: VimMotion,
        modified: &mut bool,
    ) {
        // Select the range covered by the motion
        let start = doc.cursors.primary().position;

        // For inner word: select the word under cursor
        if matches!(motion, VimMotion::InnerWord) {
            select_inner_word(doc);
        } else {
            // Set anchor and move
            doc.cursors.primary_mut().anchor = Some(start);
            for _ in 0..count {
                match motion {
                    VimMotion::Word => doc.move_cursor_word_right(true),
                    VimMotion::WordEnd => {
                        doc.move_cursor_right(true);
                        // Move to end of word
                        move_to_word_end_select(doc);
                    }
                    VimMotion::BackWord => doc.move_cursor_word_left(true),
                    VimMotion::ToEnd => doc.move_cursor_end(true),
                    VimMotion::ToStart => doc.move_cursor_home(true),
                    VimMotion::CharRight => doc.move_cursor_right(true),
                    VimMotion::CharLeft => doc.move_cursor_left(true),
                    _ => {}
                }
            }
        }

        let selected = doc.selected_text();
        if selected.is_empty() {
            doc.cursors.primary_mut().anchor = None;
            return;
        }

        match op {
            'd' => {
                self.yank_register = selected;
                self.yank_linewise = false;
                doc.delete_selection_public();
                *modified = true;
                self.last_action = Some(VimAction::Delete { motion, count });
            }
            'c' => {
                self.yank_register = selected;
                self.yank_linewise = false;
                doc.delete_selection_public();
                self.mode = VimMode::Insert;
                self.insert_text_buffer.clear();
                *modified = true;
                self.last_action = Some(VimAction::Change { motion, count });
            }
            'y' => {
                self.yank_register = selected;
                self.yank_linewise = false;
                // Deselect
                doc.cursors.primary_mut().anchor = None;
                doc.cursors.primary_mut().position = start;
                self.last_action = Some(VimAction::Yank { motion, count });
            }
            _ => {
                doc.cursors.primary_mut().anchor = None;
            }
        }
    }

    fn handle_insert(&mut self, key: &str, doc: &mut Document, _modified: &mut bool) -> bool {
        match key {
            "Escape" => {
                self.mode = VimMode::Normal;
                // Move cursor left (vim behavior: cursor backs up one on Escape)
                doc.move_cursor_left(false);
                // Save insert text for `.` repeat
                if !self.insert_text_buffer.is_empty() {
                    self.last_action = Some(VimAction::InsertText(self.insert_text_buffer.clone()));
                }
                true
            }
            _ => {
                // Let normal editor handle it
                false
            }
        }
    }

    fn handle_visual(&mut self, key: &str, doc: &mut Document, modified: &mut bool) -> bool {
        match key {
            "Escape" => {
                self.mode = VimMode::Normal;
                doc.cursors.primary_mut().anchor = None;
                self.visual_anchor = None;
                return true;
            }
            // Movement with selection
            "h" => {
                let count = self.get_count();
                for _ in 0..count {
                    doc.move_cursor_left(true);
                }
                self.update_visual_selection(doc);
                return true;
            }
            "j" => {
                let count = self.get_count();
                for _ in 0..count {
                    doc.move_cursor_down(true);
                }
                self.update_visual_selection(doc);
                return true;
            }
            "k" => {
                let count = self.get_count();
                for _ in 0..count {
                    doc.move_cursor_up(true);
                }
                self.update_visual_selection(doc);
                return true;
            }
            "l" => {
                let count = self.get_count();
                for _ in 0..count {
                    doc.move_cursor_right(true);
                }
                self.update_visual_selection(doc);
                return true;
            }
            "w" => {
                doc.move_cursor_word_right(true);
                self.update_visual_selection(doc);
                return true;
            }
            "b" => {
                doc.move_cursor_word_left(true);
                self.update_visual_selection(doc);
                return true;
            }
            "$" => {
                doc.move_cursor_end(true);
                self.update_visual_selection(doc);
                return true;
            }
            "0" | "^" => {
                doc.move_cursor_home(true);
                self.update_visual_selection(doc);
                return true;
            }
            "G" => {
                doc.move_cursor_doc_end(true);
                self.update_visual_selection(doc);
                return true;
            }
            // Operations on selection
            "d" | "x" => {
                let text = doc.selected_text();
                self.yank_register = text;
                self.yank_linewise = self.mode == VimMode::VisualLine;
                doc.delete_selection_public();
                *modified = true;
                self.mode = VimMode::Normal;
                self.visual_anchor = None;
                return true;
            }
            "y" => {
                let text = doc.selected_text();
                self.yank_register = text;
                self.yank_linewise = self.mode == VimMode::VisualLine;
                let anchor = self.visual_anchor.unwrap_or(doc.cursors.primary().position);
                doc.cursors.primary_mut().anchor = None;
                doc.cursors.primary_mut().position = anchor;
                self.mode = VimMode::Normal;
                self.visual_anchor = None;
                return true;
            }
            "c" => {
                let text = doc.selected_text();
                self.yank_register = text;
                self.yank_linewise = false;
                doc.delete_selection_public();
                *modified = true;
                self.mode = VimMode::Insert;
                self.insert_text_buffer.clear();
                self.visual_anchor = None;
                return true;
            }
            ">" => {
                // Indent selected lines
                doc.insert_text("    ");
                *modified = true;
                self.mode = VimMode::Normal;
                doc.cursors.primary_mut().anchor = None;
                self.visual_anchor = None;
                return true;
            }
            "<" => {
                doc.unindent();
                *modified = true;
                self.mode = VimMode::Normal;
                doc.cursors.primary_mut().anchor = None;
                self.visual_anchor = None;
                return true;
            }
            _ => {}
        }
        false
    }

    fn update_visual_selection(&self, doc: &mut Document) {
        if let Some(anchor) = self.visual_anchor {
            if self.mode == VimMode::VisualLine {
                let cur = doc.cursors.primary().position;
                let (start_line, end_line) = if anchor.line <= cur.line {
                    (anchor.line, cur.line)
                } else {
                    (cur.line, anchor.line)
                };
                let end_col = doc.buffer.line_len_chars_no_newline(end_line);
                doc.cursors.primary_mut().anchor = Some(Position::new(start_line, 0));
                doc.cursors.primary_mut().position = Position::new(end_line, end_col);
            }
            // For character-wise visual mode, anchor is already set
        }
    }

    fn handle_command(&mut self, key: &str, doc: &mut Document, modified: &mut bool) -> bool {
        match key {
            "Escape" => {
                self.mode = VimMode::Normal;
                self.command_line.clear();
                true
            }
            "Enter" => {
                let cmd = self.command_line.clone();
                self.mode = VimMode::Normal;
                self.command_line.clear();
                self.execute_ex_command(&cmd, doc, modified);
                true
            }
            "Backspace" => {
                self.command_line.pop();
                if self.command_line.is_empty() {
                    self.mode = VimMode::Normal;
                }
                true
            }
            _ => {
                if key.len() == 1 && !key.chars().next().unwrap().is_control() {
                    self.command_line.push_str(key);
                }
                true
            }
        }
    }

    fn execute_ex_command(&mut self, cmd: &str, doc: &mut Document, _modified: &mut bool) {
        let cmd = cmd.trim();

        // Search commands
        if let Some(pattern) = cmd.strip_prefix('/') {
            if !pattern.is_empty() {
                let _ = doc.search.set_query(pattern);
                let text = doc.buffer.to_string();
                doc.search.find_all(&text);
                // Jump to first match
                if let Some(m) = doc.search.matches.first() {
                    let (line, col) = doc.buffer.char_to_line_col(m.start);
                    doc.cursors
                        .primary_mut()
                        .move_to(Position::new(line, col), false);
                }
            }
            return;
        }
        if let Some(pattern) = cmd.strip_prefix('?') {
            if !pattern.is_empty() {
                let _ = doc.search.set_query(pattern);
                let text = doc.buffer.to_string();
                doc.search.find_all(&text);
            }
            return;
        }

        // Line number
        if let Ok(line_num) = cmd.parse::<usize>() {
            doc.go_to_line(line_num.saturating_sub(1));
            return;
        }

        // Ex commands
        match cmd {
            "w" => {
                // Save — handled at app level, we signal via a no-op
            }
            "q" => {
                // Quit — handled at app level
            }
            "wq" | "x" => {
                // Save and quit
            }
            "q!" => {
                // Force quit
            }
            _ => {}
        }
    }

    fn put_after(&mut self, doc: &mut Document) {
        if self.yank_register.is_empty() {
            return;
        }
        if self.yank_linewise {
            let line = doc.cursors.primary().position.line;
            let next_line_start = if line + 1 < doc.buffer.len_lines() {
                doc.buffer.line_col_to_char(line + 1, 0)
            } else {
                let len = doc.buffer.len_chars();
                // Insert newline at end
                doc.cursors.primary_mut().move_to(
                    Position::new(line, doc.buffer.line_len_chars_no_newline(line)),
                    false,
                );
                doc.insert_text("\n");
                len + 1
            };
            let (target_line, target_col) = doc.buffer.char_to_line_col(next_line_start);
            doc.cursors
                .primary_mut()
                .move_to(Position::new(target_line, target_col), false);
            let text = self.yank_register.trim_end_matches('\n');
            doc.insert_text(text);
        } else {
            doc.move_cursor_right(false);
            doc.insert_text(&self.yank_register);
        }
    }

    fn put_before(&mut self, doc: &mut Document) {
        if self.yank_register.is_empty() {
            return;
        }
        if self.yank_linewise {
            let line = doc.cursors.primary().position.line;
            doc.cursors
                .primary_mut()
                .move_to(Position::new(line, 0), false);
            let text = self.yank_register.trim_end_matches('\n');
            doc.insert_text(text);
            doc.insert_text("\n");
            doc.move_cursor_up(false);
        } else {
            doc.insert_text(&self.yank_register);
        }
    }

    fn repeat_last_action(&mut self, doc: &mut Document, modified: &mut bool) {
        let action = match self.last_action.clone() {
            Some(a) => a,
            None => return,
        };
        match action {
            VimAction::InsertText(text) => {
                doc.insert_text(&text);
                *modified = true;
            }
            VimAction::DeleteLine(count) => {
                for _ in 0..count {
                    doc.delete_line();
                }
                *modified = true;
            }
            VimAction::Put => {
                self.put_after(doc);
                *modified = true;
            }
            VimAction::ReplaceChar(ch) => {
                doc.delete_forward();
                doc.insert_text(&ch.to_string());
                doc.move_cursor_left(false);
                *modified = true;
            }
            VimAction::JoinLines => {
                let pos = doc.cursors.primary().position;
                let line_len = doc.buffer.line_len_chars_no_newline(pos.line);
                doc.cursors
                    .primary_mut()
                    .move_to(Position::new(pos.line, line_len), false);
                doc.delete_forward();
                doc.insert_text(" ");
                *modified = true;
            }
            VimAction::OpenLineBelow => {
                doc.move_cursor_end(false);
                doc.insert_newline_with_indent();
                *modified = true;
                self.mode = VimMode::Insert;
                self.insert_text_buffer.clear();
            }
            VimAction::OpenLineAbove => {
                doc.move_cursor_home(false);
                doc.insert_newline_with_indent();
                doc.move_cursor_up(false);
                *modified = true;
                self.mode = VimMode::Insert;
                self.insert_text_buffer.clear();
            }
            _ => {}
        }
    }

    /// Called when text is inserted in insert mode (for `.` repeat tracking).
    pub fn record_insert_text(&mut self, text: &str) {
        if self.mode == VimMode::Insert {
            self.insert_text_buffer.push_str(text);
        }
    }

    /// Check if we're in a mode that should pass text input through to the editor.
    pub fn passes_text_input(&self) -> bool {
        matches!(self.mode, VimMode::Insert | VimMode::Command)
    }

    /// Whether the cursor should be a block (Normal/Visual) or line (Insert).
    pub fn block_cursor(&self) -> bool {
        !matches!(self.mode, VimMode::Insert)
    }
}

fn move_to_word_end(doc: &mut Document) {
    doc.move_cursor_right(false);
    doc.move_cursor_word_right(false);
    doc.move_cursor_left(false);
}

fn move_to_word_end_select(doc: &mut Document) {
    doc.move_cursor_word_right(true);
    doc.move_cursor_left(true);
}

fn select_inner_word(doc: &mut Document) {
    let pos = doc.cursors.primary().position;
    let line = doc.buffer.line(pos.line).to_string();
    let chars: Vec<char> = line.chars().collect();

    if pos.col >= chars.len() {
        return;
    }

    let ch = chars[pos.col];
    let is_word_char = |c: char| c.is_alphanumeric() || c == '_';

    if is_word_char(ch) {
        // Find word boundaries
        let mut start = pos.col;
        while start > 0 && is_word_char(chars[start - 1]) {
            start -= 1;
        }
        let mut end = pos.col;
        while end < chars.len() && is_word_char(chars[end]) {
            end += 1;
        }
        doc.cursors.primary_mut().anchor = Some(Position::new(pos.line, start));
        doc.cursors.primary_mut().position = Position::new(pos.line, end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openedit_core::Document;

    #[test]
    fn test_vim_mode_default() {
        let state = VimState::new();
        assert_eq!(state.mode, VimMode::Normal);
        assert!(!state.enabled);
    }

    #[test]
    fn test_vim_insert_mode_switch() {
        let mut state = VimState::new();
        state.enabled = true;
        let mut doc = Document::from_str("hello world");
        let mut modified = false;

        assert!(state.handle_key("i", &mut doc, &mut modified));
        assert_eq!(state.mode, VimMode::Insert);

        assert!(state.handle_key("Escape", &mut doc, &mut modified));
        assert_eq!(state.mode, VimMode::Normal);
    }

    #[test]
    fn test_vim_hjkl_movement() {
        let mut state = VimState::new();
        state.enabled = true;
        let mut doc = Document::from_str("hello\nworld\nfoo");
        let mut modified = false;

        // Move right
        state.handle_key("l", &mut doc, &mut modified);
        assert_eq!(doc.cursors.primary().position.col, 1);

        // Move down
        state.handle_key("j", &mut doc, &mut modified);
        assert_eq!(doc.cursors.primary().position.line, 1);

        // Move left
        state.handle_key("h", &mut doc, &mut modified);
        assert_eq!(doc.cursors.primary().position.col, 0);

        // Move up
        state.handle_key("k", &mut doc, &mut modified);
        assert_eq!(doc.cursors.primary().position.line, 0);
    }

    #[test]
    fn test_vim_dd_deletes_line() {
        let mut state = VimState::new();
        state.enabled = true;
        let mut doc = Document::from_str("line1\nline2\nline3");
        let mut modified = false;

        state.handle_key("d", &mut doc, &mut modified);
        state.handle_key("d", &mut doc, &mut modified);
        assert!(modified);
        assert!(!doc.buffer.to_string().contains("line1"));
    }

    #[test]
    fn test_vim_yy_p() {
        let mut state = VimState::new();
        state.enabled = true;
        let mut doc = Document::from_str("line1\nline2");
        let mut modified = false;

        // yy
        state.handle_key("y", &mut doc, &mut modified);
        state.handle_key("y", &mut doc, &mut modified);
        assert!(!state.yank_register.is_empty());

        // p
        state.handle_key("p", &mut doc, &mut modified);
        assert!(modified);
    }

    #[test]
    fn test_vim_visual_mode() {
        let mut state = VimState::new();
        state.enabled = true;
        let mut doc = Document::from_str("hello world");
        let mut modified = false;

        state.handle_key("v", &mut doc, &mut modified);
        assert_eq!(state.mode, VimMode::Visual);

        state.handle_key("l", &mut doc, &mut modified);
        state.handle_key("l", &mut doc, &mut modified);

        state.handle_key("d", &mut doc, &mut modified);
        assert_eq!(state.mode, VimMode::Normal);
        assert!(modified);
    }

    #[test]
    fn test_vim_count_prefix() {
        let mut state = VimState::new();
        state.enabled = true;
        let mut doc = Document::from_str("hello world");
        let mut modified = false;

        state.handle_key("3", &mut doc, &mut modified);
        state.handle_key("l", &mut doc, &mut modified);
        assert_eq!(doc.cursors.primary().position.col, 3);
    }

    #[test]
    fn test_vim_command_mode() {
        let mut state = VimState::new();
        state.enabled = true;
        let mut doc = Document::from_str("hello\nworld");
        let mut modified = false;

        state.handle_key(":", &mut doc, &mut modified);
        assert_eq!(state.mode, VimMode::Command);

        state.handle_key("2", &mut doc, &mut modified);
        state.handle_key("Enter", &mut doc, &mut modified);
        assert_eq!(state.mode, VimMode::Normal);
        assert_eq!(doc.cursors.primary().position.line, 1);
    }

    #[test]
    fn test_vim_ciw() {
        let mut state = VimState::new();
        state.enabled = true;
        let mut doc = Document::from_str("hello world");
        let mut modified = false;

        // Move to 'h' of "hello"
        state.handle_key("c", &mut doc, &mut modified);
        state.handle_key("i", &mut doc, &mut modified);
        state.handle_key("w", &mut doc, &mut modified);
        assert_eq!(state.mode, VimMode::Insert);
        assert!(modified);
    }

    #[test]
    fn test_vim_x_delete_char() {
        let mut state = VimState::new();
        state.enabled = true;
        let mut doc = Document::from_str("hello");
        let mut modified = false;

        state.handle_key("x", &mut doc, &mut modified);
        assert!(modified);
        assert_eq!(doc.buffer.to_string(), "ello");
    }

    #[test]
    fn test_vim_gg_and_G() {
        let mut state = VimState::new();
        state.enabled = true;
        let mut doc = Document::from_str("line1\nline2\nline3\nline4");
        let mut modified = false;

        // G goes to last line
        state.handle_key("G", &mut doc, &mut modified);
        assert_eq!(doc.cursors.primary().position.line, 3);

        // gg goes to first line
        state.handle_key("g", &mut doc, &mut modified);
        state.handle_key("g", &mut doc, &mut modified);
        assert_eq!(doc.cursors.primary().position.line, 0);
    }

    #[test]
    fn test_vim_dot_repeat() {
        let mut state = VimState::new();
        state.enabled = true;
        let mut doc = Document::from_str("aaa\nbbb\nccc");
        let mut modified = false;

        // dd to delete first line
        state.handle_key("d", &mut doc, &mut modified);
        state.handle_key("d", &mut doc, &mut modified);

        // . to repeat
        state.handle_key(".", &mut doc, &mut modified);
        // Two lines should be deleted
        assert_eq!(doc.buffer.len_lines(), 1);
    }

    #[test]
    fn test_vim_disabled() {
        let mut state = VimState::new();
        // Not enabled
        let mut doc = Document::from_str("hello");
        let mut modified = false;

        assert!(!state.handle_key("j", &mut doc, &mut modified));
    }
}
