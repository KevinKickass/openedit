use crate::git::{git_status_indicator, FileGitStatus, GitManager};
use crate::theme::EditorTheme;
use egui;
use std::path::PathBuf;

/// Directories and file prefixes to hide in the file tree.
const HIDDEN_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "build",
    "dist",
    "__pycache__",
    ".idea",
    ".vscode",
];

/// Represents a node in the file tree.
pub struct FileTreeNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub children: Vec<FileTreeNode>,
    pub expanded: bool,
    /// Whether children have been loaded (for lazy loading).
    pub children_loaded: bool,
}

/// State for the file explorer sidebar.
pub struct SidebarState {
    pub visible: bool,
    pub root: Option<FileTreeNode>,
    pub width: f32,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            visible: false,
            root: None,
            width: 200.0,
        }
    }
}

impl SidebarState {
    /// Build the file tree from a root directory.
    pub fn load_tree(&mut self, root_path: &PathBuf) {
        self.root = Some(build_tree_node(root_path, true));
    }
}

/// Build a single tree node. If `load_children` is true, also load immediate children.
fn build_tree_node(path: &PathBuf, load_children: bool) -> FileTreeNode {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());

    let is_dir = path.is_dir();

    let mut node = FileTreeNode {
        name,
        path: path.clone(),
        is_dir,
        children: Vec::new(),
        expanded: false,
        children_loaded: false,
    };

    if is_dir && load_children {
        node.children = load_directory_children(path);
        node.children_loaded = true;
        // Auto-expand root
        node.expanded = true;
    }

    node
}

/// Load the children of a directory, sorted (directories first, then files, alphabetically).
fn load_directory_children(dir: &PathBuf) -> Vec<FileTreeNode> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut dirs: Vec<FileTreeNode> = Vec::new();
    let mut files: Vec<FileTreeNode> = Vec::new();

    for entry in entries.flatten() {
        let entry_path = entry.path();
        let file_name = entry.file_name().to_string_lossy().into_owned();

        // Skip hidden files (starting with .)
        if file_name.starts_with('.') {
            continue;
        }

        // Skip filtered directories
        if entry_path.is_dir() && HIDDEN_DIRS.contains(&file_name.as_str()) {
            continue;
        }

        let node = FileTreeNode {
            name: file_name,
            path: entry_path.clone(),
            is_dir: entry_path.is_dir(),
            children: Vec::new(),
            expanded: false,
            children_loaded: false,
        };

        if node.is_dir {
            dirs.push(node);
        } else {
            files.push(node);
        }
    }

    // Sort alphabetically (case-insensitive)
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    // Directories first, then files
    dirs.extend(files);
    dirs
}

/// Render the sidebar. Returns `Some(path)` if a file was clicked to open.
pub fn render_sidebar(
    ui: &mut egui::Ui,
    state: &mut SidebarState,
    theme: &EditorTheme,
    _font_size: f32,
    git_manager: Option<&GitManager>,
) -> Option<PathBuf> {
    // Fill background
    let rect = ui.available_rect_before_wrap();
    ui.painter().rect_filled(rect, 0.0, theme.gutter_bg);

    let mut clicked_file = None;

    // Header
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("EXPLORER")
                .small()
                .color(theme.gutter_fg),
        );
    });

    ui.add_space(4.0);

    // Scrollable tree content
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if let Some(ref mut root) = state.root {
                // Render children of root directly (don't show root directory node itself)
                let children = &mut root.children;
                for child in children.iter_mut() {
                    if let Some(path) = render_tree_node(ui, child, 0, theme, git_manager) {
                        clicked_file = Some(path);
                    }
                }
            } else {
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("No folder open")
                            .color(theme.gutter_fg)
                            .italics(),
                    );
                });
            }
        });

    clicked_file
}

/// Render a single tree node recursively. Returns `Some(path)` if a file was clicked.
fn render_tree_node(
    ui: &mut egui::Ui,
    node: &mut FileTreeNode,
    depth: usize,
    theme: &EditorTheme,
    git_manager: Option<&GitManager>,
) -> Option<PathBuf> {
    let indent = depth as f32 * 16.0 + 4.0;
    let mut clicked_file = None;

    let response = ui.horizontal(|ui| {
        ui.add_space(indent);

        if node.is_dir {
            let arrow = if node.expanded {
                "\u{25BC}"
            } else {
                "\u{25B6}"
            };
            let arrow_response = ui.add(
                egui::Label::new(egui::RichText::new(arrow).size(10.0).color(theme.gutter_fg))
                    .sense(egui::Sense::click()),
            );

            let folder_icon = if node.expanded {
                "[-]"
            } else {
                "[+]"
            };
            let label_response = ui.add(
                egui::Label::new(
                    egui::RichText::new(format!("{} {}", folder_icon, &node.name))
                        .color(theme.foreground),
                )
                .sense(egui::Sense::click()),
            );

            if arrow_response.clicked() || label_response.clicked() {
                node.expanded = !node.expanded;
                if node.expanded && !node.children_loaded {
                    node.children = load_directory_children(&node.path);
                    node.children_loaded = true;
                }
            }
        } else {
            // Alignment space matching arrow width
            ui.add_space(14.0);

            let file_icon = file_icon_for_name(&node.name);
            let label_response = ui.add(
                egui::Label::new(
                    egui::RichText::new(format!("{} {}", file_icon, &node.name))
                        .color(theme.foreground),
                )
                .sense(egui::Sense::click()),
            );

            if label_response.clicked() {
                return Some(node.path.clone());
            }

            // Git status indicator for files
            if let Some(gm) = git_manager {
                let status = gm.get_file_status(&node.path);
                if status != FileGitStatus::Unchanged {
                    let (label, color) = git_status_indicator(status);
                    ui.label(egui::RichText::new(label).small().color(color));
                }
            }
        }

        None
    });

    if let Some(path) = response.inner {
        clicked_file = Some(path);
    }

    // Render expanded children
    if node.is_dir && node.expanded {
        for child in &mut node.children {
            if let Some(path) = render_tree_node(ui, child, depth + 1, theme, git_manager) {
                clicked_file = Some(path);
            }
        }
    }

    clicked_file
}

/// Get a simple text icon based on file extension.
fn file_icon_for_name(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext.to_lowercase().as_str() {
        "rs" => "[rs]",
        "py" => "[py]",
        "js" | "mjs" | "cjs" => "[js]",
        "ts" | "mts" | "cts" => "[ts]",
        "json" => "[{}]",
        "toml" | "yaml" | "yml" | "ini" | "cfg" => "[cf]",
        "md" | "txt" | "rst" => "[tx]",
        "html" | "htm" => "[ht]",
        "css" | "scss" | "sass" => "[cs]",
        "lock" => "[lk]",
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" | "bmp" | "webp" => "[im]",
        _ => " ",
    }
}
