use crate::theme::EditorTheme;
use egui;

/// Action returned by hex view when the user edits a byte.
#[derive(Debug, Clone)]
pub enum HexAction {
    /// No action.
    None,
    /// A byte was edited at the given offset with the given new value.
    EditByte { offset: usize, old_byte: u8, new_byte: u8 },
}

/// State for the hex editor view.
pub struct HexViewState {
    /// Whether hex view is active for the current tab.
    pub active: bool,
    /// Raw bytes of the file.
    pub data: Vec<u8>,
    /// Scroll offset in rows.
    pub scroll_offset: f32,
    /// Number of bytes per row (default 16).
    pub bytes_per_row: usize,
    /// Currently selected byte offset (for highlighting).
    pub selected_offset: Option<usize>,
    /// First nibble entered during hex editing (high nibble, 0-15).
    /// When set, we're waiting for the second nibble.
    pub editing_nibble: Option<u8>,
    /// Whether the go-to-offset dialog is open.
    pub go_to_offset_open: bool,
    /// Text input for go-to-offset dialog.
    pub go_to_offset_input: String,
}

impl Default for HexViewState {
    fn default() -> Self {
        Self {
            active: false,
            data: Vec::new(),
            scroll_offset: 0.0,
            bytes_per_row: 16,
            selected_offset: None,
            editing_nibble: None,
            go_to_offset_open: false,
            go_to_offset_input: String::new(),
        }
    }
}

/// Parse a hex digit character to its numeric value.
fn hex_digit_value(ch: char) -> Option<u8> {
    match ch {
        '0'..='9' => Some(ch as u8 - b'0'),
        'a'..='f' => Some(ch as u8 - b'a' + 10),
        'A'..='F' => Some(ch as u8 - b'A' + 10),
        _ => None,
    }
}

/// Parse an offset string (hex with optional "0x" prefix, or plain decimal).
fn parse_offset(s: &str) -> Option<usize> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(hex_str) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        usize::from_str_radix(hex_str, 16).ok()
    } else if s.starts_with(|c: char| c.is_ascii_hexdigit())
        && s.chars().all(|c| c.is_ascii_hexdigit())
        && s.chars().any(|c| c.is_ascii_alphabetic())
    {
        // Contains hex alpha chars -> treat as hex
        usize::from_str_radix(s, 16).ok()
    } else {
        // Try decimal first
        s.parse::<usize>().ok()
    }
}

