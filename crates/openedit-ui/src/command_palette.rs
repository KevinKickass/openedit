use egui;

/// A command that can be executed from the palette.
#[derive(Clone)]
pub struct Command {
    pub id: String,
    pub label: String,
    pub shortcut: &'static str,
}

/// All available built-in commands.
pub fn all_commands() -> Vec<Command> {
    vec![
        // File operations
        Command {
            id: "file.new".into(),
            label: "New File".into(),
            shortcut: "Ctrl+N",
        },
        Command {
            id: "file.open".into(),
            label: "Open File".into(),
            shortcut: "Ctrl+O",
        },
        Command {
            id: "file.save".into(),
            label: "Save".into(),
            shortcut: "Ctrl+S",
        },
        Command {
            id: "file.save_as".into(),
            label: "Save As".into(),
            shortcut: "Ctrl+Shift+S",
        },
        Command {
            id: "file.close_tab".into(),
            label: "Close Tab".into(),
            shortcut: "Ctrl+W",
        },
        Command {
            id: "file.recent_files".into(),
            label: "Open Recent File".into(),
            shortcut: "",
        },
        // Edit operations
        Command {
            id: "edit.undo".into(),
            label: "Undo".into(),
            shortcut: "Ctrl+Z",
        },
        Command {
            id: "edit.redo".into(),
            label: "Redo".into(),
            shortcut: "Ctrl+Y",
        },
        Command {
            id: "edit.select_all".into(),
            label: "Select All".into(),
            shortcut: "Ctrl+A",
        },
        Command {
            id: "edit.select_next_occurrence".into(),
            label: "Select Next Occurrence".into(),
            shortcut: "Ctrl+D",
        },
        Command {
            id: "edit.duplicate_line".into(),
            label: "Duplicate Line".into(),
            shortcut: "",
        },
        Command {
            id: "edit.delete_line".into(),
            label: "Delete Line".into(),
            shortcut: "Ctrl+Shift+K",
        },
        Command {
            id: "edit.move_line_up".into(),
            label: "Move Line Up".into(),
            shortcut: "Alt+Up",
        },
        Command {
            id: "edit.move_line_down".into(),
            label: "Move Line Down".into(),
            shortcut: "Alt+Down",
        },
        Command {
            id: "edit.indent".into(),
            label: "Indent".into(),
            shortcut: "Tab",
        },
        Command {
            id: "edit.unindent".into(),
            label: "Unindent".into(),
            shortcut: "Shift+Tab",
        },
        Command {
            id: "edit.toggle_comment".into(),
            label: "Toggle Comment".into(),
            shortcut: "Ctrl+/",
        },
        Command {
            id: "edit.column_editor".into(),
            label: "Column Editor".into(),
            shortcut: "",
        },
        // Navigation
        Command {
            id: "nav.go_to_file".into(),
            label: "Go to File".into(),
            shortcut: "Ctrl+P",
        },
        Command {
            id: "nav.go_to_line".into(),
            label: "Go to Line".into(),
            shortcut: "Ctrl+G",
        },
        Command {
            id: "nav.go_to_symbol".into(),
            label: "Go to Symbol".into(),
            shortcut: "Ctrl+Shift+O",
        },
        Command {
            id: "nav.go_to_definition".into(),
            label: "Go to Definition".into(),
            shortcut: "F12",
        },
        Command {
            id: "nav.find_references".into(),
            label: "Find All References".into(),
            shortcut: "Shift+F12",
        },
        Command {
            id: "nav.rename_symbol".into(),
            label: "Rename Symbol".into(),
            shortcut: "F2",
        },
        Command {
            id: "nav.hover_info".into(),
            label: "Show Hover Information".into(),
            shortcut: "",
        },
        Command {
            id: "nav.find".into(),
            label: "Find".into(),
            shortcut: "Ctrl+F",
        },
        Command {
            id: "nav.replace".into(),
            label: "Find and Replace".into(),
            shortcut: "Ctrl+H",
        },
        Command {
            id: "nav.find_in_files".into(),
            label: "Find in Files".into(),
            shortcut: "Ctrl+Shift+F",
        },
        Command {
            id: "nav.next_tab".into(),
            label: "Next Tab".into(),
            shortcut: "Ctrl+Tab",
        },
        Command {
            id: "nav.prev_tab".into(),
            label: "Previous Tab".into(),
            shortcut: "Ctrl+Shift+Tab",
        },
        // Bookmarks
        Command {
            id: "nav.toggle_bookmark".into(),
            label: "Toggle Bookmark".into(),
            shortcut: "Ctrl+F2",
        },
        Command {
            id: "nav.next_bookmark".into(),
            label: "Next Bookmark".into(),
            shortcut: "",
        },
        Command {
            id: "nav.prev_bookmark".into(),
            label: "Previous Bookmark".into(),
            shortcut: "Shift+F2",
        },
        Command {
            id: "nav.clear_bookmarks".into(),
            label: "Clear All Bookmarks".into(),
            shortcut: "",
        },
        // Text tools
        Command {
            id: "tools.sort_asc".into(),
            label: "Sort Lines (Ascending)".into(),
            shortcut: "",
        },
        Command {
            id: "tools.sort_desc".into(),
            label: "Sort Lines (Descending)".into(),
            shortcut: "",
        },
        Command {
            id: "tools.sort_case_insensitive".into(),
            label: "Sort Lines (Case Insensitive)".into(),
            shortcut: "",
        },
        Command {
            id: "tools.sort_numeric".into(),
            label: "Sort Lines (Numeric)".into(),
            shortcut: "",
        },
        Command {
            id: "tools.uppercase".into(),
            label: "Transform to Uppercase".into(),
            shortcut: "",
        },
        Command {
            id: "tools.lowercase".into(),
            label: "Transform to Lowercase".into(),
            shortcut: "",
        },
        Command {
            id: "tools.title_case".into(),
            label: "Transform to Title Case".into(),
            shortcut: "",
        },
        Command {
            id: "tools.remove_duplicates".into(),
            label: "Remove Duplicate Lines".into(),
            shortcut: "",
        },
        Command {
            id: "tools.remove_empty".into(),
            label: "Remove Empty Lines".into(),
            shortcut: "",
        },
        Command {
            id: "tools.join_lines".into(),
            label: "Join Lines".into(),
            shortcut: "",
        },
        Command {
            id: "tools.reverse_lines".into(),
            label: "Reverse Lines".into(),
            shortcut: "",
        },
        Command {
            id: "tools.trim_trailing".into(),
            label: "Trim Trailing Whitespace".into(),
            shortcut: "",
        },
        // Encoding / JSON transforms
        Command {
            id: "tools.base64_encode".into(),
            label: "Base64 Encode".into(),
            shortcut: "",
        },
        Command {
            id: "tools.base64_decode".into(),
            label: "Base64 Decode".into(),
            shortcut: "",
        },
        Command {
            id: "tools.url_encode".into(),
            label: "URL Encode".into(),
            shortcut: "",
        },
        Command {
            id: "tools.url_decode".into(),
            label: "URL Decode".into(),
            shortcut: "",
        },
        Command {
            id: "tools.json_pretty".into(),
            label: "JSON Pretty Print".into(),
            shortcut: "",
        },
        Command {
            id: "tools.json_minify".into(),
            label: "JSON Minify".into(),
            shortcut: "",
        },
        Command {
            id: "tools.xml_pretty".into(),
            label: "XML: Pretty Print".into(),
            shortcut: "",
        },
        Command {
            id: "tools.xml_minify".into(),
            label: "XML: Minify".into(),
            shortcut: "",
        },
        // Hash
        Command {
            id: "tools.hash_md5".into(),
            label: "Hash: MD5".into(),
            shortcut: "",
        },
        Command {
            id: "tools.hash_sha1".into(),
            label: "Hash: SHA-1".into(),
            shortcut: "",
        },
        Command {
            id: "tools.hash_sha256".into(),
            label: "Hash: SHA-256".into(),
            shortcut: "",
        },
        // HTML entities
        Command {
            id: "tools.html_encode".into(),
            label: "HTML Entity Encode".into(),
            shortcut: "",
        },
        Command {
            id: "tools.html_decode".into(),
            label: "HTML Entity Decode".into(),
            shortcut: "",
        },
        // Conversion
        Command {
            id: "tools.dec_to_hex".into(),
            label: "Convert Decimal to Hex".into(),
            shortcut: "",
        },
        Command {
            id: "tools.hex_to_dec".into(),
            label: "Convert Hex to Decimal".into(),
            shortcut: "",
        },
        Command {
            id: "tools.timestamp_to_date".into(),
            label: "Convert Unix Timestamp to Date".into(),
            shortcut: "",
        },
        // Case conversions
        Command {
            id: "tools.camel_case".into(),
            label: "Transform to camelCase".into(),
            shortcut: "",
        },
        Command {
            id: "tools.snake_case".into(),
            label: "Transform to snake_case".into(),
            shortcut: "",
        },
        Command {
            id: "tools.pascal_case".into(),
            label: "Transform to PascalCase".into(),
            shortcut: "",
        },
        Command {
            id: "tools.kebab_case".into(),
            label: "Transform to kebab-case".into(),
            shortcut: "",
        },
        // View
        Command {
            id: "view.toggle_word_wrap".into(),
            label: "Toggle Word Wrap".into(),
            shortcut: "",
        },
        Command {
            id: "view.zoom_in".into(),
            label: "Zoom In".into(),
            shortcut: "Ctrl+=",
        },
        Command {
            id: "view.zoom_out".into(),
            label: "Zoom Out".into(),
            shortcut: "Ctrl+-",
        },
        Command {
            id: "view.zoom_reset".into(),
            label: "Reset Zoom".into(),
            shortcut: "Ctrl+0",
        },
        Command {
            id: "view.toggle_whitespace".into(),
            label: "Show/Hide Whitespace".into(),
            shortcut: "",
        },
        Command {
            id: "view.toggle_theme".into(),
            label: "Toggle Light/Dark Theme".into(),
            shortcut: "",
        },
        Command {
            id: "view.toggle_minimap".into(),
            label: "Toggle Minimap".into(),
            shortcut: "",
        },
        Command {
            id: "view.toggle_markdown_preview".into(),
            label: "Toggle Markdown Preview".into(),
            shortcut: "Ctrl+Shift+M",
        },
        Command {
            id: "view.toggle_sidebar".into(),
            label: "Toggle File Explorer".into(),
            shortcut: "Ctrl+B",
        },
        Command {
            id: "view.toggle_function_list".into(),
            label: "Toggle Function List".into(),
            shortcut: "",
        },
        // Split view
        Command {
            id: "view.split_horizontal".into(),
            label: "Split Editor Right".into(),
            shortcut: "",
        },
        Command {
            id: "view.split_vertical".into(),
            label: "Split Editor Down".into(),
            shortcut: "",
        },
        Command {
            id: "view.close_split".into(),
            label: "Close Split Pane".into(),
            shortcut: "",
        },
        // Folding
        Command {
            id: "view.fold_toggle".into(),
            label: "Toggle Fold".into(),
            shortcut: "",
        },
        Command {
            id: "view.fold_all".into(),
            label: "Fold All".into(),
            shortcut: "",
        },
        Command {
            id: "view.unfold_all".into(),
            label: "Unfold All".into(),
            shortcut: "",
        },
        // Hex editor
        Command {
            id: "view.toggle_hex".into(),
            label: "Toggle Hex Editor".into(),
            shortcut: "",
        },
        Command {
            id: "hex.go_to_offset".into(),
            label: "Hex: Go to Offset".into(),
            shortcut: "Ctrl+G (in hex mode)",
        },
        // Diff/compare
        Command {
            id: "view.compare_files".into(),
            label: "Compare Open Files".into(),
            shortcut: "",
        },
        Command {
            id: "view.close_compare".into(),
            label: "Close Compare View".into(),
            shortcut: "",
        },
        Command {
            id: "diff.next_hunk".into(),
            label: "Diff: Next Change".into(),
            shortcut: "F7",
        },
        Command {
            id: "diff.prev_hunk".into(),
            label: "Diff: Previous Change".into(),
            shortcut: "Shift+F7",
        },
        // Terminal
        Command {
            id: "view.toggle_terminal".into(),
            label: "Toggle Terminal".into(),
            shortcut: "Ctrl+`",
        },
        Command {
            id: "terminal.new".into(),
            label: "Terminal: New Terminal".into(),
            shortcut: "",
        },
        Command {
            id: "terminal.send_selection".into(),
            label: "Terminal: Send Selection".into(),
            shortcut: "",
        },
        // Git
        Command {
            id: "view.toggle_git_blame".into(),
            label: "Toggle Git Blame".into(),
            shortcut: "",
        },
        Command {
            id: "git.stage_file".into(),
            label: "Git: Stage Current File".into(),
            shortcut: "",
        },
        Command {
            id: "git.commit".into(),
            label: "Git: Commit".into(),
            shortcut: "",
        },
        // Bracket colors
        Command {
            id: "view.toggle_bracket_colors".into(),
            label: "Toggle Bracket Pair Colorization".into(),
            shortcut: "",
        },
        // Multi-cursor
        Command {
            id: "edit.select_all_occurrences".into(),
            label: "Select All Occurrences".into(),
            shortcut: "Ctrl+Shift+L",
        },
        // Read-only
        Command {
            id: "edit.toggle_read_only".into(),
            label: "Toggle Read-Only Mode".into(),
            shortcut: "",
        },
        // Macro recording
        Command {
            id: "macro.toggle_recording".into(),
            label: "Start/Stop Macro Recording".into(),
            shortcut: "Ctrl+Q",
        },
        Command {
            id: "macro.playback".into(),
            label: "Playback Macro".into(),
            shortcut: "Ctrl+Shift+Q",
        },
        Command {
            id: "macro.run_multiple".into(),
            label: "Run Macro Multiple Times...".into(),
            shortcut: "",
        },
        Command {
            id: "macro.save_as".into(),
            label: "Save Macro As...".into(),
            shortcut: "",
        },
        Command {
            id: "macro.load".into(),
            label: "Load Macro...".into(),
            shortcut: "",
        },
        // Read-only
        Command {
            id: "edit.toggle_read_only".into(),
            label: "Toggle Read-Only Mode".into(),
            shortcut: "",
        },
        // Vim mode
        Command {
            id: "edit.toggle_vim_mode".into(),
            label: "Toggle Vim Mode".into(),
            shortcut: "",
        },
        // Zen mode
        Command {
            id: "view.zen_mode".into(),
            label: "Toggle Zen Mode".into(),
            shortcut: "F11",
        },
        // Snippets
        Command {
            id: "snippets.open_user_file".into(),
            label: "Snippets: Open User Snippets File".into(),
            shortcut: "",
        },
        // Theme management
        Command {
            id: "theme.open_folder".into(),
            label: "Theme: Open Themes Folder".into(),
            shortcut: "",
        },
        Command {
            id: "theme.create_from_current".into(),
            label: "Theme: Create Theme from Current".into(),
            shortcut: "",
        },
        Command {
            id: "theme.reload".into(),
            label: "Theme: Reload User Themes".into(),
            shortcut: "",
        },
        // Language / i18n
        Command {
            id: "settings.change_language".into(),
            label: "Settings: Change Language".into(),
            shortcut: "",
        },
        Command {
            id: "settings.language.en".into(),
            label: "Language: English".into(),
            shortcut: "",
        },
        Command {
            id: "settings.language.de".into(),
            label: "Language: Deutsch".into(),
            shortcut: "",
        },
        Command {
            id: "settings.language.fr".into(),
            label: "Language: Fran\u{00E7}ais".into(),
            shortcut: "",
        },
        Command {
            id: "settings.language.es".into(),
            label: "Language: Espa\u{00F1}ol".into(),
            shortcut: "",
        },
        Command {
            id: "settings.language.ja".into(),
            label: "Language: \u{65E5}\u{672C}\u{8A9E}".into(),
            shortcut: "",
        },
        Command {
            id: "settings.language.zh".into(),
            label: "Language: \u{4E2D}\u{6587}".into(),
            shortcut: "",
        },
        Command {
            id: "settings.language.ko".into(),
            label: "Language: \u{D55C}\u{AD6D}\u{C5B4}".into(),
            shortcut: "",
        },
        Command {
            id: "settings.language.pt".into(),
            label: "Language: Portugu\u{00EA}s".into(),
            shortcut: "",
        },
        Command {
            id: "settings.language.ru".into(),
            label: "Language: \u{0420}\u{0443}\u{0441}\u{0441}\u{043A}\u{0438}\u{0439}".into(),
            shortcut: "",
        },
        // Plugins
        Command {
            id: "plugins.manage".into(),
            label: "Plugins: Manage Plugins".into(),
            shortcut: "",
        },
        // Theme selector (individual built-in themes)
        Command {
            id: "view.theme.monokai".into(),
            label: "Theme: Monokai".into(),
            shortcut: "",
        },
        Command {
            id: "view.theme.dracula".into(),
            label: "Theme: Dracula".into(),
            shortcut: "",
        },
        Command {
            id: "view.theme.solarized_dark".into(),
            label: "Theme: Solarized Dark".into(),
            shortcut: "",
        },
        Command {
            id: "view.theme.solarized_light".into(),
            label: "Theme: Solarized Light".into(),
            shortcut: "",
        },
        Command {
            id: "view.theme.nord".into(),
            label: "Theme: Nord".into(),
            shortcut: "",
        },
        Command {
            id: "view.theme.one_dark".into(),
            label: "Theme: One Dark".into(),
            shortcut: "",
        },
        Command {
            id: "view.theme.gruvbox".into(),
            label: "Theme: Gruvbox".into(),
            shortcut: "",
        },
        Command {
            id: "view.theme.tokyo_night".into(),
            label: "Theme: Tokyo Night".into(),
            shortcut: "",
        },
        Command {
            id: "view.theme.dark".into(),
            label: "Theme: Dark (Default)".into(),
            shortcut: "",
        },
        Command {
            id: "view.theme.light".into(),
            label: "Theme: Light".into(),
            shortcut: "",
        },
    ]
}

/// State for the command palette.
pub struct CommandPaletteState {
    pub open: bool,
    pub query: String,
    pub selected: usize,
    /// Extra dynamic commands (e.g. user-defined themes) appended to the built-in list.
    pub dynamic_commands: Vec<Command>,
}

impl Default for CommandPaletteState {
    fn default() -> Self {
        Self {
            open: false,
            query: String::new(),
            selected: 0,
            dynamic_commands: Vec::new(),
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
) -> Option<String> {
    let mut commands = all_commands();
    commands.extend(state.dynamic_commands.clone());

    let filtered: Vec<&Command> = if state.query.is_empty() {
        commands.iter().collect()
    } else {
        commands
            .iter()
            .filter(|c| fuzzy_match(&state.query, &c.label))
            .collect()
    };

    // Clamp selection
    if state.selected >= filtered.len() {
        state.selected = filtered.len().saturating_sub(1);
    }

    let mut executed: Option<String> = None;
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
                executed = Some(filtered[state.selected].id.clone());
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
                                    ui.label(&*cmd.label);
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
                            executed = Some(cmd.id.clone());
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
