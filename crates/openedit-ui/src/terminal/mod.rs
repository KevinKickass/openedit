//! Integrated terminal emulator panel.
//!
//! Uses `portable-pty` for PTY allocation and a simple VT100 parser for rendering.

pub mod pty;
pub mod vt100;

use self::vt100::Vt100Parser;

/// State for the integrated terminal panel.
pub struct TerminalState {
    /// Whether the terminal panel is visible.
    pub visible: bool,
    /// Whether the terminal process is running.
    pub running: bool,
    /// Height fraction (0.0 - 1.0) of the editor area.
    pub height_fraction: f32,
    /// Height in pixels (computed from fraction).
    pub height: f32,
    /// The PTY session (if active).
    session: Option<pty::PtySession>,
    /// VT100 parser/screen buffer.
    parser: Vt100Parser,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            visible: false,
            running: false,
            height_fraction: 0.30,
            height: 250.0,
            session: None,
            parser: Vt100Parser::new(80, 24),
        }
    }
}

impl TerminalState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new shell session.
    pub fn start(&mut self) {
        match pty::PtySession::spawn_shell(80, 24) {
            Ok(session) => {
                self.session = Some(session);
                self.running = true;
                log::info!("Terminal: shell session started");
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
    pub fn kill(&mut self) {
        if let Some(mut session) = self.session.take() {
            session.kill();
        }
        self.running = false;
    }
}

impl Drop for TerminalState {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Render the terminal panel.
pub fn render_terminal(ui: &mut egui::Ui, state: &mut TerminalState, font_size: f32) {
    // Poll for new output
    state.poll();

    let rect = ui.available_rect_before_wrap();
    let char_width = crate::editor_view::char_width_for_font(font_size);
    let line_height = crate::editor_view::line_height_for_font(font_size);
    let font_id = egui::FontId::monospace(font_size);

    // Background
    let bg = egui::Color32::from_rgb(20, 20, 20);
    ui.painter().rect_filled(rect, 0.0, bg);

    // Header bar
    let header_height = 24.0;
    let header_rect = egui::Rect::from_min_size(
        rect.left_top(),
        egui::Vec2::new(rect.width(), header_height),
    );
    ui.painter()
        .rect_filled(header_rect, 0.0, egui::Color32::from_rgb(40, 40, 40));

    let title = if state.running {
        "Terminal"
    } else {
        "Terminal (inactive)"
    };
    ui.painter().text(
        egui::Pos2::new(rect.left() + 8.0, rect.top() + 4.0),
        egui::Align2::LEFT_TOP,
        title,
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(200, 200, 200),
    );

    // Close button
    let close_rect = egui::Rect::from_min_size(
        egui::Pos2::new(rect.right() - 24.0, rect.top() + 2.0),
        egui::Vec2::new(20.0, 20.0),
    );
    let close_resp = ui.allocate_rect(close_rect, egui::Sense::click());
    let close_color = if close_resp.hovered() {
        egui::Color32::from_rgb(255, 100, 100)
    } else {
        egui::Color32::from_rgb(160, 160, 160)
    };
    ui.painter().text(
        close_rect.center(),
        egui::Align2::CENTER_CENTER,
        "×",
        egui::FontId::proportional(16.0),
        close_color,
    );
    if close_resp.clicked() {
        state.visible = false;
    }

    // Terminal content area
    let content_top = rect.top() + header_height;
    let screen = &state.parser.screen;
    let rows = state.parser.rows;
    let cols = state.parser.cols;

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
                    &ch.to_string(),
                    font_id.clone(),
                    fg,
                );
                x += char_width;
            }
        }
    }

    // Cursor
    let cursor_row = state.parser.cursor_row;
    let cursor_col = state.parser.cursor_col;
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

    // Allocate area for interaction
    let _response = ui.allocate_rect(rect, egui::Sense::click());
}

/// Handle keyboard input for the terminal panel.
pub fn handle_terminal_input(ui: &mut egui::Ui, state: &mut TerminalState) {
    ui.input(|input| {
        let ctrl = input.modifiers.ctrl || input.modifiers.mac_cmd;

        for event in &input.events {
            match event {
                egui::Event::Text(text) => {
                    if !text.chars().all(|c| c.is_control()) {
                        state.send_input(text.as_bytes());
                    }
                }
                egui::Event::Key {
                    key, pressed: true, ..
                } => match key {
                    egui::Key::Enter => state.send_key("Enter"),
                    egui::Key::Backspace => state.send_key("Backspace"),
                    egui::Key::Tab => state.send_key("Tab"),
                    egui::Key::Escape => state.send_key("Escape"),
                    egui::Key::ArrowUp => state.send_key("ArrowUp"),
                    egui::Key::ArrowDown => state.send_key("ArrowDown"),
                    egui::Key::ArrowLeft => state.send_key("ArrowLeft"),
                    egui::Key::ArrowRight => state.send_key("ArrowRight"),
                    egui::Key::Home => state.send_key("Home"),
                    egui::Key::End => state.send_key("End"),
                    egui::Key::Delete => state.send_key("Delete"),
                    egui::Key::C if ctrl => state.send_input(b"\x03"),
                    egui::Key::D if ctrl => state.send_input(b"\x04"),
                    egui::Key::L if ctrl => state.send_input(b"\x0c"),
                    _ => {}
                },
                _ => {}
            }
        }
    });
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
        assert!(!state.running);
    }

    #[test]
    fn test_vt100_color_mapping() {
        let c = vt100_color_to_egui(31);
        assert_eq!(c, egui::Color32::from_rgb(205, 49, 49));
    }
}
