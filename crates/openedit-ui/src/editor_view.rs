use crate::bracket_colors;
use crate::git::{self, LineDiffStatus};
use crate::gutter;
use crate::lsp::{self, LspDiagnostic};
use crate::macro_recorder::{MacroAction, MacroRecorder};
use crate::snippets::SnippetEngine;
use crate::theme::EditorTheme;
use crate::vim::{VimMode, VimState};
use egui::{self, Pos2, Rect, Ui, Vec2};
use openedit_core::cursor::Position;
use openedit_core::syntax::{HighlightSpan, SyntaxEngine};
use openedit_core::Document;

/// A visual row in the editor, mapping a screen row to a logical line + column offset.
/// When word wrap is off, each line has one VisualRow with col_offset=0.
/// When word wrap is on, long lines produce multiple VisualRows.
struct VisualRow {
    line_idx: usize,
    col_offset: usize,
    is_first: bool,
}

/// Compute line height from font size.
pub fn line_height_for_font(font_size: f32) -> f32 {
    (font_size * 1.385).round() // ~18 at size 13
}

/// Compute monospace character width from font size.
pub fn char_width_for_font(font_size: f32) -> f32 {
    font_size * 0.646 // ~8.4 at size 13
}

/// Persistent state for the editor view (e.g., drag tracking).
#[derive(Default)]
pub struct EditorViewState {
    pub dragging: bool,
    /// Starting document position for Alt+drag block/column selection.
    pub block_select_start: Option<Position>,
    /// Whether we are currently performing a block selection drag.
    pub block_selecting: bool,
    /// Set when user Ctrl+clicks on a position (go-to-definition).
    pub ctrl_click_pos: Option<Position>,
    /// Set when user Ctrl+hovers over text (request hover info).
    /// (document position, screen position for tooltip placement)
    pub hover_request: Option<(Position, Pos2)>,
    /// Set when user hovers over a diagnostic squiggle — contains the message to display.
    pub diagnostic_hover: Option<(String, Pos2)>,
}

/// Extra rendering context passed from app to editor view.
pub struct EditorRenderContext<'a> {
    /// Line-level git diff statuses for the current file.
    pub git_line_diffs: &'a [(usize, LineDiffStatus)],
    /// Git blame info (line -> author string).
    pub git_blame_info: &'a std::collections::HashMap<usize, String>,
    /// Whether to show git blame.
    pub show_blame: bool,
    /// LSP diagnostics for the current file.
    pub lsp_diagnostics: &'a [LspDiagnostic],
    /// Whether bracket colorization is enabled.
    pub bracket_colorization: bool,
}

