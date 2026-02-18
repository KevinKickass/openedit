use egui::{self, Ui};
use openedit_core::Document;

/// State for the search panel UI.
#[derive(Default)]
pub struct SearchPanelState {
    pub visible: bool,
    pub query: String,
    pub replace: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
    pub show_replace: bool,
}

/// Render the search/replace panel. Returns true if it should be closed.
pub fn render_search_panel(
    ui: &mut Ui,
    state: &mut SearchPanelState,
    doc: &mut Document,
) -> bool {
    let mut close = false;

    egui::Frame::none()
        .fill(egui::Color32::from_rgb(50, 50, 50))
        .inner_margin(egui::Margin::same(8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Find:");
                let search_response = ui.text_edit_singleline(&mut state.query);

                if search_response.changed() {
                    // Update search
                    doc.search.options.case_sensitive = state.case_sensitive;
                    doc.search.options.whole_word = state.whole_word;
                    doc.search.options.use_regex = state.use_regex;
                    let _ = doc.search.set_query(&state.query);
                    let text = doc.buffer.to_string();
                    doc.search.find_all(&text);
                }

                // Match count
                let count = doc.search.match_count();
                let current = doc.search.current_match_index().map(|i| i + 1).unwrap_or(0);
                ui.label(format!("{}/{}", current, count));

                // Prev/Next buttons
                if ui.button("\u{25B2}").on_hover_text("Previous (Shift+F3)").clicked() {
                    let offset = doc.buffer.line_col_to_char(
                        doc.cursors.primary().position.line,
                        doc.cursors.primary().position.col,
                    );
                    if let Some(m) = doc.search.find_prev(offset) {
                        let (line, col) = doc.buffer.char_to_line_col(m.start);
                        doc.cursors.primary_mut().move_to(
                            openedit_core::cursor::Position::new(line, col),
                            false,
                        );
                    }
                }

                if ui.button("\u{25BC}").on_hover_text("Next (F3)").clicked() {
                    let offset = doc.buffer.line_col_to_char(
                        doc.cursors.primary().position.line,
                        doc.cursors.primary().position.col,
                    );
                    if let Some(m) = doc.search.find_next(offset + 1) {
                        let (line, col) = doc.buffer.char_to_line_col(m.start);
                        doc.cursors.primary_mut().move_to(
                            openedit_core::cursor::Position::new(line, col),
                            false,
                        );
                    }
                }

                // Toggle options
                ui.toggle_value(&mut state.case_sensitive, "Aa")
                    .on_hover_text("Case Sensitive");
                ui.toggle_value(&mut state.whole_word, "W")
                    .on_hover_text("Whole Word");
                ui.toggle_value(&mut state.use_regex, ".*")
                    .on_hover_text("Regex");

                // Toggle replace row
                ui.toggle_value(&mut state.show_replace, "Replace");

                // Close button
                if ui.button("\u{00D7}").on_hover_text("Close (Esc)").clicked() {
                    close = true;
                }
            });

            // Replace row
            if state.show_replace {
                ui.horizontal(|ui| {
                    ui.label("Replace:");
                    ui.text_edit_singleline(&mut state.replace);

                    if ui.button("Replace").clicked() {
                        // Replace current match
                        if let Some(idx) = doc.search.current_match_index() {
                            let m = doc.search.matches[idx].clone();
                            let start_offset = m.start;
                            let end_offset = m.end;
                            doc.buffer.replace(start_offset..end_offset, &state.replace);
                            doc.modified = true;
                            // Re-search
                            let text = doc.buffer.to_string();
                            doc.search.find_all(&text);
                        }
                    }

                    if ui.button("Replace All").clicked() {
                        let matches = doc.search.matches.clone();
                        let text = doc.buffer.to_string();
                        let new_text = openedit_core::search::replace_all(&text, &matches, &state.replace);
                        doc.buffer = openedit_core::Buffer::from_str(&new_text);
                        doc.modified = true;
                        // Re-search
                        doc.search.find_all(&new_text);
                    }
                });
            }
        });

    close
}
