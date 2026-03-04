use egui::{self, Ui};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

/// A single match within a line.
#[derive(Clone, Debug)]
pub struct LineMatch {
    pub line_num: usize,
    pub line_text: String,
    pub match_start: usize,
    pub match_end: usize,
}

/// All matches within a single file.
#[derive(Clone, Debug)]
pub struct FileMatch {
    pub path: PathBuf,
    pub matches: Vec<LineMatch>,
}

/// State for the Find in Files panel.
pub struct FindInFilesState {
    pub visible: bool,
    pub query: String,
    pub search_path: String,
    pub include_pattern: String,
    pub results: Vec<FileMatch>,
    pub case_sensitive: bool,
    pub use_regex: bool,
    pub searching: bool,
    /// Replace text for find-and-replace.
    pub replace_text: String,
    /// Whether to show the replace input.
    pub show_replace: bool,
    /// Channel receiver for background search results.
    result_rx: Option<mpsc::Receiver<Vec<FileMatch>>>,
    /// Which file indices are collapsed in the results view.
    collapsed_files: std::collections::HashSet<usize>,
    /// Total match count across all files (cached).
    total_matches: usize,
}

impl Default for FindInFilesState {
    fn default() -> Self {
        let search_path = std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self {
            visible: false,
            query: String::new(),
            search_path,
            include_pattern: String::new(),
            results: Vec::new(),
            case_sensitive: false,
            use_regex: false,
            searching: false,
            replace_text: String::new(),
            show_replace: false,
            result_rx: None,
            collapsed_files: std::collections::HashSet::new(),
            total_matches: 0,
        }
    }
}

impl FindInFilesState {
    /// Poll for search results from the background thread.
    pub fn poll_results(&mut self) {
        if let Some(ref rx) = self.result_rx {
            match rx.try_recv() {
                Ok(results) => {
                    self.total_matches = results.iter().map(|f| f.matches.len()).sum();
                    self.results = results;
                    self.searching = false;
                    self.result_rx = None;
                    self.collapsed_files.clear();
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.searching = false;
                    self.result_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // Still searching
                }
            }
        }
    }
}

/// Launch a background search. Results will arrive via `state.poll_results()`.
fn start_search(state: &mut FindInFilesState) {
    let query = state.query.clone();
    let search_path = state.search_path.clone();
    let include_pattern = state.include_pattern.clone();
    let case_sensitive = state.case_sensitive;
    let use_regex = state.use_regex;

    if query.is_empty() {
        return;
    }

    state.searching = true;
    state.results.clear();
    state.total_matches = 0;
    state.collapsed_files.clear();

    let (tx, rx) = mpsc::channel();
    state.result_rx = Some(rx);

    thread::spawn(move || {
        let results = perform_search(
            &query,
            &search_path,
            &include_pattern,
            case_sensitive,
            use_regex,
        );
        let _ = tx.send(results);
    });
}