/// Render the editor viewport for a document.
/// Returns true if the document was modified by user input.
pub fn render_editor(
    ui: &mut Ui,
    doc: &mut Document,
    theme: &EditorTheme,
    _show_search: bool,
    view_state: &mut EditorViewState,
    syntax_engine: &mut SyntaxEngine,
    font_size: f32,
    show_whitespace: bool,
    show_minimap: bool,
    autocomplete: &mut crate::autocomplete::AutocompleteState,
    word_wrap: bool,
    macro_rec: &mut MacroRecorder,
    render_ctx: Option<&EditorRenderContext<'_>>,
    snippet_engine: &mut SnippetEngine,
    vim_state: Option<&mut VimState>,
) -> bool {
    let line_height = line_height_for_font(font_size);
    let char_width = char_width_for_font(font_size);
    let rect = ui.available_rect_before_wrap();

    // Background
    ui.painter().rect_filled(rect, 0.0, theme.background);

    let total_lines = doc.buffer.len_lines();
    let current_line = doc.cursors.primary().position.line;

    // Update fold ranges from buffer content
    doc.update_fold_ranges();

    // Build the full list of visible (not-hidden) line indices
    let all_visible_lines: Vec<usize> = (0..total_lines)
        .filter(|&l| !doc.folding.is_line_hidden(l))
        .collect();

    // Preliminary gutter width calculation (needed for wrap column count)
    let digit_count = format!("{}", total_lines).len().max(3);
    let fold_col_width = char_width * 1.5;
    let gutter_width_est = (digit_count as f32 + 2.0) * char_width + fold_col_width + 8.0;

    // Word wrap: compute how many columns fit in the text area
    let text_area_width = rect.width() - gutter_width_est - 8.0; // 4px padding each side
    let wrap_cols = if word_wrap {
        (text_area_width / char_width).floor().max(10.0) as usize
    } else {
        usize::MAX // effectively no wrapping
    };

    // Build all visual rows for visible (non-hidden) lines
    let mut all_visual_rows: Vec<VisualRow> = Vec::new();
    for &line_idx in &all_visible_lines {
        if word_wrap {
            let line = doc.buffer.line(line_idx);
            let line_str = line.to_string();
            let display = line_str.trim_end_matches(&['\n', '\r'][..]);
            let char_count = display.chars().count();
            if char_count == 0 {
                all_visual_rows.push(VisualRow {
                    line_idx,
                    col_offset: 0,
                    is_first: true,
                });
            } else {
                let mut offset = 0;
                let mut first = true;
                while offset < char_count {
                    all_visual_rows.push(VisualRow {
                        line_idx,
                        col_offset: offset,
                        is_first: first,
                    });
                    offset += wrap_cols;
                    first = false;
                }
            }
        } else {
            all_visual_rows.push(VisualRow {
                line_idx,
                col_offset: 0,
                is_first: true,
            });
        }
    }
    let total_visual_rows = all_visual_rows.len();

    // Calculate visible line range based on visual row count
    let visible_lines = (rect.height() / line_height).ceil() as usize;
    let max_scroll = total_visual_rows.saturating_sub(visible_lines);
    doc.scroll_line = doc.scroll_line.min(max_scroll);
    let first_visible_row = doc.scroll_line;

    // Build displayed visual rows on screen
    let displayed_vrows: Vec<(usize, &VisualRow)> = all_visual_rows
        .iter()
        .skip(first_visible_row)
        .take(visible_lines + 1)
        .enumerate()
        .collect();

    // Build displayed_lines for gutter: only first visual rows of each line
    let displayed_lines: Vec<(usize, usize)> = displayed_vrows
        .iter()
        .filter(|(_, vr)| vr.is_first)
        .map(|&(screen_row, vr)| (screen_row, vr.line_idx))
        .collect();

    // Helper: map (line, col) to screen row (accounting for wrapping)
    let pos_to_screen_row = |line: usize, col: usize| -> Option<usize> {
        displayed_vrows
            .iter()
            .find(|(_, vr)| {
                if vr.line_idx != line {
                    return false;
                }
                if !word_wrap {
                    return true;
                }
                let next_offset = vr.col_offset + wrap_cols;
                col >= vr.col_offset && col < next_offset
            })
            .map(|&(r, _)| r)
    };

    // Helper: map line to first screen row
    let _line_to_screen_row = |line: usize| -> Option<usize> {
        displayed_vrows
            .iter()
            .find(|(_, vr)| vr.line_idx == line && vr.is_first)
            .map(|&(r, _)| r)
    };

    // Gutter (with fold indicators)
    let gutter_width = gutter::render_gutter(
        ui,
        total_lines,
        &displayed_lines,
        current_line,
        line_height,
        rect.top(),
        theme,
        font_size,
        &doc.folding,
        &doc.bookmarks,
    );

    let text_left = rect.left() + gutter_width;

    // Current line highlight (highlight all visual rows of the current line)
    for &(screen_row, vr) in &displayed_vrows {
        if vr.line_idx == current_line {
            let y = rect.top() + screen_row as f32 * line_height;
            ui.painter().rect_filled(
                Rect::from_min_size(
                    Pos2::new(text_left, y),
                    Vec2::new(rect.width() - gutter_width, line_height),
                ),
                0.0,
                theme.current_line_bg,
            );
        }
    }

    // Render text and selections
    let font_id = egui::FontId::monospace(font_size);
    let cursor_pos = doc.cursors.primary().position;

    // Scroll col is disabled in word wrap mode
    let scroll_col = if word_wrap { 0 } else { doc.scroll_col };

    // Highlight current line (only when no selection active)
    if doc.cursors.primary().anchor.is_none() {
        for &(screen_row, vr) in &displayed_vrows {
            if vr.line_idx == cursor_pos.line {
                let y = rect.top() + screen_row as f32 * line_height;
                let line_rect = Rect::from_min_size(
                    Pos2::new(rect.left(), y),
                    Vec2::new(rect.width(), line_height),
                );
                ui.painter()
                    .rect_filled(line_rect, 0.0, theme.current_line_bg);
            }
        }
    }

    // Render search matches
    if !doc.search.matches.is_empty() {
        for (i, m) in doc.search.matches.iter().enumerate() {
            let start_pos = doc.buffer.char_to_line_col(m.start);
            let end_pos = doc.buffer.char_to_line_col(m.end);

            // Simple single-line match rendering
            if start_pos.0 == end_pos.0 {
                let bg = if doc.search.current_match == Some(i) {
                    theme.search_current_match_bg
                } else {
                    theme.search_match_bg
                };
                // Find the visual row(s) containing this match
                for &(screen_row, vr) in &displayed_vrows {
                    if vr.line_idx != start_pos.0 {
                        continue;
                    }
                    let vr_end = if word_wrap {
                        vr.col_offset + wrap_cols
                    } else {
                        usize::MAX
                    };
                    // Check if any part of the match is in this visual row
                    if start_pos.1 < vr_end && end_pos.1 > vr.col_offset {
                        let y = rect.top() + screen_row as f32 * line_height;
                        let s_col = start_pos.1.max(vr.col_offset) - vr.col_offset;
                        let e_col = end_pos.1.min(vr_end) - vr.col_offset;
                        let x1 = text_left + 4.0 + s_col as f32 * char_width;
                        let x2 = text_left + 4.0 + e_col as f32 * char_width;
                        let match_rect = Rect::from_min_max(
                            Pos2::new(x1.max(text_left), y),
                            Pos2::new(x2.min(rect.right()), y + line_height),
                        );
                        ui.painter().rect_filled(match_rect, 0.0, bg);
                    }
                }
            }
        }
    }

    // Render selections for all cursors
    for cursor in doc.cursors.cursors() {
        if let Some((sel_start, sel_end)) = cursor.selection_range() {
            render_selection_wrapped(
                ui,
                &sel_start,
                &sel_end,
                &displayed_vrows,
                text_left,
                rect,
                doc,
                theme,
                line_height,
                char_width,
                word_wrap,
                wrap_cols,
            );
        }
    }

    // Compute syntax highlights (per-line spans with char-based columns)
    let line_highlights: Vec<Vec<HighlightSpan>> = if let Some(ref lang_name) = doc.language {
        if let Some(lang_key) = SyntaxEngine::language_key(lang_name) {
            let source = doc.buffer.to_string();
            syntax_engine.highlight_lines(&source, lang_key)
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Render text lines (with syntax highlighting), only displayed visual rows
    let ws_color = egui::Color32::from_rgba_premultiplied(
        theme.gutter_fg.r(),
        theme.gutter_fg.g(),
        theme.gutter_fg.b(),
        140,
    );
    let fold_ellipsis_color = theme.gutter_fg;

    for &(screen_row, vr) in &displayed_vrows {
        let y = rect.top() + screen_row as f32 * line_height;
        let line = doc.buffer.line(vr.line_idx);
        let line_str = line.to_string();
        let display = line_str.trim_end_matches(&['\n', '\r'][..]);

        // In word wrap mode, render only the chars for this visual row
        let (visible_text_start, visible_text_end) = if word_wrap {
            let char_count = display.chars().count();
            (vr.col_offset, (vr.col_offset + wrap_cols).min(char_count))
        } else {
            let char_count = display.chars().count();
            (scroll_col, char_count)
        };

        let spans = line_highlights.get(vr.line_idx);
        if spans.is_none_or(|s| s.is_empty()) {
            // No syntax highlighting -- render plain text
            let visible_text: String = display
                .chars()
                .skip(visible_text_start)
                .take(visible_text_end - visible_text_start)
                .collect();
            ui.painter().text(
                Pos2::new(text_left + 4.0, y),
                egui::Align2::LEFT_TOP,
                &visible_text,
                font_id.clone(),
                theme.foreground,
            );
        } else {
            render_highlighted_line(
                ui,
                display,
                visible_text_start,
                spans.unwrap(),
                text_left,
                y,
                &font_id,
                theme,
                char_width,
                if word_wrap { Some(wrap_cols) } else { None },
            );
        }

        // Fold ellipsis only on first visual row
        if vr.is_first && doc.folding.is_folded(vr.line_idx) {
            let display_len = display.chars().count();
            let ellipsis_col = display_len.saturating_sub(visible_text_start);
            let ellipsis_x = text_left + 4.0 + ellipsis_col as f32 * char_width + char_width;
            ui.painter().text(
                Pos2::new(ellipsis_x, y),
                egui::Align2::LEFT_TOP,
                " ...",
                font_id.clone(),
                fold_ellipsis_color,
            );
        }

        // Whitespace visualization overlay
        if show_whitespace {
            for (i, ch) in display.chars().enumerate() {
                if i < visible_text_start || i >= visible_text_end {
                    continue;
                }
                let symbol = match ch {
                    ' ' => "\u{00B7}",  // middle dot
                    '\t' => "\u{2192}", // rightwards arrow
                    _ => continue,
                };
                let x = text_left + 4.0 + (i - visible_text_start) as f32 * char_width;
                ui.painter().text(
                    Pos2::new(x, y),
                    egui::Align2::LEFT_TOP,
                    symbol,
                    font_id.clone(),
                    ws_color,
                );
            }
        }
    }

    // Rainbow indent guide colors
    const INDENT_COLORS: [egui::Color32; 6] = [
        egui::Color32::from_rgba_premultiplied(255, 215, 0, 40), // Gold
        egui::Color32::from_rgba_premultiplied(218, 112, 214, 40), // Orchid
        egui::Color32::from_rgba_premultiplied(0, 191, 255, 40), // Sky blue
        egui::Color32::from_rgba_premultiplied(255, 165, 0, 40), // Orange
        egui::Color32::from_rgba_premultiplied(50, 205, 50, 40), // Lime
        egui::Color32::from_rgba_premultiplied(255, 105, 180, 40), // Pink
    ];
    let indent_guide_line_color = egui::Color32::from_rgba_premultiplied(
        theme.gutter_fg.r(),
        theme.gutter_fg.g(),
        theme.gutter_fg.b(),
        60,
    );
    let tab_size = 4usize;
    for &(screen_row, vr) in &displayed_vrows {
        if !vr.is_first {
            continue;
        }
        let line = doc.buffer.line(vr.line_idx);
        let line_str = line.to_string();
        let display = line_str.trim_end_matches(&['\n', '\r'][..]);
        let indent_chars = display.chars().take_while(|c| *c == ' ').count();
        let indent_levels = indent_chars / tab_size;
        let y = rect.top() + screen_row as f32 * line_height;
        for level in 1..=indent_levels {
            let col = level * tab_size;
            let x = text_left + 4.0 + (col as f32 - scroll_col as f32) * char_width;
            if x > text_left && x < rect.right() {
                // Rainbow indent background band
                let band_x = text_left + 4.0 + ((level - 1) * tab_size) as f32 * char_width;
                let band_w = tab_size as f32 * char_width;
                let band_color = INDENT_COLORS[(level - 1) % INDENT_COLORS.len()];
                ui.painter().rect_filled(
                    Rect::from_min_size(Pos2::new(band_x, y), Vec2::new(band_w, line_height)),
                    0.0,
                    band_color,
                );
                // Vertical guide line
                ui.painter().line_segment(
                    [Pos2::new(x, y), Pos2::new(x, y + line_height)],
                    egui::Stroke::new(1.0, indent_guide_line_color),
                );
            }
        }
    }

    // Git gutter marks
    if let Some(ctx) = render_ctx {
        if !ctx.git_line_diffs.is_empty() {
            let gutter_right_x = rect.left() + gutter_width_est;
            for &(screen_row, vr) in &displayed_vrows {
                if !vr.is_first {
                    continue;
                }
                let y = rect.top() + screen_row as f32 * line_height;
                git::render_git_gutter_mark(
                    ui,
                    ctx.git_line_diffs,
                    vr.line_idx,
                    gutter_right_x,
                    y,
                    line_height,
                );
            }
        }

        // Git blame annotations (right of line text)
        if ctx.show_blame && !ctx.git_blame_info.is_empty() {
            let blame_font = egui::FontId::monospace(font_size * 0.85);
            let blame_color = egui::Color32::from_rgba_premultiplied(120, 120, 120, 160);
            for &(screen_row, vr) in &displayed_vrows {
                if !vr.is_first {
                    continue;
                }
                if let Some(author) = ctx.git_blame_info.get(&vr.line_idx) {
                    let y = rect.top() + screen_row as f32 * line_height;
                    // Place blame text far right
                    let blame_x = rect.right() - 200.0;
                    if blame_x > text_left + 100.0 {
                        let truncated: String = author.chars().take(20).collect();
                        ui.painter().text(
                            Pos2::new(blame_x, y),
                            egui::Align2::LEFT_TOP,
                            &truncated,
                            blame_font.clone(),
                            blame_color,
                        );
                    }
                }
            }
        }

        // LSP diagnostic squiggles
        if !ctx.lsp_diagnostics.is_empty() {
            for &(screen_row, vr) in &displayed_vrows {
                if !vr.is_first {
                    continue;
                }
                let y = rect.top() + screen_row as f32 * line_height;
                lsp::render_diagnostic_squiggles(
                    ui,
                    ctx.lsp_diagnostics,
                    vr.line_idx,
                    text_left,
                    y,
                    line_height,
                    char_width,
                    scroll_col,
                );
            }
        }
    }

    // Bracket pair colorization
    let do_bracket_colors = render_ctx.is_some_and(|c| c.bracket_colorization);
    if do_bracket_colors {
        // Collect all lines for bracket depth calculation
        let all_line_strs: Vec<String> = (0..total_lines)
            .map(|i| {
                let l = doc.buffer.line(i).to_string();
                l.trim_end_matches(&['\n', '\r'][..]).to_string()
            })
            .collect();
        let all_refs: Vec<&str> = all_line_strs.iter().map(|s| s.as_str()).collect();
        let first_vis = displayed_vrows
            .first()
            .map(|&(_, vr)| vr.line_idx)
            .unwrap_or(0);
        let last_vis = displayed_vrows
            .last()
            .map(|&(_, vr)| vr.line_idx + 1)
            .unwrap_or(0);
        let colored = bracket_colors::colorize_brackets(&all_refs, first_vis, last_vis);
        for (line_idx, brackets) in &colored {
            // Find the screen row for this line
            if let Some(&(screen_row, _vr)) = displayed_vrows
                .iter()
                .find(|(_, vr)| vr.line_idx == *line_idx && vr.is_first)
            {
                let y = rect.top() + screen_row as f32 * line_height;
                let line_str = &all_line_strs[*line_idx];
                bracket_colors::render_bracket_colors(
                    ui, brackets, text_left, y, char_width, &font_id, scroll_col, line_str,
                );
            }
        }
    }

    // Bracket matching highlight
    if let Some(match_pos) = find_matching_bracket(doc, &cursor_pos) {
        let bracket_color = egui::Color32::from_rgba_premultiplied(100, 100, 100, 80);
        for bp in [&cursor_pos, &match_pos] {
            if let Some(screen_row) = pos_to_screen_row(bp.line, bp.col) {
                let y = rect.top() + screen_row as f32 * line_height;
                let col_in_row = if word_wrap {
                    let vr = &displayed_vrows[screen_row].1;
                    bp.col - vr.col_offset
                } else {
                    bp.col - scroll_col
                };
                let x = text_left + 4.0 + col_in_row as f32 * char_width;
                let bracket_rect =
                    Rect::from_min_size(Pos2::new(x, y), Vec2::new(char_width, line_height));
                ui.painter().rect_filled(bracket_rect, 2.0, bracket_color);
            }
        }
    }

    // Snippet placeholder highlighting
    if snippet_engine.is_active() {
        let placeholder_positions = snippet_engine.state.placeholder_positions();
        for (ph_pos, ph_len, is_current) in &placeholder_positions {
            if let Some(screen_row) = pos_to_screen_row(ph_pos.line, ph_pos.col) {
                let y = rect.top() + screen_row as f32 * line_height;
                let col_in_row = if word_wrap {
                    let vr = &displayed_vrows[screen_row].1;
                    ph_pos.col.saturating_sub(vr.col_offset)
                } else {
                    ph_pos.col.saturating_sub(scroll_col)
                };
                let x = text_left + 4.0 + col_in_row as f32 * char_width;
                let width = if *ph_len > 0 {
                    *ph_len as f32 * char_width
                } else {
                    char_width * 0.5 // thin marker for zero-length placeholders
                };
                if *is_current {
                    // Active placeholder: distinct teal/blue background
                    let active_bg = egui::Color32::from_rgba_premultiplied(0, 180, 200, 60);
                    let ph_rect =
                        Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, line_height));
                    ui.painter().rect_filled(ph_rect, 2.0, active_bg);
                    // Border for visibility
                    ui.painter().rect_stroke(
                        ph_rect,
                        2.0,
                        egui::Stroke::new(
                            1.0,
                            egui::Color32::from_rgba_premultiplied(0, 200, 220, 120),
                        ),
                    );
                } else {
                    // Inactive placeholders: subtle underline
                    let inactive_color = egui::Color32::from_rgba_premultiplied(150, 150, 150, 80);
                    let underline_y = y + line_height - 1.0;
                    ui.painter().line_segment(
                        [Pos2::new(x, underline_y), Pos2::new(x + width, underline_y)],
                        egui::Stroke::new(1.5, inactive_color),
                    );
                }
            }
        }
    }

    // Minimap
    let first_actual = displayed_vrows
        .first()
        .map(|&(_, vr)| vr.line_idx)
        .unwrap_or(0);
    let last_actual = displayed_vrows
        .last()
        .map(|&(_, vr)| vr.line_idx + 1)
        .unwrap_or(total_lines)
        .min(total_lines);
    let mut minimap_scroll_target = None;
    if show_minimap {
        let (_, scroll_target) = crate::minimap::render_minimap_with_scroll(
            ui,
            doc,
            theme,
            rect,
            first_actual,
            last_actual,
            visible_lines,
            font_size,
        );
        minimap_scroll_target = scroll_target;
    }

    if let Some(target) = minimap_scroll_target {
        doc.scroll_line = target.min(max_scroll);
    }

    // Render cursors (all cursors, not just primary)
    for cursor in doc.cursors.cursors() {
        let pos = cursor.position;
        if let Some(screen_row) = pos_to_screen_row(pos.line, pos.col) {
            let y = rect.top() + screen_row as f32 * line_height;
            let col_in_row = if word_wrap {
                let vr = &displayed_vrows[screen_row].1;
                pos.col - vr.col_offset
            } else {
                pos.col - scroll_col
            };
            let x = text_left + 4.0 + col_in_row as f32 * char_width;
            let cursor_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(2.0, line_height));
            ui.painter()
                .rect_filled(cursor_rect, 0.0, theme.cursor_color);
        }
    }

    // Handle input
    let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

    // Text cursor icon when hovering over the editor area
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
    }

    // Mouse position to document position helper (accounts for folding + wrapping)
    let mouse_to_doc_pos = |pos: Pos2| -> Position {
        let screen_row = ((pos.y - rect.top()) / line_height) as usize;
        // Find the visual row for this screen row
        let (line_idx, col_offset) = displayed_vrows
            .iter()
            .find(|(r, _)| *r == screen_row)
            .map(|&(_, vr)| (vr.line_idx, vr.col_offset))
            .unwrap_or_else(|| {
                displayed_vrows
                    .last()
                    .map(|&(_, vr)| (vr.line_idx, vr.col_offset))
                    .unwrap_or((0, 0))
            });
        let raw_col = (((pos.x - text_left - 4.0) / char_width).round() as isize).max(0) as usize;
        let col = col_offset + raw_col + (if word_wrap { 0 } else { scroll_col });
        let line = line_idx.min(total_lines.saturating_sub(1));
        let col = col.min(doc.buffer.line_len_chars_no_newline(line));
        Position::new(line, col)
    };

    // Clear per-frame event fields
    view_state.ctrl_click_pos = None;
    view_state.hover_request = None;
    view_state.diagnostic_hover = None;

    // Mouse click to position cursor (with Alt+drag block selection, Ctrl+click go-to-definition)
    if response.drag_started() {
        if let Some(pos) = response.interact_pointer_pos() {
            if pos.x >= text_left {
                let doc_pos = mouse_to_doc_pos(pos);
                let alt = ui.input(|i| i.modifiers.alt);
                let shift = ui.input(|i| i.modifiers.shift);
                let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.mac_cmd);
                if ctrl && !shift && !alt {
                    // Ctrl+Click: go to definition
                    view_state.ctrl_click_pos = Some(doc_pos);
                    doc.cursors.primary_mut().move_to(doc_pos, false);
                    view_state.dragging = false;
                } else if alt {
                    // Start block/column selection
                    view_state.block_selecting = true;
                    view_state.block_select_start = Some(doc_pos);
                    doc.cursors.clear_extra_cursors();
                    doc.cursors.primary_mut().move_to(doc_pos, false);
                    view_state.dragging = true;
                } else {
                    // Normal drag
                    view_state.block_selecting = false;
                    view_state.block_select_start = None;
                    doc.cursors.primary_mut().move_to(doc_pos, shift);
                    view_state.dragging = true;
                }
            }
        }
    }

    // Mouse drag to extend selection (or block selection)
    if response.dragged() && view_state.dragging {
        if let Some(pos) = response.interact_pointer_pos() {
            let doc_pos = mouse_to_doc_pos(pos);
            if view_state.block_selecting {
                if let Some(start) = view_state.block_select_start {
                    create_block_cursors(doc, &start, &doc_pos);
                }
            } else {
                doc.cursors.primary_mut().move_to(doc_pos, true);
            }
        }
    }

    if response.drag_stopped() {
        view_state.dragging = false;
        view_state.block_selecting = false;
        // Keep block_select_start so multi-cursors remain active
    }

    // Ctrl+hover: request LSP hover info, or show diagnostic tooltip on hover
    if response.hovered() {
        if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
            if hover_pos.x >= text_left {
                // Compute document position from hover pixel pos (inline, to avoid closure borrow)
                let h_screen_row = ((hover_pos.y - rect.top()) / line_height) as usize;
                let (h_line_idx, h_col_offset) = displayed_vrows
                    .iter()
                    .find(|(r, _)| *r == h_screen_row)
                    .map(|&(_, vr)| (vr.line_idx, vr.col_offset))
                    .unwrap_or_else(|| {
                        displayed_vrows
                            .last()
                            .map(|&(_, vr)| (vr.line_idx, vr.col_offset))
                            .unwrap_or((0, 0))
                    });
                let h_raw_col = (((hover_pos.x - text_left - 4.0) / char_width).round() as isize)
                    .max(0) as usize;
                let h_col = h_col_offset + h_raw_col + (if word_wrap { 0 } else { scroll_col });
                let h_line = h_line_idx.min(total_lines.saturating_sub(1));
                let h_col = h_col.min(doc.buffer.line_len_chars_no_newline(h_line));
                let doc_pos = Position::new(h_line, h_col);

                let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.mac_cmd);

                // Check if hovering over a diagnostic squiggle
                if let Some(ctx) = render_ctx {
                    for diag in ctx.lsp_diagnostics {
                        let on_diag_line = diag.line == doc_pos.line
                            || (diag.line <= doc_pos.line && diag.end_line >= doc_pos.line);
                        if on_diag_line {
                            let col = doc_pos.col;
                            let in_range = if diag.line == diag.end_line {
                                col >= diag.col && col <= diag.end_col
                            } else if doc_pos.line == diag.line {
                                col >= diag.col
                            } else if doc_pos.line == diag.end_line {
                                col <= diag.end_col
                            } else {
                                true
                            };
                            if in_range {
                                view_state.diagnostic_hover =
                                    Some((diag.message.clone(), hover_pos));
                                break;
                            }
                        }
                    }
                }

                // Ctrl+hover: request hover info from LSP
                if ctrl {
                    view_state.hover_request = Some((doc_pos, hover_pos));
                }
            }
        }
    }

    // Scroll
    let (scroll_delta, ctrl_held) =
        ui.input(|i| (i.raw_scroll_delta, i.modifiers.ctrl || i.modifiers.mac_cmd));
    if scroll_delta.y != 0.0 && !ctrl_held {
        let lines_delta = -(scroll_delta.y / line_height * 3.0) as isize;
        let new_scroll = (doc.scroll_line as isize + lines_delta)
            .max(0)
            .min(max_scroll as isize) as usize;
        doc.scroll_line = new_scroll;
    }

    // Keyboard input
    let cursor_before = doc.cursors.primary().position;
    let modified = handle_keyboard_input(ui, doc, macro_rec, snippet_engine, vim_state);
    let cursor_moved_by_keyboard = doc.cursors.primary().position != cursor_before || modified;

    // Update autocomplete after edits
    if modified {
        autocomplete.update(doc);
    }

    // Autocomplete popup
    if autocomplete.visible {
        let cpos = doc.cursors.primary().position;
        if let Some(screen_row) = pos_to_screen_row(cpos.line, cpos.col) {
            let y = rect.top() + screen_row as f32 * line_height;
            let col_in_row = if word_wrap {
                let vr = &displayed_vrows[screen_row].1;
                cpos.col - vr.col_offset
            } else {
                cpos.col - scroll_col
            };
            let cx = text_left + 4.0 + col_in_row as f32 * char_width;
            crate::autocomplete::render_autocomplete(
                ui,
                autocomplete,
                Pos2::new(cx, y),
                line_height,
                theme,
            );
        }
    }

    // Ensure cursor is visible only when cursor moved via keyboard.
    // When the user scrolls with the mouse, cursor_moved_by_keyboard is false,
    // so we don't fight the scroll by pulling the viewport back to the cursor.
    if cursor_moved_by_keyboard {
        ensure_cursor_visible_wrapped(doc, visible_lines, &all_visual_rows, word_wrap, wrap_cols);
    }

    modified
}