/// Render the hex editor view.
///
/// Classic hex editor layout:
/// ```text
/// OFFSET   | HH HH HH HH HH HH HH HH  HH HH HH HH HH HH HH HH | ASCII.TEXT.HERE.
/// ```
///
/// - Offset column: 8-digit hex address
/// - Hex column: 16 bytes shown as 2-digit hex, split into two groups of 8
/// - ASCII column: printable chars shown as-is, non-printable as '.'
///
/// Returns a `HexAction` if the user edited a byte.
pub fn render_hex_view(
    ui: &mut egui::Ui,
    state: &mut HexViewState,
    theme: &EditorTheme,
    font_size: f32,
) -> HexAction {
    let mut action = HexAction::None;

    let rect = ui.available_rect_before_wrap();
    let line_height = (font_size * 1.385).round();
    let char_width = font_size * 0.646;

    // Fill background
    ui.painter().rect_filled(rect, 0.0, theme.background);

    // Handle scroll
    let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
    if scroll_delta != 0.0 {
        state.scroll_offset = (state.scroll_offset - scroll_delta / line_height).max(0.0);
    }

    let total_rows = if state.data.is_empty() {
        0
    } else {
        (state.data.len() + state.bytes_per_row - 1) / state.bytes_per_row
    };
    let visible_rows = (rect.height() / line_height).ceil() as usize;
    let max_scroll = if total_rows > visible_rows {
        total_rows - visible_rows
    } else {
        0
    };
    state.scroll_offset = state.scroll_offset.min(max_scroll as f32);

    let start_row = state.scroll_offset as usize;
    let end_row = (start_row + visible_rows + 1).min(total_rows);

    // Column layout positions
    let offset_x = rect.left() + 8.0;
    let hex_x = offset_x + char_width * 10.0;
    let ascii_x = hex_x + char_width * 50.0 + char_width * 3.0;

    // Header
    let header_y = rect.top() + 4.0;
    let header_color = theme.gutter_fg;
    ui.painter().text(
        egui::Pos2::new(offset_x, header_y),
        egui::Align2::LEFT_TOP,
        "Offset",
        egui::FontId::monospace(font_size),
        header_color,
    );
    ui.painter().text(
        egui::Pos2::new(hex_x, header_y),
        egui::Align2::LEFT_TOP,
        "00 01 02 03 04 05 06 07  08 09 0A 0B 0C 0D 0E 0F",
        egui::FontId::monospace(font_size),
        header_color,
    );
    ui.painter().text(
        egui::Pos2::new(ascii_x, header_y),
        egui::Align2::LEFT_TOP,
        "ASCII",
        egui::FontId::monospace(font_size),
        header_color,
    );

    let data_y_start = header_y + line_height + 4.0;

    // Handle keyboard input for hex editing and go-to-offset
    let hex_input = ui.input(|input| {
        let ctrl = input.modifiers.ctrl || input.modifiers.mac_cmd;

        let mut typed_chars: Vec<char> = Vec::new();
        let mut escape_pressed = false;
        let mut go_to_offset = false;
        let mut arrow_left = false;
        let mut arrow_right = false;
        let mut arrow_up = false;
        let mut arrow_down = false;

        for event in &input.events {
            match event {
                egui::Event::Text(text) => {
                    for ch in text.chars() {
                        if ch.is_ascii_hexdigit() {
                            typed_chars.push(ch);
                        }
                    }
                }
                egui::Event::Key {
                    key, pressed: true, ..
                } => match key {
                    egui::Key::Escape => escape_pressed = true,
                    egui::Key::G if ctrl => go_to_offset = true,
                    egui::Key::ArrowLeft => arrow_left = true,
                    egui::Key::ArrowRight => arrow_right = true,
                    egui::Key::ArrowUp => arrow_up = true,
                    egui::Key::ArrowDown => arrow_down = true,
                    _ => {}
                },
                _ => {}
            }
        }

        (
            typed_chars,
            escape_pressed,
            go_to_offset,
            arrow_left,
            arrow_right,
            arrow_up,
            arrow_down,
        )
    });

    let (typed_chars, escape_pressed, go_to_offset_pressed, arrow_left, arrow_right, arrow_up, arrow_down) =
        hex_input;

    // Handle escape: clear editing state
    if escape_pressed {
        state.editing_nibble = None;
        if state.go_to_offset_open {
            state.go_to_offset_open = false;
        }
    }

    // Handle Ctrl+G: open go-to-offset dialog
    if go_to_offset_pressed && !state.go_to_offset_open {
        state.go_to_offset_open = true;
        state.go_to_offset_input.clear();
    }

    // Arrow key navigation
    if let Some(sel) = state.selected_offset {
        let bpr = state.bytes_per_row;
        let new_sel = if arrow_left && sel > 0 {
            Some(sel - 1)
        } else if arrow_right && sel + 1 < state.data.len() {
            Some(sel + 1)
        } else if arrow_up && sel >= bpr {
            Some(sel - bpr)
        } else if arrow_down && sel + bpr < state.data.len() {
            Some(sel + bpr)
        } else {
            None
        };
        if let Some(ns) = new_sel {
            state.selected_offset = Some(ns);
            state.editing_nibble = None;
            // Auto-scroll to keep selection visible
            let sel_row = ns / state.bytes_per_row;
            if (sel_row as f32) < state.scroll_offset {
                state.scroll_offset = sel_row as f32;
            } else if sel_row >= start_row + visible_rows {
                state.scroll_offset = (sel_row as f32 - visible_rows as f32 + 1.0).max(0.0);
            }
        }
    }

    // Handle hex digit typing
    if !state.go_to_offset_open {
        for ch in &typed_chars {
            if let (Some(sel), Some(nibble_val)) = (state.selected_offset, hex_digit_value(*ch)) {
                if sel < state.data.len() {
                    if let Some(high_nibble) = state.editing_nibble {
                        // Second nibble: combine and apply edit
                        let old_byte = state.data[sel];
                        let new_byte = (high_nibble << 4) | nibble_val;
                        state.data[sel] = new_byte;
                        state.editing_nibble = None;
                        action = HexAction::EditByte {
                            offset: sel,
                            old_byte,
                            new_byte,
                        };
                        // Advance to next byte
                        if sel + 1 < state.data.len() {
                            state.selected_offset = Some(sel + 1);
                        }
                    } else {
                        // First nibble: store it, wait for second
                        state.editing_nibble = Some(nibble_val);
                    }
                }
            }
        }
    }

    // Handle click to select byte
    let response = ui.allocate_rect(rect, egui::Sense::click());
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let row_f = (pos.y - data_y_start) / line_height;
            if row_f >= 0.0 {
                let row = row_f as usize + start_row;
                // Check if click is in hex area
                if pos.x >= hex_x {
                    let hex_col = ((pos.x - hex_x) / char_width) as usize;
                    if hex_col < 50 {
                        // Map hex column to byte index within row
                        // Layout: "HH HH HH HH HH HH HH HH  HH HH HH HH HH HH HH HH"
                        let adjusted = if hex_col >= 25 { hex_col - 1 } else { hex_col };
                        let byte_in_row = adjusted / 3;
                        if byte_in_row < state.bytes_per_row {
                            let offset = row * state.bytes_per_row + byte_in_row;
                            if offset < state.data.len() {
                                state.selected_offset = Some(offset);
                                state.editing_nibble = None;
                            }
                        }
                    }
                }
            }
        }
    }

    // Render rows
    for row in start_row..end_row {
        let y = data_y_start + (row - start_row) as f32 * line_height;
        let byte_offset = row * state.bytes_per_row;

        // Offset column
        let offset_text = format!("{:08X}", byte_offset);
        ui.painter().text(
            egui::Pos2::new(offset_x, y),
            egui::Align2::LEFT_TOP,
            &offset_text,
            egui::FontId::monospace(font_size),
            theme.gutter_fg,
        );

        // Build hex + ASCII strings
        let end = (byte_offset + state.bytes_per_row).min(state.data.len());
        let mut hex_str = String::with_capacity(50);
        let mut ascii_str = String::with_capacity(state.bytes_per_row);

        for i in 0..state.bytes_per_row {
            let global_offset = byte_offset + i;
            if global_offset < end {
                let byte = state.data[global_offset];

                // Extra space between first and second group of 8
                if i == 8 {
                    hex_str.push(' ');
                }

                // Show editing state for selected byte
                if state.selected_offset == Some(global_offset) {
                    if let Some(high_nibble) = state.editing_nibble {
                        hex_str.push_str(&format!("{:X}_", high_nibble));
                    } else {
                        hex_str.push_str(&format!("{:02X} ", byte));
                    }
                } else {
                    hex_str.push_str(&format!("{:02X} ", byte));
                }

                // ASCII representation
                if byte.is_ascii_graphic() || byte == b' ' {
                    ascii_str.push(byte as char);
                } else {
                    ascii_str.push('.');
                }
            } else {
                if i == 8 {
                    hex_str.push(' ');
                }
                hex_str.push_str("   ");
                ascii_str.push(' ');
            }
        }

        // Highlight selected byte in hex and ASCII areas
        if let Some(sel) = state.selected_offset {
            if sel >= byte_offset && sel < byte_offset + state.bytes_per_row {
                let byte_in_row = sel - byte_offset;
                let hex_char_offset = byte_in_row * 3 + if byte_in_row >= 8 { 1 } else { 0 };
                let sel_color = if state.editing_nibble.is_some() {
                    egui::Color32::from_rgba_premultiplied(80, 80, 160, 120)
                } else {
                    egui::Color32::from_rgba_premultiplied(60, 60, 120, 100)
                };
                let sel_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(hex_x + hex_char_offset as f32 * char_width, y),
                    egui::Vec2::new(char_width * 2.0, line_height),
                );
                ui.painter().rect_filled(sel_rect, 0.0, sel_color);

                // Highlight in ASCII area
                let ascii_sel_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(ascii_x + byte_in_row as f32 * char_width, y),
                    egui::Vec2::new(char_width, line_height),
                );
                ui.painter().rect_filled(ascii_sel_rect, 0.0, sel_color);
            }
        }

        // Draw hex text
        ui.painter().text(
            egui::Pos2::new(hex_x, y),
            egui::Align2::LEFT_TOP,
            &hex_str,
            egui::FontId::monospace(font_size),
            theme.foreground,
        );

        // Draw separator
        ui.painter().text(
            egui::Pos2::new(ascii_x - char_width * 2.0, y),
            egui::Align2::LEFT_TOP,
            "|",
            egui::FontId::monospace(font_size),
            theme.gutter_fg,
        );

        // Draw ASCII text
        ui.painter().text(
            egui::Pos2::new(ascii_x, y),
            egui::Align2::LEFT_TOP,
            &ascii_str,
            egui::FontId::monospace(font_size),
            theme.foreground,
        );
    }

    // Show file info at bottom
    let info_parts = if let Some(sel) = state.selected_offset {
        let editing_hint = if state.editing_nibble.is_some() {
            " [editing]"
        } else {
            ""
        };
        format!(
            "{} bytes ({} rows) | Offset: 0x{:X} ({}){}  |  Ctrl+G: Go to offset",
            state.data.len(),
            total_rows,
            sel,
            sel,
            editing_hint,
        )
    } else {
        format!(
            "{} bytes ({} rows)  |  Ctrl+G: Go to offset",
            state.data.len(),
            total_rows
        )
    };
    ui.painter().text(
        egui::Pos2::new(rect.left() + 8.0, rect.bottom() - line_height - 4.0),
        egui::Align2::LEFT_TOP,
        &info_parts,
        egui::FontId::monospace(font_size * 0.9),
        theme.gutter_fg,
    );

    // Go to offset dialog
    if state.go_to_offset_open {
        render_go_to_offset_dialog(ui, state, theme, font_size, rect);
    }

    action
}

