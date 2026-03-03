use egui;

/// A command that can be executed from the palette.
#[derive(Clone)]
pub struct Command {
    pub id: &'static str,
    pub label: &'static str,
    pub shortcut: &'static str,
}

/// All available commands.
pub fn all_commands() -> Vec<Command> {
    vec![
        // File operations
        Command {
            id: "file.new",
            label: "New File",
            shortcut: "Ctrl+N",
        },
        Command {
            id: "file.open",
            label: "Open File",
            shortcut: "Ctrl+O",
        },
        Command {
            id: "file.save",
            label: "Save",
            shortcut: "Ctrl+S",
        },
        Command {
            id: "file.save_as",
            label: "Save As",
            shortcut: "Ctrl+Shift+S",
        },
        Command {
            id: "file.close_tab",
            label: "Close Tab",
            shortcut: "Ctrl+W",
        },
        Command {
            id: "file.recent_files",
            label: "Open Recent File",
            shortcut: "",
        },
        // Edit operations
        Command {
            id: "edit.undo",
            label: "Undo",
            shortcut: "Ctrl+Z",
        },
        Command {
            id: "edit.redo",
            label: "Redo",
            shortcut: "Ctrl+Y",
        },
        Command {
            id: "edit.select_all",
            label: "Select All",
            shortcut: "Ctrl+A",
        },
        Command {
            id: "edit.select_next_occurrence",
            label: "Select Next Occurrence",
            shortcut: "Ctrl+D",
        },
        Command {
            id: "edit.duplicate_line",
            label: "Duplicate Line",
            shortcut: "",
        },
        Command {
            id: "edit.delete_line",
            label: "Delete Line",
            shortcut: "Ctrl+Shift+K",
        },
        Command {
            id: "edit.move_line_up",
            label: "Move Line Up",
            shortcut: "Alt+Up",
        },
        Command {
            id: "edit.move_line_down",
            label: "Move Line Down",
            shortcut: "Alt+Down",
        },
        Command {
            id: "edit.indent",
            label: "Indent",
            shortcut: "Tab",
        },
        Command {
            id: "edit.unindent",
            label: "Unindent",
            shortcut: "Shift+Tab",
        },
        Command {
            id: "edit.toggle_comment",
            label: "Toggle Comment",
            shortcut: "Ctrl+/",
        },
        Command {
            id: "edit.column_editor",
            label: "Column Editor",
            shortcut: "",
        },
        // Navigation
        Command {
            id: "nav.go_to_file",
            label: "Go to File",
            shortcut: "Ctrl+P",
        },
        Command {
            id: "nav.go_to_line",
            label: "Go to Line",
            shortcut: "Ctrl+G",
        },
        Command {
            id: "nav.go_to_symbol",
            label: "Go to Symbol",
            shortcut: "Ctrl+Shift+O",
        },
        Command {
            id: "nav.find",
            label: "Find",
            shortcut: "Ctrl+F",
        },
        Command {
            id: "nav.replace",
            label: "Find and Replace",
            shortcut: "Ctrl+H",
        },
        Command {
            id: "nav.find_in_files",
            label: "Find in Files",
            shortcut: "Ctrl+Shift+F",
        },
        Command {
            id: "nav.next_tab",
            label: "Next Tab",
            shortcut: "Ctrl+Tab",
        },
        Command {
            id: "nav.prev_tab",
            label: "Previous Tab",
            shortcut: "Ctrl+Shift+Tab",
        },
        // Bookmarks
        Command {
            id: "nav.toggle_bookmark",
            label: "Toggle Bookmark",
            shortcut: "Ctrl+F2",
        },
        Command {
            id: "nav.next_bookmark",
            label: "Next Bookmark",
            shortcut: "F2",
        },
        Command {
            id: "nav.prev_bookmark",
            label: "Previous Bookmark",
            shortcut: "Shift+F2",
        },
        Command {
            id: "nav.clear_bookmarks",
            label: "Clear All Bookmarks",
            shortcut: "",
        },
        // Text tools
        Command {
            id: "tools.sort_asc",
            label: "Sort Lines (Ascending)",
            shortcut: "",
        },
        Command {
            id: "tools.sort_desc",
            label: "Sort Lines (Descending)",
            shortcut: "",
        },
        Command {
            id: "tools.sort_case_insensitive",
            label: "Sort Lines (Case Insensitive)",
            shortcut: "",
        },
        Command {
            id: "tools.sort_numeric",
            label: "Sort Lines (Numeric)",
            shortcut: "",
        },
        Command {
            id: "tools.uppercase",
            label: "Transform to Uppercase",
            shortcut: "",
        },
        Command {
            id: "tools.lowercase",
            label: "Transform to Lowercase",
            shortcut: "",
        },
        Command {
            id: "tools.title_case",
            label: "Transform to Title Case",
            shortcut: "",
        },
        Command {
            id: "tools.remove_duplicates",
            label: "Remove Duplicate Lines",
            shortcut: "",
        },
        Command {
            id: "tools.remove_empty",
            label: "Remove Empty Lines",
            shortcut: "",
        },
        Command {
            id: "tools.join_lines",
            label: "Join Lines",
            shortcut: "",
        },
        Command {
            id: "tools.reverse_lines",
            label: "Reverse Lines",
            shortcut: "",
        },
        Command {
            id: "tools.trim_trailing",
            label: "Trim Trailing Whitespace",
            shortcut: "",
        },
        // Encoding / JSON transforms
        Command {
            id: "tools.base64_encode",
            label: "Base64 Encode",
            shortcut: "",
        },
        Command {
            id: "tools.base64_decode",
            label: "Base64 Decode",
            shortcut: "",
        },
        Command {
            id: "tools.url_encode",
            label: "URL Encode",
            shortcut: "",
        },
        Command {
            id: "tools.url_decode",
            label: "URL Decode",
            shortcut: "",
        },
        Command {
            id: "tools.json_pretty",
            label: "JSON Pretty Print",
            shortcut: "",
        },
        Command {
            id: "tools.json_minify",
            label: "JSON Minify",
            shortcut: "",
        },
        Command {
            id: "tools.xml_pretty",
            label: "XML: Pretty Print",
            shortcut: "",
        },
        Command {
            id: "tools.xml_minify",
            label: "XML: Minify",
            shortcut: "",
        },
        // Hash
        Command {
            id: "tools.hash_md5",
            label: "Hash: MD5",
            shortcut: "",
        },
        Command {
            id: "tools.hash_sha1",
            label: "Hash: SHA-1",
            shortcut: "",
        },
        Command {
            id: "tools.hash_sha256",
            label: "Hash: SHA-256",
            shortcut: "",
        },
        // HTML entities
        Command {
            id: "tools.html_encode",
            label: "HTML Entity Encode",
            shortcut: "",
        },
        Command {
            id: "tools.html_decode",
            label: "HTML Entity Decode",
            shortcut: "",
        },
        // Conversion
        Command {
            id: "tools.dec_to_hex",
            label: "Convert Decimal to Hex",
            shortcut: "",
        },
        Command {
            id: "tools.hex_to_dec",
            label: "Convert Hex to Decimal",
            shortcut: "",
        },
        Command {
            id: "tools.timestamp_to_date",
            label: "Convert Unix Timestamp to Date",
            shortcut: "",
        },
        // Case conversions
        Command {
            id: "tools.camel_case",
            label: "Transform to camelCase",
            shortcut: "",
        },
        Command {
            id: "tools.snake_case",
            label: "Transform to snake_case",
            shortcut: "",
        },
        Command {
            id: "tools.pascal_case",
            label: "Transform to PascalCase",
            shortcut: "",
        },
        Command {
            id: "tools.kebab_case",
            label: "Transform to kebab-case",
            shortcut: "",
        },
        // View
        Command {
            id: "view.toggle_word_wrap",
            label: "Toggle Word Wrap",
            shortcut: "",
        },
        Command {
            id: "view.zoom_in",
            label: "Zoom In",
            shortcut: "Ctrl+=",
        },
        Command {
            id: "view.zoom_out",
            label: "Zoom Out",
            shortcut: "Ctrl+-",
        },
        Command {
            id: "view.zoom_reset",
            label: "Reset Zoom",
            shortcut: "Ctrl+0",
        },
        Command {
            id: "view.toggle_whitespace",
            label: "Show/Hide Whitespace",
            shortcut: "",
        },
        Command {
            id: "view.toggle_theme",
            label: "Toggle Light/Dark Theme",
            shortcut: "",
        },
        Command {
            id: "view.toggle_minimap",
            label: "Toggle Minimap",
            shortcut: "",
        },
        Command {
            id: "view.toggle_markdown_preview",
            label: "Toggle Markdown Preview",
            shortcut: "Ctrl+Shift+M",
        },
        Command {
            id: "view.toggle_sidebar",
            label: "Toggle File Explorer",
            shortcut: "Ctrl+B",
        },
        Command {
            id: "view.toggle_function_list",
            label: "Toggle Function List",
            shortcut: "",
        },
        // Split view
        Command {
            id: "view.split_horizontal",
            label: "Split Editor Right",
            shortcut: "",
        },
        Command {
            id: "view.split_vertical",
            label: "Split Editor Down",
            shortcut: "",
        },
        Command {
            id: "view.close_split",
            label: "Close Split Pane",
            shortcut: "",
        },
        // Folding
        Command {
            id: "view.fold_toggle",
            label: "Toggle Fold",
            shortcut: "",
        },
        Command {
            id: "view.fold_all",
            label: "Fold All",
            shortcut: "",
        },
        Command {
            id: "view.unfold_all",
            label: "Unfold All",
            shortcut: "",
        },
        // Hex editor
        Command {
            id: "view.toggle_hex",
            label: "Toggle Hex Editor",
            shortcut: "",
        },
        Command {
            id: "hex.go_to_offset",
            label: "Hex: Go to Offset",
            shortcut: "Ctrl+G (in hex mode)",
        },
        // Diff/compare
        Command {
            id: "view.compare_files",
            label: "Compare Open Files",
            shortcut: "",
        },
        Command {
            id: "view.close_compare",
            label: "Close Compare View",
            shortcut: "",
        },
        Command {
            id: "diff.next_hunk",
            label: "Diff: Next Change",
            shortcut: "F7",
        },
        Command {
            id: "diff.prev_hunk",
            label: "Diff: Previous Change",
            shortcut: "Shift+F7",
        },
        // Terminal
        Command {
            id: "view.toggle_terminal",
            label: "Toggle Terminal",
            shortcut: "Ctrl+`",
        },
        Command {
            id: "terminal.new",
            label: "Terminal: New Terminal",
            shortcut: "",
        },
        Command {
            id: "terminal.send_selection",
            label: "Terminal: Send Selection",
            shortcut: "",
        },
        // Git
        Command {
            id: "view.toggle_git_blame",
            label: "Toggle Git Blame",
            shortcut: "",
        },
        // Bracket colors
        Command {
            id: "view.toggle_bracket_colors",
            label: "Toggle Bracket Pair Colorization",
            shortcut: "",
        },
        // Multi-cursor
        Command {
            id: "edit.select_all_occurrences",
            label: "Select All Occurrences",
            shortcut: "Ctrl+Shift+L",
        },
        // Read-only
        Command {
            id: "edit.toggle_read_only",
            label: "Toggle Read-Only Mode",
            shortcut: "",
        },
        // Macro recording
        Command {
            id: "macro.toggle_recording",
            label: "Start/Stop Macro Recording",
            shortcut: "Ctrl+Q",
        },
        Command {
            id: "macro.playback",
            label: "Playback Macro",
            shortcut: "Ctrl+Shift+Q",
        },
        Command {
            id: "macro.run_multiple",
            label: "Run Macro Multiple Times...",
            shortcut: "",
        },
        Command {
            id: "macro.save_as",
            label: "Save Macro As...",
            shortcut: "",
        },
        Command {
            id: "macro.load",
            label: "Load Macro...",
            shortcut: "",
        },
        // Read-only
        Command {
            id: "edit.toggle_read_only",
            label: "Toggle Read-Only Mode",
            shortcut: "",
        },
        // Vim mode
        Command {
            id: "edit.toggle_vim_mode",
            label: "Toggle Vim Mode",
            shortcut: "",
        },
        // Zen mode
        Command {
            id: "view.zen_mode",
            label: "Toggle Zen Mode",
            shortcut: "F11",
        },
        // Theme selector (individual themes)
        Command {
            id: "view.theme.monokai",
            label: "Theme: Monokai",
            shortcut: "",
        },
        Command {
            id: "view.theme.dracula",
            label: "Theme: Dracula",
            shortcut: "",
        },
        Command {
            id: "view.theme.solarized_dark",
            label: "Theme: Solarized Dark",
            shortcut: "",
        },
        Command {
            id: "view.theme.solarized_light",
            label: "Theme: Solarized Light",
            shortcut: "",
        },
        Command {
            id: "view.theme.nord",
            label: "Theme: Nord",
            shortcut: "",
        },
        Command {
            id: "view.theme.one_dark",
            label: "Theme: One Dark",
            shortcut: "",
        },
        Command {
            id: "view.theme.gruvbox",
            label: "Theme: Gruvbox",
            shortcut: "",
        },
        Command {
            id: "view.theme.tokyo_night",
            label: "Theme: Tokyo Night",
            shortcut: "",
        },
        Command {
            id: "view.theme.dark",
            label: "Theme: Dark (Default)",
            shortcut: "",
        },
        Command {
            id: "view.theme.light",
            label: "Theme: Light",
            shortcut: "",
        },
    ]
}