/// Create multi-cursors forming a rectangular block selection.
///
/// Each line in the block gets a cursor positioned at `end_col` with an anchor at `start_col`,
/// clamped to the actual line length.
fn create_block_cursors(doc: &mut Document, start: &Position, end: &Position) {
    let min_line = start.line.min(end.line);
    let max_line = start.line.max(end.line);
    let min_col = start.col.min(end.col);
    let max_col = start.col.max(end.col);

    doc.cursors.clear_extra_cursors();

    // Set primary cursor on the first line of the block
    let first_line = min_line;
    if first_line < doc.buffer.len_lines() {
        let line_len = doc.buffer.line_len_chars_no_newline(first_line);
        let actual_start = min_col.min(line_len);
        let actual_end = max_col.min(line_len);
        let primary = doc.cursors.primary_mut();
        primary.anchor = Some(Position::new(first_line, actual_start));
        primary.position = Position::new(first_line, actual_end);
    }

    // Add cursors for remaining lines in the block
    for line in (min_line + 1)..=max_line {
        if line >= doc.buffer.len_lines() {
            break;
        }
        let line_len = doc.buffer.line_len_chars_no_newline(line);
        let actual_start = min_col.min(line_len);
        let actual_end = max_col.min(line_len);

        let mut cursor = openedit_core::cursor::Cursor::new(line, actual_end);
        cursor.anchor = Some(Position::new(line, actual_start));
        doc.cursors.add_cursor(cursor);
    }
}

