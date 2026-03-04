//! Simple VT100/ANSI escape sequence parser for terminal rendering.
//!
//! Handles basic cursor movement, colors, and screen manipulation.

/// A single cell in the terminal screen buffer.
#[derive(Debug, Clone)]
pub struct Cell {
    pub ch: char,
    pub fg: u8, // ANSI color code (0 = default, 30-37 = standard, 90-97 = bright)
    pub bg: u8,
    pub bold: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: 0,
            bg: 0,
            bold: false,
        }
    }
}

/// Parser state for VT100 escape sequences.
enum ParseState {
    Normal,
    Escape,   // received ESC
    CsiEntry, // received ESC [
    OscEntry, // received ESC ]
}

/// VT100 terminal parser with a screen buffer.
pub struct Vt100Parser {
    pub cols: usize,
    pub rows: usize,
    pub screen: Vec<Cell>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    state: ParseState,
    /// Accumulated CSI parameter bytes.
    csi_params: Vec<u8>,
    /// Current foreground color.
    current_fg: u8,
    /// Current background color.
    current_bg: u8,
    /// Bold attribute.
    current_bold: bool,
    /// Scrollback buffer (lines that scrolled off the top).
    pub scrollback: Vec<Vec<Cell>>,
    /// Maximum scrollback lines.
    max_scrollback: usize,
    /// OSC accumulation buffer.
    osc_buf: Vec<u8>,
    /// Saved cursor position.
    saved_cursor: (usize, usize),
}