/// State for the command palette.
pub struct CommandPaletteState {
    pub open: bool,
    pub query: String,
    pub selected: usize,
}

impl Default for CommandPaletteState {
    fn default() -> Self {
        Self {
            open: false,
            query: String::new(),
            selected: 0,
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

/// Render the command palette. Returns the ID of the selected command, if any.
pub fn render_command_palette(
    ctx: &egui::Context,
    state: &mut CommandPaletteState,
) -> Option<&'static str> {
    let commands = all_commands();
    let filtered: Vec<&Command> = if state.query.is_empty() {
        commands.iter().collect()
    } else {
        commands
            .iter()
            .filter(|c| fuzzy_match(&state.query, c.label))
            .collect()
    };

    // Clamp selection
    if state.selected >= filtered.len() {
        state.selected = filtered.len().saturating_sub(1);
    }

    let mut executed: Option<&'static str> = None;
    let mut open = state.open;

    egui::Window::new("Command Palette")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 60.0])
        .fixed_size([400.0, 300.0])
        .show(ctx, |ui| {
            // Search input
            let response = ui.add(
                egui::TextEdit::singleline(&mut state.query)
                    .hint_text("Type a command...")
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
            if down && state.selected + 1 < filtered.len() {
                state.selected += 1;
            }
            if enter && !filtered.is_empty() {
                executed = Some(filtered[state.selected].id);
                state.open = false;
                return;
            }

            // Reset selection when query changes
            if response.changed() {
                state.selected = 0;
            }

            ui.separator();

            // Command list
            egui::ScrollArea::vertical()
                .max_height(250.0)
                .show(ui, |ui| {
                    for (i, cmd) in filtered.iter().enumerate() {
                        let is_selected = i == state.selected;
                        let bg = if is_selected {
                            egui::Color32::from_rgb(60, 60, 80)
                        } else {
                            egui::Color32::TRANSPARENT
                        };

                        egui::Frame::none()
                            .fill(bg)
                            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(cmd.label);
                                    if !cmd.shortcut.is_empty() {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(egui::RichText::new(cmd.shortcut).weak());
                                            },
                                        );
                                    }
                                });
                            });

                        let rect = ui.min_rect();
                        let response = ui.interact(rect, ui.id().with(i), egui::Sense::click());
                        if response.clicked() {
                            executed = Some(cmd.id);
                            state.open = false;
                        }
                        if response.hovered() {
                            state.selected = i;
                        }
                    }
                });
        });

    state.open = open;
    executed
}
