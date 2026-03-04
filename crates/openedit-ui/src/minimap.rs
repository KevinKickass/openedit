use crate::theme::EditorTheme;
use egui::{self, Color32, Pos2, Rect, Ui, Vec2};
use openedit_core::Document;

/// Width of the minimap sidebar in pixels.
const MINIMAP_WIDTH: f32 = 80.0;
/// Scale factor for minimap text (chars per pixel).
const MINIMAP_CHAR_HEIGHT: f32 = 2.0;
/// Width of a minimap character.
const MINIMAP_CHAR_WIDTH: f32 = 1.2;

/// Render minimap and return (width, optional target scroll line from click).
pub fn render_minimap_with_scroll(
    ui: &mut Ui,
    doc: &Document,
    theme: &EditorTheme,
    editor_rect: Rect,
    visible_start_line: usize,
    visible_end_line: usize,
    visible_lines_count: usize,
    font_size: f32,
) -> (f32, Option<usize>) {
    let _ = font_size; // reserved for future use
    let total_lines = doc.buffer.len_lines();
    if total_lines == 0 {
        return (0.0, None);
    }

    let minimap_rect = Rect::from_min_size(
        Pos2::new(editor_rect.right() - MINIMAP_WIDTH, editor_rect.top()),
        Vec2::new(MINIMAP_WIDTH, editor_rect.height()),
    );

    // Background -- slightly lighter/darker than editor background
    let bg = Color32::from_rgba_premultiplied(
        theme.background.r().saturating_add(10),
        theme.background.g().saturating_add(10),
        theme.background.b().saturating_add(10),
        255,
    );
    ui.painter().rect_filled(minimap_rect, 0.0, bg);

    // Calculate how many lines fit in minimap
    let max_minimap_lines = (minimap_rect.height() / MINIMAP_CHAR_HEIGHT) as usize;

    let minimap_start = if total_lines <= max_minimap_lines {
        0
    } else {
        let center = (visible_start_line + visible_end_line) / 2;
        center
            .saturating_sub(max_minimap_lines / 2)
            .min(total_lines.saturating_sub(max_minimap_lines))
    };
    let minimap_end = (minimap_start + max_minimap_lines).min(total_lines);

    // Viewport indicator
    let vp_start = visible_start_line
        .max(minimap_start)
        .saturating_sub(minimap_start);
    let vp_end = visible_end_line
        .min(minimap_end)
        .saturating_sub(minimap_start);

    let viewport_rect = Rect::from_min_max(
        Pos2::new(
            minimap_rect.left(),
            minimap_rect.top() + vp_start as f32 * MINIMAP_CHAR_HEIGHT,
        ),
        Pos2::new(
            minimap_rect.right(),
            minimap_rect.top() + vp_end as f32 * MINIMAP_CHAR_HEIGHT,
        ),
    );
    ui.painter().rect_filled(
        viewport_rect,
        0.0,
        Color32::from_rgba_premultiplied(120, 120, 120, 50),
    );

    // Draw minimap content
    let text_color = Color32::from_rgba_premultiplied(
        theme.foreground.r(),
        theme.foreground.g(),
        theme.foreground.b(),
        100,
    );

    for line_idx in minimap_start..minimap_end {
        let y = minimap_rect.top() + (line_idx - minimap_start) as f32 * MINIMAP_CHAR_HEIGHT;
        let line_str = doc.buffer.line_str(line_idx);
        let trimmed = line_str.trim_end_matches(&['\n', '\r'][..]);

        if trimmed.is_empty() {
            continue;
        }

        let indent = trimmed.chars().take_while(|c| c.is_whitespace()).count();
        let content_len = trimmed.len().saturating_sub(indent).min(60);

        if content_len > 0 {
            let x_start = minimap_rect.left() + 2.0 + indent as f32 * MINIMAP_CHAR_WIDTH;
            let width = (content_len as f32 * MINIMAP_CHAR_WIDTH)
                .min(minimap_rect.right() - x_start - 2.0)
                .max(0.0);

            let line_rect = Rect::from_min_size(
                Pos2::new(x_start, y),
                Vec2::new(width, (MINIMAP_CHAR_HEIGHT - 0.5).max(1.0)),
            );
            ui.painter().rect_filled(line_rect, 0.0, text_color);
        }
    }

    // Handle click/drag to scroll
    let response = ui.allocate_rect(minimap_rect, egui::Sense::click_and_drag());
    let mut scroll_target = None;
    if response.clicked() || response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            let relative_y = pos.y - minimap_rect.top();
            let target_line = minimap_start + (relative_y / MINIMAP_CHAR_HEIGHT) as usize;
            let target_line = target_line.min(total_lines.saturating_sub(1));
            // Center the viewport on the clicked line
            scroll_target = Some(target_line.saturating_sub(visible_lines_count / 2));
        }
    }

    (MINIMAP_WIDTH, scroll_target)
}
