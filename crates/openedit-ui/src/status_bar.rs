use crate::theme::EditorTheme;
use egui::{self, Ui};
use openedit_core::encoding::Encoding;
use openedit_core::line_ending::LineEnding;
use openedit_core::Document;

const STATUS_BAR_HEIGHT: f32 = 24.0;

/// Actions that can be triggered by clicking status bar items.
#[derive(Debug, Clone)]
pub enum StatusBarAction {
    ChangeEncoding(Encoding),
    ChangeLineEnding(LineEnding),
    ChangeLanguage(String),
}

/// Render the status bar at the bottom. Returns the height consumed and any action triggered.
/// When `macro_recording` is true, a red "REC" indicator is shown.
pub fn render_status_bar(
    ui: &mut Ui,
    doc: Option<&Document>,
    theme: &EditorTheme,
    macro_recording: bool,
    git_branch: Option<&str>,
) -> (f32, Option<StatusBarAction>) {
    let rect = ui.available_rect_before_wrap();
    let bar_rect = egui::Rect::from_min_size(
        egui::Pos2::new(rect.left(), rect.bottom() - STATUS_BAR_HEIGHT),
        egui::vec2(rect.width(), STATUS_BAR_HEIGHT),
    );

    // Background
    ui.painter().rect_filled(bar_rect, 0.0, theme.status_bar_bg);

    let Some(doc) = doc else {
        return (STATUS_BAR_HEIGHT, None);
    };

    let cursor = doc.cursors.primary();
    let line = cursor.position.line + 1;
    let col = cursor.position.col + 1;

    // Left side: cursor position
    let pos_text = format!("Ln {}, Col {}", line, col);
    let font = egui::FontId::proportional(12.0);

    ui.painter().text(
        egui::Pos2::new(bar_rect.left() + 10.0, bar_rect.center().y),
        egui::Align2::LEFT_CENTER,
        &pos_text,
        font.clone(),
        theme.status_bar_fg,
    );

    // Selection info
    if cursor.has_selection() {
        let sel_text = doc.selected_text();
        let char_count = sel_text.len();
        let line_count = sel_text.lines().count();
        let sel_info = format!("({} chars, {} lines selected)", char_count, line_count);
        ui.painter().text(
            egui::Pos2::new(bar_rect.left() + 140.0, bar_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &sel_info,
            font.clone(),
            theme.status_bar_fg,
        );
    }

    // Macro recording indicator (red "REC")
    if macro_recording {
        let rec_x = bar_rect.left() + 340.0;
        ui.painter().text(
            egui::Pos2::new(rec_x, bar_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "REC",
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(255, 60, 60),
        );
    }

    // Read-only indicator
    if doc.read_only {
        let ro_x = bar_rect.left() + if macro_recording { 390.0 } else { 340.0 };
        ui.painter().text(
            egui::Pos2::new(ro_x, bar_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "READ-ONLY",
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(255, 180, 50),
        );
    }

    // Git branch indicator
    if let Some(branch) = git_branch {
        let git_x = bar_rect.left() + if doc.read_only { 440.0 } else if macro_recording { 390.0 } else { 340.0 };
        let branch_text = format!("⎇ {}", branch);
        ui.painter().text(
            egui::Pos2::new(git_x, bar_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &branch_text,
            egui::FontId::proportional(12.0),
            theme.status_bar_fg,
        );
    }

    let mut action = None;

    // Right side: clickable encoding, line ending, language
    // We use a child UI constrained to the bar rect for interactive widgets
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(bar_rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );

    child.spacing_mut().item_spacing.x = 4.0;
    child.add_space(8.0);

    // Language selector
    let lang = doc.language.as_deref().unwrap_or("Plain Text");
    let lang_resp = child.add(
        egui::Button::new(
            egui::RichText::new(lang).font(font.clone()).color(theme.status_bar_fg),
        )
        .frame(false),
    );
    lang_resp.context_menu(|ui| {
        for lang_name in COMMON_LANGUAGES {
            if ui.button(*lang_name).clicked() {
                action = Some(StatusBarAction::ChangeLanguage(lang_name.to_string()));
                ui.close_menu();
            }
        }
    });
    if lang_resp.clicked() {
        // Also open on left click — egui doesn't natively support this for context_menu,
        // so we use a popup instead
    }

    child.add_space(8.0);

    // Line ending selector
    let le = doc.line_ending.display_name();
    let le_resp = child.add(
        egui::Button::new(
            egui::RichText::new(le).font(font.clone()).color(theme.status_bar_fg),
        )
        .frame(false),
    );
    le_resp.context_menu(|ui| {
        for le_opt in [LineEnding::LF, LineEnding::CRLF, LineEnding::CR] {
            let selected = doc.line_ending == le_opt;
            let label = if selected {
                format!("  {} (current)", le_opt.display_name())
            } else {
                format!("  {}", le_opt.display_name())
            };
            if ui.button(&label).clicked() {
                action = Some(StatusBarAction::ChangeLineEnding(le_opt));
                ui.close_menu();
            }
        }
    });

    child.add_space(8.0);

    // Encoding selector
    let enc = doc.encoding.display_name();
    let enc_resp = child.add(
        egui::Button::new(
            egui::RichText::new(enc).font(font.clone()).color(theme.status_bar_fg),
        )
        .frame(false),
    );
    enc_resp.context_menu(|ui| {
        for enc_opt in Encoding::all() {
            let selected = doc.encoding == *enc_opt;
            let label = if selected {
                format!("  {} (current)", enc_opt.display_name())
            } else {
                format!("  {}", enc_opt.display_name())
            };
            if ui.button(&label).clicked() {
                action = Some(StatusBarAction::ChangeEncoding(*enc_opt));
                ui.close_menu();
            }
        }
    });

    (STATUS_BAR_HEIGHT, action)
}

/// Common languages shown in the language selector.
static COMMON_LANGUAGES: &[&str] = &[
    "Plain Text",
    "Bash",
    "C",
    "C++",
    "C#",
    "CSS",
    "Dart",
    "Dockerfile",
    "Elixir",
    "Erlang",
    "F#",
    "Go",
    "Haskell",
    "HTML",
    "INI",
    "Java",
    "JavaScript",
    "JSON",
    "Kotlin",
    "Lua",
    "Makefile",
    "Markdown",
    "OCaml",
    "PHP",
    "PowerShell",
    "Python",
    "R",
    "Ruby",
    "Rust",
    "SCSS",
    "SQL",
    "Swift",
    "TOML",
    "TSX",
    "TypeScript",
    "XML",
    "YAML",
    "Zig",
];
