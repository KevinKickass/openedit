use crate::theme::EditorTheme;
use egui;

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
}

impl Default for HexViewState {
    fn default() -> Self {
        Self {
            active: false,
            data: Vec::new(),
            scroll_offset: 0.0,
            bytes_per_row: 16,
            selected_offset: None,
        }
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
pub fn render_hex_view(
    ui: &mut egui::Ui,
    state: &mut HexViewState,
    theme: &EditorTheme,
    font_size: f32,
) {
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
                hex_str.push_str(&format!("{:02X} ", byte));

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
                let hex_char_offset =
                    byte_in_row * 3 + if byte_in_row >= 8 { 1 } else { 0 };
                let sel_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(hex_x + hex_char_offset as f32 * char_width, y),
                    egui::Vec2::new(char_width * 2.0, line_height),
                );
                ui.painter().rect_filled(
                    sel_rect,
                    0.0,
                    egui::Color32::from_rgba_premultiplied(60, 60, 120, 100),
                );

                // Highlight in ASCII area
                let ascii_sel_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(ascii_x + byte_in_row as f32 * char_width, y),
                    egui::Vec2::new(char_width, line_height),
                );
                ui.painter().rect_filled(
                    ascii_sel_rect,
                    0.0,
                    egui::Color32::from_rgba_premultiplied(60, 60, 120, 100),
                );
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
    let info_text = format!("{} bytes ({} rows)", state.data.len(), total_rows);
    ui.painter().text(
        egui::Pos2::new(rect.left() + 8.0, rect.bottom() - line_height - 4.0),
        egui::Align2::LEFT_TOP,
        &info_text,
        egui::FontId::monospace(font_size * 0.9),
        theme.gutter_fg,
    );
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
}
