//! Git integration — repository status, line diffs, and blame.
//!
//! Uses `git2` to query working-tree state and per-line diff information.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Per-line diff status for the gutter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineDiffStatus {
    Added,
    Modified,
    Removed,
}

/// File-level status in the working tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileGitStatus {
    Modified,
    Staged,
    Untracked,
    Conflict,
    Deleted,
    Renamed,
    New,
    Unchanged,
}

/// Summary for a single file in the sidebar.
#[derive(Debug, Clone)]
pub struct FileStatusEntry {
    pub path: PathBuf,
    pub status: FileGitStatus,
}

/// Manages git repository state for the editor.
pub struct GitManager {
    /// Cached repo root path.
    repo_root: Option<PathBuf>,
    /// Current branch name.
    pub branch: Option<String>,
    /// Per-file line diff status (file path → line index → status).
    pub line_diffs: HashMap<PathBuf, Vec<(usize, LineDiffStatus)>>,
    /// File-level status list for sidebar display.
    pub file_statuses: Vec<FileStatusEntry>,
    /// Whether to show git blame annotations.
    pub show_blame: bool,
    /// Cached blame info (line → author + date).
    pub blame_info: HashMap<usize, String>,
    /// Last refresh timestamp.
    last_refresh: Option<std::time::Instant>,
    /// Minimum interval between refreshes.
    refresh_interval: std::time::Duration,
}

impl Default for GitManager {
    fn default() -> Self {
        Self {
            repo_root: None,
            branch: None,
            line_diffs: HashMap::new(),
            file_statuses: Vec::new(),
            show_blame: false,
            blame_info: HashMap::new(),
            last_refresh: None,
            refresh_interval: std::time::Duration::from_secs(5),
        }
    }
}

impl GitManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize git state from a workspace directory.
    pub fn init(&mut self, workspace: &Path) {
        match git2::Repository::discover(workspace) {
            Ok(repo) => {
                self.repo_root = repo.workdir().map(|p| p.to_path_buf());
                // Get branch name
                self.branch = repo.head().ok().and_then(|h| {
                    h.shorthand().map(|s| s.to_string())
                });
                self.refresh_statuses();
            }
            Err(_) => {
                self.repo_root = None;
                self.branch = None;
            }
        }
    }

    /// Periodic refresh check — call each frame.
    pub fn maybe_refresh(&mut self) {
        if self.repo_root.is_none() {
            return;
        }
        let should_refresh = match self.last_refresh {
            Some(t) => t.elapsed() >= self.refresh_interval,
            None => true,
        };
        if should_refresh {
            self.refresh_statuses();
            self.last_refresh = Some(std::time::Instant::now());
        }
    }

    /// Refresh file statuses from git.
    pub fn refresh_statuses(&mut self) {
        self.file_statuses.clear();
        let Some(repo) = self.open_repo() else { return };

        // Update branch
        self.branch = repo.head().ok().and_then(|h| {
            h.shorthand().map(|s| s.to_string())
        });

        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false);

        let Ok(statuses) = repo.statuses(Some(&mut opts)) else { return };

        for entry in statuses.iter() {
            let Some(path_str) = entry.path() else { continue };
            let status = entry.status();
            let root = self.repo_root.as_ref().unwrap();
            let full_path = root.join(path_str);

            let git_status = if status.contains(git2::Status::CONFLICTED) {
                FileGitStatus::Conflict
            } else if status.contains(git2::Status::INDEX_NEW) {
                FileGitStatus::New
            } else if status.contains(git2::Status::INDEX_MODIFIED)
                || status.contains(git2::Status::INDEX_RENAMED)
            {
                FileGitStatus::Staged
            } else if status.contains(git2::Status::WT_MODIFIED) {
                FileGitStatus::Modified
            } else if status.contains(git2::Status::WT_DELETED)
                || status.contains(git2::Status::INDEX_DELETED)
            {
                FileGitStatus::Deleted
            } else if status.contains(git2::Status::WT_NEW) {
                FileGitStatus::Untracked
            } else if status.contains(git2::Status::WT_RENAMED)
                || status.contains(git2::Status::INDEX_RENAMED)
            {
                FileGitStatus::Renamed
            } else {
                continue;
            };

            self.file_statuses.push(FileStatusEntry {
                path: full_path,
                status: git_status,
            });
        }
    }

    /// Invalidate cached file-level info (call after save).
    pub fn invalidate_file_cache(&mut self) {
        self.line_diffs.clear();
        self.blame_info.clear();
    }

    fn open_repo(&self) -> Option<git2::Repository> {
        self.repo_root.as_ref().and_then(|root| git2::Repository::open(root).ok())
    }

    /// Compute line-level diffs for a file (working tree vs HEAD).
    pub fn compute_line_diff(&mut self, file_path: &Path) {
        // Only recompute if not cached
        if self.line_diffs.contains_key(file_path) {
            return;
        }

        let Some(repo) = self.open_repo() else { return };
        let Some(root) = &self.repo_root else { return };

        let Ok(rel_path) = file_path.strip_prefix(root) else {
            return;
        };

        let mut diffs = Vec::new();

        // Get the HEAD blob for this file
        let head_blob = (|| -> Option<git2::Blob<'_>> {
            let head = repo.head().ok()?;
            let tree = head.peel_to_tree().ok()?;
            let entry = tree.get_path(rel_path).ok()?;
            let obj = entry.to_object(&repo).ok()?;
            obj.into_blob().ok()
        })();

        let Some(head_blob) = head_blob else {
            // File is new — all lines are added
            if let Ok(content) = std::fs::read_to_string(file_path) {
                for i in 0..content.lines().count() {
                    diffs.push((i, LineDiffStatus::Added));
                }
            }
            self.line_diffs.insert(file_path.to_path_buf(), diffs);
            return;
        };

        let Ok(current_content) = std::fs::read_to_string(file_path) else {
            return;
        };

        let old_content = String::from_utf8_lossy(head_blob.content()).into_owned();
        let old_lines: Vec<&str> = old_content.lines().collect();
        let new_lines: Vec<&str> = current_content.lines().collect();

        diffs = compute_line_diff(&old_lines, &new_lines);
        self.line_diffs.insert(file_path.to_path_buf(), diffs);
    }

    /// Compute blame annotations for a file.
    pub fn compute_blame(&mut self, file_path: &Path) {
        self.blame_info.clear();
        let Some(repo) = self.open_repo() else { return };
        let Some(root) = &self.repo_root else { return };
        let Ok(rel_path) = file_path.strip_prefix(root) else { return };

        let Ok(blame) = repo.blame_file(rel_path, None) else { return };

        for hunk_idx in 0..blame.len() {
            let Some(hunk) = blame.get_index(hunk_idx) else { continue };
            let sig = hunk.final_signature();
            let author = sig.name().unwrap_or("?");
            let start_line = hunk.final_start_line().saturating_sub(1);
            let line_count = hunk.lines_in_hunk();
            for offset in 0..line_count {
                self.blame_info.insert(start_line + offset, author.to_string());
            }
        }
    }

    /// Get line diff statuses for a given file.
    pub fn get_line_diffs(&self, path: &Path) -> &[(usize, LineDiffStatus)] {
        self.line_diffs
            .get(path)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get file status for a given path.
    pub fn get_file_status(&self, path: &Path) -> FileGitStatus {
        self.file_statuses
            .iter()
            .find(|e| e.path == path)
            .map(|e| e.status)
            .unwrap_or(FileGitStatus::Unchanged)
    }
}

