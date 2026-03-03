use crate::editor_view::{char_width_for_font, line_height_for_font};
use crate::theme::EditorTheme;
use egui::{self, Color32, Pos2, Rect, Vec2};
use openedit_core::diff::{diff_lines, DiffOp};
use std::collections::HashSet;

/// A hunk is a contiguous group of non-Equal diff ops, tracked by its
/// starting display-row index and the range of DiffOp indices it covers.
#[derive(Debug, Clone)]
pub struct DiffHunk {
    /// Display-row index where this hunk starts (used for scrolling).
    pub row_start: usize,
    /// Start index into `diff_ops` (inclusive).
    pub op_start: usize,
    /// End index into `diff_ops` (exclusive).
    pub op_end: usize,
}

/// Action returned by the diff view when the user clicks a merge button.
#[derive(Debug, Clone)]
pub enum DiffAction {
    /// No action.
    None,
    /// Copy the left side's lines for this hunk into the right document.
    /// Contains (right_tab, new_full_content_for_right).
    MergeLeftToRight(String),
    /// Copy the right side's lines for this hunk into the left document.
    /// Contains (left_tab, new_full_content_for_left).
    MergeRightToLeft(String),
}

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
    /// Cached hunk positions (recomputed when diff changes).
    pub hunks: Vec<DiffHunk>,
    /// Index of the currently focused hunk (for next/prev navigation).
    pub current_hunk: usize,
}

impl DiffViewState {
    /// Invalidate the cached diff so it will be recomputed on next render.
    pub fn invalidate_cache(&mut self) {
        self.left_hash = 0;
        self.right_hash = 0;
    }
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
            hunks: Vec::new(),
            current_hunk: 0,
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

/// Compute hunks from diff ops. A hunk is a maximal contiguous run of
/// non-Equal ops. We track the display-row offset so we can scroll to it.
fn compute_hunks(ops: &[DiffOp]) -> Vec<DiffHunk> {
    let mut hunks = Vec::new();
    let mut row = 0usize;
    let mut i = 0;
    while i < ops.len() {
        match &ops[i] {
            DiffOp::Equal(_) => {
                row += 1; // one display row for Equal
                i += 1;
            }
            _ => {
                // Start of a hunk
                let hunk_row_start = row;
                let op_start = i;
                while i < ops.len() && !matches!(&ops[i], DiffOp::Equal(_)) {
                    row += 1; // each Insert or Delete is one display row
                    i += 1;
                }
                hunks.push(DiffHunk {
                    row_start: hunk_row_start,
                    op_start,
                    op_end: i,
                });
            }
        }
    }
    hunks
}

/// Apply a merge: given the original left and right content and a hunk,
/// produce new content for the target side.
fn merge_hunk_left_to_right(
    ops: &[DiffOp],
    hunk: &DiffHunk,
    left_content: &str,
    right_content: &str,
) -> String {
    // Build the merged right content by replaying all ops, but for the hunk
    // range, replace Insert lines with Delete lines (copy left -> right).
    let _ = (left_content, right_content); // content is embedded in the ops
    let mut right_lines: Vec<String> = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        if i >= hunk.op_start && i < hunk.op_end {
            // In hunk: take left-side lines (Delete = left only, Equal = both)
            match op {
                DiffOp::Delete(text) => {
                    right_lines.push(text.clone());
                }
                DiffOp::Insert(_) => {
                    // Skip: we're replacing right with left
                }
                DiffOp::Equal(text) => {
                    right_lines.push(text.clone());
                }
            }
        } else {
            // Outside hunk: keep original right content
            match op {
                DiffOp::Equal(text) | DiffOp::Insert(text) => {
                    right_lines.push(text.clone());
                }
                DiffOp::Delete(_) => {
                    // Not in right side
                }
            }
        }
    }
    if right_lines.is_empty() {
        String::new()
    } else {
        right_lines.join("\n") + "\n"
    }
}