fn render_highlighted_line(
    ui: &mut Ui,
    display: &str,
    scroll_col: usize,
    spans: &[HighlightSpan],
    text_left: f32,
    y: f32,
    font_id: &egui::FontId,
    theme: &EditorTheme,
    char_width: f32,
    max_chars: Option<usize>,
) {
    let chars: Vec<char> = display.chars().collect();
    if chars.is_empty() {
        return;
    }

    // Build a color map for each character in the display line
    let mut char_colors: Vec<egui::Color32> = vec![theme.foreground; chars.len()];

    // Apply highlight spans (start_col/end_col are char indices)
    for span in spans {
        if let Some(color) = theme.syntax_colors.color_for_highlight(span.highlight_idx) {
            let start = span.start_col.min(chars.len());
            let end = span.end_col.min(chars.len());
            for color_slot in char_colors[start..end].iter_mut() {
                *color_slot = color;
            }
        }
    }

    // Render consecutive characters with the same color as a single text call
    let visible_start = scroll_col.min(chars.len());
    let visible_end = if let Some(max) = max_chars {
        (visible_start + max).min(chars.len())
    } else {
        chars.len()
    };
    let visible_chars = &chars[visible_start..visible_end];
    let visible_colors = &char_colors[visible_start..visible_end];

    if visible_chars.is_empty() {
        return;
    }

    let mut x = text_left + 4.0;
    let mut run_start = 0;
    let mut current_color = visible_colors[0];

    for i in 1..=visible_chars.len() {
        let color_changed = i >= visible_chars.len() || visible_colors[i] != current_color;
        if color_changed {
            let run_text: String = visible_chars[run_start..i].iter().collect();
            ui.painter().text(
                Pos2::new(x, y),
                egui::Align2::LEFT_TOP,
                &run_text,
                font_id.clone(),
                current_color,
            );
            x += (i - run_start) as f32 * char_width;
            if i < visible_chars.len() {
                run_start = i;
                current_color = visible_colors[i];
            }
        }
    }
}

