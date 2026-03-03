use crate::autocomplete::AutocompleteState;
use crate::breadcrumb;
use crate::command_palette::{self, CommandPaletteState};
use crate::config::{self, EditorConfig};
use crate::diff_view::{self, DiffViewState};
use crate::editor_view::{self, EditorRenderContext, EditorViewState};
use crate::find_in_files::FindInFilesState;
use crate::function_list::{self, FunctionListState};
use crate::git::GitManager;
use crate::go_to_file::{self, GoToFileState};
use crate::go_to_symbol::{self, GoToSymbolState};
use crate::hex_view::{self, HexViewState};
use crate::lsp::LspManager;
use crate::macro_recorder::{MacroAction, MacroRecorder};
use crate::search_panel::{self, SearchPanelState};
use crate::sidebar::{self, SidebarState};
use crate::snippets::SnippetEngine;
use crate::status_bar;
use crate::tab_bar;
use crate::terminal::TerminalState;
use crate::theme::EditorTheme;
use crate::vim::VimState;
use eframe::egui;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use openedit_core::syntax::SyntaxEngine;
use openedit_core::{Buffer, Document, Encoding};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc;
use url::Url;

/// Main application state.
pub struct OpenEditApp {
    documents: Vec<Document>,
    active_tab: usize,
    theme: EditorTheme,
    search_state: SearchPanelState,
    editor_view_state: EditorViewState,
    syntax_engine: SyntaxEngine,
    /// Files to open (set from command line args).
    pending_opens: Vec<PathBuf>,
    /// Go to Line dialog state.
    go_to_line_open: bool,
    go_to_line_input: String,
    /// Unsaved changes dialog state.
    unsaved_close_tab: Option<usize>,
    /// Command palette state.
    command_palette: CommandPaletteState,
    /// Word wrap toggle.
    word_wrap: bool,
    /// Font size for the editor (default 13.0).
    font_size: f32,
    /// Show whitespace characters (spaces, tabs).
    show_whitespace: bool,
    /// Show minimap (code overview sidebar).
    show_minimap: bool,
    /// File watcher for external modifications.
    _watcher: Option<RecommendedWatcher>,
    watcher_rx: Option<mpsc::Receiver<PathBuf>>,
    /// Paths currently being watched.
    watched_paths: HashSet<PathBuf>,
    /// Tab index for "file changed externally" dialog.
    external_change_tab: Option<usize>,
    /// Pending text to copy to clipboard (set by context menu actions).
    pending_clipboard: Option<String>,
    /// Autocomplete popup state.
    autocomplete: AutocompleteState,
    /// Recently opened file paths (most recent first).
    recent_files: Vec<PathBuf>,
    /// Find in Files panel state.
    find_in_files_state: FindInFilesState,
    /// Go to File (Ctrl+P) dialog state.
    go_to_file_state: GoToFileState,
    /// Go to Symbol (Ctrl+Shift+O) dialog state.
    go_to_symbol_state: GoToSymbolState,
    /// File explorer sidebar state.
    sidebar_state: SidebarState,
    /// Split view state.
    split: SplitState,
    /// Tab drag state for reordering.
    tab_drag_state: tab_bar::TabDragState,
    /// Whether to show Markdown preview panel.
    show_markdown_preview: bool,
    /// Scroll offset for Markdown preview.
    markdown_preview_scroll: f32,
    /// Macro recording and playback state.
    macro_recorder: MacroRecorder,
    /// Hex editor view state.
    hex_view_state: HexViewState,
    /// Function list panel state.
    function_list_state: FunctionListState,
    /// Breadcrumb bar state.
    breadcrumb_state: breadcrumb::BreadcrumbState,
    /// Diff/compare view state.
    diff_state: DiffViewState,
    /// Column editor dialog state.
    column_editor_open: bool,
    column_editor_mode: ColumnEditorMode,
    column_editor_col: String,
    column_editor_start_line: String,
    column_editor_end_line: String,
    column_editor_text: String,
    column_editor_initial: String,
    column_editor_step: String,
    column_editor_pad_width: String,
    /// LSP manager for code intelligence.
    lsp_manager: LspManager,
    /// Integrated terminal state.
    terminal_state: TerminalState,
    /// Git integration state.
    git_state: GitManager,
    /// Current git branch name.
    git_branch: Option<String>,
    /// Git commit dialog state.
    git_commit_dialog_open: bool,
    git_commit_message: String,
    /// Status message from last git operation (shown briefly).
    git_status_message: Option<String>,
    git_status_message_time: Option<std::time::Instant>,
    /// Whether bracket pair colorization is enabled.
    bracket_colorization: bool,
    /// LSP completion items (separate from word-based autocomplete).
    lsp_completions: Vec<crate::lsp::LspCompletionItem>,
    lsp_completion_selected: usize,
    lsp_completions_visible: bool,
    /// Hover tooltip state.
    hover_text: Option<String>,
    hover_pos: Option<egui::Pos2>,
    /// Track which files have been opened in LSP.
    lsp_opened_files: HashSet<String>,
    /// Debounce timer for LSP didChange.
    lsp_change_timer: Option<std::time::Instant>,
    /// Track if terminal has focus for keyboard input.
    terminal_focused: bool,
    /// Vim mode state.
    vim_state: VimState,
    /// Snippet engine.
    snippet_engine: SnippetEngine,
    /// Zen mode (distraction-free).
    zen_mode: bool,
    /// Split divider drag state.
    split_ratio: f32,
    split_dragging: bool,
    /// Show line numbers in the gutter.
    show_line_numbers: bool,
    /// Auto-save on focus loss / timer.
    auto_save: bool,
    /// Show the "About OpenEdit" window.
    show_about: bool,
    /// Show the keyboard shortcuts cheatsheet overlay.
    show_shortcuts: bool,
    /// "Run Macro Multiple Times" dialog state.
    macro_run_n_open: bool,
    macro_run_n_input: String,
    /// "Save Macro As" dialog state.
    macro_save_as_open: bool,
    macro_save_as_input: String,
    /// "Load Macro" dialog state.
    macro_load_open: bool,
    macro_load_selected: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum ColumnEditorMode {
    Text,
    Number,
}

/// Direction of a split view.
#[derive(Clone, Copy, PartialEq)]
enum SplitDirection {
    Horizontal, // side-by-side (left | right)
    Vertical,   // stacked (top / bottom)
}

/// State for split editor view.
struct SplitState {
    /// Whether split view is active.
    active: bool,
    /// Split direction.
    direction: SplitDirection,
    /// Document index shown in the second pane.
    second_tab: usize,
    /// Editor view state for the second pane.
    second_view_state: EditorViewState,
    /// Autocomplete state for the second pane.
    second_autocomplete: AutocompleteState,
}

impl Default for SplitState {
    fn default() -> Self {
        Self {
            active: false,
            direction: SplitDirection::Horizontal,
            second_tab: 0,
            second_view_state: EditorViewState::default(),
            second_autocomplete: AutocompleteState::new(),
        }
    }
}

impl OpenEditApp {
    pub fn new(files_to_open: Vec<PathBuf>) -> Self {
        let (tx, rx) = mpsc::channel();
        let watcher =
            notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    if matches!(event.kind, notify::EventKind::Modify(_)) {
                        for path in event.paths {
                            let _ = tx.send(path);
                        }
                    }
                }
            })
            .ok();

        // Load persistent configuration
        let cfg = config::load_config();
        let theme = EditorTheme::by_name(&cfg.ui.theme);

        let mut sidebar_state = SidebarState::default();
        sidebar_state.visible = cfg.ui.show_sidebar;

        let mut app = Self {
            documents: Vec::new(),
            active_tab: 0,
            theme,
            search_state: SearchPanelState::default(),
            editor_view_state: EditorViewState::default(),
            syntax_engine: SyntaxEngine::new(),
            pending_opens: files_to_open,
            go_to_line_open: false,
            go_to_line_input: String::new(),
            unsaved_close_tab: None,
            command_palette: CommandPaletteState::default(),
            word_wrap: cfg.editor.word_wrap,
            font_size: cfg.editor.font_size,
            show_whitespace: cfg.editor.show_whitespace,
            show_minimap: cfg.ui.show_minimap,
            _watcher: watcher,
            watcher_rx: Some(rx),
            watched_paths: HashSet::new(),
            external_change_tab: None,
            pending_clipboard: None,
            autocomplete: AutocompleteState::new(),
            recent_files: Vec::new(),
            find_in_files_state: FindInFilesState::default(),
            go_to_file_state: GoToFileState::default(),
            go_to_symbol_state: GoToSymbolState::default(),
            sidebar_state,
            split: SplitState::default(),
            tab_drag_state: tab_bar::TabDragState::default(),
            show_markdown_preview: false,
            markdown_preview_scroll: 0.0,
            macro_recorder: MacroRecorder::new(),
            hex_view_state: HexViewState::default(),
            function_list_state: FunctionListState::default(),
            breadcrumb_state: breadcrumb::BreadcrumbState::default(),
            diff_state: DiffViewState::default(),
            column_editor_open: false,
            column_editor_mode: ColumnEditorMode::Text,
            column_editor_col: String::new(),
            column_editor_start_line: String::new(),
            column_editor_end_line: String::new(),
            column_editor_text: String::new(),
            column_editor_initial: "1".to_string(),
            column_editor_step: "1".to_string(),
            column_editor_pad_width: "0".to_string(),
            lsp_manager: LspManager::new(),
            terminal_state: TerminalState::default(),
            git_state: GitManager::new(),
            git_branch: None,
            git_commit_dialog_open: false,
            git_commit_message: String::new(),
            git_status_message: None,
            git_status_message_time: None,
            bracket_colorization: true,
            lsp_completions: Vec::new(),
            lsp_completion_selected: 0,
            lsp_completions_visible: false,
            hover_text: None,
            hover_pos: None,
            lsp_opened_files: HashSet::new(),
            lsp_change_timer: None,
            terminal_focused: false,
            vim_state: VimState::new(),
            snippet_engine: SnippetEngine::new(),
            zen_mode: false,
            split_ratio: 0.5,
            split_dragging: false,
            show_line_numbers: true,
            auto_save: false,
            show_about: false,
            show_shortcuts: false,
            macro_run_n_open: false,
            macro_run_n_input: String::new(),
            macro_save_as_open: false,
            macro_save_as_input: String::new(),
            macro_load_open: false,
            macro_load_selected: None,
        };

        // Load saved macros from disk
        app.macro_recorder.load_macros_from_disk();

        // If no files specified, try to restore session; otherwise open untitled doc
        if app.pending_opens.is_empty() {
            app.load_session();
            if app.documents.is_empty() {
                app.documents.push(Document::new());
            }
        }

        app
    }

    fn open_file(&mut self, path: PathBuf) {
        match Buffer::load_file(&path) {
            Ok(buffer) => {
                let mut doc = Document::new();
                doc.buffer = buffer;
                doc.path = Some(path.clone());

                // Detect language from extension
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    doc.language = Some(language_from_extension(ext));
                }

                self.documents.push(doc);
                self.active_tab = self.documents.len() - 1;
                self.watch_path(&path);

                // Add to recent files
                self.recent_files.retain(|p| p != &path);
                self.recent_files.insert(0, path.clone());
                self.recent_files.truncate(20);

                // Initialize git if not already done
                if self.git_branch.is_none() {
                    if let Some(parent) = path.parent() {
                        self.git_state.init(parent);
                        self.git_branch = self.git_state.branch.clone();
                    }
                }

                self.save_session();
            }
            Err(e) => {
                log::error!("Failed to read {}: {}", path.display(), e);
            }
        }
    }

    /// Returns the path to the session file.
    fn session_path() -> Option<PathBuf> {
        let config_dir = if cfg!(target_os = "windows") {
            std::env::var("APPDATA").ok().map(PathBuf::from)
        } else {
            std::env::var("XDG_CONFIG_HOME")
                .ok()
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var("HOME")
                        .ok()
                        .map(|h| PathBuf::from(h).join(".config"))
                })
        };
        config_dir.map(|d| d.join("openedit").join("session.json"))
    }

    /// Save the current session (open files and recent files) to disk.
    fn save_session(&self) {
        let Some(path) = Self::session_path() else {
            return;
        };

        let open_files: Vec<String> = self
            .documents
            .iter()
            .filter_map(|d| d.path.as_ref())
            .map(|p| p.to_string_lossy().into_owned())
            .collect();

        let recent: Vec<String> = self
            .recent_files
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();

        let tab_states: Vec<serde_json::Value> = self
            .documents
            .iter()
            .map(|d| {
                serde_json::json!({
                    "cursor_line": d.cursors.primary().position.line,
                    "cursor_col": d.cursors.primary().position.col,
                    "scroll_line": d.scroll_line,
                    "scroll_col": d.scroll_col,
                })
            })
            .collect();

        let session = serde_json::json!({
            "files": open_files,
            "active_tab": self.active_tab,
            "recent_files": recent,
            "tab_states": tab_states,
            "split_active": self.split.active,
            "split_direction": if self.split.direction == SplitDirection::Horizontal { "horizontal" } else { "vertical" },
            "split_second_tab": self.split.second_tab,
            "split_ratio": self.split_ratio,
        });

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(
            &path,
            serde_json::to_string_pretty(&session).unwrap_or_default(),
        );
    }

    /// Load the session from disk, restoring open files and recent files.
    fn load_session(&mut self) {
        let Some(path) = Self::session_path() else {
            return;
        };

        let Ok(content) = std::fs::read_to_string(&path) else {
            return;
        };
        let Ok(session) = serde_json::from_str::<serde_json::Value>(&content) else {
            return;
        };

        if let Some(files) = session["files"].as_array() {
            for file_val in files {
                if let Some(file_str) = file_val.as_str() {
                    let file_path = PathBuf::from(file_str);
                    if file_path.exists() {
                        self.open_file(file_path);
                    }
                }
            }
        }

        // Restore cursor positions and scroll
        if let Some(tab_states) = session["tab_states"].as_array() {
            for (i, state) in tab_states.iter().enumerate() {
                if let Some(doc) = self.documents.get_mut(i) {
                    if let Some(line) = state["cursor_line"].as_u64() {
                        if let Some(col) = state["cursor_col"].as_u64() {
                            let line = line as usize;
                            let col = col as usize;
                            let max_line = doc.buffer.len_lines().saturating_sub(1);
                            let target_line = line.min(max_line);
                            let max_col = doc.buffer.line_len_chars_no_newline(target_line);
                            let target_col = col.min(max_col);
                            doc.cursors.primary_mut().move_to(
                                openedit_core::cursor::Position::new(target_line, target_col),
                                false,
                            );
                        }
                    }
                    if let Some(sl) = state["scroll_line"].as_u64() {
                        doc.scroll_line = sl as usize;
                    }
                    if let Some(sc) = state["scroll_col"].as_u64() {
                        doc.scroll_col = sc as usize;
                    }
                }
            }
        }

        if let Some(tab) = session["active_tab"].as_u64() {
            let tab = tab as usize;
            if tab < self.documents.len() {
                self.active_tab = tab;
            }
        }

        // Restore split layout
        if let Some(true) = session["split_active"].as_bool() {
            self.split.active = true;
            if session["split_direction"].as_str() == Some("vertical") {
                self.split.direction = SplitDirection::Vertical;
            } else {
                self.split.direction = SplitDirection::Horizontal;
            }
            if let Some(st) = session["split_second_tab"].as_u64() {
                self.split.second_tab = (st as usize).min(self.documents.len().saturating_sub(1));
            }
            if let Some(sr) = session["split_ratio"].as_f64() {
                self.split_ratio = sr as f32;
            }
        }

        if let Some(recent) = session["recent_files"].as_array() {
            for file_val in recent {
                if let Some(file_str) = file_val.as_str() {
                    let file_path = PathBuf::from(file_str);
                    if !self.recent_files.contains(&file_path) {
                        self.recent_files.push(file_path);
                    }
                }
            }
            self.recent_files.truncate(20);
        }
    }

    fn watch_path(&mut self, path: &PathBuf) {
        if self.watched_paths.contains(path) {
            return;
        }
        if let Some(ref mut watcher) = self._watcher {
            if watcher.watch(path, RecursiveMode::NonRecursive).is_ok() {
                self.watched_paths.insert(path.clone());
            }
        }
    }

    fn save_current(&mut self) {
        let Some(doc) = self.documents.get_mut(self.active_tab) else {
            return;
        };

        if let Some(ref path) = doc.path {
            let bytes = doc.bytes_for_save();
            match std::fs::write(path, &bytes) {
                Ok(()) => {
                    doc.modified = false;
                    log::info!("Saved {}", path.display());
                    self.save_session();
                    // Refresh git status after save
                    self.git_state.refresh_statuses();
                    self.git_state.invalidate_file_cache();
                }
                Err(e) => {
                    log::error!("Failed to save {}: {}", path.display(), e);
                }
            }
        } else {
            // Save As
            self.save_as();
        }
    }

    fn save_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new().save_file() {
            if let Some(doc) = self.documents.get_mut(self.active_tab) {
                doc.path = Some(path.clone());
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    doc.language = Some(language_from_extension(ext));
                }
                let bytes = doc.bytes_for_save();
                match std::fs::write(&path, &bytes) {
                    Ok(()) => {
                        doc.modified = false;
                    }
                    Err(e) => {
                        log::error!("Failed to save {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    fn open_dialog(&mut self) {
        if let Some(paths) = rfd::FileDialog::new().pick_files() {
            for path in paths {
                self.open_file(path);
            }
        }
    }

    fn close_tab(&mut self, idx: usize) {
        if idx >= self.documents.len() {
            return;
        }

        // If the document is modified, show unsaved changes dialog
        if self.documents[idx].modified {
            self.unsaved_close_tab = Some(idx);
            return;
        }

        self.force_close_tab(idx);
    }

    /// Determine the workspace root: directory of the current file, or cwd.
    fn workspace_root(&self) -> PathBuf {
        self.documents
            .get(self.active_tab)
            .and_then(|d| d.path.as_ref())
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    /// Replay the last recorded macro on the active document.
    fn replay_macro(&mut self) {
        let actions: Vec<MacroAction> = self.macro_recorder.last_recorded().to_vec();
        if actions.is_empty() {
            return;
        }
        let Some(doc) = self.documents.get_mut(self.active_tab) else {
            return;
        };
        for action in &actions {
            match action {
                MacroAction::InsertText(text) => {
                    // Replicate the bracket auto-close logic from editor_view for single chars
                    if text.len() == 1 {
                        let ch = text.chars().next().unwrap();
                        if let Some(close) = match ch {
                            '(' => Some(')'),
                            '[' => Some(']'),
                            '{' => Some('}'),
                            '"' => Some('"'),
                            '\'' => Some('\''),
                            _ => None,
                        } {
                            let cursor = doc.cursors.primary();
                            let offset = doc
                                .buffer
                                .line_col_to_char(cursor.position.line, cursor.position.col);
                            let next_char = if offset < doc.buffer.len_chars() {
                                Some(doc.buffer.char_at(offset))
                            } else {
                                None
                            };

                            if (ch == '"' || ch == '\'') && next_char == Some(ch) {
                                doc.move_cursor_right(false);
                            } else if ch == close && next_char == Some(close) {
                                doc.move_cursor_right(false);
                            } else {
                                let pair = format!("{}{}", ch, close);
                                doc.insert_text(&pair);
                                doc.move_cursor_left(false);
                            }
                        } else if matches!(ch, ')' | ']' | '}') {
                            let cursor = doc.cursors.primary();
                            let offset = doc
                                .buffer
                                .line_col_to_char(cursor.position.line, cursor.position.col);
                            let next_char = if offset < doc.buffer.len_chars() {
                                Some(doc.buffer.char_at(offset))
                            } else {
                                None
                            };
                            if next_char == Some(ch) {
                                doc.move_cursor_right(false);
                            } else {
                                doc.insert_text(text);
                            }
                        } else {
                            doc.insert_text(text);
                        }
                    } else {
                        doc.insert_text(text);
                    }
                }
                MacroAction::Paste(text) => {
                    doc.insert_text(text);
                }
                MacroAction::KeyAction {
                    key,
                    ctrl,
                    shift,
                    alt,
                } => {
                    Self::replay_key_action(doc, key, *ctrl, *shift, *alt);
                }
            }
        }
    }

    /// Replay a single key action on a document.
    fn replay_key_action(doc: &mut Document, key: &str, ctrl: bool, shift: bool, alt: bool) {
        match key {
            // Clipboard (copy is a no-op during replay, cut deletes selection)
            "C" if ctrl => { /* copy: no-op during replay */ }
            "X" if ctrl => {
                if doc.cursors.primary().has_selection() {
                    doc.delete_selection_public();
                }
            }
            // Line operations
            "ArrowUp" if alt => {
                doc.move_line_up();
            }
            "ArrowDown" if alt => {
                doc.move_line_down();
            }
            "K" if ctrl && shift => {
                doc.delete_line();
            }
            // Navigation
            "ArrowLeft" if ctrl => doc.move_cursor_word_left(shift),
            "ArrowRight" if ctrl => doc.move_cursor_word_right(shift),
            "ArrowLeft" => doc.move_cursor_left(shift),
            "ArrowRight" => doc.move_cursor_right(shift),
            "ArrowUp" => doc.move_cursor_up(shift),
            "ArrowDown" => doc.move_cursor_down(shift),
            "Home" if ctrl => doc.move_cursor_doc_start(shift),
            "End" if ctrl => doc.move_cursor_doc_end(shift),
            "Home" => doc.move_cursor_home(shift),
            "End" => doc.move_cursor_end(shift),
            "PageUp" => doc.move_cursor_page_up(30, shift),
            "PageDown" => doc.move_cursor_page_down(30, shift),
            // Editing
            "Backspace" if ctrl => {
                doc.delete_word_left();
            }
            "Backspace" => {
                doc.backspace();
            }
            "Delete" if ctrl => {
                doc.delete_word_right();
            }
            "Delete" => {
                doc.delete_forward();
            }
            "Enter" => {
                doc.insert_newline_with_indent();
            }
            "Tab" if shift => {
                doc.unindent();
            }
            "Tab" => {
                doc.insert_text("    ");
            }
            // Selection/undo
            "A" if ctrl => doc.select_all(),
            "Z" if ctrl && shift => doc.redo(),
            "Z" if ctrl => doc.undo(),
            "Y" if ctrl => doc.redo(),
            "Slash" if ctrl => {
                doc.toggle_comment();
            }
            "D" if ctrl => {
                doc.select_next_occurrence();
            }
            "Escape" => {
                if doc.cursors.cursor_count() > 1 {
                    doc.cursors.clear_extra_cursors();
                }
            }
            _ => {}
        }
    }

    fn execute_command(&mut self, cmd_id: &str) {
        match cmd_id {
            // File
            "file.new" => {
                self.documents.push(Document::new());
                self.active_tab = self.documents.len() - 1;
            }
            "file.open" => self.open_dialog(),
            "file.save" => self.save_current(),
            "file.save_as" => self.save_as(),
            "file.close_tab" => {
                let idx = self.active_tab;
                self.close_tab(idx);
            }
            "file.recent_files" => {
                // Open the most recent file that isn't already open
                let open_paths: HashSet<PathBuf> = self
                    .documents
                    .iter()
                    .filter_map(|d| d.path.clone())
                    .collect();
                if let Some(path) = self
                    .recent_files
                    .iter()
                    .find(|p| !open_paths.contains(*p) && p.exists())
                    .cloned()
                {
                    self.open_file(path);
                }
            }
            // Edit
            "edit.undo" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    doc.undo();
                }
            }
            "edit.redo" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    doc.redo();
                }
            }
            "edit.select_all" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    doc.select_all();
                }
            }
            "edit.select_next_occurrence" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    doc.select_next_occurrence();
                }
            }
            "edit.duplicate_line" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    doc.duplicate_line();
                }
            }
            "edit.delete_line" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    doc.delete_line();
                }
            }
            "edit.move_line_up" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    doc.move_line_up();
                }
            }
            "edit.move_line_down" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    doc.move_line_down();
                }
            }
            "edit.indent" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    doc.insert_text("    ");
                }
            }
            "edit.unindent" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    doc.unindent();
                }
            }
            "edit.toggle_comment" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    doc.toggle_comment();
                }
            }
            "edit.toggle_read_only" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    doc.read_only = !doc.read_only;
                }
            }
            // Navigation
            "nav.go_to_line" => {
                self.go_to_line_open = true;
                self.go_to_line_input.clear();
            }
            "nav.go_to_file" => {
                self.go_to_file_state.open = !self.go_to_file_state.open;
                if self.go_to_file_state.open {
                    let root = self.workspace_root();
                    self.go_to_file_state.scan(&root);
                }
                self.go_to_file_state.query.clear();
                self.go_to_file_state.selected = 0;
            }
            "nav.go_to_symbol" => {
                if let Some(doc) = self.documents.get(self.active_tab) {
                    let source = doc.buffer.to_string();
                    let lang_key = doc.language.as_deref().and_then(SyntaxEngine::language_key);
                    if let Some(key) = lang_key {
                        self.go_to_symbol_state.symbols =
                            self.syntax_engine.extract_symbols(&source, key);
                    } else {
                        self.go_to_symbol_state.symbols.clear();
                    }
                } else {
                    self.go_to_symbol_state.symbols.clear();
                }
                self.go_to_symbol_state.open = true;
                self.go_to_symbol_state.query.clear();
                self.go_to_symbol_state.selected = 0;
            }
            "nav.find" => {
                self.search_state.visible = !self.search_state.visible;
                self.search_state.show_replace = false;
            }
            "nav.find_in_files" => {
                self.find_in_files_state.visible = !self.find_in_files_state.visible;
            }
            "nav.replace" => {
                self.search_state.visible = true;
                self.search_state.show_replace = true;
            }
            "nav.next_tab" if !self.documents.is_empty() => {
                self.active_tab = (self.active_tab + 1) % self.documents.len();
            }
            "nav.prev_tab" if !self.documents.is_empty() => {
                self.active_tab = if self.active_tab == 0 {
                    self.documents.len() - 1
                } else {
                    self.active_tab - 1
                };
            }
            "nav.toggle_bookmark" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    let line = doc.cursors.primary().position.line;
                    doc.toggle_bookmark(line);
                }
            }
            "nav.next_bookmark" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    let current_line = doc.cursors.primary().position.line;
                    if let Some(target) = doc.next_bookmark(current_line) {
                        doc.go_to_line(target);
                    }
                }
            }
            "nav.prev_bookmark" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    let current_line = doc.cursors.primary().position.line;
                    if let Some(target) = doc.prev_bookmark(current_line) {
                        doc.go_to_line(target);
                    }
                }
            }
            "nav.clear_bookmarks" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    doc.clear_bookmarks();
                }
            }
            // Text tools — operate on selection or entire document
            cmd if cmd.starts_with("tools.") => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    apply_text_tool(doc, cmd);
                }
            }
            // View
            "view.toggle_word_wrap" => {
                self.word_wrap = !self.word_wrap;
                self.save_config_state();
            }
            "view.zoom_in" => {
                self.font_size = (self.font_size + 1.0).min(48.0);
                self.save_config_state();
            }
            "view.zoom_out" => {
                self.font_size = (self.font_size - 1.0).max(6.0);
                self.save_config_state();
            }
            "view.zoom_reset" => {
                self.font_size = 13.0;
                self.save_config_state();
            }
            "view.toggle_whitespace" => {
                self.show_whitespace = !self.show_whitespace;
                self.save_config_state();
            }
            "view.toggle_minimap" => {
                self.show_minimap = !self.show_minimap;
                self.save_config_state();
            }
            "view.toggle_markdown_preview" => {
                self.show_markdown_preview = !self.show_markdown_preview;
                self.markdown_preview_scroll = 0.0;
            }
            "view.toggle_sidebar" => {
                self.sidebar_state.visible = !self.sidebar_state.visible;
                if self.sidebar_state.visible && self.sidebar_state.root.is_none() {
                    let root = self.workspace_root();
                    self.sidebar_state.load_tree(&root);
                }
                self.save_config_state();
            }
            "view.toggle_theme" => {
                // Cycle through themes
                let names = EditorTheme::all_names();
                let current_idx = names
                    .iter()
                    .position(|n| *n == self.theme.name)
                    .unwrap_or(0);
                let next_idx = (current_idx + 1) % names.len();
                self.theme = EditorTheme::by_name(names[next_idx]);
                self.save_config_state();
            }
            "view.zen_mode" | "view.toggle_zen" => {
                self.zen_mode = !self.zen_mode;
                if self.zen_mode {
                    self.sidebar_state.visible = false;
                }
            }
            "edit.toggle_vim_mode" => {
                self.vim_state.enabled = !self.vim_state.enabled;
                if self.vim_state.enabled {
                    self.vim_state.mode = crate::vim::VimMode::Normal;
                }
            }
            cmd if cmd.starts_with("view.theme.") => {
                let theme_key = &cmd["view.theme.".len()..];
                self.theme = EditorTheme::by_name(theme_key);
                self.save_config_state();
            }
            "view.split_horizontal" => {
                self.split.active = true;
                self.split.direction = SplitDirection::Horizontal;
                self.split.second_tab = self.active_tab;
            }
            "view.split_vertical" => {
                self.split.active = true;
                self.split.direction = SplitDirection::Vertical;
                self.split.second_tab = self.active_tab;
            }
            "view.close_split" => {
                self.split.active = false;
            }
            "view.fold_toggle" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    let line = doc.cursors.primary().position.line;
                    doc.update_fold_ranges();
                    doc.toggle_fold(line);
                }
            }
            "view.toggle_hex" => {
                self.hex_view_state.active = !self.hex_view_state.active;
                if self.hex_view_state.active {
                    // Load raw bytes from the current document's file
                    if let Some(doc) = self.documents.get(self.active_tab) {
                        if let Some(ref path) = doc.path {
                            if let Ok(bytes) = std::fs::read(path) {
                                self.hex_view_state.data = bytes;
                            }
                        } else {
                            // No file path - use document content as UTF-8 bytes
                            self.hex_view_state.data = doc.buffer.to_string().into_bytes();
                        }
                    }
                    self.hex_view_state.scroll_offset = 0.0;
                    self.hex_view_state.selected_offset = None;
                }
            }
            // Macro recording
            "macro.toggle_recording" => {
                if self.macro_recorder.is_recording() {
                    self.macro_recorder.stop_recording();
                } else {
                    self.macro_recorder.start_recording();
                }
            }
            "macro.playback" => {
                if !self.macro_recorder.is_recording() {
                    self.replay_macro();
                }
            }
            "macro.run_multiple" => {
                if !self.macro_recorder.is_recording()
                    && !self.macro_recorder.last_recorded().is_empty()
                {
                    self.macro_run_n_open = true;
                    self.macro_run_n_input.clear();
                }
            }
            "macro.save_as" => {
                if !self.macro_recorder.last_recorded().is_empty() {
                    self.macro_save_as_open = true;
                    self.macro_save_as_input.clear();
                }
            }
            "macro.load" => {
                if !self.macro_recorder.macro_names().is_empty() {
                    self.macro_load_open = true;
                    self.macro_load_selected = None;
                }
            }
            "edit.column_editor" => {
                self.column_editor_open = true;
                // Pre-fill from selection if available
                if let Some(doc) = self.documents.get(self.active_tab) {
                    let primary = doc.cursors.primary();
                    if primary.has_selection() {
                        if let Some((start, end)) = primary.selection_range() {
                            self.column_editor_start_line = (start.line + 1).to_string();
                            self.column_editor_end_line = (end.line + 1).to_string();
                            self.column_editor_col = (start.col + 1).to_string();
                        }
                    } else {
                        let line = primary.position.line + 1;
                        self.column_editor_start_line = line.to_string();
                        self.column_editor_end_line = doc.buffer.len_lines().to_string();
                        self.column_editor_col = (primary.position.col + 1).to_string();
                    }
                }
            }
            "view.fold_all" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    doc.update_fold_ranges();
                    doc.folding.fold_all();
                }
            }
            "view.unfold_all" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    doc.folding.unfold_all();
                }
            }
            "view.compare_files" => {
                if self.documents.len() >= 2 {
                    self.diff_state.active = true;
                    self.diff_state.left_tab = self.active_tab;
                    self.diff_state.right_tab = (self.active_tab + 1) % self.documents.len();
                    let left = self.documents[self.diff_state.left_tab].buffer.to_string();
                    let right = self.documents[self.diff_state.right_tab].buffer.to_string();
                    self.diff_state.diff_ops = openedit_core::diff::diff_lines(&left, &right);
                    self.diff_state.scroll_offset = 0.0;
                }
            }
            "view.close_compare" => {
                self.diff_state.active = false;
            }
            "diff.next_hunk" => {
                if self.diff_state.active {
                    let line_height =
                        crate::editor_view::line_height_for_font(self.font_size);
                    diff_view::navigate_next_hunk(&mut self.diff_state, line_height);
                }
            }
            "diff.prev_hunk" => {
                if self.diff_state.active {
                    let line_height =
                        crate::editor_view::line_height_for_font(self.font_size);
                    diff_view::navigate_prev_hunk(&mut self.diff_state, line_height);
                }
            }
            "hex.go_to_offset" => {
                if self.hex_view_state.active {
                    self.hex_view_state.go_to_offset_open = true;
                    self.hex_view_state.go_to_offset_input.clear();
                }
            }
            "view.toggle_terminal" => {
                if !self.terminal_state.visible {
                    self.terminal_state.visible = true;
                    if !self.terminal_state.running() {
                        self.terminal_state.start();
                    }
                    self.terminal_focused = true;
                } else {
                    self.terminal_state.visible = false;
                    self.terminal_focused = false;
                }
            }
            "terminal.new" => {
                self.terminal_state.visible = true;
                self.terminal_state.start();
                self.terminal_focused = true;
            }
            "terminal.send_selection" => {
                if let Some(doc) = self.documents.get(self.active_tab) {
                    let text = doc.selected_text();
                    if !text.is_empty() {
                        if !self.terminal_state.visible {
                            self.terminal_state.visible = true;
                        }
                        if !self.terminal_state.running() {
                            self.terminal_state.start();
                        }
                        self.terminal_state.send_text_to_active(&text);
                    }
                }
            }
            "view.toggle_bracket_colors" => {
                self.bracket_colorization = !self.bracket_colorization;
            }
            "view.toggle_git_blame" => {
                self.git_state.show_blame = !self.git_state.show_blame;
                if self.git_state.show_blame {
                    if let Some(doc) = self.documents.get(self.active_tab) {
                        if let Some(ref path) = doc.path {
                            self.git_state.compute_blame(path);
                        }
                    }
                }
            }
            "git.stage_file" => {
                if let Some(doc) = self.documents.get(self.active_tab) {
                    if let Some(ref path) = doc.path {
                        let path = path.clone();
                        match self.git_state.stage_file(&path) {
                            Ok(()) => {
                                let name = path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "file".to_string());
                                self.git_status_message =
                                    Some(format!("Staged: {}", name));
                                self.git_status_message_time =
                                    Some(std::time::Instant::now());
                            }
                            Err(e) => {
                                self.git_status_message =
                                    Some(format!("Stage failed: {}", e));
                                self.git_status_message_time =
                                    Some(std::time::Instant::now());
                            }
                        }
                    } else {
                        self.git_status_message =
                            Some("Cannot stage: file has no path (save first)".to_string());
                        self.git_status_message_time = Some(std::time::Instant::now());
                    }
                }
            }
            "git.commit" => {
                self.git_commit_dialog_open = true;
                self.git_commit_message.clear();
            }
            "edit.select_all_occurrences" => {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    doc.select_all_occurrences();
                }
            }
            "view.toggle_function_list" => {
                self.function_list_state.visible = !self.function_list_state.visible;
                if self.function_list_state.visible {
                    self.refresh_function_list_symbols();
                }
            }
            _ => {}
        }
    }

    /// Refresh the function list symbols from the current document.
    fn refresh_function_list_symbols(&mut self) {
        if let Some(doc) = self.documents.get(self.active_tab) {
            let source = doc.buffer.to_string();
            let lang_key = doc.language.as_deref().and_then(SyntaxEngine::language_key);
            if let Some(key) = lang_key {
                self.function_list_state.symbols = self.syntax_engine.extract_symbols(&source, key);
            } else {
                self.function_list_state.symbols.clear();
            }
        } else {
            self.function_list_state.symbols.clear();
        }
    }

    fn force_close_tab(&mut self, idx: usize) {
        if idx >= self.documents.len() {
            return;
        }
        self.documents.remove(idx);

        if self.documents.is_empty() {
            self.documents.push(Document::new());
            self.active_tab = 0;
        } else if self.active_tab >= self.documents.len() {
            self.active_tab = self.documents.len() - 1;
        }

        self.save_session();
    }

    /// Build an `EditorConfig` from the current app state.
    fn current_config(&self) -> EditorConfig {
        let theme_name = self.theme.config_name();
        EditorConfig {
            editor: config::EditorSection {
                font_size: self.font_size,
                tab_size: 4,
                word_wrap: self.word_wrap,
                show_whitespace: self.show_whitespace,
            },
            ui: config::UiSection {
                theme: theme_name.to_string(),
                show_minimap: self.show_minimap,
                show_sidebar: self.sidebar_state.visible,
            },
        }
    }

    /// Persist the current settings to config.toml.
    fn save_config_state(&self) {
        config::save_config(&self.current_config());
    }
}