fn merge_hunk_right_to_left(
    ops: &[DiffOp],
    hunk: &DiffHunk,
    _left_content: &str,
    _right_content: &str,
) -> String {
    let mut left_lines: Vec<String> = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        if i >= hunk.op_start && i < hunk.op_end {
            // In hunk: take right-side lines
            match op {
                DiffOp::Insert(text) => {
                    left_lines.push(text.clone());
                }
                DiffOp::Delete(_) => {
                    // Skip: we're replacing left with right
                }
                DiffOp::Equal(text) => {
                    left_lines.push(text.clone());
                }
            }
        } else {
            // Outside hunk: keep original left content
            match op {
                DiffOp::Equal(text) | DiffOp::Delete(text) => {
                    left_lines.push(text.clone());
                }
                DiffOp::Insert(_) => {
                    // Not in left side
                }
            }
        }
    }
    if left_lines.is_empty() {
        String::new()
    } else {
        left_lines.join("\n") + "\n"
    }
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
    /// Index of the hunk this row belongs to, if any.
    hunk_idx: Option<usize>,
}

/// Build left and right pane rows from diff ops, annotating rows with hunk index.
fn build_rows(
    ops: &[DiffOp],
    hunks: &[DiffHunk],
    theme: &EditorTheme,
) -> (Vec<DiffRow>, Vec<DiffRow>) {
    let mut left_rows = Vec::new();
    let mut right_rows = Vec::new();
    let mut left_line = 0usize;
    let mut right_line = 0usize;
    let normal_bg = Color32::TRANSPARENT;
    let normal_fg = theme.foreground;

    for (op_idx, op) in ops.iter().enumerate() {
        // Determine which hunk this op belongs to (if any).
        let hunk_idx = hunks
            .iter()
            .position(|h| op_idx >= h.op_start && op_idx < h.op_end);

        match op {
            DiffOp::Equal(text) => {
                left_line += 1;
                right_line += 1;
                left_rows.push(DiffRow {
                    line_number: Some(left_line),
                    text: text.clone(),
                    bg: normal_bg,
                    fg: normal_fg,
                    hunk_idx: None,
                });
                right_rows.push(DiffRow {
                    line_number: Some(right_line),
                    text: text.clone(),
                    bg: normal_bg,
                    fg: normal_fg,
                    hunk_idx: None,
                });
            }
            DiffOp::Delete(text) => {
                left_line += 1;
                left_rows.push(DiffRow {
                    line_number: Some(left_line),
                    text: text.clone(),
                    bg: delete_bg(theme),
                    fg: delete_fg(theme),
                    hunk_idx,
                });
                // Add a blank filler row on the right side to keep alignment
                right_rows.push(DiffRow {
                    line_number: None,
                    text: String::new(),
                    bg: delete_bg(theme),
                    fg: normal_fg,
                    hunk_idx,
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
                    hunk_idx,
                });
                right_rows.push(DiffRow {
                    line_number: Some(right_line),
                    text: text.clone(),
                    bg: insert_bg(theme),
                    fg: insert_fg(theme),
                    hunk_idx,
                });
            }
        }
    }

    (left_rows, right_rows)
}

/// Navigate to the next diff hunk. Returns the scroll offset in pixels.
pub fn navigate_next_hunk(state: &mut DiffViewState, line_height: f32) {
    if state.hunks.is_empty() {
        return;
    }
    if state.current_hunk + 1 < state.hunks.len() {
        state.current_hunk += 1;
    } else {
        state.current_hunk = 0; // wrap around
    }
    let hunk = &state.hunks[state.current_hunk];
    state.scroll_offset = (hunk.row_start as f32 * line_height).max(0.0);
}

/// Navigate to the previous diff hunk.
pub fn navigate_prev_hunk(state: &mut DiffViewState, line_height: f32) {
    if state.hunks.is_empty() {
        return;
    }
    if state.current_hunk > 0 {
        state.current_hunk -= 1;
    } else {
        state.current_hunk = state.hunks.len() - 1; // wrap around
    }
    let hunk = &state.hunks[state.current_hunk];
    state.scroll_offset = (hunk.row_start as f32 * line_height).max(0.0);
}

