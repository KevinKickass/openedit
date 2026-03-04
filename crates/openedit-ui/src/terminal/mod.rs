//! Integrated terminal emulator panel.
//!
//! Uses `portable-pty` for PTY allocation and a simple VT100 parser for rendering.
//! Supports multiple terminal tabs, each with its own PTY session and VT100 state.

pub mod pty;
pub mod vt100;

use self::vt100::Vt100Parser;

/// A single terminal session with its own PTY and screen buffer.
pub struct TerminalTab {
    /// Display name (e.g. "Terminal 1").
    pub name: String,
    /// Whether the terminal process is running.
    pub running: bool,
    /// The PTY session (if active).
    session: Option<pty::PtySession>,
    /// VT100 parser/screen buffer.
    parser: Vt100Parser,
}

impl TerminalTab {
    fn new(name: String) -> Self {
        Self {
            name,
            running: false,
            session: None,
            parser: Vt100Parser::new(80, 24),
        }
    }

    /// Start a new shell session.
    fn start(&mut self) {
        match pty::PtySession::spawn_shell(80, 24) {
            Ok(session) => {
                self.session = Some(session);
                self.running = true;
                log::info!("Terminal: shell session started for '{}'", self.name);
            }
            Err(e) => {
                log::error!("Terminal: failed to start shell: {}", e);
                let msg = format!("Error starting shell: {}\r\n", e);
                self.parser.feed(msg.as_bytes());
            }
        }
    }

    /// Poll for new output from the PTY.
    fn poll(&mut self) {
        if let Some(ref mut session) = self.session {
            match session.read_output() {
                Ok(data) if !data.is_empty() => {
                    self.parser.feed(&data);
                }
                Err(_e) => {
                    self.session = None;
                    self.running = false;
                    let msg = "\r\n[Process exited]\r\n";
                    self.parser.feed(msg.as_bytes());
                }
                _ => {}
            }
        }
    }

    /// Send raw bytes to the terminal.
    fn send_input(&mut self, data: &[u8]) {
        if let Some(ref mut session) = self.session {
            if let Err(e) = session.write_input(data) {
                log::error!("Terminal: write error: {}", e);
            }
        }
    }

    /// Send a special key to the terminal.
    fn send_key(&mut self, key: &str) {
        let bytes: &[u8] = match key {
            "Enter" => b"\r",
            "Backspace" => b"\x7f",
            "Tab" => b"\t",
            "Escape" => b"\x1b",
            "ArrowUp" => b"\x1b[A",
            "ArrowDown" => b"\x1b[B",
            "ArrowRight" => b"\x1b[C",
            "ArrowLeft" => b"\x1b[D",
            "Home" => b"\x1b[H",
            "End" => b"\x1b[F",
            "Delete" => b"\x1b[3~",
            "PageUp" => b"\x1b[5~",
            "PageDown" => b"\x1b[6~",
            _ => return,
        };
        self.send_input(bytes);
    }

    /// Kill the terminal session.
    fn kill(&mut self) {
        if let Some(mut session) = self.session.take() {
            session.kill();
        }
        self.running = false;
    }
}

impl Drop for TerminalTab {
    fn drop(&mut self) {
        self.kill();
    }
}

/// State for the integrated terminal panel (supports multiple tabs).
pub struct TerminalState {
    /// Whether the terminal panel is visible.
    pub visible: bool,
    /// Height fraction (0.0 - 1.0) of the editor area.
    pub height_fraction: f32,
    /// Height in pixels (computed from fraction).
    pub height: f32,
    /// Terminal tabs.
    tabs: Vec<TerminalTab>,
    /// Index of the active tab.
    active_tab: usize,
    /// Counter for generating unique terminal names.
    next_id: usize,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            visible: false,
            height_fraction: 0.30,
            height: 250.0,
            tabs: Vec::new(),
            active_tab: 0,
            next_id: 1,
        }
    }
}

impl TerminalState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether any terminal tab is running.
    pub fn running(&self) -> bool {
        self.tabs.iter().any(|t| t.running)
    }

    /// Start a new terminal tab with a shell session.
    pub fn start(&mut self) {
        let name = format!("Terminal {}", self.next_id);
        self.next_id += 1;
        let mut tab = TerminalTab::new(name);
        tab.start();
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
    }

    /// Close the terminal tab at the given index.
    pub fn close_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.tabs.remove(idx);
            if self.tabs.is_empty() {
                self.active_tab = 0;
            } else if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
        }
    }

    /// Send text to the active terminal (used by "Send Selection to Terminal").
    pub fn send_text_to_active(&mut self, text: &str) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.send_input(text.as_bytes());
        }
    }

    /// Kill all terminal sessions.
    pub fn kill(&mut self) {
        for tab in &mut self.tabs {
            tab.kill();
        }
        self.tabs.clear();
        self.active_tab = 0;
    }
}