impl eframe::App for OpenEditApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll LSP events
        self.lsp_manager.poll_events();

        // Check for LSP completions
        if let Some(items) = self.lsp_manager.take_completions() {
            if !items.is_empty() {
                self.lsp_completions = items;
                self.lsp_completion_selected = 0;
                self.lsp_completions_visible = true;
            }
        }
        if let Some(hover) = self.lsp_manager.take_hover() {
            self.hover_text = Some(hover);
        }
        if let Some(loc) = self.lsp_manager.take_definition() {
            // Navigate to definition
            if let Ok(url) = Url::parse(&loc.uri) {
                if let Ok(path) = url.to_file_path() {
                    let existing = self
                        .documents
                        .iter()
                        .position(|d| d.path.as_ref() == Some(&path));
                    if let Some(tab_idx) = existing {
                        self.active_tab = tab_idx;
                    } else {
                        self.open_file(path);
                    }
                    if let Some(doc) = self.documents.get_mut(self.active_tab) {
                        doc.go_to_line(loc.line);
                    }
                }
            }
        }

        // Periodic git refresh
        self.git_state.maybe_refresh();

        // LSP: ensure server running and file opened for current doc
        if let Some(doc) = self.documents.get(self.active_tab) {
            if let Some(ref lang) = doc.language {
                let root = self.workspace_root();
                self.lsp_manager.ensure_server(lang, &root);

                if let Some(ref path) = doc.path {
                    let uri = Url::from_file_path(path)
                        .map(|u| u.to_string())
                        .unwrap_or_default();
                    if !self.lsp_opened_files.contains(&uri) {
                        let text = doc.buffer.to_string();
                        self.lsp_manager.did_open(lang, &uri, &text);
                        self.lsp_opened_files.insert(uri);
                    }
                }
            }

            // Git: compute line diff for current file
            if let Some(ref path) = doc.path {
                self.git_state.compute_line_diff(path);
            }
        }

        // LSP debounced didChange
        if let Some(timer) = self.lsp_change_timer {
            if timer.elapsed() > std::time::Duration::from_millis(300) {
                self.lsp_change_timer = None;
                if let Some(doc) = self.documents.get(self.active_tab) {
                    if let (Some(ref lang), Some(ref path)) = (&doc.language, &doc.path) {
                        let uri = Url::from_file_path(path)
                            .map(|u| u.to_string())
                            .unwrap_or_default();
                        let text = doc.buffer.to_string();
                        self.lsp_manager.did_change(lang, &uri, &text);
                    }
                }
            }
        }

        // Process pending file opens
        let pending: Vec<PathBuf> = std::mem::take(&mut self.pending_opens);
        for path in pending {
            self.open_file(path);
        }

        // Handle drag & drop files
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        for path in dropped {
            self.open_file(path);
        }

        // Check for external file modifications
        if let Some(ref rx) = self.watcher_rx {
            while let Ok(changed_path) = rx.try_recv() {
                // Find which tab has this path
                if self.external_change_tab.is_none() {
                    for (i, doc) in self.documents.iter().enumerate() {
                        if let Some(ref doc_path) = doc.path {
                            if *doc_path == changed_path {
                                self.external_change_tab = Some(i);
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Global keyboard shortcuts
        let mut open_file = false;
        let mut save_file = false;
        let mut save_as = false;
        let mut new_tab = false;
        let mut close_tab = false;
        let mut toggle_search = false;
        let mut toggle_replace = false;
        let mut go_to_line = false;
        let mut next_tab = false;
        let mut prev_tab = false;
        let mut toggle_command_palette = false;
        let mut toggle_find_in_files = false;
        let mut toggle_go_to_file = false;
        let mut toggle_go_to_symbol = false;
        let mut toggle_sidebar = false;
        let mut zoom_in = false;
        let mut zoom_out = false;
        let mut zoom_reset = false;
        let mut toggle_bookmark = false;
        let mut next_bookmark = false;
        let mut prev_bookmark = false;
        let mut toggle_md_preview = false;
        let mut toggle_macro_recording = false;
        let mut playback_macro = false;
        let mut toggle_terminal = false;
        let mut select_all_occurrences = false;
        let mut toggle_zen = false;
        let mut toggle_split = false;
        let mut diff_next_hunk = false;
        let mut diff_prev_hunk = false;

        ctx.input(|input| {
            let ctrl = input.modifiers.ctrl || input.modifiers.mac_cmd;
            let shift = input.modifiers.shift;

            for event in &input.events {
                if let egui::Event::Key {
                    key, pressed: true, ..
                } = event
                {
                    match key {
                        egui::Key::F7 if shift => diff_prev_hunk = true,
                        egui::Key::F7 => diff_next_hunk = true,
                        egui::Key::F11 => toggle_zen = true,
                        egui::Key::Backslash if ctrl => toggle_split = true,
                        egui::Key::Q if ctrl && shift => playback_macro = true,
                        egui::Key::Q if ctrl => toggle_macro_recording = true,
                        egui::Key::L if ctrl && shift => select_all_occurrences = true,
                        egui::Key::Backtick if ctrl => toggle_terminal = true,
                        egui::Key::O if ctrl && shift => toggle_go_to_symbol = true,
                        egui::Key::O if ctrl => open_file = true,
                        egui::Key::S if ctrl && shift => save_as = true,
                        egui::Key::S if ctrl => save_file = true,
                        egui::Key::N if ctrl => new_tab = true,
                        egui::Key::W if ctrl => close_tab = true,
                        egui::Key::M if ctrl && shift => toggle_md_preview = true,
                        egui::Key::F if ctrl && shift => toggle_find_in_files = true,
                        egui::Key::F if ctrl => toggle_search = true,
                        egui::Key::H if ctrl => toggle_replace = true,
                        egui::Key::G if ctrl => go_to_line = true,
                        egui::Key::P if ctrl && shift => toggle_command_palette = true,
                        egui::Key::P if ctrl => toggle_go_to_file = true,
                        egui::Key::B if ctrl => toggle_sidebar = true,
                        egui::Key::Tab if ctrl && shift => prev_tab = true,
                        egui::Key::Tab if ctrl => next_tab = true,
                        egui::Key::Plus if ctrl => zoom_in = true,
                        egui::Key::Equals if ctrl => zoom_in = true,
                        egui::Key::Minus if ctrl => zoom_out = true,
                        egui::Key::Num0 if ctrl => zoom_reset = true,
                        egui::Key::F2 if ctrl => toggle_bookmark = true,
                        egui::Key::F2 if shift => prev_bookmark = true,
                        egui::Key::F2 => next_bookmark = true,
                        egui::Key::Escape => {
                            if self.diff_state.active {
                                self.diff_state.active = false;
                            } else if self.go_to_symbol_state.open {
                                self.go_to_symbol_state.open = false;
                            } else if self.go_to_file_state.open {
                                self.go_to_file_state.open = false;
                            } else if self.go_to_line_open {
                                self.go_to_line_open = false;
                            } else if self.search_state.visible {
                                self.search_state.visible = false;
                            } else if self.find_in_files_state.visible {
                                self.find_in_files_state.visible = false;
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Ctrl+Scroll for zoom
            if ctrl && input.raw_scroll_delta.y != 0.0 {
                if input.raw_scroll_delta.y > 0.0 {
                    zoom_in = true;
                } else {
                    zoom_out = true;
                }
            }
        });

        if open_file {
            self.open_dialog();
        }
        if save_file {
            self.save_current();
        }
        if save_as {
            self.save_as();
        }
        if new_tab {
            self.documents.push(Document::new());
            self.active_tab = self.documents.len() - 1;
        }
        if close_tab {
            let idx = self.active_tab;
            self.close_tab(idx);
        }
        if toggle_search {
            self.search_state.visible = !self.search_state.visible;
            self.search_state.show_replace = false;
        }
        if toggle_replace {
            self.search_state.visible = true;
            self.search_state.show_replace = true;
        }
        if go_to_line {
            self.go_to_line_open = true;
            self.go_to_line_input.clear();
        }
        if next_tab && !self.documents.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.documents.len();
        }
        if prev_tab && !self.documents.is_empty() {
            self.active_tab = if self.active_tab == 0 {
                self.documents.len() - 1
            } else {
                self.active_tab - 1
            };
        }
        if toggle_find_in_files {
            self.find_in_files_state.visible = !self.find_in_files_state.visible;
        }
        if toggle_md_preview {
            self.show_markdown_preview = !self.show_markdown_preview;
            self.markdown_preview_scroll = 0.0;
        }
        if toggle_command_palette {
            self.command_palette.open = !self.command_palette.open;
            self.command_palette.query.clear();
            self.command_palette.selected = 0;
        }
        if toggle_sidebar {
            self.sidebar_state.visible = !self.sidebar_state.visible;
            if self.sidebar_state.visible && self.sidebar_state.root.is_none() {
                let root = self.workspace_root();
                self.sidebar_state.load_tree(&root);
            }
            self.save_config_state();
        }
        if toggle_go_to_file {
            self.go_to_file_state.open = !self.go_to_file_state.open;
            if self.go_to_file_state.open {
                let root = self.workspace_root();
                self.go_to_file_state.scan(&root);
            }
            self.go_to_file_state.query.clear();
            self.go_to_file_state.selected = 0;
        }
        if toggle_go_to_symbol {
            self.execute_command("nav.go_to_symbol");
        }
        if zoom_in {
            self.font_size = (self.font_size + 1.0).min(48.0);
            self.save_config_state();
        }
        if zoom_out {
            self.font_size = (self.font_size - 1.0).max(6.0);
            self.save_config_state();
        }
        if zoom_reset {
            self.font_size = 13.0;
            self.save_config_state();
        }
        if toggle_bookmark {
            if let Some(doc) = self.documents.get_mut(self.active_tab) {
                let line = doc.cursors.primary().position.line;
                doc.toggle_bookmark(line);
            }
        }
        if next_bookmark {
            if let Some(doc) = self.documents.get_mut(self.active_tab) {
                let current_line = doc.cursors.primary().position.line;
                if let Some(target) = doc.next_bookmark(current_line) {
                    doc.go_to_line(target);
                }
            }
        }
        if prev_bookmark {
            if let Some(doc) = self.documents.get_mut(self.active_tab) {
                let current_line = doc.cursors.primary().position.line;
                if let Some(target) = doc.prev_bookmark(current_line) {
                    doc.go_to_line(target);
                }
            }
        }
        if toggle_macro_recording {
            if self.macro_recorder.is_recording() {
                self.macro_recorder.stop_recording();
            } else {
                self.macro_recorder.start_recording();
            }
        }
        if playback_macro {
            // Only replay if not currently recording
            if !self.macro_recorder.is_recording() {
                self.replay_macro();
            }
        }
        if toggle_terminal {
            if !self.terminal_state.visible {
                self.terminal_state.visible = true;
                if !self.terminal_state.running() {
                    self.terminal_state.start();
                }
                self.terminal_focused = true;
            } else {
                self.terminal_focused = !self.terminal_focused;
            }
        }
        if select_all_occurrences {
            if let Some(doc) = self.documents.get_mut(self.active_tab) {
                doc.select_all_occurrences();
            }
        }
        if toggle_zen {
            self.zen_mode = !self.zen_mode;
            if self.zen_mode {
                self.sidebar_state.visible = false;
            }
        }
        if toggle_split {
            if self.split.active {
                self.split.active = false;
            } else {
                self.split.active = true;
                self.split.direction = SplitDirection::Horizontal;
                self.split.second_tab = self.active_tab;
            }
        }
        // Diff hunk navigation (F7 / Shift+F7)
        if diff_next_hunk && self.diff_state.active {
            let line_height =
                crate::editor_view::line_height_for_font(self.font_size);
            diff_view::navigate_next_hunk(&mut self.diff_state, line_height);
        }
        if diff_prev_hunk && self.diff_state.active {
            let line_height =
                crate::editor_view::line_height_for_font(self.font_size);
            diff_view::navigate_prev_hunk(&mut self.diff_state, line_height);
        }

        // Build tab data
        let tabs: Vec<(String, bool, Option<String>)> = self
            .documents
            .iter()
            .map(|d| {
                (
                    d.display_name(),
                    d.modified,
                    d.path.as_ref().map(|p| p.display().to_string()),
                )
            })
            .collect();

        // ── Menu bar ──────────────────────────────────────────────
        if !self.zen_mode {
            egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    // ── File ──
                    ui.menu_button("File", |ui| {
                        if ui.button("New File          Ctrl+N").clicked() {
                            self.documents.push(Document::new());
                            self.active_tab = self.documents.len() - 1;
                            ui.close_menu();
                        }
                        if ui.button("Open File         Ctrl+O").clicked() {
                            self.open_dialog();
                            ui.close_menu();
                        }
                        if ui.button("Open Folder").clicked() {
                            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                                self.sidebar_state.visible = true;
                                self.sidebar_state.load_tree(&dir);
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Save              Ctrl+S").clicked() {
                            self.save_current();
                            ui.close_menu();
                        }
                        if ui.button("Save As       Ctrl+Shift+S").clicked() {
                            self.save_as();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Close Tab         Ctrl+W").clicked() {
                            let idx = self.active_tab;
                            self.close_tab(idx);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Exit              Ctrl+Q").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            ui.close_menu();
                        }
                    });

                    // ── Edit ──
                    ui.menu_button("Edit", |ui| {
                        if ui.button("Undo              Ctrl+Z").clicked() {
                            self.execute_command("edit.undo");
                            ui.close_menu();
                        }
                        if ui.button("Redo              Ctrl+Y").clicked() {
                            self.execute_command("edit.redo");
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Cut               Ctrl+X").clicked() {
                            if let Some(doc) = self.documents.get_mut(self.active_tab) {
                                if doc.cursors.primary().has_selection() {
                                    let text = doc.selected_text();
                                    self.pending_clipboard = Some(text);
                                    doc.delete_selection_public();
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Copy              Ctrl+C").clicked() {
                            if let Some(doc) = self.documents.get(self.active_tab) {
                                if doc.cursors.primary().has_selection() {
                                    self.pending_clipboard = Some(doc.selected_text());
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Paste             Ctrl+V").clicked() {
                            log::info!("Paste: use Ctrl+V in editor");
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Find              Ctrl+F").clicked() {
                            self.execute_command("nav.find");
                            ui.close_menu();
                        }
                        if ui.button("Find in Files Ctrl+Shift+F").clicked() {
                            self.execute_command("nav.find_in_files");
                            ui.close_menu();
                        }
                        if ui.button("Replace           Ctrl+H").clicked() {
                            self.execute_command("nav.replace");
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Select All        Ctrl+A").clicked() {
                            self.execute_command("edit.select_all");
                            ui.close_menu();
                        }
                        if ui.button("Toggle Comment    Ctrl+/").clicked() {
                            self.execute_command("edit.toggle_comment");
                            ui.close_menu();
                        }
                    });

                    // ── View ──
                    ui.menu_button("View", |ui| {
                        if ui.button("Command Palette Ctrl+Shift+P").clicked() {
                            self.command_palette.open = true;
                            self.command_palette.query.clear();
                            self.command_palette.selected = 0;
                            ui.close_menu();
                        }
                        if ui.button("Toggle Sidebar    Ctrl+B").clicked() {
                            self.execute_command("view.toggle_sidebar");
                            ui.close_menu();
                        }
                        if ui.button("Toggle Terminal   Ctrl+`").clicked() {
                            self.execute_command("view.toggle_terminal");
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .button(if self.show_minimap {
                                "✓ Minimap"
                            } else {
                                "  Minimap"
                            })
                            .clicked()
                        {
                            self.execute_command("view.toggle_minimap");
                            ui.close_menu();
                        }
                        if ui
                            .button(if self.show_line_numbers {
                                "✓ Line Numbers"
                            } else {
                                "  Line Numbers"
                            })
                            .clicked()
                        {
                            self.show_line_numbers = !self.show_line_numbers;
                            ui.close_menu();
                        }
                        if ui
                            .button(if self.word_wrap {
                                "✓ Word Wrap"
                            } else {
                                "  Word Wrap"
                            })
                            .clicked()
                        {
                            self.execute_command("view.toggle_word_wrap");
                            ui.close_menu();
                        }
                        if ui.button(if self.show_markdown_preview { "✓ Markdown Preview  Ctrl+Shift+M" } else { "  Markdown Preview  Ctrl+Shift+M" }).clicked() {
                            self.show_markdown_preview = !self.show_markdown_preview;
                            self.markdown_preview_scroll = 0.0;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Zen Mode          F11").clicked() {
                            self.execute_command("view.zen_mode");
                            ui.close_menu();
                        }
                        if ui.button("Zoom In           Ctrl+=").clicked() {
                            self.execute_command("view.zoom_in");
                            ui.close_menu();
                        }
                        if ui.button("Zoom Out          Ctrl+-").clicked() {
                            self.execute_command("view.zoom_out");
                            ui.close_menu();
                        }
                    });

                    // ── Selection ──
                    ui.menu_button("Selection", |ui| {
                        if ui.button("Add Cursor Above  Ctrl+Alt+Up").clicked() {
                            log::info!("Add Cursor Above: not yet implemented");
                            ui.close_menu();
                        }
                        if ui.button("Add Cursor Below  Ctrl+Alt+Down").clicked() {
                            log::info!("Add Cursor Below: not yet implemented");
                            ui.close_menu();
                        }
                        if ui.button("Select All Occurrences Ctrl+Shift+L").clicked() {
                            self.execute_command("edit.select_all_occurrences");
                            ui.close_menu();
                        }
                        if ui.button("Add Next Occurrence    Ctrl+D").clicked() {
                            self.execute_command("edit.select_next_occurrence");
                            ui.close_menu();
                        }
                    });

                    // ── Go ──
                    ui.menu_button("Go", |ui| {
                        if ui.button("Go to Line        Ctrl+G").clicked() {
                            self.execute_command("nav.go_to_line");
                            ui.close_menu();
                        }
                        if ui.button("Go to File        Ctrl+P").clicked() {
                            self.execute_command("nav.go_to_file");
                            ui.close_menu();
                        }
                        if ui.button("Go to Definition  F12").clicked() {
                            log::info!("Go to Definition: LSP required");
                            ui.close_menu();
                        }
                        if ui.button("Go to References  Shift+F12").clicked() {
                            log::info!("Go to References: LSP required");
                            ui.close_menu();
                        }
                    });

                    // ── Terminal ──
                    ui.menu_button("Terminal", |ui| {
                        if ui.button("New Terminal").clicked() {
                            self.execute_command("terminal.new");
                            ui.close_menu();
                        }
                        if ui.button("Send Selection to Terminal").clicked() {
                            self.execute_command("terminal.send_selection");
                            ui.close_menu();
                        }
                    });

                    // ── Settings ──
                    ui.menu_button("Settings", |ui| {
                        ui.menu_button("Theme", |ui| {
                            for name in EditorTheme::all_names() {
                                let label = if self.theme.name == *name {
                                    format!("✓ {}", name)
                                } else {
                                    format!("  {}", name)
                                };
                                if ui.button(label).clicked() {
                                    self.theme = EditorTheme::by_name(name);
                                    self.save_config_state();
                                    ui.close_menu();
                                }
                            }
                        });
                        if ui
                            .button(if self.vim_state.enabled {
                                "✓ Vim Mode"
                            } else {
                                "  Vim Mode"
                            })
                            .clicked()
                        {
                            self.execute_command("edit.toggle_vim_mode");
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label("Font Size:");
                            if ui.small_button("−").clicked() {
                                self.font_size = (self.font_size - 1.0).max(6.0);
                                self.save_config_state();
                            }
                            ui.label(format!("{:.0}", self.font_size));
                            if ui.small_button("+").clicked() {
                                self.font_size = (self.font_size + 1.0).min(48.0);
                                self.save_config_state();
                            }
                        });
                        ui.menu_button("Tab Size", |ui| {
                            for &size in &[2u32, 4, 8] {
                                let label = if self.current_config().editor.tab_size == size {
                                    format!("✓ {}", size)
                                } else {
                                    format!("  {}", size)
                                };
                                if ui.button(label).clicked() {
                                    log::info!("Tab size set to {}", size);
                                    ui.close_menu();
                                }
                            }
                        });
                        if ui
                            .button(if self.auto_save {
                                "✓ Auto Save"
                            } else {
                                "  Auto Save"
                            })
                            .clicked()
                        {
                            self.auto_save = !self.auto_save;
                            log::info!(
                                "Auto Save: {}",
                                if self.auto_save {
                                    "enabled"
                                } else {
                                    "disabled"
                                }
                            );
                            ui.close_menu();
                        }
                    });

                    // ── Help ──
                    ui.menu_button("Help", |ui| {
                        if ui.button("About OpenEdit").clicked() {
                            self.show_about = true;
                            ui.close_menu();
                        }
                        if ui.button("Keyboard Shortcuts").clicked() {
                            self.show_shortcuts = true;
                            ui.close_menu();
                        }
                    });
                });
            });
        }

        // Render
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(self.theme.background))
            .show(ctx, |ui| {
                // In zen mode, skip tab bar, sidebar, status bar
                if self.zen_mode {
                    // Centered editor only
                    let rect = ui.available_rect_before_wrap();
                    let max_text_width = 800.0_f32;
                    let margin = ((rect.width() - max_text_width) / 2.0).max(0.0);
                    let zen_rect = egui::Rect::from_min_size(
                        egui::Pos2::new(rect.left() + margin, rect.top()),
                        egui::Vec2::new(rect.width() - margin * 2.0, rect.height()),
                    );
                    let mut zen_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(zen_rect)
                            .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    );

                    let empty_diffs: Vec<(usize, crate::git::LineDiffStatus)> = Vec::new();
                    let empty_blame: std::collections::HashMap<usize, String> =
                        std::collections::HashMap::new();
                    let empty_diags: Vec<crate::lsp::LspDiagnostic> = Vec::new();
                    let render_context = EditorRenderContext {
                        git_line_diffs: &empty_diffs,
                        git_blame_info: &empty_blame,
                        show_blame: false,
                        lsp_diagnostics: &empty_diags,
                        bracket_colorization: self.bracket_colorization,
                    };

                    if let Some(doc) = self.documents.get_mut(self.active_tab) {
                        editor_view::render_editor(
                            &mut zen_ui,
                            doc,
                            &self.theme,
                            false,
                            &mut self.editor_view_state,
                            &mut self.syntax_engine,
                            self.font_size,
                            self.show_whitespace,
                            false,
                            &mut self.autocomplete,
                            self.word_wrap,
                            &mut self.macro_recorder,
                            Some(&render_context),
                        );
                    }

                    // Vim mode indicator in zen mode
                    if self.vim_state.enabled {
                        let mode_text = format!("-- {} --", self.vim_state.mode);
                        let text_pos = egui::Pos2::new(rect.center().x, rect.bottom() - 30.0);
                        ui.painter().text(
                            text_pos,
                            egui::Align2::CENTER_TOP,
                            &mode_text,
                            egui::FontId::monospace(14.0),
                            self.theme.status_bar_fg,
                        );
                    }

                    // Command palette still works in zen mode
                    if self.command_palette.open {
                        if let Some(cmd_id) =
                            command_palette::render_command_palette(ctx, &mut self.command_palette)
                        {
                            self.execute_command(cmd_id);
                        }
                    }

                    ctx.request_repaint_after(std::time::Duration::from_millis(500));
                    return;
                }

                // Tab bar
                let tab_response = tab_bar::render_tab_bar(
                    ui,
                    &tabs,
                    self.active_tab,
                    &self.theme,
                    &mut self.tab_drag_state,
                );

                if let Some(idx) = tab_response.activate {
                    if idx == usize::MAX {
                        // New tab
                        self.documents.push(Document::new());
                        self.active_tab = self.documents.len() - 1;
                    } else if idx < self.documents.len() {
                        self.active_tab = idx;
                    }
                }
                if let Some(idx) = tab_response.close {
                    self.close_tab(idx);
                }

                // Handle tab reordering
                if let Some((from_idx, to_idx)) = tab_response.reorder {
                    if from_idx < self.documents.len() && to_idx < self.documents.len() {
                        let doc = self.documents.remove(from_idx);
                        self.documents.insert(to_idx, doc);
                        if self.active_tab == from_idx {
                            self.active_tab = to_idx;
                        } else if from_idx < self.active_tab && self.active_tab <= to_idx {
                            self.active_tab -= 1;
                        } else if from_idx > self.active_tab && self.active_tab >= to_idx {
                            self.active_tab += 1;
                        }
                    }
                }

                if let Some((idx, action)) = tab_response.context_menu {
                    use crate::tab_bar::TabContextAction;
                    match action {
                        TabContextAction::CloseOthers => {
                            // Collect indices to remove (all except idx), remove in reverse order
                            let indices: Vec<usize> =
                                (0..self.documents.len()).filter(|&i| i != idx).collect();
                            for &i in indices.iter().rev() {
                                self.force_close_tab(i);
                            }
                            self.active_tab = 0;
                        }
                        TabContextAction::CloseAll => {
                            self.documents.clear();
                            self.documents.push(Document::new());
                            self.active_tab = 0;
                        }
                        TabContextAction::CloseToRight => {
                            let count = self.documents.len();
                            for i in (idx + 1..count).rev() {
                                self.force_close_tab(i);
                            }
                            if self.active_tab > idx {
                                self.active_tab = idx;
                            }
                        }
                        TabContextAction::CopyPath(path) => {
                            self.pending_clipboard = Some(path);
                        }
                        TabContextAction::RevealInFileManager(path) => {
                            let path_buf = std::path::Path::new(&path);
                            let dir = path_buf.parent().unwrap_or(path_buf);
                            #[cfg(target_os = "linux")]
                            {
                                let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
                            }
                            #[cfg(target_os = "macos")]
                            {
                                let _ = std::process::Command::new("open")
                                    .arg("-R")
                                    .arg(&path)
                                    .spawn();
                            }
                            #[cfg(target_os = "windows")]
                            {
                                let _ = std::process::Command::new("explorer")
                                    .arg(format!("/select,{}", path))
                                    .spawn();
                            }
                        }
                    }
                }

                ui.separator();

                // Determine sidebar width for layout splitting
                let sidebar_visible = self.sidebar_state.visible;
                let sidebar_width = if sidebar_visible {
                    self.sidebar_state.width
                } else {
                    0.0
                };

                // Available area after tab bar (for sidebar + main content)
                let full_available = ui.available_rect_before_wrap();

                // --- Sidebar ---
                let mut sidebar_file_clicked: Option<PathBuf> = None;
                if sidebar_visible {
                    let sidebar_rect = egui::Rect::from_min_size(
                        full_available.left_top(),
                        egui::Vec2::new(sidebar_width, full_available.height()),
                    );
                    let mut sidebar_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(sidebar_rect)
                            .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    );
                    sidebar_file_clicked = sidebar::render_sidebar(
                        &mut sidebar_ui,
                        &mut self.sidebar_state,
                        &self.theme,
                        self.font_size,
                        Some(&self.git_state),
                    );
                }

                // --- Function list panel (right side) ---
                let fn_list_visible = self.function_list_state.visible;
                let fn_list_width = if fn_list_visible { 250.0 } else { 0.0 };
                let mut fn_list_navigate: Option<usize> = None;

                if fn_list_visible {
                    // Refresh symbols if the panel is visible (live update)
                    if let Some(doc) = self.documents.get(self.active_tab) {
                        let source = doc.buffer.to_string();
                        let lang_key = doc.language.as_deref().and_then(SyntaxEngine::language_key);
                        if let Some(key) = lang_key {
                            self.function_list_state.symbols =
                                self.syntax_engine.extract_symbols(&source, key);
                        } else {
                            self.function_list_state.symbols.clear();
                        }
                    }

                    let fn_list_rect = egui::Rect::from_min_size(
                        egui::Pos2::new(
                            full_available.right() - fn_list_width,
                            full_available.top(),
                        ),
                        egui::Vec2::new(fn_list_width, full_available.height()),
                    );
                    let mut fn_list_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(fn_list_rect)
                            .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    );
                    fn_list_navigate = function_list::render_function_list(
                        &mut fn_list_ui,
                        &mut self.function_list_state,
                        &self.theme,
                    );
                }

                // --- Main content area (right of sidebar, left of function list) ---
                let main_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(full_available.left() + sidebar_width, full_available.top()),
                    egui::Vec2::new(
                        full_available.width() - sidebar_width - fn_list_width,
                        full_available.height(),
                    ),
                );
                let mut main_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(main_rect)
                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                );

                // Search panel
                if self.search_state.visible {
                    if let Some(doc) = self.documents.get_mut(self.active_tab) {
                        let should_close = search_panel::render_search_panel(
                            &mut main_ui,
                            &mut self.search_state,
                            doc,
                        );
                        if should_close {
                            self.search_state.visible = false;
                            doc.search.clear();
                        }
                    }
                    main_ui.separator();
                }

                // Breadcrumb bar
                if self.breadcrumb_state.visible {
                    if let Some(doc) = self.documents.get(self.active_tab) {
                        let cursor = doc.cursors.primary().position;
                        breadcrumb::render_breadcrumb(
                            &mut main_ui,
                            &self.breadcrumb_state,
                            &self.theme,
                            &self.function_list_state.symbols,
                            cursor.line,
                        );
                    }
                }

                // Editor viewport (main area minus status bar and optional find-in-files panel)
                let show_search = self.search_state.visible;
                let show_find_in_files = self.find_in_files_state.visible;

                // Reserve space for status bar at bottom
                let available = main_ui.available_rect_before_wrap();
                let status_bar_height = 24.0;
                let content_height = available.height() - status_bar_height;

                // Split between editor, find-in-files panel, and terminal
                let terminal_height = if self.terminal_state.visible {
                    (content_height * self.terminal_state.height_fraction)
                        .max(80.0)
                        .min(content_height - 100.0)
                } else {
                    0.0
                };
                let remaining_height = content_height - terminal_height;
                let find_panel_height = if show_find_in_files {
                    (remaining_height * 0.30)
                        .max(150.0)
                        .min(remaining_height - 100.0)
                } else {
                    0.0
                };
                let editor_height = remaining_height - find_panel_height;

                let editor_rect = egui::Rect::from_min_size(
                    available.left_top(),
                    egui::Vec2::new(available.width(), editor_height),
                );

                // Editor (with optional split view or hex view)
                let split_active = self.split.active;
                let split_dir = self.split.direction;
                let second_tab = self
                    .split
                    .second_tab
                    .min(self.documents.len().saturating_sub(1));

                // Build render context for editor (git + LSP + bracket colors)
                let empty_diffs: Vec<(usize, crate::git::LineDiffStatus)> = Vec::new();
                let empty_blame: std::collections::HashMap<usize, String> =
                    std::collections::HashMap::new();
                let empty_diags: Vec<crate::lsp::LspDiagnostic> = Vec::new();
                let cur_path = self
                    .documents
                    .get(self.active_tab)
                    .and_then(|d| d.path.clone());
                let git_line_diffs = cur_path
                    .as_ref()
                    .map(|p| self.git_state.get_line_diffs(p).to_vec())
                    .unwrap_or_default();
                let lsp_diagnostics = cur_path
                    .as_ref()
                    .map(|p| self.lsp_manager.get_diagnostics(p).to_vec())
                    .unwrap_or_default();
                let render_context = EditorRenderContext {
                    git_line_diffs: if git_line_diffs.is_empty() {
                        &empty_diffs
                    } else {
                        &git_line_diffs
                    },
                    git_blame_info: if self.git_state.show_blame {
                        &self.git_state.blame_info
                    } else {
                        &empty_blame
                    },
                    show_blame: self.git_state.show_blame,
                    lsp_diagnostics: if lsp_diagnostics.is_empty() {
                        &empty_diags
                    } else {
                        &lsp_diagnostics
                    },
                    bracket_colorization: self.bracket_colorization,
                };

                if self.diff_state.active {
                    // Diff/compare view replaces the normal editor
                    let left_tab = self
                        .diff_state
                        .left_tab
                        .min(self.documents.len().saturating_sub(1));
                    let right_tab = self
                        .diff_state
                        .right_tab
                        .min(self.documents.len().saturating_sub(1));
                    let left_content = self
                        .documents
                        .get(left_tab)
                        .map(|d| d.buffer.to_string())
                        .unwrap_or_default();
                    let right_content = self
                        .documents
                        .get(right_tab)
                        .map(|d| d.buffer.to_string())
                        .unwrap_or_default();
                    let left_name = self
                        .documents
                        .get(left_tab)
                        .map(|d| d.display_name())
                        .unwrap_or_else(|| "Left".to_string());
                    let right_name = self
                        .documents
                        .get(right_tab)
                        .map(|d| d.display_name())
                        .unwrap_or_else(|| "Right".to_string());
                    let mut editor_ui = main_ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(editor_rect)
                            .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    );
                    let diff_action = diff_view::render_diff_view(
                        &mut editor_ui,
                        &mut self.diff_state,
                        &left_content,
                        &right_content,
                        &left_name,
                        &right_name,
                        &self.theme,
                        self.font_size,
                    );
                    // Handle merge actions from diff view
                    match diff_action {
                        diff_view::DiffAction::MergeLeftToRight(new_content) => {
                            if let Some(doc) = self.documents.get_mut(right_tab) {
                                let cursor = *doc.cursors.primary();
                                let old_len = doc.buffer.len_chars();
                                let old_text = doc.buffer.to_string();
                                doc.undo_manager.record(
                                    openedit_core::edit::EditOp::Replace {
                                        offset: 0,
                                        old_text,
                                        new_text: new_content.clone(),
                                    },
                                    cursor,
                                );
                                doc.buffer.remove(0..old_len);
                                doc.buffer.insert(0, &new_content);
                                doc.modified = true;
                                // Force diff recompute
                                self.diff_state.invalidate_cache();
                            }
                        }
                        diff_view::DiffAction::MergeRightToLeft(new_content) => {
                            if let Some(doc) = self.documents.get_mut(left_tab) {
                                let cursor = *doc.cursors.primary();
                                let old_len = doc.buffer.len_chars();
                                let old_text = doc.buffer.to_string();
                                doc.undo_manager.record(
                                    openedit_core::edit::EditOp::Replace {
                                        offset: 0,
                                        old_text,
                                        new_text: new_content.clone(),
                                    },
                                    cursor,
                                );
                                doc.buffer.remove(0..old_len);
                                doc.buffer.insert(0, &new_content);
                                doc.modified = true;
                                // Force diff recompute
                                self.diff_state.invalidate_cache();
                            }
                        }
                        diff_view::DiffAction::None => {}
                    }
                } else if self.hex_view_state.active {
                    // Hex editor view replaces the normal editor
                    let mut editor_ui = main_ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(editor_rect)
                            .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    );
                    let hex_action = hex_view::render_hex_view(
                        &mut editor_ui,
                        &mut self.hex_view_state,
                        &self.theme,
                        self.font_size,
                    );
                    // Handle hex edit actions through the document undo system
                    if let hex_view::HexAction::EditByte {
                        offset,
                        old_byte,
                        new_byte,
                    } = hex_action
                    {
                        if let Some(doc) = self.documents.get_mut(self.active_tab) {
                            // Convert byte offset to a char-level edit in the document.
                            // The hex view works on raw bytes, so we replace the full
                            // document content with the updated bytes re-encoded.
                            let cursor = *doc.cursors.primary();
                            let old_text = doc.buffer.to_string();
                            // Build new text from the modified hex data
                            let new_text =
                                String::from_utf8_lossy(&self.hex_view_state.data).to_string();
                            let old_len = doc.buffer.len_chars();
                            doc.undo_manager.record(
                                openedit_core::edit::EditOp::Replace {
                                    offset: 0,
                                    old_text: old_text.clone(),
                                    new_text: new_text.clone(),
                                },
                                cursor,
                            );
                            doc.buffer.remove(0..old_len);
                            doc.buffer.insert(0, &new_text);
                            doc.modified = true;
                            let _ = (offset, old_byte, new_byte); // used in the HexAction
                        }
                    }
                } else if split_active && !self.documents.is_empty() {
                    let ratio = self.split_ratio.clamp(0.15, 0.85);
                    // Compute the two sub-rects with draggable divider
                    let (pane1_rect, pane2_rect, divider_rect) = if split_dir
                        == SplitDirection::Horizontal
                    {
                        let sep = 6.0;
                        let first_w = (editor_rect.width() - sep) * ratio;
                        let r1 = egui::Rect::from_min_size(
                            editor_rect.left_top(),
                            egui::Vec2::new(first_w, editor_rect.height()),
                        );
                        let div = egui::Rect::from_min_size(
                            egui::Pos2::new(editor_rect.left() + first_w, editor_rect.top()),
                            egui::Vec2::new(sep, editor_rect.height()),
                        );
                        let r2 = egui::Rect::from_min_size(
                            egui::Pos2::new(editor_rect.left() + first_w + sep, editor_rect.top()),
                            egui::Vec2::new(
                                editor_rect.width() - first_w - sep,
                                editor_rect.height(),
                            ),
                        );
                        (r1, r2, div)
                    } else {
                        let sep = 6.0;
                        let first_h = (editor_rect.height() - sep) * ratio;
                        let r1 = egui::Rect::from_min_size(
                            editor_rect.left_top(),
                            egui::Vec2::new(editor_rect.width(), first_h),
                        );
                        let div = egui::Rect::from_min_size(
                            egui::Pos2::new(editor_rect.left(), editor_rect.top() + first_h),
                            egui::Vec2::new(editor_rect.width(), sep),
                        );
                        let r2 = egui::Rect::from_min_size(
                            egui::Pos2::new(editor_rect.left(), editor_rect.top() + first_h + sep),
                            egui::Vec2::new(
                                editor_rect.width(),
                                editor_rect.height() - first_h - sep,
                            ),
                        );
                        (r1, r2, div)
                    };

                    // Handle divider drag
                    let divider_response = main_ui.interact(
                        divider_rect,
                        main_ui.id().with("split_divider"),
                        egui::Sense::drag(),
                    );
                    if divider_response.hovered() {
                        ctx.set_cursor_icon(if split_dir == SplitDirection::Horizontal {
                            egui::CursorIcon::ResizeHorizontal
                        } else {
                            egui::CursorIcon::ResizeVertical
                        });
                    }
                    if divider_response.dragged() {
                        if let Some(pos) = divider_response.interact_pointer_pos() {
                            if split_dir == SplitDirection::Horizontal {
                                self.split_ratio = ((pos.x - editor_rect.left())
                                    / editor_rect.width())
                                .clamp(0.15, 0.85);
                            } else {
                                self.split_ratio = ((pos.y - editor_rect.top())
                                    / editor_rect.height())
                                .clamp(0.15, 0.85);
                            }
                        }
                    }

                    // Draw separator
                    let sep_color = self.theme.gutter_fg;
                    main_ui.painter().rect_filled(
                        divider_rect,
                        0.0,
                        egui::Color32::from_rgb(60, 60, 60),
                    );
                    if split_dir == SplitDirection::Horizontal {
                        let x = divider_rect.center().x;
                        main_ui.painter().line_segment(
                            [
                                egui::Pos2::new(x, divider_rect.top()),
                                egui::Pos2::new(x, divider_rect.bottom()),
                            ],
                            egui::Stroke::new(1.0, sep_color),
                        );
                    } else {
                        let y = divider_rect.center().y;
                        main_ui.painter().line_segment(
                            [
                                egui::Pos2::new(divider_rect.left(), y),
                                egui::Pos2::new(divider_rect.right(), y),
                            ],
                            egui::Stroke::new(1.0, sep_color),
                        );
                    }

                    // Render pane 1 (active tab)
                    {
                        let mut pane1_ui = main_ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(pane1_rect)
                                .layout(egui::Layout::top_down(egui::Align::LEFT)),
                        );
                        if let Some(doc) = self.documents.get_mut(self.active_tab) {
                            editor_view::render_editor(
                                &mut pane1_ui,
                                doc,
                                &self.theme,
                                show_search,
                                &mut self.editor_view_state,
                                &mut self.syntax_engine,
                                self.font_size,
                                self.show_whitespace,
                                self.show_minimap,
                                &mut self.autocomplete,
                                self.word_wrap,
                                &mut self.macro_recorder,
                                Some(&render_context),
                            );
                        }
                    }

                    // Render pane 2 (second tab)
                    {
                        let mut pane2_ui = main_ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(pane2_rect)
                                .layout(egui::Layout::top_down(egui::Align::LEFT)),
                        );
                        if let Some(doc) = self.documents.get_mut(second_tab) {
                            editor_view::render_editor(
                                &mut pane2_ui,
                                doc,
                                &self.theme,
                                false,
                                &mut self.split.second_view_state,
                                &mut self.syntax_engine,
                                self.font_size,
                                self.show_whitespace,
                                self.show_minimap,
                                &mut self.split.second_autocomplete,
                                self.word_wrap,
                                &mut self.macro_recorder,
                                Some(&render_context),
                            );
                        }
                    }
                } else if self.show_markdown_preview {
                    // Split: editor on left, markdown preview on right
                    let half_w = editor_rect.width() / 2.0;
                    let left_rect = egui::Rect::from_min_size(
                        editor_rect.left_top(),
                        egui::Vec2::new(half_w - 1.0, editor_rect.height()),
                    );
                    let right_rect = egui::Rect::from_min_size(
                        egui::Pos2::new(editor_rect.left() + half_w + 1.0, editor_rect.top()),
                        egui::Vec2::new(half_w - 1.0, editor_rect.height()),
                    );

                    // Divider line
                    main_ui.painter().line_segment(
                        [
                            egui::Pos2::new(editor_rect.left() + half_w, editor_rect.top()),
                            egui::Pos2::new(editor_rect.left() + half_w, editor_rect.bottom()),
                        ],
                        egui::Stroke::new(2.0, self.theme.gutter_bg),
                    );

                    // Editor pane
                    let mut editor_ui = main_ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(left_rect)
                            .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    );
                    let source_for_preview;
                    if let Some(doc) = self.documents.get_mut(self.active_tab) {
                        source_for_preview = doc.buffer.to_string();
                        editor_view::render_editor(
                            &mut editor_ui,
                            doc,
                            &self.theme,
                            show_search,
                            &mut self.editor_view_state,
                            &mut self.syntax_engine,
                            self.font_size,
                            self.show_whitespace,
                            self.show_minimap,
                            &mut self.autocomplete,
                            self.word_wrap,
                            &mut self.macro_recorder,
                            Some(&render_context),
                        );
                    } else {
                        source_for_preview = String::new();
                    }

                    // Markdown preview pane
                    let mut preview_ui = main_ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(right_rect)
                            .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    );
                    crate::markdown_preview::render_markdown_preview(
                        &mut preview_ui,
                        &source_for_preview,
                        &self.theme,
                        &mut self.markdown_preview_scroll,
                    );
                } else {
                    // Single editor pane
                    let mut editor_ui = main_ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(editor_rect)
                            .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    );
                    if let Some(doc) = self.documents.get_mut(self.active_tab) {
                        let was_modified = editor_view::render_editor(
                            &mut editor_ui,
                            doc,
                            &self.theme,
                            show_search,
                            &mut self.editor_view_state,
                            &mut self.syntax_engine,
                            self.font_size,
                            self.show_whitespace,
                            self.show_minimap,
                            &mut self.autocomplete,
                            self.word_wrap,
                            &mut self.macro_recorder,
                            Some(&render_context),
                        );
                        if was_modified {
                            // Trigger debounced LSP didChange and request completions
                            self.lsp_change_timer = Some(std::time::Instant::now());
                            if let (Some(ref lang), Some(ref path)) = (&doc.language, &doc.path) {
                                let uri = url::Url::from_file_path(path)
                                    .map(|u| u.to_string())
                                    .unwrap_or_default();
                                let cursor = doc.cursors.primary().position;
                                self.lsp_manager.request_completion(
                                    lang,
                                    &uri,
                                    cursor.line as u32,
                                    cursor.col as u32,
                                );
                            }
                        }
                    }
                }

                // LSP hover tooltip
                if let Some(ref hover_text) = self.hover_text {
                    if let Some(pos) = self.hover_pos {
                        let mut hover_ui = main_ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(editor_rect)
                                .layout(egui::Layout::top_down(egui::Align::LEFT)),
                        );
                        crate::lsp::render_hover_tooltip(&mut hover_ui, hover_text, pos);
                    }
                }

                // LSP autocomplete popup (when visible, overlays on the editor)
                if self.lsp_completions_visible && !self.lsp_completions.is_empty() {
                    if let Some(doc) = self.documents.get(self.active_tab) {
                        let cpos = doc.cursors.primary().position;
                        let char_w = crate::editor_view::char_width_for_font(self.font_size);
                        let line_h = crate::editor_view::line_height_for_font(self.font_size);
                        // Approximate cursor screen position
                        let digit_count = format!("{}", doc.buffer.len_lines()).len().max(3);
                        let gutter_w = (digit_count as f32 + 2.0) * char_w + char_w * 1.5 + 8.0;
                        let visible_line = cpos.line.saturating_sub(doc.scroll_line);
                        let cursor_screen = egui::Pos2::new(
                            editor_rect.left() + gutter_w + 4.0 + cpos.col as f32 * char_w,
                            editor_rect.top() + visible_line as f32 * line_h,
                        );
                        let mut lsp_ui = main_ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(editor_rect)
                                .layout(egui::Layout::top_down(egui::Align::LEFT)),
                        );
                        crate::lsp::render_lsp_autocomplete(
                            &mut lsp_ui,
                            &self.lsp_completions,
                            self.lsp_completion_selected,
                            cursor_screen,
                            line_h,
                        );
                    }
                }

                // Find in Files panel
                let mut find_navigate: Option<(PathBuf, usize)> = None;
                if show_find_in_files {
                    let find_rect = egui::Rect::from_min_size(
                        egui::Pos2::new(available.left(), available.top() + editor_height),
                        egui::Vec2::new(available.width(), find_panel_height),
                    );
                    let mut find_ui = main_ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(find_rect)
                            .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    );
                    find_ui.separator();
                    find_navigate = crate::find_in_files::render_find_in_files_panel(
                        &mut find_ui,
                        &mut self.find_in_files_state,
                    );
                }

                // Terminal panel
                if self.terminal_state.visible {
                    let terminal_top = available.top() + editor_height + find_panel_height;
                    let terminal_rect = egui::Rect::from_min_size(
                        egui::Pos2::new(available.left(), terminal_top),
                        egui::Vec2::new(available.width(), terminal_height),
                    );
                    let mut terminal_ui = main_ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(terminal_rect)
                            .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    );
                    crate::terminal::render_terminal(
                        &mut terminal_ui,
                        &mut self.terminal_state,
                        self.font_size,
                    );

                    if self.terminal_focused {
                        crate::terminal::handle_terminal_input(
                            &mut terminal_ui,
                            &mut self.terminal_state,
                        );
                    }
                }

                // Handle navigation from find-in-files result click
                if let Some((path, line)) = find_navigate {
                    // Check if the file is already open
                    let existing_tab = self
                        .documents
                        .iter()
                        .position(|d| d.path.as_ref().map_or(false, |p| *p == path));
                    if let Some(tab_idx) = existing_tab {
                        self.active_tab = tab_idx;
                    } else {
                        // Open the file
                        self.open_file(path);
                    }
                    // Navigate to the line
                    if let Some(doc) = self.documents.get_mut(self.active_tab) {
                        doc.go_to_line(line);
                    }
                }

                // Status bar (spans full width across sidebar + editor)
                let doc_ref = self.documents.get(self.active_tab);
                let git_branch = self.git_state.branch.as_deref();
                let vim_mode_str = if self.vim_state.enabled {
                    Some(self.vim_state.mode.to_string())
                } else {
                    None
                };
                let (_, sb_action) = status_bar::render_status_bar(
                    &mut main_ui,
                    doc_ref,
                    &self.theme,
                    self.macro_recorder.is_recording(),
                    git_branch,
                    vim_mode_str.as_deref(),
                );

                // Handle status bar actions
                if let Some(action) = sb_action {
                    if let Some(doc) = self.documents.get_mut(self.active_tab) {
                        match action {
                            status_bar::StatusBarAction::ChangeEncoding(enc) => {
                                doc.encoding = enc;
                            }
                            status_bar::StatusBarAction::ChangeLineEnding(le) => {
                                doc.line_ending = le;
                            }
                            status_bar::StatusBarAction::ChangeLanguage(lang) => {
                                doc.language = if lang == "Plain Text" {
                                    None
                                } else {
                                    Some(lang)
                                };
                            }
                        }
                    }
                }

                // Handle sidebar file click (after layout to avoid borrow conflicts)
                if let Some(path) = sidebar_file_clicked {
                    // Check if already open
                    let existing = self
                        .documents
                        .iter()
                        .position(|d| d.path.as_ref().map_or(false, |p| *p == path));
                    if let Some(tab_idx) = existing {
                        self.active_tab = tab_idx;
                    } else {
                        self.open_file(path);
                    }
                }

                // Handle function list symbol click
                if let Some(line) = fn_list_navigate {
                    if let Some(doc) = self.documents.get_mut(self.active_tab) {
                        doc.go_to_line(line);
                    }
                }
            });

        // Apply pending clipboard copy (from tab context menu)
        if let Some(text) = self.pending_clipboard.take() {
            ctx.output_mut(|o| o.copied_text = text);
        }

        // Command palette
        if self.command_palette.open {
            if let Some(cmd_id) =
                command_palette::render_command_palette(ctx, &mut self.command_palette)
            {
                self.execute_command(cmd_id);
            }
        }

        // Go to File dialog
        if self.go_to_file_state.open {
            if let Some(rel_path) = go_to_file::render_go_to_file(ctx, &mut self.go_to_file_state) {
                let root = self.workspace_root();
                let abs_path = root.join(&rel_path);
                self.open_file(abs_path);
            }
        }

        // Go to Symbol dialog
        if self.go_to_symbol_state.open {
            if let Some(line) = go_to_symbol::render_go_to_symbol(ctx, &mut self.go_to_symbol_state)
            {
                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                    doc.go_to_line(line);
                }
            }
        }

        // Go to Line dialog
        if self.go_to_line_open {
            let mut open = self.go_to_line_open;
            egui::Window::new("Go to Line")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Line:");
                        let response = ui.text_edit_singleline(&mut self.go_to_line_input);
                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            if let Ok(line_num) = self.go_to_line_input.trim().parse::<usize>() {
                                if let Some(doc) = self.documents.get_mut(self.active_tab) {
                                    // User-facing line numbers are 1-based
                                    doc.go_to_line(line_num.saturating_sub(1));
                                }
                            }
                            self.go_to_line_open = false;
                        }
                        // Auto-focus the text input
                        response.request_focus();
                    });
                });
            self.go_to_line_open = open;
        }

        // Run Macro Multiple Times dialog
        if self.macro_run_n_open {
            let mut open = self.macro_run_n_open;
            egui::Window::new("Run Macro Multiple Times")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Count:");
                        let response = ui.text_edit_singleline(&mut self.macro_run_n_input);
                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            if let Ok(count) = self.macro_run_n_input.trim().parse::<usize>() {
                                if count > 0 {
                                    for _ in 0..count {
                                        self.replay_macro();
                                    }
                                }
                            }
                            self.macro_run_n_open = false;
                        }
                        response.request_focus();
                    });
                });
            self.macro_run_n_open = open;
        }

        // Save Macro As dialog
        if self.macro_save_as_open {
            let mut open = self.macro_save_as_open;
            egui::Window::new("Save Macro As")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        let response = ui.text_edit_singleline(&mut self.macro_save_as_input);
                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            let name = self.macro_save_as_input.trim().to_string();
                            if !name.is_empty() {
                                self.macro_recorder.save_macro(name);
                                self.macro_recorder.save_macros_to_disk();
                            }
                            self.macro_save_as_open = false;
                        }
                        response.request_focus();
                    });
                });
            self.macro_save_as_open = open;
        }

        // Load Macro dialog
        if self.macro_load_open {
            let mut open = self.macro_load_open;
            let names = self.macro_recorder.macro_names();
            egui::Window::new("Load Macro")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_size([300.0, 200.0])
                .show(ctx, |ui| {
                    if names.is_empty() {
                        ui.label("No saved macros.");
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(150.0)
                            .show(ui, |ui| {
                                for name in &names {
                                    let is_selected =
                                        self.macro_load_selected.as_deref() == Some(name);
                                    let bg = if is_selected {
                                        egui::Color32::from_rgb(60, 60, 80)
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    };
                                    let response = ui.add(
                                        egui::Button::new(name.as_str())
                                            .fill(bg)
                                            .min_size(egui::vec2(ui.available_width(), 0.0)),
                                    );
                                    if response.clicked() {
                                        self.macro_load_selected = Some(name.clone());
                                    }
                                    if response.double_clicked() {
                                        self.macro_recorder.load_named_macro(name);
                                        self.macro_load_open = false;
                                    }
                                }
                            });
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Load").clicked() {
                                if let Some(ref name) = self.macro_load_selected.clone() {
                                    self.macro_recorder.load_named_macro(name);
                                    self.macro_load_open = false;
                                }
                            }
                            if ui.button("Delete").clicked() {
                                if let Some(ref name) = self.macro_load_selected.clone() {
                                    self.macro_recorder.delete_named_macro(name);
                                    self.macro_recorder.save_macros_to_disk();
                                    self.macro_load_selected = None;
                                }
                            }
                            if ui.button("Cancel").clicked() {
                                self.macro_load_open = false;
                            }
                        });
                    }
                });
            self.macro_load_open = open;
        }

        // Unsaved changes dialog
        if let Some(tab_idx) = self.unsaved_close_tab {
            let doc_name = self
                .documents
                .get(tab_idx)
                .map(|d| d.display_name())
                .unwrap_or_else(|| "Untitled".to_string());

            let mut still_open = true;
            egui::Window::new("Unsaved Changes")
                .open(&mut still_open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("\"{}\" has unsaved changes.", doc_name));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save & Close").clicked() {
                            // Save then close
                            self.active_tab = tab_idx;
                            self.save_current();
                            self.unsaved_close_tab = None;
                            self.force_close_tab(tab_idx);
                        }
                        if ui.button("Discard").clicked() {
                            self.unsaved_close_tab = None;
                            self.force_close_tab(tab_idx);
                        }
                        if ui.button("Cancel").clicked() {
                            self.unsaved_close_tab = None;
                        }
                    });
                });
            if !still_open {
                self.unsaved_close_tab = None;
            }
        }

        // External file change dialog
        if let Some(tab_idx) = self.external_change_tab {
            let doc_name = self
                .documents
                .get(tab_idx)
                .map(|d| d.display_name())
                .unwrap_or_else(|| "Untitled".to_string());

            let mut still_open = true;
            egui::Window::new("File Changed")
                .open(&mut still_open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("\"{}\" has been modified externally.", doc_name));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Reload").clicked() {
                            // Reload the file from disk
                            if let Some(doc) = self.documents.get_mut(tab_idx) {
                                if let Some(ref path) = doc.path.clone() {
                                    if let Ok(bytes) = std::fs::read(path) {
                                        let encoding = Encoding::detect(&bytes);
                                        if let Ok(text) = encoding.decode(&bytes) {
                                            *doc = Document::from_str(&text);
                                            doc.path = Some(path.clone());
                                            doc.encoding = encoding;
                                            if let Some(ext) =
                                                path.extension().and_then(|e| e.to_str())
                                            {
                                                doc.language = Some(language_from_extension(ext));
                                            }
                                        }
                                    }
                                }
                            }
                            self.external_change_tab = None;
                        }
                        if ui.button("Keep Current").clicked() {
                            self.external_change_tab = None;
                        }
                    });
                });
            if !still_open {
                self.external_change_tab = None;
            }
        }

        // Column editor dialog
        if self.column_editor_open {
            let mut open = self.column_editor_open;
            egui::Window::new("Column Editor")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_size([300.0, 0.0])
                .show(ctx, |ui| {
                    // Line range
                    ui.horizontal(|ui| {
                        ui.label("Start line:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.column_editor_start_line)
                                .desired_width(60.0),
                        );
                        ui.label("End line:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.column_editor_end_line)
                                .desired_width(60.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Column:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.column_editor_col)
                                .desired_width(60.0),
                        );
                    });

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Mode selector
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.column_editor_mode,
                            ColumnEditorMode::Text,
                            "Insert Text",
                        );
                        ui.selectable_value(
                            &mut self.column_editor_mode,
                            ColumnEditorMode::Number,
                            "Insert Numbers",
                        );
                    });

                    ui.add_space(4.0);

                    match self.column_editor_mode {
                        ColumnEditorMode::Text => {
                            ui.horizontal(|ui| {
                                ui.label("Text:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.column_editor_text)
                                        .desired_width(200.0),
                                );
                            });
                        }
                        ColumnEditorMode::Number => {
                            ui.horizontal(|ui| {
                                ui.label("Initial:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.column_editor_initial)
                                        .desired_width(60.0),
                                );
                                ui.label("Step:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.column_editor_step)
                                        .desired_width(60.0),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Pad width:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.column_editor_pad_width)
                                        .desired_width(60.0),
                                );
                                ui.label("(0 = no padding)");
                            });
                        }
                    }

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        if ui.button("Apply").clicked() {
                            // Parse inputs
                            let col = self
                                .column_editor_col
                                .trim()
                                .parse::<usize>()
                                .unwrap_or(1)
                                .saturating_sub(1); // 1-based to 0-based
                            let start_line = self
                                .column_editor_start_line
                                .trim()
                                .parse::<usize>()
                                .unwrap_or(1)
                                .saturating_sub(1);
                            let end_line = self
                                .column_editor_end_line
                                .trim()
                                .parse::<usize>()
                                .unwrap_or(1)
                                .saturating_sub(1);

                            if let Some(doc) = self.documents.get_mut(self.active_tab) {
                                match self.column_editor_mode {
                                    ColumnEditorMode::Text => {
                                        doc.column_insert_text(
                                            start_line,
                                            end_line,
                                            col,
                                            &self.column_editor_text,
                                        );
                                    }
                                    ColumnEditorMode::Number => {
                                        let initial = self
                                            .column_editor_initial
                                            .trim()
                                            .parse::<i64>()
                                            .unwrap_or(1);
                                        let step = self
                                            .column_editor_step
                                            .trim()
                                            .parse::<i64>()
                                            .unwrap_or(1);
                                        let pad = self
                                            .column_editor_pad_width
                                            .trim()
                                            .parse::<usize>()
                                            .unwrap_or(0);
                                        doc.column_insert_numbers(
                                            start_line, end_line, col, initial, step, pad,
                                        );
                                    }
                                }
                            }
                            self.column_editor_open = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.column_editor_open = false;
                        }
                    });
                });
            self.column_editor_open = open;
        }

        // Git commit dialog
        if self.git_commit_dialog_open {
            let mut open = self.git_commit_dialog_open;
            egui::Window::new("Git Commit")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_size([400.0, 160.0])
                .show(ctx, |ui| {
                    ui.label("Commit message:");
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut self.git_commit_message)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY)
                            .hint_text("Enter commit message..."),
                    );
                    response.request_focus();

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let commit_enabled = !self.git_commit_message.trim().is_empty();
                        if ui
                            .add_enabled(commit_enabled, egui::Button::new("Commit"))
                            .clicked()
                        {
                            let msg = self.git_commit_message.trim().to_string();
                            match self.git_state.commit(&msg) {
                                Ok(oid) => {
                                    self.git_status_message =
                                        Some(format!("Committed: {}", oid));
                                    self.git_status_message_time =
                                        Some(std::time::Instant::now());
                                    self.git_branch = self.git_state.branch.clone();
                                }
                                Err(e) => {
                                    self.git_status_message =
                                        Some(format!("Commit failed: {}", e));
                                    self.git_status_message_time =
                                        Some(std::time::Instant::now());
                                }
                            }
                            self.git_commit_dialog_open = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.git_commit_dialog_open = false;
                        }
                    });
                });
            self.git_commit_dialog_open = open;
        }

        // Git status message toast (shown for 4 seconds)
        if let Some(ref msg) = self.git_status_message.clone() {
            let elapsed = self
                .git_status_message_time
                .map(|t| t.elapsed().as_secs_f32())
                .unwrap_or(10.0);
            if elapsed < 4.0 {
                egui::Window::new("git_status_toast")
                    .title_bar(false)
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -40.0])
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(msg)
                                    .color(egui::Color32::from_rgb(200, 200, 200)),
                            );
                        });
                    });
            } else {
                self.git_status_message = None;
                self.git_status_message_time = None;
            }
        }

        // About dialog
        if self.show_about {
            let mut open = self.show_about;
            egui::Window::new("About OpenEdit")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("OpenEdit");
                        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                        ui.add_space(8.0);
                        ui.label("A cross-platform text and code editor");
                        ui.label("Built with egui and Rust");
                        ui.add_space(8.0);
                        ui.label("MIT / Apache-2.0 License");
                    });
                });
            self.show_about = open;
        }

        // Keyboard shortcuts cheatsheet
        if self.show_shortcuts {
            let mut open = self.show_shortcuts;
            egui::Window::new("Keyboard Shortcuts")
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .default_size([500.0, 500.0])
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let shortcuts = [
                            (
                                "File",
                                &[
                                    ("Ctrl+N", "New File"),
                                    ("Ctrl+O", "Open File"),
                                    ("Ctrl+S", "Save"),
                                    ("Ctrl+Shift+S", "Save As"),
                                    ("Ctrl+W", "Close Tab"),
                                ] as &[(&str, &str)],
                            ),
                            (
                                "Edit",
                                &[
                                    ("Ctrl+Z", "Undo"),
                                    ("Ctrl+Y", "Redo"),
                                    ("Ctrl+X", "Cut"),
                                    ("Ctrl+C", "Copy"),
                                    ("Ctrl+V", "Paste"),
                                    ("Ctrl+/", "Toggle Comment"),
                                    ("Ctrl+D", "Add Next Occurrence"),
                                    ("Ctrl+Shift+L", "Select All Occurrences"),
                                ],
                            ),
                            (
                                "Navigation",
                                &[
                                    ("Ctrl+G", "Go to Line"),
                                    ("Ctrl+P", "Go to File"),
                                    ("Ctrl+F", "Find"),
                                    ("Ctrl+H", "Replace"),
                                    ("Ctrl+Shift+F", "Find in Files"),
                                    ("Ctrl+Shift+P", "Command Palette"),
                                ],
                            ),
                            (
                                "View",
                                &[
                                    ("Ctrl+B", "Toggle Sidebar"),
                                    ("Ctrl+`", "Toggle Terminal"),
                                    ("Ctrl+=", "Zoom In"),
                                    ("Ctrl+-", "Zoom Out"),
                                    ("F11", "Zen Mode"),
                                ],
                            ),
                        ];
                        for (section, items) in &shortcuts {
                            ui.heading(*section);
                            egui::Grid::new(format!("shortcuts_{}", section))
                                .striped(true)
                                .show(ui, |ui| {
                                    for (key, desc) in *items {
                                        ui.label(egui::RichText::new(*key).monospace().strong());
                                        ui.label(*desc);
                                        ui.end_row();
                                    }
                                });
                            ui.add_space(8.0);
                        }
                    });
                });
            self.show_shortcuts = open;
        }

        // Vim command line
        if self.vim_state.enabled && self.vim_state.mode == crate::vim::VimMode::Command {
            egui::TopBottomPanel::bottom("vim_command_line").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(":").monospace().strong());
                    ui.label(egui::RichText::new(&self.vim_state.command_line).monospace());
                });
            });
        }

        // Request continuous repaint for cursor blink etc.
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}

