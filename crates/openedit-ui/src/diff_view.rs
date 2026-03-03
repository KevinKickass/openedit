use crate::editor_view::{char_width_for_font, line_height_for_font};
use crate::theme::EditorTheme;
use egui::{self, Color32, Pos2, Rect, Vec2};
use openedit_core::diff::{diff_lines, DiffOp};

/// State for the diff view.
pub struct DiffViewState {
    /// Whether diff view is active.
    pub active: bool,
    /// Index of left document in the documents Vec.
    pub left_tab: usize,
    /// Index of right document in the documents Vec.
    pub right_tab: usize,
    /// Cached diff operations.
    pub diff_ops: Vec<DiffOp>,
    /// Scroll offset (shared between panes) in pixels.
    pub scroll_offset: f32,
    /// Hash of left content when diff was last computed, for cache invalidation.
    left_hash: u64,
    /// Hash of right content when diff was last computed, for cache invalidation.
    right_hash: u64,
}

impl Default for DiffViewState {
    fn default() -> Self {
        Self {
            active: false,
            left_tab: 0,
            right_tab: 0,
            diff_ops: Vec::new(),
            scroll_offset: 0.0,
            left_hash: 0,
            right_hash: 0,
        }
    }
}

/// Simple hash for cache invalidation (not cryptographic).
fn simple_hash(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Background color for inserted lines (green tint).
fn insert_bg(theme: &EditorTheme) -> Color32 {
    // Check if dark or light theme by looking at background brightness
    let is_dark = theme.background.r() < 128;
    if is_dark {
        Color32::from_rgba_premultiplied(40, 80, 40, 60)
    } else {
        Color32::from_rgba_premultiplied(180, 255, 180, 80)
    }
}

/// Background color for deleted lines (red tint).
fn delete_bg(theme: &EditorTheme) -> Color32 {
    let is_dark = theme.background.r() < 128;
    if is_dark {
        Color32::from_rgba_premultiplied(80, 40, 40, 60)
    } else {
        Color32::from_rgba_premultiplied(255, 200, 200, 80)
    }
}

/// Text color for inserted lines.
fn insert_fg(theme: &EditorTheme) -> Color32 {
    let is_dark = theme.background.r() < 128;
    if is_dark {
        Color32::from_rgb(130, 220, 130)
    } else {
        Color32::from_rgb(0, 100, 0)
    }
}

/// Text color for deleted lines.
fn delete_fg(theme: &EditorTheme) -> Color32 {
    let is_dark = theme.background.r() < 128;
    if is_dark {
        Color32::from_rgb(220, 130, 130)
    } else {
        Color32::from_rgb(150, 0, 0)
    }
}

/// A row in the diff view for one pane.
struct DiffRow {
    /// Line number to display (1-based), or None for blank filler rows.
    line_number: Option<usize>,
    /// Text content of the line.
    text: String,
    /// Background color for the row.
    bg: Color32,
    /// Foreground (text) color for the row.
    fg: Color32,
}

/// Build left and right pane rows from diff ops.
fn build_rows(ops: &[DiffOp], theme: &EditorTheme) -> (Vec<DiffRow>, Vec<DiffRow>) {
    let mut left_rows = Vec::new();
    let mut right_rows = Vec::new();
    let mut left_line = 0usize;
    let mut right_line = 0usize;
    let normal_bg = Color32::TRANSPARENT;
    let normal_fg = theme.foreground;

    for op in ops {
        match op {
            DiffOp::Equal(text) => {
                left_line += 1;
                right_line += 1;
                left_rows.push(DiffRow {
                    line_number: Some(left_line),
                    text: text.clone(),
                    bg: normal_bg,
                    fg: normal_fg,
                });
                right_rows.push(DiffRow {
                    line_number: Some(right_line),
                    text: text.clone(),
                    bg: normal_bg,
                    fg: normal_fg,
                });
            }
            DiffOp::Delete(text) => {
                left_line += 1;
                left_rows.push(DiffRow {
                    line_number: Some(left_line),
                    text: text.clone(),
                    bg: delete_bg(theme),
                    fg: delete_fg(theme),
                });
                // Add a blank filler row on the right side to keep alignment
                right_rows.push(DiffRow {
                    line_number: None,
                    text: String::new(),
                    bg: delete_bg(theme),
                    fg: normal_fg,
                });
            }
            DiffOp::Insert(text) => {
                right_line += 1;
                // Add a blank filler row on the left side to keep alignment
                left_rows.push(DiffRow {
                    line_number: None,
                    text: String::new(),
                    bg: insert_bg(theme),
                    fg: normal_fg,
                });
                right_rows.push(DiffRow {
                    line_number: Some(right_line),
                    text: text.clone(),
                    bg: insert_bg(theme),
                    fg: insert_fg(theme),
                });
            }
        }
    }

    (left_rows, right_rows)
}

/// Render the diff view showing two documents side-by-side with highlighted differences.
pub fn render_diff_view(
    ui: &mut egui::Ui,
    state: &mut DiffViewState,
    left_content: &str,
    right_content: &str,
    left_name: &str,
    right_name: &str,
    theme: &EditorTheme,
    font_size: f32,
) {
    // Recompute diff if content has changed
    let lh = simple_hash(left_content);
    let rh = simple_hash(right_content);
    if lh != state.left_hash || rh != state.right_hash {
        state.diff_ops = diff_lines(left_content, right_content);
        state.left_hash = lh;
        state.right_hash = rh;
    }

    let line_height = line_height_for_font(font_size);
    let char_width = char_width_for_font(font_size);
    let font_id = egui::FontId::monospace(font_size);

    let (left_rows, right_rows) = build_rows(&state.diff_ops, theme);
    let total_rows = left_rows.len();

    let available = ui.available_rect_before_wrap();

    // Header bar with file names
    let header_height = line_height + 4.0;
    let header_rect = Rect::from_min_size(
        available.left_top(),
        Vec2::new(available.width(), header_height),
    );
    let half_width = available.width() / 2.0;
    let separator_x = available.left() + half_width;

    // Draw header background
    ui.painter().rect_filled(header_rect, 0.0, theme.gutter_bg);

    // Left header label
    let left_header_rect =
        Rect::from_min_size(header_rect.left_top(), Vec2::new(half_width, header_height));
    ui.painter().text(
        Pos2::new(left_header_rect.left() + 8.0, left_header_rect.center().y),
        egui::Align2::LEFT_CENTER,
        left_name,
        egui::FontId::monospace(font_size * 0.9),
        theme.foreground,
    );

    // Right header label
    let right_header_rect = Rect::from_min_size(
        Pos2::new(separator_x, header_rect.top()),
        Vec2::new(half_width, header_height),
    );
    ui.painter().text(
        Pos2::new(right_header_rect.left() + 8.0, right_header_rect.center().y),
        egui::Align2::LEFT_CENTER,
        right_name,
        egui::FontId::monospace(font_size * 0.9),
        theme.foreground,
    );

    // Header separator line
    ui.painter().line_segment(
        [
            Pos2::new(separator_x, header_rect.top()),
            Pos2::new(separator_x, header_rect.bottom()),
        ],
        egui::Stroke::new(1.0, theme.gutter_fg),
    );

    // Content area below header
    let content_rect = Rect::from_min_max(
        Pos2::new(available.left(), available.top() + header_height),
        available.right_bottom(),
    );

    // Handle scroll input
    let content_response = ui.allocate_rect(content_rect, egui::Sense::hover());
    if content_response.hovered() {
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        state.scroll_offset -= scroll_delta;
    }

    // Clamp scroll offset
    let total_content_height = total_rows as f32 * line_height;
    let max_scroll = (total_content_height - content_rect.height()).max(0.0);
    state.scroll_offset = state.scroll_offset.clamp(0.0, max_scroll);

    // Compute visible range
    let first_visible_row = (state.scroll_offset / line_height).floor() as usize;
    let visible_rows_count = (content_rect.height() / line_height).ceil() as usize + 1;
    let last_visible_row = (first_visible_row + visible_rows_count).min(total_rows);

    // Draw background
    ui.painter()
        .rect_filled(content_rect, 0.0, theme.background);

    // Clip to content area
    let painter = ui.painter_at(content_rect);

    // Gutter width (enough for line numbers)
    let max_line_left = left_rows
        .iter()
        .filter_map(|r| r.line_number)
        .max()
        .unwrap_or(0);
    let max_line_right = right_rows
        .iter()
        .filter_map(|r| r.line_number)
        .max()
        .unwrap_or(0);
    let max_line = max_line_left.max(max_line_right);
    let digit_count = format!("{}", max_line).len().max(3);
    let gutter_width = (digit_count as f32 + 2.0) * char_width;

    let left_pane_rect = Rect::from_min_size(
        content_rect.left_top(),
        Vec2::new(half_width - 1.0, content_rect.height()),
    );
    let right_pane_rect = Rect::from_min_size(
        Pos2::new(separator_x + 1.0, content_rect.top()),
        Vec2::new(half_width - 1.0, content_rect.height()),
    );

    // Draw vertical separator for content area
    painter.line_segment(
        [
            Pos2::new(separator_x, content_rect.top()),
            Pos2::new(separator_x, content_rect.bottom()),
        ],
        egui::Stroke::new(1.0, theme.gutter_fg),
    );

    // Render visible rows
    for row_idx in first_visible_row..last_visible_row {
        let y = content_rect.top() + (row_idx as f32 * line_height) - state.scroll_offset;

        // Left pane row
        if let Some(left_row) = left_rows.get(row_idx) {
            render_diff_row(
                &painter,
                left_row,
                left_pane_rect.left(),
                y,
                half_width - 1.0,
                line_height,
                gutter_width,
                char_width,
                digit_count,
                &font_id,
                theme,
            );
        }

        // Right pane row
        if let Some(right_row) = right_rows.get(row_idx) {
            render_diff_row(
                &painter,
                right_row,
                right_pane_rect.left(),
                y,
                half_width - 1.0,
                line_height,
                gutter_width,
                char_width,
                digit_count,
                &font_id,
                theme,
            );
        }
    }

    // Summary bar at the bottom
    let additions = state
        .diff_ops
        .iter()
        .filter(|o| matches!(o, DiffOp::Insert(_)))
        .count();
    let deletions = state
        .diff_ops
        .iter()
        .filter(|o| matches!(o, DiffOp::Delete(_)))
        .count();
    let unchanged = state
        .diff_ops
        .iter()
        .filter(|o| matches!(o, DiffOp::Equal(_)))
        .count();
    let summary = format!(
        "+{} additions  -{} deletions  {} unchanged",
        additions, deletions, unchanged
    );

    // Paint summary text at the bottom of content area
    let summary_y = content_rect.bottom() - line_height;
    let summary_rect = Rect::from_min_size(
        Pos2::new(content_rect.left(), summary_y),
        Vec2::new(content_rect.width(), line_height),
    );
    painter.rect_filled(summary_rect, 0.0, theme.gutter_bg);
    painter.text(
        Pos2::new(summary_rect.left() + 8.0, summary_rect.center().y),
        egui::Align2::LEFT_CENTER,
        &summary,
        egui::FontId::monospace(font_size * 0.85),
        theme.gutter_fg,
    );
}

/// Render a single diff row (line number gutter + text content).
fn render_diff_row(
    painter: &egui::Painter,
    row: &DiffRow,
    left_x: f32,
    y: f32,
    pane_width: f32,
    line_height: f32,
    gutter_width: f32,
    char_width: f32,
    digit_count: usize,
    font_id: &egui::FontId,
    theme: &EditorTheme,
) {
    let row_rect = Rect::from_min_size(Pos2::new(left_x, y), Vec2::new(pane_width, line_height));

    // Draw row background color
    if row.bg != Color32::TRANSPARENT {
        painter.rect_filled(row_rect, 0.0, row.bg);
    }

    // Draw gutter background
    let gutter_rect =
        Rect::from_min_size(Pos2::new(left_x, y), Vec2::new(gutter_width, line_height));
    painter.rect_filled(gutter_rect, 0.0, theme.gutter_bg);

    // Draw line number
    if let Some(line_num) = row.line_number {
        let line_str = format!("{:>width$}", line_num, width = digit_count);
        painter.text(
            Pos2::new(left_x + char_width, y + line_height / 2.0),
            egui::Align2::LEFT_CENTER,
            &line_str,
            font_id.clone(),
            theme.gutter_fg,
        );
    }

    // Draw text content
    if !row.text.is_empty() {
        let text_x = left_x + gutter_width + 4.0;
        painter.text(
            Pos2::new(text_x, y + line_height / 2.0),
            egui::Align2::LEFT_CENTER,
            &row.text,
            font_id.clone(),
            row.fg,
        );
    }
}
