use crate::theme::EditorTheme;
use egui::{self, Pos2, Rect, Ui};
use openedit_core::folding::FoldingState;

/// Render the line number gutter. Returns the width consumed.
///
/// `displayed_lines` is a slice of `(screen_row, actual_line_idx)` pairs,
/// where `screen_row` is the row offset from the top of the viewport (0-based).
pub fn render_gutter(
    ui: &mut Ui,
    total_lines: usize,
    displayed_lines: &[(usize, usize)],
    current_line: usize,
    line_height: f32,
    top_y: f32,
    theme: &EditorTheme,
    font_size: f32,
    folding: &FoldingState,
    bookmarks: &[usize],
) -> f32 {
    let digit_count = format!("{}", total_lines).len().max(3);
    let char_width = crate::editor_view::char_width_for_font(font_size);
    // Extra space for fold indicator column
    let fold_col_width = char_width * 1.5;
    let gutter_width = (digit_count as f32 + 2.0) * char_width + fold_col_width;
    let gutter_padding_right = 8.0;

    let rect = ui.available_rect_before_wrap();
    let gutter_rect = Rect::from_min_size(
        rect.left_top(),
        egui::vec2(gutter_width + gutter_padding_right, rect.height()),
    );

    // Draw gutter background
    ui.painter()
        .rect_filled(gutter_rect, 0.0, theme.gutter_bg);

    // Draw line numbers and fold indicators
    let font_id = egui::FontId::monospace(font_size);
    let fold_font = egui::FontId::monospace(font_size * 0.75);
    let fold_color = theme.gutter_fg;

    for &(screen_row, line_idx) in displayed_lines {
        let y = top_y + screen_row as f32 * line_height;
        let line_num = format!("{:>width$}", line_idx + 1, width = digit_count);

        let color = if line_idx == current_line {
            theme.foreground
        } else {
            theme.gutter_fg
        };

        ui.painter().text(
            Pos2::new(gutter_rect.left() + 4.0, y),
            egui::Align2::LEFT_TOP,
            &line_num,
            font_id.clone(),
            color,
        );

        // Fold indicator
        if folding.is_fold_start(line_idx) {
            let indicator = if folding.is_folded(line_idx) {
                "\u{25B6}" // right-pointing triangle (collapsed)
            } else {
                "\u{25BC}" // down-pointing triangle (expanded)
            };
            let fold_x = gutter_rect.left() + 4.0 + (digit_count as f32 + 0.5) * char_width;
            ui.painter().text(
                Pos2::new(fold_x, y + 1.0),
                egui::Align2::LEFT_TOP,
                indicator,
                fold_font.clone(),
                fold_color,
            );
        }

        // Bookmark indicator
        if bookmarks.binary_search(&line_idx).is_ok() {
            let bookmark_color = egui::Color32::from_rgb(70, 130, 220); // blue
            let radius = font_size * 0.22;
            let cx = gutter_rect.left() + 4.0 + radius;
            let cy = y + line_height * 0.5;
            ui.painter().circle_filled(
                Pos2::new(cx, cy),
                radius,
                bookmark_color,
            );
        }
    }

    gutter_width + gutter_padding_right
}
