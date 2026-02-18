use egui::{self, Color32, Pos2, Rect, Ui, Vec2};
use openedit_core::Document;
use crate::theme::EditorTheme;
use std::collections::HashSet;

/// State for the auto-completion popup.
#[derive(Default)]
pub struct AutocompleteState {
    /// Whether the popup is visible.
    pub visible: bool,
    /// Current suggestions.
    pub suggestions: Vec<String>,
    /// Selected index.
    pub selected: usize,
    /// The prefix being completed (word fragment before cursor).
    pub prefix: String,
}

impl AutocompleteState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update suggestions based on current cursor position. Call after each edit.
    pub fn update(&mut self, doc: &Document) {
        let cursor = doc.cursors.primary();
        let pos = cursor.position;
        let line = doc.buffer.line(pos.line).to_string();
        let before_cursor: String = line.chars().take(pos.col).collect();

        let prefix: String = before_cursor
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>()
            .chars()
            .rev()
            .collect();

        if prefix.len() < 2 {
            self.dismiss();
            return;
        }

        self.prefix = prefix.clone();

        // Collect words from document
        let text = doc.buffer.to_string();
        let mut words: HashSet<String> = HashSet::new();
        let mut current_word = String::new();
        for ch in text.chars() {
            if ch.is_alphanumeric() || ch == '_' {
                current_word.push(ch);
            } else {
                if current_word.len() >= 2 {
                    words.insert(current_word.clone());
                }
                current_word.clear();
            }
        }
        if current_word.len() >= 2 {
            words.insert(current_word);
        }

        let prefix_lower = prefix.to_lowercase();
        let mut matches: Vec<String> = words
            .into_iter()
            .filter(|w| {
                let w_lower = w.to_lowercase();
                w_lower.starts_with(&prefix_lower) && w_lower != prefix_lower
            })
            .collect();
        matches.sort();
        matches.truncate(10);

        self.suggestions = matches;
        self.selected = 0;
        self.visible = !self.suggestions.is_empty();
    }

    /// Dismiss the autocomplete popup.
    pub fn dismiss(&mut self) {
        self.visible = false;
        self.suggestions.clear();
        self.prefix.clear();
    }
}

/// Render the autocomplete popup near the cursor.
pub fn render_autocomplete(
    ui: &mut Ui,
    state: &AutocompleteState,
    cursor_screen_pos: Pos2,
    line_height: f32,
    _theme: &EditorTheme,
) {
    if !state.visible || state.suggestions.is_empty() {
        return;
    }

    let item_height = 20.0;
    let popup_width = 220.0;
    let popup_height = state.suggestions.len() as f32 * item_height;

    let popup_pos = Pos2::new(cursor_screen_pos.x, cursor_screen_pos.y + line_height);
    let popup_rect = Rect::from_min_size(popup_pos, Vec2::new(popup_width, popup_height));

    // Shadow
    let shadow_rect = popup_rect.translate(Vec2::new(2.0, 2.0));
    ui.painter().rect_filled(
        shadow_rect,
        4.0,
        Color32::from_rgba_premultiplied(0, 0, 0, 60),
    );

    // Background
    ui.painter()
        .rect_filled(popup_rect, 4.0, Color32::from_rgb(37, 37, 38));
    ui.painter().rect_stroke(
        popup_rect,
        4.0,
        egui::Stroke::new(1.0, Color32::from_rgb(69, 69, 69)),
    );

    let font_id = egui::FontId::monospace(12.0);

    for (i, suggestion) in state.suggestions.iter().enumerate() {
        let y = popup_pos.y + i as f32 * item_height;
        let item_rect = Rect::from_min_size(
            Pos2::new(popup_pos.x, y),
            Vec2::new(popup_width, item_height),
        );

        if i == state.selected {
            ui.painter()
                .rect_filled(item_rect, 0.0, Color32::from_rgb(4, 57, 94));
        }

        ui.painter().text(
            Pos2::new(popup_pos.x + 8.0, y + 2.0),
            egui::Align2::LEFT_TOP,
            suggestion,
            font_id.clone(),
            if i == state.selected {
                Color32::WHITE
            } else {
                Color32::from_rgb(188, 188, 188)
            },
        );
    }
}
