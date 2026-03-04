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

/// Drag state for tab reordering
#[derive(Default)]
pub struct TabDragState {
    pub dragging_tab: Option<usize>,
    pub target_index: Option<usize>,
}

/// Renders the tab bar and returns the index of the tab to activate (if changed),
/// and the index of a tab to close (if any).
pub struct TabBarResponse {
    pub activate: Option<usize>,
    pub close: Option<usize>,
    pub context_menu: Option<(usize, TabContextAction)>,
    pub reorder: Option<(usize, usize)>, // (from, to)
}

pub fn render_tab_bar(
    ui: &mut Ui,
    tabs: &[(String, bool, Option<String>)], // (display_name, is_modified, file_path)
    active_tab: usize,
    theme: &EditorTheme,
    drag_state: &mut TabDragState,
) -> TabBarResponse {
    let mut response = TabBarResponse {
        activate: None,
        close: None,
        context_menu: None,
        reorder: None,
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
                    let text = egui::RichText::new(&label).color(theme.tab_text).size(13.0);
                    let tab_resp =
                        ui.add(egui::Label::new(text).sense(egui::Sense::click_and_drag()));

                    // Close button — use a Button so it gets its own interaction rect
                    let close_text =
                        egui::RichText::new(" \u{00D7}") // ×
                            .color(theme.tab_text.linear_multiply(0.6))
                            .size(13.0);
                    let close_btn = egui::Button::new(close_text).frame(false).small();
                    if ui.add(close_btn).on_hover_text("Close").clicked() {
                        response.close = Some(i);
                    }

                    tab_resp
                })
                .inner
            });

            // Handle drag start on the tab label itself
            if resp.inner.drag_started() {
                drag_state.dragging_tab = Some(i);
            }

            // Handle drag and drop for reordering
            if let Some(dragging_idx) = drag_state.dragging_tab {
                let cursor_pos = ui.input(|i| i.pointer.interact_pos());
                if dragging_idx != i
                    && cursor_pos.is_some_and(|pos| resp.response.rect.contains(pos))
                {
                    drag_state.target_index = Some(i);
                }
            }

            // Click tab to activate (only if not dragging and not closing)
            if response.close != Some(i)
                && resp.inner.clicked()
                && drag_state.dragging_tab.is_none()
            {
                response.activate = Some(i);
            }

            // Middle-click to close
            if resp.inner.middle_clicked() {
                response.close = Some(i);
            }

            // Right-click context menu
            let has_path = file_path.is_some();
            let file_path_clone = file_path.clone();
            let tab_count = tabs.len();
            resp.inner.context_menu(|ui| {
                if ui.button("Close Others").clicked() {
                    response.context_menu = Some((i, TabContextAction::CloseOthers));
                    ui.close_menu();
                }
                if ui.button("Close All").clicked() {
                    response.context_menu = Some((i, TabContextAction::CloseAll));
                    ui.close_menu();
                }
                let can_close_right = i + 1 < tab_count;
                if ui
                    .add_enabled(can_close_right, egui::Button::new("Close to the Right"))
                    .clicked()
                {
                    response.context_menu = Some((i, TabContextAction::CloseToRight));
                    ui.close_menu();
                }

                ui.separator();

                if ui
                    .add_enabled(has_path, egui::Button::new("Copy Path"))
                    .clicked()
                {
                    if let Some(ref path) = file_path_clone {
                        response.context_menu = Some((i, TabContextAction::CopyPath(path.clone())));
                    }
                    ui.close_menu();
                }
                if ui
                    .add_enabled(has_path, egui::Button::new("Reveal in File Manager"))
                    .clicked()
                {
                    if let Some(ref path) = file_path_clone {
                        response.context_menu =
                            Some((i, TabContextAction::RevealInFileManager(path.clone())));
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
        let plus = egui::RichText::new(" + ").color(theme.tab_text).size(13.0);
        if ui.button(plus).on_hover_text("New Tab").clicked() {
            response.activate = Some(usize::MAX); // sentinel for "new tab"
        }
    });

    // Handle drag end and reorder
    if let (Some(from_idx), Some(to_idx)) = (drag_state.dragging_tab, drag_state.target_index) {
        if from_idx != to_idx {
            response.reorder = Some((from_idx, to_idx));
        }
        drag_state.dragging_tab = None;
        drag_state.target_index = None;
    }

    response
}