/// Render the diff view showing two documents side-by-side with highlighted differences.
/// Returns a `DiffAction` if the user clicked a merge button.
pub fn render_diff_view(
    ui: &mut egui::Ui,
    state: &mut DiffViewState,
    left_content: &str,
    right_content: &str,
    left_name: &str,
    right_name: &str,
    theme: &EditorTheme,
    font_size: f32,
) -> DiffAction {
    let mut action = DiffAction::None;

    // Recompute diff if content has changed
    let lh = simple_hash(left_content);
    let rh = simple_hash(right_content);
    if lh != state.left_hash || rh != state.right_hash {
        state.diff_ops = diff_lines(left_content, right_content);
        state.left_hash = lh;
        state.right_hash = rh;
        state.hunks = compute_hunks(&state.diff_ops);
        state.current_hunk = 0;
    }

    let line_height = line_height_for_font(font_size);
    let char_width = char_width_for_font(font_size);
    let font_id = egui::FontId::monospace(font_size);

    let (left_rows, right_rows) = build_rows(&state.diff_ops, &state.hunks, theme);
    let total_rows = left_rows.len();

    let available = ui.available_rect_before_wrap();

    // Toolbar height for nav buttons
    let toolbar_height = line_height + 8.0;
    let toolbar_rect = Rect::from_min_size(
        available.left_top(),
        Vec2::new(available.width(), toolbar_height),
    );

    // Draw toolbar background
    ui.painter().rect_filled(toolbar_rect, 0.0, theme.gutter_bg);

    // Navigation buttons in toolbar
    let btn_font = egui::FontId::monospace(font_size * 0.85);
    let hunk_count = state.hunks.len();
    let nav_label = if hunk_count == 0 {
        "No differences".to_string()
    } else {
        format!("Diff {}/{} ", state.current_hunk + 1, hunk_count)
    };

    // Draw nav label
    let nav_text_pos = Pos2::new(toolbar_rect.left() + 8.0, toolbar_rect.center().y);
    let nav_galley =
        ui.painter()
            .layout_no_wrap(nav_label.clone(), btn_font.clone(), theme.foreground);
    let nav_text_width = nav_galley.size().x;
    ui.painter()
        .galley(nav_text_pos, nav_galley, theme.foreground);

    // Prev button
    let prev_btn_rect = Rect::from_min_size(
        Pos2::new(
            toolbar_rect.left() + 8.0 + nav_text_width + 8.0,
            toolbar_rect.top() + 2.0,
        ),
        Vec2::new(char_width * 6.0, toolbar_height - 4.0),
    );
    let prev_response = ui.allocate_rect(prev_btn_rect, egui::Sense::click());
    let prev_bg = if prev_response.hovered() {
        Color32::from_rgba_premultiplied(80, 80, 80, 60)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(prev_btn_rect, 3.0, prev_bg);
    ui.painter().text(
        prev_btn_rect.center(),
        egui::Align2::CENTER_CENTER,
        "Prev",
        btn_font.clone(),
        theme.foreground,
    );
    if prev_response.clicked() && hunk_count > 0 {
        navigate_prev_hunk(state, line_height);
    }

    // Next button
    let next_btn_rect = Rect::from_min_size(
        Pos2::new(prev_btn_rect.right() + 4.0, toolbar_rect.top() + 2.0),
        Vec2::new(char_width * 6.0, toolbar_height - 4.0),
    );
    let next_response = ui.allocate_rect(next_btn_rect, egui::Sense::click());
    let next_bg = if next_response.hovered() {
        Color32::from_rgba_premultiplied(80, 80, 80, 60)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(next_btn_rect, 3.0, next_bg);
    ui.painter().text(
        next_btn_rect.center(),
        egui::Align2::CENTER_CENTER,
        "Next",
        btn_font.clone(),
        theme.foreground,
    );
    if next_response.clicked() && hunk_count > 0 {
        navigate_next_hunk(state, line_height);
    }

    // Header bar with file names (below toolbar)
    let header_height = line_height + 4.0;
    let header_rect = Rect::from_min_size(
        Pos2::new(available.left(), available.top() + toolbar_height),
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
        Pos2::new(
            available.left(),
            available.top() + toolbar_height + header_height,
        ),
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

    // Merge button width
    let merge_btn_w = char_width * 3.0;

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

    // Track which hunks we've already drawn merge buttons for, to draw
    // them only once per hunk (on the first visible row of the hunk).
    let mut hunks_with_buttons: HashSet<usize> = HashSet::new();

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

        // Merge buttons: show on the first visible row of each hunk
        let hunk_idx = left_rows
            .get(row_idx)
            .and_then(|r| r.hunk_idx)
            .or_else(|| right_rows.get(row_idx).and_then(|r| r.hunk_idx));

        if let Some(hi) = hunk_idx {
            if !hunks_with_buttons.contains(&hi) {
                hunks_with_buttons.insert(hi);

                // ">" button on the left pane (copy left -> right)
                let l2r_rect = Rect::from_min_size(
                    Pos2::new(separator_x - merge_btn_w - 2.0, y),
                    Vec2::new(merge_btn_w, line_height),
                );
                let l2r_resp = ui.allocate_rect(l2r_rect, egui::Sense::click());
                let l2r_bg = if l2r_resp.hovered() {
                    Color32::from_rgba_premultiplied(60, 120, 60, 120)
                } else {
                    Color32::from_rgba_premultiplied(60, 100, 60, 80)
                };
                ui.painter().rect_filled(l2r_rect, 2.0, l2r_bg);
                ui.painter().text(
                    l2r_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    ">",
                    btn_font.clone(),
                    Color32::WHITE,
                );
                if l2r_resp.clicked() {
                    if let Some(hunk) = state.hunks.get(hi) {
                        let new_right = merge_hunk_left_to_right(
                            &state.diff_ops,
                            hunk,
                            left_content,
                            right_content,
                        );
                        action = DiffAction::MergeLeftToRight(new_right);
                    }
                }

                // "<" button on the right pane (copy right -> left)
                let r2l_rect = Rect::from_min_size(
                    Pos2::new(separator_x + 2.0, y),
                    Vec2::new(merge_btn_w, line_height),
                );
                let r2l_resp = ui.allocate_rect(r2l_rect, egui::Sense::click());
                let r2l_bg = if r2l_resp.hovered() {
                    Color32::from_rgba_premultiplied(60, 120, 60, 120)
                } else {
                    Color32::from_rgba_premultiplied(60, 100, 60, 80)
                };
                ui.painter().rect_filled(r2l_rect, 2.0, r2l_bg);
                ui.painter().text(
                    r2l_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "<",
                    btn_font.clone(),
                    Color32::WHITE,
                );
                if r2l_resp.clicked() {
                    if let Some(hunk) = state.hunks.get(hi) {
                        let new_left = merge_hunk_right_to_left(
                            &state.diff_ops,
                            hunk,
                            left_content,
                            right_content,
                        );
                        action = DiffAction::MergeRightToLeft(new_left);
                    }
                }
            }
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
        "+{} additions  -{} deletions  {} unchanged  |  F7: Next  Shift+F7: Prev",
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

    action
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hunks_empty() {
        let ops: Vec<DiffOp> = vec![];
        let hunks = compute_hunks(&ops);
        assert!(hunks.is_empty());
    }

    #[test]
    fn test_compute_hunks_no_diffs() {
        let ops = vec![DiffOp::Equal("a".into()), DiffOp::Equal("b".into())];
        let hunks = compute_hunks(&ops);
        assert!(hunks.is_empty());
    }

    #[test]
    fn test_compute_hunks_single_hunk() {
        let ops = vec![
            DiffOp::Equal("a".into()),
            DiffOp::Delete("b".into()),
            DiffOp::Insert("c".into()),
            DiffOp::Equal("d".into()),
        ];
        let hunks = compute_hunks(&ops);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].op_start, 1);
        assert_eq!(hunks[0].op_end, 3);
        assert_eq!(hunks[0].row_start, 1); // after one Equal row
    }

    #[test]
    fn test_compute_hunks_multiple_hunks() {
        let ops = vec![
            DiffOp::Delete("a".into()),
            DiffOp::Equal("b".into()),
            DiffOp::Insert("c".into()),
            DiffOp::Delete("d".into()),
            DiffOp::Equal("e".into()),
        ];
        let hunks = compute_hunks(&ops);
        assert_eq!(hunks.len(), 2);
        assert_eq!(hunks[0].op_start, 0);
        assert_eq!(hunks[0].op_end, 1);
        assert_eq!(hunks[1].op_start, 2);
        assert_eq!(hunks[1].op_end, 4);
    }

    #[test]
    fn test_merge_left_to_right() {
        let ops = vec![
            DiffOp::Equal("a".into()),
            DiffOp::Delete("b".into()),
            DiffOp::Insert("c".into()),
            DiffOp::Equal("d".into()),
        ];
        let hunks = compute_hunks(&ops);
        let result = merge_hunk_left_to_right(&ops, &hunks[0], "a\nb\nd\n", "a\nc\nd\n");
        // After merging left->right for the hunk, the right side should get "b" instead of "c"
        assert_eq!(result, "a\nb\nd\n");
    }

    #[test]
    fn test_merge_right_to_left() {
        let ops = vec![
            DiffOp::Equal("a".into()),
            DiffOp::Delete("b".into()),
            DiffOp::Insert("c".into()),
            DiffOp::Equal("d".into()),
        ];
        let hunks = compute_hunks(&ops);
        let result = merge_hunk_right_to_left(&ops, &hunks[0], "a\nb\nd\n", "a\nc\nd\n");
        // After merging right->left, the left side should get "c" instead of "b"
        assert_eq!(result, "a\nc\nd\n");
    }

    #[test]
    fn test_merge_deletion_left_to_right() {
        // Left has line, right doesn't
        let ops = vec![
            DiffOp::Equal("a".into()),
            DiffOp::Delete("x".into()),
            DiffOp::Equal("b".into()),
        ];
        let hunks = compute_hunks(&ops);
        let result = merge_hunk_left_to_right(&ops, &hunks[0], "a\nx\nb\n", "a\nb\n");
        // After merge, right should gain "x"
        assert_eq!(result, "a\nx\nb\n");
    }

    #[test]
    fn test_merge_insertion_right_to_left() {
        // Right has extra line, left doesn't
        let ops = vec![
            DiffOp::Equal("a".into()),
            DiffOp::Insert("y".into()),
            DiffOp::Equal("b".into()),
        ];
        let hunks = compute_hunks(&ops);
        let result = merge_hunk_right_to_left(&ops, &hunks[0], "a\nb\n", "a\ny\nb\n");
        // After merge, left should gain "y"
        assert_eq!(result, "a\ny\nb\n");
    }

    #[test]
    fn test_navigate_next_hunk() {
        let mut state = DiffViewState::default();
        state.hunks = vec![
            DiffHunk {
                row_start: 0,
                op_start: 0,
                op_end: 1,
            },
            DiffHunk {
                row_start: 5,
                op_start: 3,
                op_end: 5,
            },
        ];
        state.current_hunk = 0;
        navigate_next_hunk(&mut state, 20.0);
        assert_eq!(state.current_hunk, 1);
        assert_eq!(state.scroll_offset, 100.0); // 5 * 20.0
    }

    #[test]
    fn test_navigate_prev_hunk_wraps() {
        let mut state = DiffViewState::default();
        state.hunks = vec![
            DiffHunk {
                row_start: 0,
                op_start: 0,
                op_end: 1,
            },
            DiffHunk {
                row_start: 10,
                op_start: 3,
                op_end: 5,
            },
        ];
        state.current_hunk = 0;
        navigate_prev_hunk(&mut state, 20.0);
        assert_eq!(state.current_hunk, 1); // wrapped to last
        assert_eq!(state.scroll_offset, 200.0); // 10 * 20.0
    }

    #[test]
    fn test_navigate_next_hunk_wraps() {
        let mut state = DiffViewState::default();
        state.hunks = vec![
            DiffHunk {
                row_start: 0,
                op_start: 0,
                op_end: 1,
            },
            DiffHunk {
                row_start: 5,
                op_start: 3,
                op_end: 5,
            },
        ];
        state.current_hunk = 1;
        navigate_next_hunk(&mut state, 20.0);
        assert_eq!(state.current_hunk, 0); // wrapped to first
        assert_eq!(state.scroll_offset, 0.0);
    }

    #[test]
    fn test_navigate_empty_hunks() {
        let mut state = DiffViewState::default();
        state.hunks = vec![];
        navigate_next_hunk(&mut state, 20.0);
        assert_eq!(state.current_hunk, 0);
        navigate_prev_hunk(&mut state, 20.0);
        assert_eq!(state.current_hunk, 0);
    }
}