/// Apply a text tool to the document's selection (or entire content if no selection).
fn apply_text_tool(doc: &mut Document, tool_id: &str) {
    use openedit_core::cursor::Position;

    let has_selection = doc.cursors.primary().has_selection();
    let input = if has_selection {
        doc.selected_text()
    } else {
        doc.buffer.to_string()
    };

    let result = match tool_id {
        "tools.sort_asc" => Some(openedit_tools::sort::sort_lines_asc(&input)),
        "tools.sort_desc" => Some(openedit_tools::sort::sort_lines_desc(&input)),
        "tools.sort_case_insensitive" => {
            Some(openedit_tools::sort::sort_lines_case_insensitive(&input))
        }
        "tools.sort_numeric" => Some(openedit_tools::sort::sort_lines_numeric(&input)),
        "tools.uppercase" => Some(openedit_tools::case::to_uppercase(&input)),
        "tools.lowercase" => Some(openedit_tools::case::to_lowercase(&input)),
        "tools.title_case" => Some(openedit_tools::case::to_title_case(&input)),
        "tools.remove_duplicates" => Some(openedit_tools::lines::remove_duplicates(&input)),
        "tools.remove_empty" => Some(openedit_tools::lines::remove_empty_lines(&input)),
        "tools.join_lines" => Some(openedit_tools::lines::join_lines(&input)),
        "tools.reverse_lines" => Some(openedit_tools::lines::reverse_lines(&input)),
        "tools.trim_trailing" => Some(openedit_tools::lines::trim_trailing(&input)),
        // Encoding / JSON transforms
        "tools.base64_encode" => Some(openedit_tools::transform::base64_encode(&input)),
        "tools.base64_decode" => openedit_tools::transform::base64_decode(&input).ok(),
        "tools.url_encode" => Some(openedit_tools::transform::url_encode(&input)),
        "tools.url_decode" => openedit_tools::transform::url_decode(&input).ok(),
        "tools.json_pretty" => openedit_tools::transform::json_pretty_print(&input).ok(),
        "tools.json_minify" => openedit_tools::transform::json_minify(&input).ok(),
        "tools.xml_pretty" => openedit_tools::transform::xml_pretty_print(&input).ok(),
        "tools.xml_minify" => openedit_tools::transform::xml_minify(&input).ok(),
        // Additional case conversions
        "tools.camel_case" => Some(openedit_tools::case::to_camel_case(&input)),
        "tools.snake_case" => Some(openedit_tools::case::to_snake_case(&input)),
        "tools.pascal_case" => Some(openedit_tools::case::to_pascal_case(&input)),
        "tools.kebab_case" => Some(openedit_tools::case::to_kebab_case(&input)),
        // Hash
        "tools.hash_md5" => Some(openedit_tools::hash::md5_hash(&input)),
        "tools.hash_sha1" => Some(openedit_tools::hash::sha1_hash(&input)),
        "tools.hash_sha256" => Some(openedit_tools::hash::sha256_hash(&input)),
        // HTML entities
        "tools.html_encode" => Some(openedit_tools::transform::html_encode(&input)),
        "tools.html_decode" => Some(openedit_tools::transform::html_decode(&input)),
        // Conversion
        "tools.dec_to_hex" => openedit_tools::transform::dec_to_hex(&input).ok(),
        "tools.hex_to_dec" => openedit_tools::transform::hex_to_dec(&input).ok(),
        "tools.timestamp_to_date" => openedit_tools::transform::timestamp_to_date(&input).ok(),
        _ => None,
    };

    if let Some(output) = result {
        if output == input {
            return; // no change
        }

        if has_selection {
            // Replace selection
            doc.delete_selection_public();
            doc.insert_text(&output);
        } else {
            // Replace entire document content
            doc.select_all();
            doc.delete_selection_public();
            doc.insert_text(&output);
            // Move cursor to start
            doc.cursors.primary_mut().move_to(Position::zero(), false);
        }
    }
}