fn render_selection_wrapped(
    ui: &mut Ui,
    sel_start: &Position,
    sel_end: &Position,
    displayed_vrows: &[(usize, &VisualRow)],
    text_left: f32,
    rect: Rect,
    doc: &Document,
    theme: &EditorTheme,
    line_height: f32,
    char_width: f32,
    word_wrap: bool,
    wrap_cols: usize,
) {
    for &(screen_row, vr) in displayed_vrows {
        if vr.line_idx < sel_start.line || vr.line_idx > sel_end.line {
            continue;
        }

        let line_len = doc.buffer.line_len_chars_no_newline(vr.line_idx);
        let vr_end = if word_wrap {
            (vr.col_offset + wrap_cols).min(line_len)
        } else {
            line_len
        };

        let sel_line_start = if vr.line_idx == sel_start.line {
            sel_start.col
        } else {
            0
        };
        let sel_line_end = if vr.line_idx == sel_end.line {
            sel_end.col
        } else {
            line_len
        };

        // Clamp selection to this visual row's range
        let start_col = sel_line_start.max(vr.col_offset);
        let end_col = sel_line_end.min(vr_end);

        if start_col >= end_col {
            continue;
        }

        let y = rect.top() + screen_row as f32 * line_height;
        let x1 = text_left + 4.0 + (start_col - vr.col_offset) as f32 * char_width;
        let x2 = text_left + 4.0 + (end_col - vr.col_offset) as f32 * char_width;

        let sel_rect = Rect::from_min_max(
            Pos2::new(x1.max(text_left), y),
            Pos2::new(x2.min(rect.right()), y + line_height),
        );
        ui.painter().rect_filled(sel_rect, 0.0, theme.selection_bg);
    }
}

