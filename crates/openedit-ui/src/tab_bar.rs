use crate::theme::EditorTheme;
use egui::{self, Ui};

/// Actions available from the tab context menu.
#[derive(Debug, Clone)]
pub enum TabContextAction {
    CloseOthers,
    CloseAll,
    CloseToRight,
    CopyPath(String),
    RevealInFileManager(String),
}

/// Renders the tab bar and returns the index of the tab to activate (if changed),
/// and the index of a tab to close (if any).
pub struct TabBarResponse {
    pub activate: Option<usize>,
    pub close: Option<usize>,
    pub context_menu: Option<(usize, TabContextAction)>,
}

pub fn render_tab_bar(
    ui: &mut Ui,
    tabs: &[(String, bool, Option<String>)], // (display_name, is_modified, file_path)
    active_tab: usize,
    theme: &EditorTheme,
) -> TabBarResponse {
    let mut response = TabBarResponse {
        activate: None,
        close: None,
        context_menu: None,
    };

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;

        for (i, (name, modified, file_path)) in tabs.iter().enumerate() {
            let is_active = i == active_tab;
            let bg = if is_active {
                theme.tab_active_bg
            } else {
                theme.tab_inactive_bg
            };

            let label = if *modified {
                format!("{} \u{25CF}", name) // ● dot for modified
            } else {
                name.clone()
            };

            let frame = egui::Frame::none()
                .inner_margin(egui::Margin::symmetric(12.0, 6.0))
                .fill(bg);

            let resp = frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    let text = egui::RichText::new(&label)
                        .color(theme.tab_text)
                        .size(13.0);
                    let tab_resp = ui.label(text);

                    // Close button
                    let close_text = egui::RichText::new(" \u{00D7}") // ×
                        .color(theme.tab_text.linear_multiply(0.6))
                        .size(13.0);
                    if ui.label(close_text).on_hover_text("Close").clicked() {
                        response.close = Some(i);
                    }

                    tab_resp
                })
                .inner
            });

            // Click tab to activate
            if resp.response.clicked() {
                response.activate = Some(i);
            }

            // Middle-click to close
            if resp.response.middle_clicked() {
                response.close = Some(i);
            }

            // Right-click context menu
            let has_path = file_path.is_some();
            let file_path_clone = file_path.clone();
            let tab_count = tabs.len();
            resp.response.context_menu(|ui| {
                if ui.button("Close Others").clicked() {
                    response.context_menu = Some((i, TabContextAction::CloseOthers));
                    ui.close_menu();
                }
                if ui.button("Close All").clicked() {
                    response.context_menu = Some((i, TabContextAction::CloseAll));
                    ui.close_menu();
                }
                let can_close_right = i + 1 < tab_count;
                if ui.add_enabled(can_close_right, egui::Button::new("Close to the Right")).clicked() {
                    response.context_menu = Some((i, TabContextAction::CloseToRight));
                    ui.close_menu();
                }

                ui.separator();

                if ui.add_enabled(has_path, egui::Button::new("Copy Path")).clicked() {
                    if let Some(ref path) = file_path_clone {
                        response.context_menu = Some((i, TabContextAction::CopyPath(path.clone())));
                    }
                    ui.close_menu();
                }
                if ui.add_enabled(has_path, egui::Button::new("Reveal in File Manager")).clicked() {
                    if let Some(ref path) = file_path_clone {
                        response.context_menu = Some((i, TabContextAction::RevealInFileManager(path.clone())));
                    }
                    ui.close_menu();
                }
            });

            // Separator between tabs
            if i + 1 < tabs.len() {
                ui.separator();
            }
        }

        // "+" button for new tab
        let plus = egui::RichText::new(" + ")
            .color(theme.tab_text)
            .size(13.0);
        if ui.button(plus).on_hover_text("New Tab").clicked() {
            response.activate = Some(usize::MAX); // sentinel for "new tab"
        }
    });

    response
}