/// Detect language name from file extension.
fn language_from_extension(ext: &str) -> String {
    match ext.to_lowercase().as_str() {
        "rs" => "Rust",
        "py" | "pyw" => "Python",
        "js" | "mjs" | "cjs" => "JavaScript",
        "ts" | "mts" | "cts" => "TypeScript",
        "tsx" => "TSX",
        "jsx" => "JSX",
        "c" | "h" => "C",
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => "C++",
        "go" => "Go",
        "java" => "Java",
        "kt" | "kts" => "Kotlin",
        "swift" => "Swift",
        "rb" => "Ruby",
        "php" => "PHP",
        "html" | "htm" => "HTML",
        "css" => "CSS",
        "scss" | "sass" => "SCSS",
        "json" => "JSON",
        "yaml" | "yml" => "YAML",
        "toml" => "TOML",
        "xml" | "xsl" | "xsd" => "XML",
        "sql" => "SQL",
        "sh" | "bash" | "zsh" => "Bash",
        "ps1" | "psm1" => "PowerShell",
        "md" | "markdown" => "Markdown",
        "lua" => "Lua",
        "r" => "R",
        "dart" => "Dart",
        "zig" => "Zig",
        "ex" | "exs" => "Elixir",
        "erl" | "hrl" => "Erlang",
        "hs" => "Haskell",
        "ml" | "mli" => "OCaml",
        "cs" => "C#",
        "fs" | "fsx" => "F#",
        "txt" | "text" | "log" => "Plain Text",
        "ini" | "cfg" | "conf" => "INI",
        "dockerfile" => "Dockerfile",
        "makefile" => "Makefile",
        _ => "Plain Text",
    }
    .to_string()
}
