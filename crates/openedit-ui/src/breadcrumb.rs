use crate::theme::EditorTheme;
use egui;
use openedit_core::syntax::{Symbol, SymbolKind};

pub struct BreadcrumbState {
    pub visible: bool,
}

impl Default for BreadcrumbState {
    fn default() -> Self {
        Self { visible: true }
    }
}

pub fn render_breadcrumb(
    ui: &mut egui::Ui,
    state: &BreadcrumbState,
    theme: &EditorTheme,
    symbols: &[Symbol],
    current_line: usize,
) {
    if !state.visible {
        return;
    }

    let container = ui.available_rect_before_wrap();
    let height = 24.0;

    ui.painter().rect_filled(
        egui::Rect::from_min_size(container.min, egui::vec2(container.width(), height)),
        0.0,
        theme.gutter_bg,
    );

    let breadcrumbs = build_breadcrumbs(symbols, current_line);

    ui.horizontal(|ui| {
        ui.add_space(8.0);

        for (i, crumb) in breadcrumbs.iter().enumerate() {
            if i > 0 {
                let sep = egui::RichText::new(" \u{203A} ") // ›
                    .color(theme.gutter_fg.linear_multiply(0.5))
                    .size(12.0);
                ui.label(sep);
            }

            let kind_color = match crumb.kind {
                SymbolKind::Class => egui::Color32::from_rgb(255, 200, 100),
                SymbolKind::Struct => egui::Color32::from_rgb(100, 200, 255),
                SymbolKind::Function | SymbolKind::Method => egui::Color32::from_rgb(100, 255, 150),
                SymbolKind::Enum => egui::Color32::from_rgb(255, 150, 255),
                SymbolKind::Module => egui::Color32::from_rgb(200, 200, 255),
                _ => theme.gutter_fg,
            };

            let text = egui::RichText::new(&crumb.name)
                .color(kind_color)
                .size(12.0);
            ui.label(text);
        }

        if breadcrumbs.is_empty() {
            let text = egui::RichText::new("No symbol")
                .color(theme.gutter_fg.linear_multiply(0.5))
                .size(12.0);
            ui.label(text);
        }
    });
}

fn build_breadcrumbs(symbols: &[Symbol], current_line: usize) -> Vec<&Symbol> {
    let mut result: Vec<&Symbol> = Vec::new();

    for symbol in symbols {
        if symbol.line <= current_line {
            let is_containable = matches!(
                symbol.kind,
                SymbolKind::Class
                    | SymbolKind::Struct
                    | SymbolKind::Enum
                    | SymbolKind::Module
                    | SymbolKind::Interface
            );

            if is_containable {
                if let Some(pos) = result.iter().position(|s| s.line == symbol.line) {
                    result[pos] = symbol;
                } else {
                    result.push(symbol);
                }
            }
        }
    }

    result.sort_by_key(|s| s.line);

    let mut final_breadcrumbs: Vec<&Symbol> = Vec::new();
    for symbol in &result {
        let should_add = match &symbol.kind {
            SymbolKind::Class | SymbolKind::Struct | SymbolKind::Enum | SymbolKind::Module => true,
            _ => final_breadcrumbs.is_empty(),
        };
        if should_add {
            final_breadcrumbs.push(symbol);
        }
    }

    final_breadcrumbs
}