impl Vt100Parser {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols,
            rows,
            screen: vec![Cell::default(); cols * rows],
            cursor_row: 0,
            cursor_col: 0,
            state: ParseState::Normal,
            csi_params: Vec::new(),
            current_fg: 0,
            current_bg: 0,
            current_bold: false,
            scrollback: Vec::new(),
            max_scrollback: 1000,
            osc_buf: Vec::new(),
            saved_cursor: (0, 0),
        }
    }

    /// Feed raw bytes from the PTY into the parser.
    pub fn feed(&mut self, data: &[u8]) {
        for &byte in data {
            match self.state {
                ParseState::Normal => self.handle_normal(byte),
                ParseState::Escape => self.handle_escape(byte),
                ParseState::CsiEntry => self.handle_csi(byte),
                ParseState::OscEntry => self.handle_osc(byte),
            }
        }
    }

    fn handle_normal(&mut self, byte: u8) {
        match byte {
            0x1b => {
                self.state = ParseState::Escape;
            }
            b'\r' => {
                self.cursor_col = 0;
            }
            b'\n' => {
                self.newline();
            }
            b'\t' => {
                // Tab: advance to next 8-column boundary
                let next_tab = ((self.cursor_col / 8) + 1) * 8;
                self.cursor_col = next_tab.min(self.cols - 1);
            }
            0x08
                // Backspace
                if self.cursor_col > 0 => {
                    self.cursor_col -= 1;
                }
            0x07 => {
                // Bell - ignore
            }
            byte if byte >= 0x20 => {
                // Printable character (handle UTF-8 simply: just use the byte as char)
                // For proper UTF-8 we'd need a separate decoder, but for basic ASCII this works
                let ch = byte as char;
                self.put_char(ch);
            }
            _ => {
                // Ignore other control characters
            }
        }
    }

    fn handle_escape(&mut self, byte: u8) {
        match byte {
            b'[' => {
                self.state = ParseState::CsiEntry;
                self.csi_params.clear();
            }
            b']' => {
                self.state = ParseState::OscEntry;
                self.osc_buf.clear();
            }
            b'7' => {
                // Save cursor
                self.saved_cursor = (self.cursor_row, self.cursor_col);
                self.state = ParseState::Normal;
            }
            b'8' => {
                // Restore cursor
                self.cursor_row = self.saved_cursor.0.min(self.rows - 1);
                self.cursor_col = self.saved_cursor.1.min(self.cols - 1);
                self.state = ParseState::Normal;
            }
            b'M' => {
                // Reverse index (scroll down)
                if self.cursor_row == 0 {
                    self.scroll_down();
                } else {
                    self.cursor_row -= 1;
                }
                self.state = ParseState::Normal;
            }
            b'c' => {
                // Full reset
                self.reset();
                self.state = ParseState::Normal;
            }
            _ => {
                self.state = ParseState::Normal;
            }
        }
    }

    fn handle_csi(&mut self, byte: u8) {
        match byte {
            b'0'..=b'9' | b';' | b'?' => {
                self.csi_params.push(byte);
            }
            _ => {
                // Final byte — execute the CSI sequence
                self.execute_csi(byte);
                self.state = ParseState::Normal;
            }
        }
    }

    fn handle_osc(&mut self, byte: u8) {
        match byte {
            0x07 => {
                // BEL terminates OSC
                // OSC commands (like window title) are ignored for now
                self.state = ParseState::Normal;
            }
            0x1b => {
                // ESC might be start of ST (ESC \)
                self.state = ParseState::Escape;
            }
            _ => {
                self.osc_buf.push(byte);
                if self.osc_buf.len() > 4096 {
                    // Safety: don't accumulate forever
                    self.state = ParseState::Normal;
                }
            }
        }
    }

    fn execute_csi(&mut self, final_byte: u8) {
        let params = self.parse_csi_params();

        match final_byte {
            b'A' => {
                // Cursor Up
                let n = params.first().copied().unwrap_or(1).max(1);
                self.cursor_row = self.cursor_row.saturating_sub(n);
            }
            b'B' => {
                // Cursor Down
                let n = params.first().copied().unwrap_or(1).max(1);
                self.cursor_row = (self.cursor_row + n).min(self.rows - 1);
            }
            b'C' => {
                // Cursor Forward
                let n = params.first().copied().unwrap_or(1).max(1);
                self.cursor_col = (self.cursor_col + n).min(self.cols - 1);
            }
            b'D' => {
                // Cursor Backward
                let n = params.first().copied().unwrap_or(1).max(1);
                self.cursor_col = self.cursor_col.saturating_sub(n);
            }
            b'E' => {
                // Cursor Next Line
                let n = params.first().copied().unwrap_or(1).max(1);
                self.cursor_row = (self.cursor_row + n).min(self.rows - 1);
                self.cursor_col = 0;
            }
            b'F' => {
                // Cursor Previous Line
                let n = params.first().copied().unwrap_or(1).max(1);
                self.cursor_row = self.cursor_row.saturating_sub(n);
                self.cursor_col = 0;
            }
            b'G' => {
                // Cursor Horizontal Absolute
                let n = params.first().copied().unwrap_or(1).max(1);
                self.cursor_col = (n - 1).min(self.cols - 1);
            }
            b'H' | b'f' => {
                // Cursor Position
                let row = params.first().copied().unwrap_or(1).max(1) - 1;
                let col = params.get(1).copied().unwrap_or(1).max(1) - 1;
                self.cursor_row = row.min(self.rows - 1);
                self.cursor_col = col.min(self.cols - 1);
            }
            b'J' => {
                // Erase in Display
                let mode = params.first().copied().unwrap_or(0);
                match mode {
                    0 => self.clear_from_cursor_to_end(),
                    1 => self.clear_from_start_to_cursor(),
                    2 | 3 => self.clear_screen(),
                    _ => {}
                }
            }
            b'K' => {
                // Erase in Line
                let mode = params.first().copied().unwrap_or(0);
                match mode {
                    0 => self.clear_line_from_cursor(),
                    1 => self.clear_line_to_cursor(),
                    2 => self.clear_entire_line(),
                    _ => {}
                }
            }
            b'L' => {
                // Insert Lines
                let n = params.first().copied().unwrap_or(1).max(1);
                self.insert_lines(n);
            }
            b'M' => {
                // Delete Lines
                let n = params.first().copied().unwrap_or(1).max(1);
                self.delete_lines(n);
            }
            b'P' => {
                // Delete Characters
                let n = params.first().copied().unwrap_or(1).max(1);
                self.delete_chars(n);
            }
            b'm' => {
                // SGR (Select Graphic Rendition)
                self.handle_sgr(&params);
            }
            b'r' => {
                // Set scrolling region (ignored for now — use full screen)
            }
            b'h' | b'l' => {
                // Set/Reset Mode (mostly private modes, ignored)
            }
            b'd' => {
                // Line Position Absolute
                let n = params.first().copied().unwrap_or(1).max(1) - 1;
                self.cursor_row = n.min(self.rows - 1);
            }
            b'@' => {
                // Insert Characters
                let n = params.first().copied().unwrap_or(1).max(1);
                self.insert_chars(n);
            }
            b'X' => {
                // Erase Characters
                let n = params.first().copied().unwrap_or(1).max(1);
                for i in 0..n {
                    let col = self.cursor_col + i;
                    if col < self.cols {
                        let idx = self.cursor_row * self.cols + col;
                        self.screen[idx] = Cell::default();
                    }
                }
            }
            _ => {
                // Unknown CSI sequence — ignore
            }
        }
    }

    fn parse_csi_params(&self) -> Vec<usize> {
        let s: String = self
            .csi_params
            .iter()
            .filter(|&&b| b != b'?') // strip private mode marker
            .map(|&b| b as char)
            .collect();
        if s.is_empty() {
            return Vec::new();
        }
        s.split(';')
            .map(|p| p.parse::<usize>().unwrap_or(0))
            .collect()
    }

    fn handle_sgr(&mut self, params: &[usize]) {
        if params.is_empty() {
            self.reset_attributes();
            return;
        }

        let mut i = 0;
        while i < params.len() {
            match params[i] {
                0 => self.reset_attributes(),
                1 => self.current_bold = true,
                22 => self.current_bold = false,
                30..=37 => self.current_fg = params[i] as u8,
                38 => {
                    // Extended foreground: 38;5;N or 38;2;R;G;B
                    if i + 2 < params.len() && params[i + 1] == 5 {
                        // 256-color: map to basic 16 for simplicity
                        let n = params[i + 2];
                        self.current_fg = if n < 8 {
                            30 + n as u8
                        } else if n < 16 {
                            82 + n as u8
                        } else {
                            0
                        };
                        i += 2;
                    } else if i + 4 < params.len() && params[i + 1] == 2 {
                        // True color: just use default
                        i += 4;
                    }
                }
                39 => self.current_fg = 0, // default fg
                40..=47 => self.current_bg = params[i] as u8,
                48 => {
                    if i + 2 < params.len() && params[i + 1] == 5 {
                        i += 2;
                    } else if i + 4 < params.len() && params[i + 1] == 2 {
                        i += 4;
                    }
                }
                49 => self.current_bg = 0, // default bg
                90..=97 => self.current_fg = params[i] as u8,
                100..=107 => self.current_bg = params[i] as u8,
                _ => {}
            }
            i += 1;
        }
    }

    fn reset_attributes(&mut self) {
        self.current_fg = 0;
        self.current_bg = 0;
        self.current_bold = false;
    }

    fn put_char(&mut self, ch: char) {
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.newline();
        }

        let idx = self.cursor_row * self.cols + self.cursor_col;
        if idx < self.screen.len() {
            self.screen[idx] = Cell {
                ch,
                fg: if self.current_bold && self.current_fg >= 30 && self.current_fg <= 37 {
                    self.current_fg + 60 // bright version
                } else {
                    self.current_fg
                },
                bg: self.current_bg,
                bold: self.current_bold,
            };
        }
        self.cursor_col += 1;
    }

    fn newline(&mut self) {
        if self.cursor_row + 1 < self.rows {
            self.cursor_row += 1;
        } else {
            self.scroll_up();
        }
    }

    fn scroll_up(&mut self) {
        // Save top line to scrollback
        let top_line: Vec<Cell> = self.screen[0..self.cols].to_vec();
        self.scrollback.push(top_line);
        if self.scrollback.len() > self.max_scrollback {
            self.scrollback.remove(0);
        }

        // Shift all rows up by one
        let total = self.cols * self.rows;
        for i in 0..(total - self.cols) {
            self.screen[i] = self.screen[i + self.cols].clone();
        }
        // Clear the bottom row
        for i in (total - self.cols)..total {
            self.screen[i] = Cell::default();
        }
    }

    fn scroll_down(&mut self) {
        // Shift all rows down by one
        let total = self.cols * self.rows;
        for i in (self.cols..total).rev() {
            self.screen[i] = self.screen[i - self.cols].clone();
        }
        // Clear the top row
        for i in 0..self.cols {
            self.screen[i] = Cell::default();
        }
    }

    fn clear_screen(&mut self) {
        for cell in &mut self.screen {
            *cell = Cell::default();
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    fn clear_from_cursor_to_end(&mut self) {
        let start = self.cursor_row * self.cols + self.cursor_col;
        for i in start..self.screen.len() {
            self.screen[i] = Cell::default();
        }
    }

    fn clear_from_start_to_cursor(&mut self) {
        let end = self.cursor_row * self.cols + self.cursor_col + 1;
        for i in 0..end.min(self.screen.len()) {
            self.screen[i] = Cell::default();
        }
    }

    fn clear_line_from_cursor(&mut self) {
        let start = self.cursor_row * self.cols + self.cursor_col;
        let end = (self.cursor_row + 1) * self.cols;
        for i in start..end.min(self.screen.len()) {
            self.screen[i] = Cell::default();
        }
    }

    fn clear_line_to_cursor(&mut self) {
        let start = self.cursor_row * self.cols;
        let end = self.cursor_row * self.cols + self.cursor_col + 1;
        for i in start..end.min(self.screen.len()) {
            self.screen[i] = Cell::default();
        }
    }

    fn clear_entire_line(&mut self) {
        let start = self.cursor_row * self.cols;
        let end = (self.cursor_row + 1) * self.cols;
        for i in start..end.min(self.screen.len()) {
            self.screen[i] = Cell::default();
        }
    }

    fn insert_lines(&mut self, n: usize) {
        let row = self.cursor_row;
        for _ in 0..n {
            // Shift lines down from current row
            let total = self.cols * self.rows;
            for i in (((row + 1) * self.cols)..total).rev() {
                self.screen[i] = self.screen[i.saturating_sub(self.cols)].clone();
            }
            // Clear current row
            for col in 0..self.cols {
                self.screen[row * self.cols + col] = Cell::default();
            }
        }
    }

    fn delete_lines(&mut self, n: usize) {
        let row = self.cursor_row;
        for _ in 0..n {
            let total = self.cols * self.rows;
            for i in (row * self.cols)..(total - self.cols) {
                self.screen[i] = self.screen[i + self.cols].clone();
            }
            for col in 0..self.cols {
                let idx = (self.rows - 1) * self.cols + col;
                self.screen[idx] = Cell::default();
            }
        }
    }

    fn delete_chars(&mut self, n: usize) {
        let row_start = self.cursor_row * self.cols;
        let row_end = row_start + self.cols;
        for i in (row_start + self.cursor_col)..(row_end - n).min(row_end) {
            if i + n < row_end {
                self.screen[i] = self.screen[i + n].clone();
            }
        }
        for i in (row_end.saturating_sub(n))..row_end {
            self.screen[i] = Cell::default();
        }
    }

    fn insert_chars(&mut self, n: usize) {
        let row_start = self.cursor_row * self.cols;
        let row_end = row_start + self.cols;
        // Shift chars right
        for i in ((row_start + self.cursor_col + n)..row_end).rev() {
            self.screen[i] = self.screen[i - n].clone();
        }
        for i in 0..n {
            let idx = row_start + self.cursor_col + i;
            if idx < row_end {
                self.screen[idx] = Cell::default();
            }
        }
    }

    fn reset(&mut self) {
        self.clear_screen();
        self.reset_attributes();
        self.scrollback.clear();
    }

    /// Resize the terminal screen buffer.
    pub fn resize(&mut self, new_cols: usize, new_rows: usize) {
        let mut new_screen = vec![Cell::default(); new_cols * new_rows];
        let copy_rows = self.rows.min(new_rows);
        let copy_cols = self.cols.min(new_cols);

        for row in 0..copy_rows {
            for col in 0..copy_cols {
                new_screen[row * new_cols + col] = self.screen[row * self.cols + col].clone();
            }
        }

        self.screen = new_screen;
        self.cols = new_cols;
        self.rows = new_rows;
        self.cursor_row = self.cursor_row.min(new_rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(new_cols.saturating_sub(1));
    }

    /// Get a line of text from the screen buffer.
    pub fn get_line(&self, row: usize) -> String {
        if row >= self.rows {
            return String::new();
        }
        let start = row * self.cols;
        let end = start + self.cols;
        self.screen[start..end].iter().map(|c| c.ch).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_parser() {
        let p = Vt100Parser::new(80, 24);
        assert_eq!(p.cols, 80);
        assert_eq!(p.rows, 24);
        assert_eq!(p.cursor_row, 0);
        assert_eq!(p.cursor_col, 0);
        assert_eq!(p.screen.len(), 80 * 24);
    }

    #[test]
    fn test_feed_text() {
        let mut p = Vt100Parser::new(80, 24);
        p.feed(b"Hello");
        assert_eq!(p.cursor_col, 5);
        assert_eq!(p.screen[0].ch, 'H');
        assert_eq!(p.screen[4].ch, 'o');
    }

    #[test]
    fn test_newline() {
        let mut p = Vt100Parser::new(80, 24);
        p.feed(b"Hello\r\nWorld");
        assert_eq!(p.cursor_row, 1);
        assert_eq!(p.cursor_col, 5);
        assert_eq!(p.screen[80].ch, 'W');
    }

    #[test]
    fn test_cursor_movement() {
        let mut p = Vt100Parser::new(80, 24);
        p.feed(b"\x1b[5;10H"); // Move to row 5, col 10
        assert_eq!(p.cursor_row, 4); // 0-indexed
        assert_eq!(p.cursor_col, 9);
    }

    #[test]
    fn test_clear_screen() {
        let mut p = Vt100Parser::new(80, 24);
        p.feed(b"Hello");
        p.feed(b"\x1b[2J");
        assert_eq!(p.screen[0].ch, ' ');
    }

    #[test]
    fn test_sgr_color() {
        let mut p = Vt100Parser::new(80, 24);
        p.feed(b"\x1b[31mR"); // Red foreground
        assert_eq!(p.screen[0].ch, 'R');
        assert_eq!(p.screen[0].fg, 31);
    }

    #[test]
    fn test_sgr_reset() {
        let mut p = Vt100Parser::new(80, 24);
        p.feed(b"\x1b[31m\x1b[0mX");
        assert_eq!(p.screen[0].ch, 'X');
        assert_eq!(p.screen[0].fg, 0);
    }

    #[test]
    fn test_erase_in_line() {
        let mut p = Vt100Parser::new(80, 24);
        p.feed(b"ABCDEF");
        p.feed(b"\x1b[3G"); // Move to col 3
        p.feed(b"\x1b[0K"); // Erase from cursor to end of line
        assert_eq!(p.screen[0].ch, 'A');
        assert_eq!(p.screen[1].ch, 'B');
        assert_eq!(p.screen[2].ch, ' ');
    }

    #[test]
    fn test_scroll_up() {
        let mut p = Vt100Parser::new(10, 3);
        p.feed(b"Line1\r\nLine2\r\nLine3\r\nLine4");
        // After scrolling, Line1 should be in scrollback
        assert_eq!(p.scrollback.len(), 1);
        assert_eq!(p.cursor_row, 2); // bottom row
    }

    #[test]
    fn test_resize() {
        let mut p = Vt100Parser::new(80, 24);
        p.feed(b"Hello");
        p.resize(40, 12);
        assert_eq!(p.cols, 40);
        assert_eq!(p.rows, 12);
        assert_eq!(p.screen[0].ch, 'H');
    }

    #[test]
    fn test_get_line() {
        let mut p = Vt100Parser::new(10, 3);
        p.feed(b"Hello");
        let line = p.get_line(0);
        assert!(line.starts_with("Hello"));
        assert_eq!(line.len(), 10);
    }

    #[test]
    fn test_tab() {
        let mut p = Vt100Parser::new(80, 24);
        p.feed(b"A\tB");
        assert_eq!(p.screen[0].ch, 'A');
        assert_eq!(p.screen[8].ch, 'B');
    }

    #[test]
    fn test_backspace() {
        let mut p = Vt100Parser::new(80, 24);
        p.feed(b"AB\x08C");
        // Backspace moves cursor back, then C overwrites B
        assert_eq!(p.screen[0].ch, 'A');
        assert_eq!(p.screen[1].ch, 'C');
    }

    #[test]
    fn test_bold_makes_bright() {
        let mut p = Vt100Parser::new(80, 24);
        p.feed(b"\x1b[1;31mX");
        assert_eq!(p.screen[0].fg, 91); // bright red
        assert!(p.screen[0].bold);
    }

    #[test]
    fn test_cursor_up_down() {
        let mut p = Vt100Parser::new(80, 24);
        p.feed(b"\x1b[5B"); // Down 5
        assert_eq!(p.cursor_row, 5);
        p.feed(b"\x1b[2A"); // Up 2
        assert_eq!(p.cursor_row, 3);
    }

    #[test]
    fn test_save_restore_cursor() {
        let mut p = Vt100Parser::new(80, 24);
        p.feed(b"\x1b[5;10H"); // Move to 5,10
        p.feed(b"\x1b7"); // Save
        p.feed(b"\x1b[1;1H"); // Move to 1,1
        p.feed(b"\x1b8"); // Restore
        assert_eq!(p.cursor_row, 4);
        assert_eq!(p.cursor_col, 9);
    }
}
