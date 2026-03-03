//! Plugin management UI panel.
//!
//! Displays a window listing all registered plugins with their name, version,
//! description, and enabled/disabled status.  Users can toggle plugins on/off
//! and access the plugins folder from here.

use crate::theme::EditorTheme;
use egui;
use openedit_core::PluginManager;

/// State for the plugin management panel.
pub struct PluginPanelState {
    /// Whether the panel window is visible.
    pub visible: bool,
}

impl Default for PluginPanelState {
    fn default() -> Self {
        Self { visible: false }
    }
}

/// Actions that the plugin panel can request the application to perform.
pub enum PluginPanelAction {
    /// No action needed.
    None,
    /// Request the app to reload all plugins from disk.
    ReloadPlugins,
    /// Request the app to open the plugins directory in the OS file manager.
    OpenPluginsFolder,
}

/// Returns the platform-specific plugins directory path.
pub fn plugins_dir() -> std::path::PathBuf {
    let base = if let Some(config) = dirs_next_base_config() {
        config
    } else {
        std::path::PathBuf::from(".")
    };
    base.join("openedit").join("plugins")
}

/// Helper to get the platform config directory without pulling in the dirs crate.
fn dirs_next_base_config() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| std::path::PathBuf::from(h).join(".config"))
            })
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME").ok().map(|h| {
            std::path::PathBuf::from(h)
                .join("Library")
                .join("Application Support")
        })
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(std::path::PathBuf::from)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

/// Render the plugin management panel as an egui window.
///
/// Returns a `PluginPanelAction` indicating what the caller should do.
pub fn render_plugin_panel(
    ctx: &egui::Context,
    state: &mut PluginPanelState,
    manager: &mut PluginManager,
    theme: &EditorTheme,
) -> PluginPanelAction {
    let mut action = PluginPanelAction::None;
    let mut open = state.visible;

    egui::Window::new("Manage Plugins")
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size([450.0, 400.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            // Top toolbar
            ui.horizontal(|ui| {
                if ui.button("Reload Plugins").clicked() {
                    action = PluginPanelAction::ReloadPlugins;
                }
                if ui.button("Open Plugins Folder").clicked() {
                    action = PluginPanelAction::OpenPluginsFolder;
                }
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Plugin list
            let plugins = manager.list();

            if plugins.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(egui::RichText::new("No plugins installed.").color(theme.gutter_fg));
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(
                            "Place plugin crates in the plugins folder to get started.",
                        )
                        .small()
                        .color(theme.gutter_fg),
                    );
                    ui.add_space(4.0);
                    let dir = plugins_dir();
                    ui.label(
                        egui::RichText::new(dir.display().to_string())
                            .small()
                            .weak(),
                    );
                });
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Collect toggle changes so we can apply them after iteration
                        let mut toggles: Vec<(String, bool)> = Vec::new();

                        for (info, enabled) in &plugins {
                            egui::Frame::none()
                                .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                                .fill(if *enabled {
                                    egui::Color32::TRANSPARENT
                                } else {
                                    egui::Color32::from_black_alpha(20)
                                })
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        // Enable/disable toggle
                                        let mut is_enabled = *enabled;
                                        if ui.checkbox(&mut is_enabled, "").changed() {
                                            toggles.push((info.id.clone(), is_enabled));
                                        }

                                        ui.vertical(|ui| {
                                            // Name and version on same line
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new(&info.name)
                                                        .strong()
                                                        .color(theme.foreground),
                                                );
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "v{}",
                                                        info.version
                                                    ))
                                                    .small()
                                                    .weak(),
                                                );
                                            });

                                            // Description
                                            if !info.description.is_empty() {
                                                ui.label(
                                                    egui::RichText::new(&info.description)
                                                        .small()
                                                        .color(theme.gutter_fg),
                                                );
                                            }

                                            // Plugin ID in muted text
                                            ui.label(
                                                egui::RichText::new(&info.id)
                                                    .small()
                                                    .weak()
                                                    .monospace(),
                                            );
                                        });
                                    });
                                });

                            ui.separator();
                        }

                        // Apply toggle changes
                        for (id, new_enabled) in toggles {
                            manager.set_enabled(&id, new_enabled);
                        }
                    });
            }

            // Footer with count
            ui.add_space(4.0);
            ui.separator();
            ui.horizontal(|ui| {
                let total = plugins.len();
                let enabled_count = plugins.iter().filter(|(_, e)| *e).count();
                ui.label(
                    egui::RichText::new(format!(
                        "{} plugin{} ({} enabled)",
                        total,
                        if total == 1 { "" } else { "s" },
                        enabled_count,
                    ))
                    .small()
                    .weak(),
                );
            });
        });

    state.visible = open;
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_panel_state_default() {
        let state = PluginPanelState::default();
        assert!(!state.visible);
    }

    #[test]
    fn test_plugins_dir_returns_path() {
        let dir = plugins_dir();
        // Should end with "openedit/plugins" regardless of platform
        let path_str = dir.to_string_lossy();
        assert!(
            path_str.contains("openedit") && path_str.ends_with("plugins"),
            "Expected path containing openedit/plugins, got: {}",
            path_str,
        );
    }
}