/// Render the go-to-offset dialog as an overlay.
fn render_go_to_offset_dialog(
    ui: &mut egui::Ui,
    state: &mut HexViewState,
    theme: &EditorTheme,
    font_size: f32,
    parent_rect: egui::Rect,
) {
    let dialog_width = 300.0f32;
    let dialog_height = (font_size * 4.0).round();
    let dialog_rect = egui::Rect::from_min_size(
        egui::Pos2::new(
            parent_rect.center().x - dialog_width / 2.0,
            parent_rect.top() + 60.0,
        ),
        egui::Vec2::new(dialog_width, dialog_height),
    );

    // Draw dialog background with border
    ui.painter().rect_filled(
        dialog_rect,
        4.0,
        theme.gutter_bg,
    );
    ui.painter().rect_stroke(
        dialog_rect,
        4.0,
        egui::Stroke::new(1.0, theme.gutter_fg),
    );

    // Label
    ui.painter().text(
        egui::Pos2::new(dialog_rect.left() + 10.0, dialog_rect.top() + 8.0),
        egui::Align2::LEFT_TOP,
        "Go to offset (hex: 0x1A, or decimal: 26):",
        egui::FontId::monospace(font_size * 0.85),
        theme.foreground,
    );

    // Text input area
    let input_rect = egui::Rect::from_min_size(
        egui::Pos2::new(dialog_rect.left() + 10.0, dialog_rect.top() + 8.0 + font_size * 1.5),
        egui::Vec2::new(dialog_width - 20.0, font_size * 1.5),
    );

    // Use egui's built-in TextEdit for the input
    let mut input_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(input_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );

    let te_response = input_ui.add(
        egui::TextEdit::singleline(&mut state.go_to_offset_input)
            .font(egui::FontId::monospace(font_size))
            .desired_width(dialog_width - 20.0),
    );

    // Auto-focus the text input
    te_response.request_focus();

    // Handle Enter to confirm
    if te_response.lost_focus()
        && input_ui.input(|i| i.key_pressed(egui::Key::Enter))
    {
        if let Some(offset) = parse_offset(&state.go_to_offset_input) {
            if offset < state.data.len() {
                state.selected_offset = Some(offset);
                state.editing_nibble = None;
                // Scroll to make the offset visible
                let row = offset / state.bytes_per_row;
                state.scroll_offset = row as f32;
            }
        }
        state.go_to_offset_open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = HexViewState::default();
        assert!(!state.active);
        assert!(state.data.is_empty());
        assert_eq!(state.bytes_per_row, 16);
        assert!(state.selected_offset.is_none());
        assert!(state.editing_nibble.is_none());
        assert!(!state.go_to_offset_open);
        assert!(state.go_to_offset_input.is_empty());
    }

    #[test]
    fn test_state_with_data() {
        let mut state = HexViewState::default();
        state.data = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F]; // "Hello"
        assert_eq!(state.data.len(), 5);
        state.selected_offset = Some(2);
        assert_eq!(state.selected_offset, Some(2));
    }

    #[test]
    fn test_active_toggle() {
        let mut state = HexViewState::default();
        assert!(!state.active);
        state.active = true;
        assert!(state.active);
        state.active = false;
        assert!(!state.active);
    }

    #[test]
    fn test_scroll_offset_default() {
        let state = HexViewState::default();
        assert_eq!(state.scroll_offset, 0.0);
    }

    #[test]
    fn test_hex_digit_value() {
        assert_eq!(hex_digit_value('0'), Some(0));
        assert_eq!(hex_digit_value('9'), Some(9));
        assert_eq!(hex_digit_value('a'), Some(10));
        assert_eq!(hex_digit_value('f'), Some(15));
        assert_eq!(hex_digit_value('A'), Some(10));
        assert_eq!(hex_digit_value('F'), Some(15));
        assert_eq!(hex_digit_value('g'), None);
        assert_eq!(hex_digit_value('z'), None);
        assert_eq!(hex_digit_value(' '), None);
    }

    #[test]
    fn test_parse_offset_hex_prefix() {
        assert_eq!(parse_offset("0x1A"), Some(26));
        assert_eq!(parse_offset("0xFF"), Some(255));
        assert_eq!(parse_offset("0X10"), Some(16));
        assert_eq!(parse_offset("0x0"), Some(0));
    }

    #[test]
    fn test_parse_offset_decimal() {
        assert_eq!(parse_offset("42"), Some(42));
        assert_eq!(parse_offset("0"), Some(0));
        assert_eq!(parse_offset("1000"), Some(1000));
    }

    #[test]
    fn test_parse_offset_hex_without_prefix() {
        // Contains hex alpha chars -> treated as hex
        assert_eq!(parse_offset("FF"), Some(255));
        assert_eq!(parse_offset("1a"), Some(26));
        assert_eq!(parse_offset("DEAD"), Some(0xDEAD));
    }

    #[test]
    fn test_parse_offset_empty() {
        assert_eq!(parse_offset(""), None);
        assert_eq!(parse_offset("  "), None);
    }

    #[test]
    fn test_parse_offset_whitespace() {
        assert_eq!(parse_offset("  42  "), Some(42));
        assert_eq!(parse_offset("  0xFF  "), Some(255));
    }

    #[test]
    fn test_editing_nibble_state() {
        let mut state = HexViewState::default();
        state.data = vec![0x00, 0x11, 0x22];
        state.selected_offset = Some(1);

        // Simulate entering first nibble
        state.editing_nibble = Some(0xA);
        assert_eq!(state.editing_nibble, Some(0xA));

        // Simulate completing the edit
        let high = state.editing_nibble.unwrap();
        let low = 0x5u8;
        let new_byte = (high << 4) | low;
        assert_eq!(new_byte, 0xA5);

        state.data[1] = new_byte;
        state.editing_nibble = None;
        assert_eq!(state.data[1], 0xA5);
        assert!(state.editing_nibble.is_none());
    }

    #[test]
    fn test_go_to_offset_state() {
        let mut state = HexViewState::default();
        state.data = vec![0; 256];

        state.go_to_offset_open = true;
        state.go_to_offset_input = "0x80".to_string();

        // Simulate what would happen when user confirms
        if let Some(offset) = parse_offset(&state.go_to_offset_input) {
            if offset < state.data.len() {
                state.selected_offset = Some(offset);
                let row = offset / state.bytes_per_row;
                state.scroll_offset = row as f32;
            }
        }
        state.go_to_offset_open = false;

        assert_eq!(state.selected_offset, Some(128));
        assert_eq!(state.scroll_offset, 8.0); // 128 / 16 = 8
        assert!(!state.go_to_offset_open);
    }
}