/// Perform the search synchronously (runs in background thread).
fn perform_search(
    query: &str,
    search_path: &str,
    include_pattern: &str,
    case_sensitive: bool,
    use_regex: bool,
) -> Vec<FileMatch> {
    let root = PathBuf::from(search_path);
    if !root.is_dir() {
        return Vec::new();
    }

    // Build the regex pattern for matching.
    let pattern = if use_regex {
        if case_sensitive {
            regex::Regex::new(query)
        } else {
            regex::Regex::new(&format!("(?i){}", query))
        }
    } else {
        let escaped = regex::escape(query);
        if case_sensitive {
            regex::Regex::new(&escaped)
        } else {
            regex::Regex::new(&format!("(?i){}", escaped))
        }
    };

    let pattern = match pattern {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    // Build glob matcher for include pattern (if provided).
    let glob_pattern = if include_pattern.is_empty() {
        None
    } else {
        // Support comma-separated patterns like "*.rs,*.toml"
        let patterns: Vec<glob::Pattern> = include_pattern
            .split(',')
            .filter_map(|p| glob::Pattern::new(p.trim()).ok())
            .collect();
        if patterns.is_empty() {
            None
        } else {
            Some(patterns)
        }
    };

    let mut file_matches = Vec::new();

    for entry in walkdir::WalkDir::new(&root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        // Skip hidden directories and common non-text directories.
        let path_str = path.to_string_lossy();
        if path_str.contains("/.git/")
            || path_str.contains("/target/")
            || path_str.contains("/node_modules/")
            || path_str.contains("/.git\\")
            || path_str.contains("\\target\\")
            || path_str.contains("\\node_modules\\")
        {
            continue;
        }

        // Apply glob filter.
        if let Some(ref patterns) = glob_pattern {
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            let matches_any = patterns.iter().any(|p| p.matches(&file_name));
            if !matches_any {
                continue;
            }
        }

        // Try to read the file as text (skip binary files).
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut line_matches = Vec::new();
        for (line_idx, line) in content.lines().enumerate() {
            for m in pattern.find_iter(line) {
                line_matches.push(LineMatch {
                    line_num: line_idx + 1,
                    line_text: line.to_string(),
                    match_start: m.start(),
                    match_end: m.end(),
                });
            }
        }

        if !line_matches.is_empty() {
            // Store a relative path for cleaner display.
            let display_path = path.strip_prefix(&root).unwrap_or(path).to_path_buf();
            file_matches.push(FileMatch {
                path: display_path,
                matches: line_matches,
            });
        }
    }

    // Sort by file path for consistent ordering.
    file_matches.sort_by(|a, b| a.path.cmp(&b.path));
    file_matches
}

/// Perform replace all across files.
fn replace_all_in_files(state: &mut FindInFilesState) {
    let pattern = if state.use_regex {
        if state.case_sensitive {
            regex::Regex::new(&state.query)
        } else {
            regex::Regex::new(&format!("(?i){}", state.query))
        }
    } else {
        let escaped = regex::escape(&state.query);
        if state.case_sensitive {
            regex::Regex::new(&escaped)
        } else {
            regex::Regex::new(&format!("(?i){}", escaped))
        }
    };

    let pattern = match pattern {
        Ok(p) => p,
        Err(_) => return,
    };

    let root = PathBuf::from(&state.search_path);
    let mut replaced_count = 0usize;

    for file_match in &state.results {
        let abs_path = root.join(&file_match.path);
        if let Ok(content) = std::fs::read_to_string(&abs_path) {
            let new_content = pattern.replace_all(&content, state.replace_text.as_str());
            if new_content != content {
                let _ = std::fs::write(&abs_path, new_content.as_ref());
                replaced_count += file_match.matches.len();
            }
        }
    }

    // Clear results after replace
    state.results.clear();
    state.total_matches = 0;
    log::info!("Replaced {} occurrences across files", replaced_count);
}

/// Render the Find in Files panel.
///
/// Returns `Some((absolute_path, line_number))` if a result was clicked,
/// where `line_number` is 0-based.
pub fn render_find_in_files_panel(
    ui: &mut Ui,
    state: &mut FindInFilesState,
) -> Option<(PathBuf, usize)> {
    // Poll for background search results.
    state.poll_results();

    let mut navigate_to: Option<(PathBuf, usize)> = None;

    egui::Frame::none()
        .fill(egui::Color32::from_rgb(37, 37, 38))
        .inner_margin(egui::Margin::same(8.0))
        .show(ui, |ui| {
            // Header row with title and close button.
            ui.horizontal(|ui| {
                ui.strong("Find in Files");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("\u{00D7}").on_hover_text("Close (Esc)").clicked() {
                        state.visible = false;
                    }
                });
            });

            ui.add_space(4.0);

            // Search query input.
            ui.horizontal(|ui| {
                ui.label("Search:");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.query)
                        .hint_text("Search term...")
                        .desired_width(ui.available_width() - 80.0),
                );
                // Enter key triggers search.
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    start_search(state);
                }
            });

            // Search path input.
            ui.horizontal(|ui| {
                ui.label("Path:    ");
                ui.add(
                    egui::TextEdit::singleline(&mut state.search_path)
                        .hint_text("Directory to search...")
                        .desired_width(ui.available_width() - 80.0),
                );
            });

            // Include pattern input.
            ui.horizontal(|ui| {
                ui.label("Include:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.include_pattern)
                        .hint_text("*.rs, *.toml")
                        .desired_width(ui.available_width() - 80.0),
                );
            });

            // Replace input
            ui.horizontal(|ui| {
                ui.toggle_value(&mut state.show_replace, "R")
                    .on_hover_text("Toggle Replace");
                if state.show_replace {
                    ui.label("Replace:");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.replace_text)
                            .hint_text("Replace with...")
                            .desired_width(ui.available_width() - 80.0),
                    );
                }
            });

            // Options row: toggles and search button.
            ui.horizontal(|ui| {
                ui.toggle_value(&mut state.case_sensitive, "Aa")
                    .on_hover_text("Case Sensitive");
                ui.toggle_value(&mut state.use_regex, ".*")
                    .on_hover_text("Regex");

                ui.add_space(8.0);

                let search_btn = ui.add_enabled(
                    !state.searching && !state.query.is_empty(),
                    egui::Button::new("Search"),
                );
                if search_btn.clicked() {
                    start_search(state);
                }

                if state.show_replace
                    && !state.results.is_empty()
                    && ui.button("Replace All").clicked()
                {
                    replace_all_in_files(state);
                }

                if state.searching {
                    ui.spinner();
                    ui.label("Searching...");
                }
            });

            ui.add_space(4.0);

            // Results summary.
            if !state.results.is_empty() {
                ui.label(format!(
                    "{} matches in {} files",
                    state.total_matches,
                    state.results.len()
                ));
            } else if !state.searching && !state.query.is_empty() && state.result_rx.is_none() {
                ui.label("No results found.");
            }

            ui.separator();

            // Results list (scrollable).
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (file_idx, file_match) in state.results.iter().enumerate() {
                        let is_collapsed = state.collapsed_files.contains(&file_idx);

                        // File header (clickable to collapse/expand).
                        let arrow = if is_collapsed { "\u{25B6}" } else { "\u{25BC}" };
                        let header_text = format!(
                            "{} {} ({})",
                            arrow,
                            file_match.path.display(),
                            file_match.matches.len()
                        );

                        let header_response = ui.add(
                            egui::Label::new(
                                egui::RichText::new(&header_text)
                                    .strong()
                                    .color(egui::Color32::from_rgb(200, 200, 200)),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if header_response.clicked() {
                            if is_collapsed {
                                state.collapsed_files.remove(&file_idx);
                            } else {
                                state.collapsed_files.insert(file_idx);
                            }
                        }

                        // Show individual matches if not collapsed.
                        if !is_collapsed {
                            for line_match in &file_match.matches {
                                ui.horizontal(|ui| {
                                    ui.add_space(16.0);

                                    // Line number.
                                    ui.label(
                                        egui::RichText::new(format!("{}:", line_match.line_num))
                                            .color(egui::Color32::from_rgb(110, 110, 110))
                                            .monospace(),
                                    );

                                    // Build a rich text layout with the match highlighted.
                                    let line = &line_match.line_text;
                                    let start = line_match.match_start;
                                    let end = line_match.match_end;

                                    // Truncate long lines for display.
                                    let display_line = if line.len() > 200 {
                                        // Show context around the match.
                                        let ctx_start = start.saturating_sub(40);
                                        let ctx_end = (end + 40).min(line.len());
                                        let prefix = if ctx_start > 0 { "..." } else { "" };
                                        let suffix = if ctx_end < line.len() { "..." } else { "" };
                                        format!("{}{}{}", prefix, &line[ctx_start..ctx_end], suffix)
                                    } else {
                                        line.clone()
                                    };

                                    // Compute adjusted offsets for truncated display.
                                    let (adj_start, adj_end) = if line.len() > 200 {
                                        let ctx_start = start.saturating_sub(40);
                                        let prefix_len = if ctx_start > 0 { 3 } else { 0 };
                                        (
                                            start - ctx_start + prefix_len,
                                            end - ctx_start + prefix_len,
                                        )
                                    } else {
                                        (start, end)
                                    };

                                    let trimmed = display_line.trim_start();
                                    let leading_ws = display_line.len() - trimmed.len();

                                    // Adjust offsets for trimmed whitespace.
                                    let final_start = adj_start.saturating_sub(leading_ws);
                                    let final_end = adj_end.saturating_sub(leading_ws);

                                    // Build job with highlighted match.
                                    let mut job = egui::text::LayoutJob::default();
                                    let mono = egui::FontId::monospace(12.0);

                                    if final_start > 0 && final_start <= trimmed.len() {
                                        job.append(
                                            &trimmed[..final_start],
                                            0.0,
                                            egui::TextFormat {
                                                font_id: mono.clone(),
                                                color: egui::Color32::from_rgb(180, 180, 180),
                                                ..Default::default()
                                            },
                                        );
                                    }

                                    if final_start < trimmed.len() && final_end <= trimmed.len() {
                                        job.append(
                                            &trimmed[final_start..final_end],
                                            0.0,
                                            egui::TextFormat {
                                                font_id: mono.clone(),
                                                color: egui::Color32::WHITE,
                                                background: egui::Color32::from_rgba_premultiplied(
                                                    180, 150, 50, 120,
                                                ),
                                                ..Default::default()
                                            },
                                        );
                                    }

                                    if final_end < trimmed.len() {
                                        job.append(
                                            &trimmed[final_end..],
                                            0.0,
                                            egui::TextFormat {
                                                font_id: mono.clone(),
                                                color: egui::Color32::from_rgb(180, 180, 180),
                                                ..Default::default()
                                            },
                                        );
                                    }

                                    let response =
                                        ui.add(egui::Label::new(job).sense(egui::Sense::click()));

                                    if response.clicked() {
                                        // Build absolute path from search root + relative path.
                                        let abs_path = PathBuf::from(&state.search_path)
                                            .join(&file_match.path);
                                        navigate_to = Some((abs_path, line_match.line_num - 1));
                                    }

                                    if response.hovered() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }
                                });
                            }
                        }

                        ui.add_space(2.0);
                    }
                });
        });

    navigate_to
}
