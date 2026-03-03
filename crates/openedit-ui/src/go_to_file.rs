use egui;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// State for the "Go to File" (Ctrl+P) fuzzy file finder dialog.
#[derive(Default)]
pub struct GoToFileState {
    pub open: bool,
    pub query: String,
    pub selected: usize,
    /// All files found during scan (relative to root).
    pub files: Vec<PathBuf>,
    /// Indices into `files` that match the current query.
    pub filtered: Vec<usize>,
}

impl GoToFileState {
    /// Scan the given root directory for files, storing relative paths.
    pub fn scan(&mut self, root: &Path) {
        self.files = scan_files(root);
        self.update_filtered();
    }

    /// Recompute the filtered list based on the current query.
    pub fn update_filtered(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..self.files.len()).collect();
        } else {
            self.filtered = self
                .files
                .iter()
                .enumerate()
                .filter(|(_, path)| fuzzy_match_path(&self.query, path))
                .map(|(i, _)| i)
                .collect();
        }
        // Clamp selection
        if self.filtered.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len() - 1;
        }
    }
}

/// Directories to skip during scanning.
const SKIP_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "target",
    "node_modules",
    "build",
    "dist",
    "__pycache__",
    ".idea",
    ".vscode",
];

/// Maximum depth for directory traversal.
const MAX_DEPTH: usize = 10;

/// Maximum number of files to collect.
const MAX_FILES: usize = 10000;

/// Recursively walk `root`, collecting regular file paths relative to `root`.
/// Respects skip-directory rules, max depth, and max file count.
fn scan_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    let walker = WalkDir::new(root)
        .max_depth(MAX_DEPTH)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            // Skip hidden directories and known junk directories
            if entry.file_type().is_dir() {
                let name = entry.file_name().to_string_lossy();
                if name.starts_with('.') && name != "." {
                    return false;
                }
                if SKIP_DIRS.contains(&name.as_ref()) {
                    return false;
                }
            }
            true
        });

    for entry in walker {
        if files.len() >= MAX_FILES {
            break;
        }
        let Ok(entry) = entry else {
            continue;
        };
        if !entry.file_type().is_file() {
            continue;
        }
        if let Ok(rel) = entry.path().strip_prefix(root) {
            files.push(rel.to_path_buf());
        }
    }

    // Sort by filename for a predictable initial order
    files.sort_by(|a, b| {
        let a_name = a.file_name().unwrap_or_default();
        let b_name = b.file_name().unwrap_or_default();
        a_name.cmp(b_name)
    });

    files
}

/// Fuzzy match: all query characters must appear in the file name in order (case insensitive).
/// Matches against the file name portion only, not the full path.
fn fuzzy_match_path(query: &str, path: &Path) -> bool {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let query_lower = query.to_lowercase();
    let mut name_chars = name.chars();
    for qc in query_lower.chars() {
        if !name_chars.any(|nc| nc == qc) {
            return false;
        }
    }
    true
}