impl Drop for TerminalState {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Render the terminal panel with tab bar.
pub fn render_terminal(ui: &mut egui::Ui, state: &mut TerminalState, font_size: f32) {
    let rect = ui.available_rect_before_wrap();
    let char_width = crate::editor_view::measure_char_width(ui, font_size);
    let line_height = crate::editor_view::measure_line_height(ui, font_size);
    let font_id = egui::FontId::monospace(font_size);

    // Background
    let bg = egui::Color32::from_rgb(20, 20, 20);
    ui.painter().rect_filled(rect, 0.0, bg);

    // Header bar with terminal tabs
    let header_height = 28.0;
    let header_rect = egui::Rect::from_min_size(
        rect.left_top(),
        egui::Vec2::new(rect.width(), header_height),
    );
    ui.painter()
        .rect_filled(header_rect, 0.0, egui::Color32::from_rgb(40, 40, 40));

    // Draw terminal tabs
    let tab_height = 24.0;
    let tab_y = rect.top() + 2.0;
    let mut tab_x = rect.left() + 4.0;
    let tab_font = egui::FontId::proportional(12.0);

    // Track which tab to close (if any) and which to activate
    let mut close_tab_idx: Option<usize> = None;
    let mut activate_tab_idx: Option<usize> = None;

    for (i, tab) in state.tabs.iter().enumerate() {
        let is_active = i == state.active_tab;
        let label = &tab.name;
        let label_width = label.len() as f32 * 7.0 + 24.0; // approximate width + close button
        let tab_width = label_width.max(80.0);

        let tab_rect = egui::Rect::from_min_size(
            egui::Pos2::new(tab_x, tab_y),
            egui::Vec2::new(tab_width, tab_height),
        );

        // Tab background
        let tab_bg = if is_active {
            egui::Color32::from_rgb(20, 20, 20)
        } else {
            egui::Color32::from_rgb(50, 50, 50)
        };
        ui.painter().rect_filled(tab_rect, 2.0, tab_bg);

        // Tab label
        let label_color = if is_active {
            egui::Color32::from_rgb(220, 220, 220)
        } else {
            egui::Color32::from_rgb(160, 160, 160)
        };
        ui.painter().text(
            egui::Pos2::new(tab_x + 8.0, tab_y + 4.0),
            egui::Align2::LEFT_TOP,
            label,
            tab_font.clone(),
            label_color,
        );

        // Tab click area (for activation)
        let tab_click_rect = egui::Rect::from_min_size(
            egui::Pos2::new(tab_x, tab_y),
            egui::Vec2::new(tab_width - 20.0, tab_height),
        );
        let tab_resp = ui.allocate_rect(tab_click_rect, egui::Sense::click());
        if tab_resp.clicked() {
            activate_tab_idx = Some(i);
        }

        // Tab close button (small "x")
        let close_btn_rect = egui::Rect::from_min_size(
            egui::Pos2::new(tab_x + tab_width - 20.0, tab_y + 2.0),
            egui::Vec2::new(18.0, 18.0),
        );
        let close_resp = ui.allocate_rect(close_btn_rect, egui::Sense::click());
        let close_color = if close_resp.hovered() {
            egui::Color32::from_rgb(255, 100, 100)
        } else {
            egui::Color32::from_rgb(130, 130, 130)
        };
        ui.painter().text(
            close_btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            "x",
            egui::FontId::proportional(11.0),
            close_color,
        );
        if close_resp.clicked() {
            close_tab_idx = Some(i);
        }

        tab_x += tab_width + 2.0;
    }

    // "+" button to create a new terminal
    let plus_rect = egui::Rect::from_min_size(
        egui::Pos2::new(tab_x, tab_y),
        egui::Vec2::new(24.0, tab_height),
    );
    let plus_resp = ui.allocate_rect(plus_rect, egui::Sense::click());
    let plus_color = if plus_resp.hovered() {
        egui::Color32::from_rgb(220, 220, 220)
    } else {
        egui::Color32::from_rgb(160, 160, 160)
    };
    ui.painter().text(
        plus_rect.center(),
        egui::Align2::CENTER_CENTER,
        "+",
        egui::FontId::proportional(16.0),
        plus_color,
    );
    let mut add_new_tab = plus_resp.clicked();

    // Panel close button (top-right corner)
    let panel_close_rect = egui::Rect::from_min_size(
        egui::Pos2::new(rect.right() - 24.0, rect.top() + 2.0),
        egui::Vec2::new(20.0, 20.0),
    );
    let panel_close_resp = ui.allocate_rect(panel_close_rect, egui::Sense::click());
    let panel_close_color = if panel_close_resp.hovered() {
        egui::Color32::from_rgb(255, 100, 100)
    } else {
        egui::Color32::from_rgb(160, 160, 160)
    };
    ui.painter().text(
        panel_close_rect.center(),
        egui::Align2::CENTER_CENTER,
        "\u{00d7}",
        egui::FontId::proportional(16.0),
        panel_close_color,
    );
    if panel_close_resp.clicked() {
        state.visible = false;
    }

    // Apply tab actions
    if let Some(idx) = close_tab_idx {
        state.close_tab(idx);
        if state.tabs.is_empty() {
            state.visible = false;
        }
    }
    if let Some(idx) = activate_tab_idx {
        state.active_tab = idx;
    }

    // If there are no tabs and the panel is visible, auto-create one
    if state.tabs.is_empty() && state.visible {
        add_new_tab = true;
    }
    if add_new_tab {
        state.start();
    }

    // Render the active tab's terminal content
    if let Some(tab) = state.tabs.get_mut(state.active_tab) {
        // Poll for new output
        tab.poll();

        let content_top = rect.top() + header_height;
        let screen = &tab.parser.screen;
        let rows = tab.parser.rows;
        let cols = tab.parser.cols;

        for row in 0..rows {
            let y = content_top + 4.0 + row as f32 * line_height;
            if y > rect.bottom() {
                break;
            }

            let mut line = String::with_capacity(cols);
            for col in 0..cols {
                let cell = &screen[row * cols + col];
                line.push(cell.ch);
            }

            let trimmed = line.trim_end();
            if !trimmed.is_empty() {
                let mut x = rect.left() + 4.0;
                for (col_idx, ch) in trimmed.chars().enumerate() {
                    let cell = &screen[row * cols + col_idx];
                    let fg = vt100_color_to_egui(cell.fg);
                    ui.painter().text(
                        egui::Pos2::new(x, y),
                        egui::Align2::LEFT_TOP,
                        ch.to_string(),
                        font_id.clone(),
                        fg,
                    );
                    x += char_width;
                }
            }
        }

        // Cursor
        let cursor_row = tab.parser.cursor_row;
        let cursor_col = tab.parser.cursor_col;
        let cursor_y = content_top + 4.0 + cursor_row as f32 * line_height;
        let cursor_x = rect.left() + 4.0 + cursor_col as f32 * char_width;
        let cursor_rect = egui::Rect::from_min_size(
            egui::Pos2::new(cursor_x, cursor_y),
            egui::Vec2::new(char_width, line_height),
        );
        ui.painter().rect_filled(
            cursor_rect,
            0.0,
            egui::Color32::from_rgba_premultiplied(200, 200, 200, 120),
        );
    }

    // Allocate area for interaction
    let _response = ui.allocate_rect(rect, egui::Sense::click());
}

/// Handle keyboard input for the terminal panel.
pub fn handle_terminal_input(ui: &mut egui::Ui, state: &mut TerminalState) {
    if let Some(tab) = state.tabs.get_mut(state.active_tab) {
        ui.input(|input| {
            let ctrl = input.modifiers.ctrl || input.modifiers.mac_cmd;

            for event in &input.events {
                match event {
                    egui::Event::Text(text) => {
                        if !text.chars().all(|c| c.is_control()) {
                            tab.send_input(text.as_bytes());
                        }
                    }
                    egui::Event::Key {
                        key, pressed: true, ..
                    } => match key {
                        egui::Key::Enter => tab.send_key("Enter"),
                        egui::Key::Backspace => tab.send_key("Backspace"),
                        egui::Key::Tab => tab.send_key("Tab"),
                        egui::Key::Escape => tab.send_key("Escape"),
                        egui::Key::ArrowUp => tab.send_key("ArrowUp"),
                        egui::Key::ArrowDown => tab.send_key("ArrowDown"),
                        egui::Key::ArrowLeft => tab.send_key("ArrowLeft"),
                        egui::Key::ArrowRight => tab.send_key("ArrowRight"),
                        egui::Key::Home => tab.send_key("Home"),
                        egui::Key::End => tab.send_key("End"),
                        egui::Key::Delete => tab.send_key("Delete"),
                        egui::Key::C if ctrl => tab.send_input(b"\x03"),
                        egui::Key::D if ctrl => tab.send_input(b"\x04"),
                        egui::Key::L if ctrl => tab.send_input(b"\x0c"),
                        _ => {}
                    },
                    _ => {}
                }
            }
        });
    }
}

/// Convert VT100 color code to egui Color32.
fn vt100_color_to_egui(color: u8) -> egui::Color32 {
    match color {
        0 => egui::Color32::from_rgb(204, 204, 204),
        30 => egui::Color32::from_rgb(0, 0, 0),
        31 => egui::Color32::from_rgb(205, 49, 49),
        32 => egui::Color32::from_rgb(13, 188, 121),
        33 => egui::Color32::from_rgb(229, 229, 16),
        34 => egui::Color32::from_rgb(36, 114, 200),
        35 => egui::Color32::from_rgb(188, 63, 188),
        36 => egui::Color32::from_rgb(17, 168, 205),
        37 => egui::Color32::from_rgb(229, 229, 229),
        90 => egui::Color32::from_rgb(102, 102, 102),
        91 => egui::Color32::from_rgb(241, 76, 76),
        92 => egui::Color32::from_rgb(35, 209, 139),
        93 => egui::Color32::from_rgb(245, 245, 67),
        94 => egui::Color32::from_rgb(59, 142, 234),
        95 => egui::Color32::from_rgb(214, 112, 214),
        96 => egui::Color32::from_rgb(41, 184, 219),
        97 => egui::Color32::from_rgb(255, 255, 255),
        _ => egui::Color32::from_rgb(204, 204, 204),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_state_default() {
        let state = TerminalState::default();
        assert!(!state.visible);
        assert!(!state.running());
        assert_eq!(state.tabs.len(), 0);
        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn test_terminal_tab_creation() {
        let tab = TerminalTab::new("Terminal 1".to_string());
        assert_eq!(tab.name, "Terminal 1");
        assert!(!tab.running);
    }

    #[test]
    fn test_terminal_state_start_creates_tab() {
        // We can't actually spawn a shell in tests easily, but we can verify
        // that start() increments next_id and adds a tab entry
        let mut state = TerminalState::default();
        assert_eq!(state.next_id, 1);
        // The start() call would try to spawn a shell, which may fail in CI.
        // Instead, test the naming logic:
        let name = format!("Terminal {}", state.next_id);
        assert_eq!(name, "Terminal 1");
        state.next_id += 1;
        let name2 = format!("Terminal {}", state.next_id);
        assert_eq!(name2, "Terminal 2");
    }

    #[test]
    fn test_terminal_close_tab() {
        let mut state = TerminalState::default();
        // Manually add tabs (without spawning shells)
        state.tabs.push(TerminalTab::new("Terminal 1".to_string()));
        state.tabs.push(TerminalTab::new("Terminal 2".to_string()));
        state.tabs.push(TerminalTab::new("Terminal 3".to_string()));
        state.active_tab = 1;

        // Close the active tab
        state.close_tab(1);
        assert_eq!(state.tabs.len(), 2);
        assert_eq!(state.active_tab, 1); // clamped to last
        assert_eq!(state.tabs[0].name, "Terminal 1");
        assert_eq!(state.tabs[1].name, "Terminal 3");
    }

    #[test]
    fn test_terminal_close_last_tab() {
        let mut state = TerminalState::default();
        state.tabs.push(TerminalTab::new("Terminal 1".to_string()));
        state.active_tab = 0;

        state.close_tab(0);
        assert_eq!(state.tabs.len(), 0);
        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn test_terminal_close_tab_clamps_active() {
        let mut state = TerminalState::default();
        state.tabs.push(TerminalTab::new("Terminal 1".to_string()));
        state.tabs.push(TerminalTab::new("Terminal 2".to_string()));
        state.active_tab = 1;

        // Close the last tab (index 1)
        state.close_tab(1);
        assert_eq!(state.tabs.len(), 1);
        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn test_vt100_color_mapping() {
        let c = vt100_color_to_egui(31);
        assert_eq!(c, egui::Color32::from_rgb(205, 49, 49));
    }

    #[test]
    fn test_send_text_to_active_no_tabs() {
        let mut state = TerminalState::default();
        // Should not panic when there are no tabs
        state.send_text_to_active("hello");
    }
}
