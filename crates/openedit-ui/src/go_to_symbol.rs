use egui;
use openedit_core::syntax::Symbol;

/// State for the Go to Symbol dialog.
#[derive(Default)]
pub struct GoToSymbolState {
    /// Whether the dialog is open.
    pub open: bool,
    /// Current search query.
    pub query: String,
    /// Currently selected index in the filtered list.
    pub selected: usize,
    /// All symbols extracted from the current document.
    pub symbols: Vec<Symbol>,
}

/// Simple fuzzy match: all query characters must appear in the label in order (case insensitive).
fn fuzzy_match(query: &str, label: &str) -> bool {
    let query_lower = query.to_lowercase();
    let label_lower = label.to_lowercase();
    let mut label_chars = label_lower.chars();
    for qc in query_lower.chars() {
        if !label_chars.any(|lc| lc == qc) {
            return false;
        }
    }
    true
}

/// Render the Go to Symbol dialog.
///
/// Returns `Some(line)` (0-based) when a symbol is selected and the dialog
/// should navigate to that line. Returns `None` otherwise.
pub fn render_go_to_symbol(ctx: &egui::Context, state: &mut GoToSymbolState) -> Option<usize> {
    let filtered: Vec<&Symbol> = if state.query.is_empty() {
        state.symbols.iter().collect()
    } else {
        state
            .symbols
            .iter()
            .filter(|s| fuzzy_match(&state.query, &s.name))
            .collect()
    };

    // Clamp selection
    if state.selected >= filtered.len() {
        state.selected = filtered.len().saturating_sub(1);
    }

    let mut result: Option<usize> = None;
    let mut open = state.open;

    egui::Window::new("Go to Symbol")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 60.0])
        .fixed_size([420.0, 300.0])
        .show(ctx, |ui| {
            // Search input
            let response = ui.add(
                egui::TextEdit::singleline(&mut state.query)
                    .hint_text("Type to filter symbols...")
                    .desired_width(f32::INFINITY),
            );
            response.request_focus();

            // Handle keyboard navigation
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            let up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
            let down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
            let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));

            if escape {
                state.open = false;
                return;
            }

            if up && state.selected > 0 {
                state.selected -= 1;
            }
            if down && state.selected + 1 < filtered.len() {
                state.selected += 1;
            }
            if enter && !filtered.is_empty() {
                result = Some(filtered[state.selected].line);
                state.open = false;
                return;
            }

            // Reset selection when query changes
            if response.changed() {
                state.selected = 0;
            }

            ui.separator();

            if filtered.is_empty() {
                ui.label("No symbols found.");
            } else {
                // Symbol list
                egui::ScrollArea::vertical()
                    .max_height(250.0)
                    .show(ui, |ui| {
                        for (i, sym) in filtered.iter().enumerate() {
                            let is_selected = i == state.selected;
                            let bg = if is_selected {
                                egui::Color32::from_rgb(60, 60, 80)
                            } else {
                                egui::Color32::TRANSPARENT
                            };

                            egui::Frame::none()
                                .fill(bg)
                                .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        // Kind label in a muted color
                                        ui.label(
                                            egui::RichText::new(sym.kind.label())
                                                .weak()
                                                .monospace(),
                                        );
                                        // Symbol name
                                        ui.label(&sym.name);
                                        // Line number on the right
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        ":{}",
                                                        sym.line + 1
                                                    ))
                                                    .weak(),
                                                );
                                            },
                                        );
                                    });
                                });

                            let rect = ui.min_rect();
                            let response =
                                ui.interact(rect, ui.id().with(("sym", i)), egui::Sense::click());
                            if response.clicked() {
                                result = Some(sym.line);
                                state.open = false;
                            }
                            if response.hovered() {
                                state.selected = i;
                            }
                        }
                    });
            }
        });

    state.open = open;
    result
}