/// Render the "Go to File" dialog. Returns the absolute path of the selected file, if any.
/// The caller must combine the returned relative path with the workspace root.
pub fn render_go_to_file(ctx: &egui::Context, state: &mut GoToFileState) -> Option<PathBuf> {
    let max_visible = 20;

    let mut selected_path: Option<PathBuf> = None;
    let mut open = state.open;

    egui::Window::new("Go to File")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 60.0])
        .fixed_size([500.0, 350.0])
        .show(ctx, |ui| {
            // Search input
            let response = ui.add(
                egui::TextEdit::singleline(&mut state.query)
                    .hint_text("Search files...")
                    .desired_width(f32::INFINITY),
            );
            response.request_focus();

            // Handle arrow keys and enter
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            let up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
            let down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));

            if up && state.selected > 0 {
                state.selected -= 1;
            }
            if down && state.selected + 1 < state.filtered.len() {
                state.selected += 1;
            }
            if enter && !state.filtered.is_empty() {
                let file_idx = state.filtered[state.selected];
                selected_path = Some(state.files[file_idx].clone());
                state.open = false;
                return;
            }

            // Update filtering when query changes
            if response.changed() {
                state.update_filtered();
            }

            ui.separator();

            // File count info
            let count_text = if state.query.is_empty() {
                format!("{} files", state.filtered.len())
            } else {
                format!("{} / {} files", state.filtered.len(), state.files.len())
            };
            ui.label(egui::RichText::new(count_text).weak().small());

            // File list
            egui::ScrollArea::vertical()
                .max_height(280.0)
                .show(ui, |ui| {
                    let visible_count = state.filtered.len().min(max_visible);
                    // Determine scroll window around selection
                    let scroll_start = if state.selected >= visible_count {
                        state.selected - visible_count + 1
                    } else {
                        0
                    };
                    let scroll_end = (scroll_start + max_visible).min(state.filtered.len());

                    for display_i in scroll_start..scroll_end {
                        let file_idx = state.filtered[display_i];
                        let path = &state.files[file_idx];
                        let is_selected = display_i == state.selected;

                        let bg = if is_selected {
                            egui::Color32::from_rgb(60, 60, 80)
                        } else {
                            egui::Color32::TRANSPARENT
                        };

                        let frame_response = egui::Frame::none()
                            .fill(bg)
                            .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    // File name (bold)
                                    let file_name =
                                        path.file_name().unwrap_or_default().to_string_lossy();
                                    ui.label(egui::RichText::new(file_name.as_ref()).strong());

                                    // Relative directory path (dimmed)
                                    if let Some(parent) = path.parent() {
                                        let parent_str = parent.to_string_lossy();
                                        if !parent_str.is_empty() {
                                            ui.label(
                                                egui::RichText::new(parent_str.as_ref()).weak(),
                                            );
                                        }
                                    }
                                });
                            });

                        let response = frame_response.response.interact(egui::Sense::click());
                        if response.clicked() {
                            selected_path = Some(state.files[file_idx].clone());
                            state.open = false;
                        }
                        if response.hovered() {
                            state.selected = display_i;
                        }
                    }
                });
        });

    state.open = open;
    selected_path
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_fuzzy_match_path_basic() {
        assert!(fuzzy_match_path("main", Path::new("src/main.rs")));
        assert!(fuzzy_match_path("mr", Path::new("src/main.rs")));
        assert!(fuzzy_match_path("MR", Path::new("src/main.rs")));
        assert!(!fuzzy_match_path("xyz", Path::new("src/main.rs")));
    }

    #[test]
    fn test_fuzzy_match_path_empty_query() {
        assert!(fuzzy_match_path("", Path::new("anything.txt")));
    }

    #[test]
    fn test_fuzzy_match_path_exact() {
        assert!(fuzzy_match_path(
            "lib.rs",
            Path::new("crates/core/src/lib.rs")
        ));
    }

    #[test]
    fn test_fuzzy_match_path_order_matters() {
        // Characters must appear in order
        assert!(fuzzy_match_path("abc", Path::new("a_b_c.txt")));
        assert!(!fuzzy_match_path("cba", Path::new("a_b_c.txt")));
    }

    #[test]
    fn test_default_state() {
        let state = GoToFileState::default();
        assert!(!state.open);
        assert!(state.query.is_empty());
        assert_eq!(state.selected, 0);
        assert!(state.files.is_empty());
        assert!(state.filtered.is_empty());
    }

    #[test]
    fn test_update_filtered_empty_query() {
        let mut state = GoToFileState::default();
        state.files = vec![
            PathBuf::from("a.rs"),
            PathBuf::from("b.rs"),
            PathBuf::from("c.rs"),
        ];
        state.update_filtered();
        assert_eq!(state.filtered.len(), 3);
    }

    #[test]
    fn test_update_filtered_with_query() {
        let mut state = GoToFileState::default();
        state.files = vec![
            PathBuf::from("main.rs"),
            PathBuf::from("lib.rs"),
            PathBuf::from("utils.rs"),
        ];
        state.query = "lib".to_string();
        state.update_filtered();
        assert_eq!(state.filtered.len(), 1);
        assert_eq!(state.filtered[0], 1);
    }

    #[test]
    fn test_update_filtered_clamps_selection() {
        let mut state = GoToFileState::default();
        state.files = vec![PathBuf::from("a.rs"), PathBuf::from("b.rs")];
        state.selected = 5;
        state.update_filtered();
        assert_eq!(state.selected, 1); // clamped to last index
    }

    #[test]
    fn test_update_filtered_no_matches_clamps_to_zero() {
        let mut state = GoToFileState::default();
        state.files = vec![PathBuf::from("a.rs")];
        state.query = "zzz".to_string();
        state.selected = 3;
        state.update_filtered();
        assert_eq!(state.selected, 0);
        assert!(state.filtered.is_empty());
    }
}
