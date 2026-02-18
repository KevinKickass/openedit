use crate::theme::EditorTheme;
use egui;
use openedit_core::syntax::Symbol;

/// State for the function list panel.
pub struct FunctionListState {
    /// Whether the panel is visible.
    pub visible: bool,
    /// Cached symbols for the current document.
    pub symbols: Vec<Symbol>,
    /// Filter query.
    pub filter: String,
}

impl Default for FunctionListState {
    fn default() -> Self {
        Self {
            visible: false,
            symbols: Vec::new(),
            filter: String::new(),
        }
    }
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

/// Render the function list panel.
/// Returns `Some(line)` if a symbol was clicked to navigate to.
pub fn render_function_list(
    ui: &mut egui::Ui,
    state: &mut FunctionListState,
    theme: &EditorTheme,
) -> Option<usize> {
    let rect = ui.available_rect_before_wrap();
    ui.painter().rect_filled(rect, 0.0, theme.gutter_bg);

    let mut clicked_line: Option<usize> = None;

    // Header
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("FUNCTION LIST")
                .small()
                .color(theme.gutter_fg),
        );
    });

    ui.add_space(4.0);

    // Filter input
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(&mut state.filter)
                .hint_text("Filter symbols...")
                .desired_width(ui.available_width() - 8.0),
        );
    });

    ui.add_space(4.0);
    ui.separator();

    // Filter symbols
    let filtered: Vec<&Symbol> = if state.filter.is_empty() {
        state.symbols.iter().collect()
    } else {
        state
            .symbols
            .iter()
            .filter(|s| fuzzy_match(&state.filter, &s.name))
            .collect()
    };

    if filtered.is_empty() {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(if state.symbols.is_empty() {
                    "No symbols found."
                } else {
                    "No matching symbols."
                })
                .weak(),
            );
        });
    } else {
        // Scrollable symbol list
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (i, sym) in filtered.iter().enumerate() {
                    let response = ui.horizontal(|ui| {
                        ui.add_space(8.0);

                        // Kind label with a muted color
                        let kind_color = kind_color(sym.kind.label(), theme);
                        ui.label(
                            egui::RichText::new(sym.kind.label())
                                .monospace()
                                .small()
                                .color(kind_color),
                        );

                        ui.add_space(4.0);

                        // Symbol name
                        ui.label(
                            egui::RichText::new(&sym.name)
                                .color(theme.foreground),
                        );

                        // Line number on the right
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.add_space(8.0);
                                ui.label(
                                    egui::RichText::new(format!(":{}", sym.line + 1))
                                        .weak()
                                        .small(),
                                );
                            },
                        );
                    });

                    // Click detection on the row
                    let row_rect = response.response.rect;
                    let interact = ui.interact(
                        row_rect,
                        ui.id().with(("fnlist", i)),
                        egui::Sense::click(),
                    );
                    if interact.clicked() {
                        clicked_line = Some(sym.line);
                    }

                    // Hover highlight
                    if interact.hovered() {
                        ui.painter().rect_filled(
                            row_rect,
                            0.0,
                            egui::Color32::from_white_alpha(15),
                        );
                    }
                }
            });
    }

    clicked_line
}

/// Choose a color for the symbol kind label.
fn kind_color(kind_label: &str, theme: &EditorTheme) -> egui::Color32 {
    // Use different tints based on the kind
    let _ = theme; // keep param for future theming
    match kind_label {
        "fn" | "method" => egui::Color32::from_rgb(86, 156, 214),   // blue
        "class" | "struct" => egui::Color32::from_rgb(78, 201, 176), // teal
        "enum" => egui::Color32::from_rgb(184, 215, 163),           // green
        "trait" => egui::Color32::from_rgb(197, 134, 192),          // purple
        "mod" => egui::Color32::from_rgb(220, 220, 170),            // yellow
        "const" | "var" => egui::Color32::from_rgb(156, 220, 254),  // light blue
        _ => egui::Color32::from_rgb(180, 180, 180),                // gray
    }
}