/// Simple line diff using greedy matching to produce per-line statuses.
fn compute_line_diff(old: &[&str], new: &[&str]) -> Vec<(usize, LineDiffStatus)> {
    let mut result = Vec::new();
    let n = old.len();
    let m = new.len();

    if n == 0 {
        for i in 0..m {
            result.push((i, LineDiffStatus::Added));
        }
        return result;
    }
    if m == 0 {
        if n > 0 {
            result.push((0, LineDiffStatus::Removed));
        }
        return result;
    }

    let mut matched_new: Vec<bool> = vec![false; m];
    let mut matched_old: Vec<bool> = vec![false; n];

    let mut oi = 0;
    let mut ni = 0;
    while oi < n && ni < m {
        if old[oi] == new[ni] {
            matched_old[oi] = true;
            matched_new[ni] = true;
            oi += 1;
            ni += 1;
        } else {
            let lookahead = 3.min(m - ni);
            let mut found_in_new = None;
            for k in 1..=lookahead {
                if ni + k < m && old[oi] == new[ni + k] {
                    found_in_new = Some(k);
                    break;
                }
            }
            let lookahead_old = 3.min(n - oi);
            let mut found_in_old = None;
            for k in 1..=lookahead_old {
                if oi + k < n && old[oi + k] == new[ni] {
                    found_in_old = Some(k);
                    break;
                }
            }

            match (found_in_new, found_in_old) {
                (Some(k), _) if found_in_old.map_or(true, |ko| k <= ko) => {
                    ni += k;
                    matched_old[oi] = true;
                    matched_new[ni] = true;
                    oi += 1;
                    ni += 1;
                }
                (_, Some(k)) => {
                    oi += k;
                    matched_old[oi] = true;
                    matched_new[ni] = true;
                    oi += 1;
                    ni += 1;
                }
                _ => {
                    oi += 1;
                    ni += 1;
                }
            }
        }
    }

    for (i, &matched) in matched_new.iter().enumerate() {
        if !matched {
            let corresponding_old = (i as f64 * n as f64 / m as f64) as usize;
            if corresponding_old < n && !matched_old[corresponding_old] {
                result.push((i, LineDiffStatus::Modified));
                matched_old[corresponding_old] = true;
            } else {
                result.push((i, LineDiffStatus::Added));
            }
        }
    }

    for (i, &matched) in matched_old.iter().enumerate() {
        if !matched {
            let corresponding_new = (i as f64 * m as f64 / n as f64) as usize;
            let new_line = corresponding_new.min(m.saturating_sub(1));
            if !result.iter().any(|&(l, _)| l == new_line) {
                result.push((new_line, LineDiffStatus::Removed));
            }
        }
    }

    result.sort_by_key(|&(line, _)| line);
    result
}