fn handle_keyboard_input(
    ui: &mut Ui,
    doc: &mut Document,
    macro_rec: &mut MacroRecorder,
    snippet_engine: &mut SnippetEngine,
    vim_state: Option<&mut VimState>,
) -> bool {
    let mut modified = false;
    let mut copy_text: Option<String> = None;
    let page_size = {
        let rect = ui.available_rect_before_wrap();
        let lh = line_height_for_font(13.0); // approximate
        (rect.height() / lh).floor().max(1.0) as usize
    };

    // Collect actions to record after input processing (cannot borrow macro_rec inside closure)
    let mut pending_macro_actions: Vec<MacroAction> = Vec::new();
    let is_recording = macro_rec.is_recording();

    // Check vim mode up front so we can use it inside the closure
    let vim_enabled = vim_state.as_ref().is_some_and(|v| v.enabled);

    // Collect events first so we can process them with vim_state outside the closure.
    let events = ui.input(|input| input.events.clone());

    // If vim mode is enabled, process events through vim first
    if vim_enabled {
        if let Some(vim) = vim_state {
            for event in &events {
                let vim_mode = vim.mode;
                match event {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers: key_mods,
                        ..
                    } => {
                        let ctrl = key_mods.ctrl || key_mods.mac_cmd;
                        let shift = key_mods.shift;
                        let alt = key_mods.alt;
                        let vim_key_str = egui_key_to_vim_str(key, ctrl, shift, alt);
                        if let Some(ref key_str) = vim_key_str {
                            let mut vim_modified = false;
                            let consumed = vim.handle_key(key_str, doc, &mut vim_modified);
                            if vim_modified {
                                modified = true;
                            }
                            if consumed {
                                continue;
                            }
                        }

                        // In insert mode, fall through to normal key handling below.
                        // In other modes, we already tried vim and it didn't consume,
                        // so we skip normal handling for non-modifier keys to avoid
                        // interfering with vim expectations.
                        if vim_mode != VimMode::Insert {
                            // Still allow Ctrl+key combos to pass through to editor
                            // (e.g., Ctrl+S for save is handled at app level)
                            if !ctrl {
                                continue;
                            }
                        }

                        // In insert mode, process keys normally (fall through below)
                        let key_name = format!("{:?}", key);
                        handle_editor_key(
                            key,
                            ctrl,
                            shift,
                            alt,
                            &key_name,
                            doc,
                            &mut modified,
                            &mut copy_text,
                            is_recording,
                            &mut pending_macro_actions,
                            page_size,
                            snippet_engine,
                        );
                    }
                    egui::Event::Paste(text) => {
                        // In insert mode, allow paste normally
                        if vim_mode == VimMode::Insert && !text.is_empty() {
                            vim.record_insert_text(text);
                            doc.insert_text(text);
                            modified = true;
                            if is_recording {
                                pending_macro_actions.push(MacroAction::Paste(text.clone()));
                            }
                        }
                        // In other modes, vim handles yank/put internally
                    }
                    egui::Event::Text(text) => {
                        if !text.chars().all(|c| c.is_control()) {
                            if vim_mode == VimMode::Insert {
                                // Track text for vim `.` repeat
                                vim.record_insert_text(text);
                                // Fall through to normal text handling
                                handle_text_input(
                                    text,
                                    doc,
                                    &mut modified,
                                    is_recording,
                                    &mut pending_macro_actions,
                                );
                            } else if vim_mode == VimMode::Command {
                                // In Command mode, vim handles text input internally
                                // via handle_key with single-char strings
                                let mut vim_modified = false;
                                vim.handle_key(text, doc, &mut vim_modified);
                                if vim_modified {
                                    modified = true;
                                }
                            } else {
                                // Normal/Visual mode: text chars are vim commands
                                // (e.g., 'd', 'w', 'y', etc.)
                                // These should already have been handled via Key events
                                // but some chars come only as Text events (e.g., shifted chars
                                // like '$', '^', etc.), so route them to vim.
                                let mut vim_modified = false;
                                vim.handle_key(text, doc, &mut vim_modified);
                                if vim_modified {
                                    modified = true;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    } else {
        // Normal (non-vim) mode: process all events as before
        for event in &events {
            match event {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers: key_mods,
                    ..
                } => {
                    let ctrl = key_mods.ctrl || key_mods.mac_cmd;
                    let shift = key_mods.shift;
                    let alt = key_mods.alt;
                    let key_name = format!("{:?}", key);
                    handle_editor_key(
                        key,
                        ctrl,
                        shift,
                        alt,
                        &key_name,
                        doc,
                        &mut modified,
                        &mut copy_text,
                        is_recording,
                        &mut pending_macro_actions,
                        page_size,
                        snippet_engine,
                    );
                }
                egui::Event::Paste(text) => {
                    if !text.is_empty() {
                        doc.insert_text(text);
                        modified = true;
                        if is_recording {
                            pending_macro_actions.push(MacroAction::Paste(text.clone()));
                        }
                    }
                }
                egui::Event::Text(text) => {
                    if !text.chars().all(|c| c.is_control()) {
                        handle_text_input(
                            text,
                            doc,
                            &mut modified,
                            is_recording,
                            &mut pending_macro_actions,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    // Record collected macro actions
    for action in pending_macro_actions {
        macro_rec.record_action(action);
    }

    // Set clipboard text (must be done outside input closure)
    if let Some(text) = copy_text {
        ui.output_mut(|o| o.copied_text = text);
    }

    modified
}

/// Convert an egui key event to a string that VimState.handle_key() expects.
/// Returns None if the key cannot be meaningfully mapped.
fn egui_key_to_vim_str(key: &egui::Key, ctrl: bool, shift: bool, _alt: bool) -> Option<String> {
    // For Ctrl+key combos, return "Ctrl+x" format
    if ctrl {
        let base = match key {
            egui::Key::R => Some("r"),
            egui::Key::F => Some("f"),
            egui::Key::B => Some("b"),
            egui::Key::D => Some("d"),
            egui::Key::U => Some("u"),
            _ => None,
        };
        if let Some(b) = base {
            let letter = if shift {
                b.to_uppercase()
            } else {
                b.to_string()
            };
            return Some(format!("Ctrl+{}", letter));
        }
        // Let Ctrl+other keys pass through to normal editor handling
        return None;
    }

    match key {
        egui::Key::Escape => Some("Escape".to_string()),
        egui::Key::Enter => Some("Enter".to_string()),
        egui::Key::Backspace => Some("Backspace".to_string()),
        egui::Key::Delete => Some("Delete".to_string()),
        egui::Key::Tab => Some("Tab".to_string()),
        egui::Key::ArrowLeft => Some("ArrowLeft".to_string()),
        egui::Key::ArrowRight => Some("ArrowRight".to_string()),
        egui::Key::ArrowUp => Some("ArrowUp".to_string()),
        egui::Key::ArrowDown => Some("ArrowDown".to_string()),
        egui::Key::Home => Some("Home".to_string()),
        egui::Key::End => Some("End".to_string()),
        egui::Key::PageUp => Some("PageUp".to_string()),
        egui::Key::PageDown => Some("PageDown".to_string()),
        // Single character keys: only return them for non-printable/special purposes.
        // For printable chars, vim gets them via Text events instead, which handles
        // shift properly (e.g., shift+a = "A", shift+4 = "$").
        // But we do need Escape, Enter, etc. from Key events since they don't
        // generate Text events.
        _ => None,
    }
}

/// Handle a single editor key event (normal non-vim processing).
/// Extracted to avoid duplication between vim insert-mode passthrough and non-vim mode.
fn handle_editor_key(
    key: &egui::Key,
    ctrl: bool,
    shift: bool,
    alt: bool,
    key_name: &str,
    doc: &mut Document,
    modified: &mut bool,
    copy_text: &mut Option<String>,
    is_recording: bool,
    pending_macro_actions: &mut Vec<MacroAction>,
    page_size: usize,
    snippet_engine: &mut SnippetEngine,
) {
    match key {
        // Clipboard
        egui::Key::C if ctrl => {
            let text = doc.selected_text();
            if !text.is_empty() {
                *copy_text = Some(text);
            }
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::X if ctrl => {
            let text = doc.selected_text();
            if !text.is_empty() {
                *copy_text = Some(text);
                doc.delete_selection_public();
                *modified = true;
            }
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        // Line operations
        egui::Key::ArrowUp if alt => {
            doc.move_line_up();
            *modified = true;
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::ArrowDown if alt => {
            doc.move_line_down();
            *modified = true;
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::K if ctrl && shift => {
            doc.delete_line();
            *modified = true;
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        // Navigation
        egui::Key::ArrowLeft if ctrl => {
            doc.move_cursor_word_left(shift);
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::ArrowRight if ctrl => {
            doc.move_cursor_word_right(shift);
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::ArrowLeft => {
            doc.move_cursor_left(shift);
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::ArrowRight => {
            doc.move_cursor_right(shift);
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::ArrowUp => {
            doc.move_cursor_up(shift);
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::ArrowDown => {
            doc.move_cursor_down(shift);
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::Home if ctrl => {
            doc.move_cursor_doc_start(shift);
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::End if ctrl => {
            doc.move_cursor_doc_end(shift);
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::Home => {
            doc.move_cursor_home(shift);
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::End => {
            doc.move_cursor_end(shift);
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::PageUp => {
            doc.move_cursor_page_up(page_size, shift);
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::PageDown => {
            doc.move_cursor_page_down(page_size, shift);
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        // Editing
        egui::Key::Backspace if ctrl => {
            doc.delete_word_left();
            *modified = true;
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::Backspace => {
            doc.backspace();
            *modified = true;
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::Delete if ctrl => {
            doc.delete_word_right();
            *modified = true;
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::Delete => {
            doc.delete_forward();
            *modified = true;
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::Enter => {
            doc.insert_newline_with_indent();
            *modified = true;
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::Tab if shift => {
            // If a snippet is active, navigate to previous placeholder
            if snippet_engine.is_active() {
                snippet_engine.prev_placeholder(doc);
            } else {
                doc.unindent();
            }
            *modified = true;
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::Tab => {
            // If a snippet is active, navigate to next placeholder
            if snippet_engine.is_active() {
                snippet_engine.next_placeholder(doc);
                *modified = true;
            } else {
                // Try to expand a snippet from the word before cursor
                if snippet_engine.try_expand(doc) {
                    *modified = true;
                } else {
                    // No snippet matched, insert regular tab (4 spaces)
                    doc.insert_text("    ");
                    *modified = true;
                }
            }
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        // Selection/undo
        egui::Key::A if ctrl => {
            doc.select_all();
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::Z if ctrl && shift => {
            doc.redo();
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::Z if ctrl => {
            doc.undo();
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::Y if ctrl => {
            doc.redo();
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::Slash if ctrl => {
            doc.toggle_comment();
            *modified = true;
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::D if ctrl => {
            doc.select_next_occurrence();
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        egui::Key::L if ctrl && shift => {
            doc.select_all_occurrences();
        }
        egui::Key::Escape => {
            // Cancel active snippet navigation
            snippet_engine.cancel();
            if doc.cursors.cursor_count() > 1 {
                doc.cursors.clear_extra_cursors();
            }
            if is_recording {
                pending_macro_actions.push(MacroAction::KeyAction {
                    key: key_name.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }
        _ => {}
    }
}

/// Handle text input (bracket auto-close etc.) extracted for reuse.
fn handle_text_input(
    text: &str,
    doc: &mut Document,
    modified: &mut bool,
    is_recording: bool,
    pending_macro_actions: &mut Vec<MacroAction>,
) {
    if is_recording {
        pending_macro_actions.push(MacroAction::InsertText(text.to_string()));
    }
    // Bracket auto-close
    if text.len() == 1 {
        let ch = text.chars().next().unwrap();
        if let Some(close) = match ch {
            '(' => Some(')'),
            '[' => Some(']'),
            '{' => Some('}'),
            '"' => Some('"'),
            '\'' => Some('\''),
            _ => None,
        } {
            // If the character after cursor is the same closing char, skip it
            let cursor = doc.cursors.primary();
            let offset = doc
                .buffer
                .line_col_to_char(cursor.position.line, cursor.position.col);
            let next_char = if offset < doc.buffer.len_chars() {
                Some(doc.buffer.char_at(offset))
            } else {
                None
            };

            // For quotes, skip if next char is the same quote
            if (ch == '"' || ch == '\'') && next_char == Some(ch) {
                // Just move cursor past the existing closing char
                doc.move_cursor_right(false);
                *modified = true;
            } else if ch == close && next_char == Some(close) {
                // Skip over existing closing bracket
                doc.move_cursor_right(false);
                *modified = true;
            } else {
                // Insert open + close, cursor between them
                let pair = format!("{}{}", ch, close);
                doc.insert_text(&pair);
                doc.move_cursor_left(false);
                *modified = true;
            }
        } else if (match ch {
            ')' | ']' | '}' => {
                let cursor = doc.cursors.primary();
                let offset = doc
                    .buffer
                    .line_col_to_char(cursor.position.line, cursor.position.col);
                let next_char = if offset < doc.buffer.len_chars() {
                    Some(doc.buffer.char_at(offset))
                } else {
                    None
                };
                if next_char == Some(ch) {
                    Some(ch)
                } else {
                    None
                }
            }
            _ => None,
        })
        .is_some()
        {
            // Skip over existing closing bracket
            doc.move_cursor_right(false);
        } else {
            doc.insert_text(text);
            *modified = true;
        }
    } else {
        doc.insert_text(text);
        *modified = true;
    }
}

/// Find the matching bracket for the character at or before the cursor position.
fn find_matching_bracket(doc: &Document, cursor_pos: &Position) -> Option<Position> {
    let offset = doc.buffer.line_col_to_char(cursor_pos.line, cursor_pos.col);
    let total = doc.buffer.len_chars();

    // Check character at cursor and character before cursor
    let at_cursor = if offset < total {
        Some(doc.buffer.char_at(offset))
    } else {
        None
    };
    let before_cursor = if offset > 0 {
        Some(doc.buffer.char_at(offset - 1))
    } else {
        None
    };

    // Try character at cursor first, then before cursor
    let (check_offset, ch) = if let Some(c) = at_cursor {
        if is_bracket(c) {
            (offset, c)
        } else if let Some(c2) = before_cursor {
            if is_bracket(c2) {
                (offset - 1, c2)
            } else {
                return None;
            }
        } else {
            return None;
        }
    } else if let Some(c) = before_cursor {
        if is_bracket(c) {
            (offset - 1, c)
        } else {
            return None;
        }
    } else {
        return None;
    };

    let (open, close, forward) = match ch {
        '(' => ('(', ')', true),
        '[' => ('[', ']', true),
        '{' => ('{', '}', true),
        ')' => ('(', ')', false),
        ']' => ('[', ']', false),
        '}' => ('{', '}', false),
        _ => return None,
    };

    let mut depth = 0i32;
    if forward {
        for i in check_offset..total {
            let c = doc.buffer.char_at(i);
            if c == open {
                depth += 1;
            }
            if c == close {
                depth -= 1;
            }
            if depth == 0 {
                let (line, col) = doc.buffer.char_to_line_col(i);
                return Some(Position::new(line, col));
            }
        }
    } else {
        let mut i = check_offset as isize;
        while i >= 0 {
            let c = doc.buffer.char_at(i as usize);
            if c == close {
                depth += 1;
            }
            if c == open {
                depth -= 1;
            }
            if depth == 0 {
                let (line, col) = doc.buffer.char_to_line_col(i as usize);
                return Some(Position::new(line, col));
            }
            i -= 1;
        }
    }

    None
}

fn is_bracket(c: char) -> bool {
    matches!(c, '(' | ')' | '[' | ']' | '{' | '}')
}

/// Ensure cursor is visible, accounting for folded lines and word wrapping.
fn ensure_cursor_visible_wrapped(
    doc: &mut Document,
    visible_lines: usize,
    all_visual_rows: &[VisualRow],
    word_wrap: bool,
    wrap_cols: usize,
) {
    let cursor_line = doc.cursors.primary().position.line;
    let cursor_col = doc.cursors.primary().position.col;
    let margin = 3;

    // Find the visual row index for the cursor position
    let cursor_vis_row = if word_wrap {
        // Find the visual row that contains this cursor column
        all_visual_rows
            .iter()
            .position(|vr| {
                vr.line_idx == cursor_line
                    && cursor_col >= vr.col_offset
                    && cursor_col < vr.col_offset + wrap_cols
            })
            .or_else(|| {
                // Cursor at end of line: find last visual row for this line
                all_visual_rows
                    .iter()
                    .enumerate()
                    .rfind(|(_, vr)| vr.line_idx == cursor_line)
                    .map(|(i, _)| i)
            })
    } else {
        all_visual_rows
            .iter()
            .position(|vr| vr.line_idx == cursor_line)
    };

    let cursor_vis_row = cursor_vis_row.unwrap_or_else(|| {
        // Cursor is on a hidden line -- find nearest visible row
        all_visual_rows
            .iter()
            .position(|vr| vr.line_idx >= cursor_line)
            .unwrap_or(all_visual_rows.len().saturating_sub(1))
    });

    if cursor_vis_row < doc.scroll_line + margin {
        doc.scroll_line = cursor_vis_row.saturating_sub(margin);
    } else if cursor_vis_row >= doc.scroll_line + visible_lines.saturating_sub(margin) {
        doc.scroll_line = cursor_vis_row.saturating_sub(visible_lines.saturating_sub(margin + 1));
    }

    // Horizontal scroll (disabled in word wrap mode)
    if !word_wrap {
        let visible_cols = 80; // approximate
        if cursor_col < doc.scroll_col + 2 {
            doc.scroll_col = cursor_col.saturating_sub(2);
        } else if cursor_col >= doc.scroll_col + visible_cols {
            doc.scroll_col = cursor_col.saturating_sub(visible_cols - 5);
        }
    }
}