/// Render git diff gutter marks for a single line.
pub fn render_git_gutter_mark(
    ui: &mut egui::Ui,
    line_diffs: &[(usize, LineDiffStatus)],
    line_idx: usize,
    gutter_right_x: f32,
    y: f32,
    line_height: f32,
) {
    let Some(&(_, status)) = line_diffs.iter().find(|&&(l, _)| l == line_idx) else {
        return;
    };

    let (color, width) = match status {
        LineDiffStatus::Added => (egui::Color32::from_rgb(80, 200, 80), 3.0),
        LineDiffStatus::Modified => (egui::Color32::from_rgb(80, 160, 255), 3.0),
        LineDiffStatus::Removed => (egui::Color32::from_rgb(220, 60, 60), 3.0),
    };

    let x = gutter_right_x - width - 1.0;

    if status == LineDiffStatus::Removed {
        let mid_y = y + line_height / 2.0;
        ui.painter().line_segment(
            [egui::Pos2::new(x, mid_y - 3.0), egui::Pos2::new(x, mid_y + 3.0)],
            egui::Stroke::new(width, color),
        );
    } else {
        ui.painter().line_segment(
            [egui::Pos2::new(x, y), egui::Pos2::new(x, y + line_height)],
            egui::Stroke::new(width, color),
        );
    }
}

/// Get display indicator for file git status.
pub fn git_status_indicator(status: FileGitStatus) -> (&'static str, egui::Color32) {
    match status {
        FileGitStatus::Modified => ("M", egui::Color32::from_rgb(80, 160, 255)),
        FileGitStatus::Staged => ("S", egui::Color32::from_rgb(80, 200, 80)),
        FileGitStatus::Untracked => ("U", egui::Color32::from_rgb(150, 150, 150)),
        FileGitStatus::Conflict => ("!", egui::Color32::from_rgb(220, 60, 60)),
        FileGitStatus::Deleted => ("D", egui::Color32::from_rgb(220, 60, 60)),
        FileGitStatus::Renamed => ("R", egui::Color32::from_rgb(200, 180, 80)),
        FileGitStatus::New => ("A", egui::Color32::from_rgb(80, 200, 80)),
        FileGitStatus::Unchanged => ("", egui::Color32::TRANSPARENT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_line_diff_identical() {
        let old = vec!["a", "b", "c"];
        let new = vec!["a", "b", "c"];
        let result = compute_line_diff(&old, &new);
        assert!(result.is_empty());
    }

    #[test]
    fn test_compute_line_diff_added_line() {
        let old = vec!["a", "c"];
        let new = vec!["a", "b", "c"];
        let result = compute_line_diff(&old, &new);
        assert!(!result.is_empty());
        assert!(result.iter().any(|&(l, s)| l == 1 && s == LineDiffStatus::Added));
    }

    #[test]
    fn test_compute_line_diff_removed_line() {
        let old = vec!["a", "b", "c"];
        let new = vec!["a", "c"];
        let result = compute_line_diff(&old, &new);
        assert!(!result.is_empty());
        assert!(result.iter().any(|&(_, s)| s == LineDiffStatus::Removed));
    }

    #[test]
    fn test_compute_line_diff_modified_line() {
        let old = vec!["a", "b", "c"];
        let new = vec!["a", "x", "c"];
        let result = compute_line_diff(&old, &new);
        assert!(!result.is_empty());
        assert!(result.iter().any(|&(l, s)| l == 1 && s == LineDiffStatus::Modified));
    }

    #[test]
    fn test_compute_line_diff_all_new() {
        let old: Vec<&str> = vec![];
        let new = vec!["a", "b"];
        let result = compute_line_diff(&old, &new);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|&(_, s)| s == LineDiffStatus::Added));
    }

    #[test]
    fn test_git_status_indicator() {
        let (label, _color) = git_status_indicator(FileGitStatus::Modified);
        assert_eq!(label, "M");
        let (label, _) = git_status_indicator(FileGitStatus::Unchanged);
        assert_eq!(label, "");
    }

    #[test]
    fn test_git_manager_new() {
        let mgr = GitManager::new();
        assert!(mgr.branch.is_none());
        assert!(mgr.file_statuses.is_empty());
        assert!(mgr.line_diffs.is_empty());
    }

    #[test]
    fn test_git_manager_default() {
        let mgr = GitManager::default();
        assert!(!mgr.show_blame);
        assert!(mgr.blame_info.is_empty());
    }
}
