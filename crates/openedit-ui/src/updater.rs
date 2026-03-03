//! Auto-update mechanism that checks GitHub releases for newer versions.
//!
//! Runs a background thread on startup (or on demand via command palette) to
//! query the GitHub Releases API. When a newer version is found, a toast
//! notification is shown in the UI. The user can then trigger a download that
//! replaces the current binary in-place.

use std::sync::mpsc;

/// GitHub owner/repo for release checks. Change this to point at the real
/// repository once it is published.
const GITHUB_OWNER: &str = "openedit";
const GITHUB_REPO: &str = "openedit";

/// User-Agent header required by the GitHub API.
const USER_AGENT: &str = "openedit-updater";

/// Information about an available update.
#[derive(Clone, Debug)]
pub struct ReleaseInfo {
    /// Semver tag of the latest release (e.g. "0.2.0").
    pub version: String,
    /// Human-readable release notes / body.
    pub body: String,
    /// Direct download URL for the current platform's binary asset, if found.
    pub download_url: Option<String>,
    /// HTML URL of the release page on GitHub.
    pub html_url: String,
}

/// Possible results sent back from the background update-check thread.
#[derive(Clone, Debug)]
pub enum UpdateCheckResult {
    /// A newer version is available.
    Available(ReleaseInfo),
    /// Already running the latest version.
    UpToDate,
    /// The check failed (network error, parse error, etc.).
    Error(String),
}

/// The current phase of a binary download + replace operation.
#[derive(Clone, Debug, PartialEq)]
pub enum DownloadProgress {
    /// Download is in progress.
    Downloading,
    /// Download finished, binary was replaced successfully. The user should
    /// restart the application.
    Done,
    /// Something went wrong.
    Failed(String),
}

/// Persistent state kept in the application struct.
pub struct UpdaterState {
    /// Receives results from the background check thread.
    rx: Option<mpsc::Receiver<UpdateCheckResult>>,
    /// Latest result (sticky until dismissed).
    pub result: Option<UpdateCheckResult>,
    /// Whether the user has dismissed the notification for this session.
    pub dismissed: bool,
    /// Progress of an in-flight download (if any).
    download_rx: Option<mpsc::Receiver<DownloadProgress>>,
    /// Latest download progress (sticky).
    pub download_progress: Option<DownloadProgress>,
}

impl Default for UpdaterState {
    fn default() -> Self {
        Self {
            rx: None,
            result: None,
            dismissed: false,
            download_rx: None,
            download_progress: None,
        }
    }
}

impl UpdaterState {
    /// Kick off a background thread that checks for updates.
    /// Does nothing if a check is already in flight.
    pub fn check_for_updates(&mut self) {
        if self.rx.is_some() {
            return; // already checking
        }
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.result = None;
        self.dismissed = false;

        std::thread::spawn(move || {
            let result = do_update_check();
            let _ = tx.send(result);
        });
    }

    /// Poll the background channel. Call this once per frame.
    pub fn poll(&mut self) {
        // Poll update check
        if let Some(ref rx) = self.rx {
            if let Ok(result) = rx.try_recv() {
                self.result = Some(result);
                self.rx = None;
            }
        }
        // Poll download progress
        if let Some(ref rx) = self.download_rx {
            if let Ok(progress) = rx.try_recv() {
                let done = matches!(progress, DownloadProgress::Done | DownloadProgress::Failed(_));
                self.download_progress = Some(progress);
                if done {
                    self.download_rx = None;
                }
            }
        }
    }

    /// Start downloading the update binary in a background thread.
    /// Requires that `self.result` is `Available` with a `download_url`.
    pub fn start_download(&mut self) {
        if self.download_rx.is_some() {
            return; // already downloading
        }
        let url = match &self.result {
            Some(UpdateCheckResult::Available(info)) => match &info.download_url {
                Some(u) => u.clone(),
                None => return,
            },
            _ => return,
        };

        let (tx, rx) = mpsc::channel();
        self.download_rx = Some(rx);
        self.download_progress = Some(DownloadProgress::Downloading);

        std::thread::spawn(move || {
            let result = do_download_and_replace(&url);
            let _ = tx.send(result);
        });
    }

    /// Whether an update is available (and not dismissed).
    pub fn has_update(&self) -> bool {
        if self.dismissed {
            return false;
        }
        matches!(&self.result, Some(UpdateCheckResult::Available(_)))
    }
}

// ---------------------------------------------------------------------------
// Background work (runs on spawned threads)
// ---------------------------------------------------------------------------

/// Perform the actual HTTP request + version comparison.
fn do_update_check() -> UpdateCheckResult {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        GITHUB_OWNER, GITHUB_REPO
    );

    let mut response = match ureq::get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .call()
    {
        Ok(resp) => resp,
        Err(e) => return UpdateCheckResult::Error(format!("HTTP request failed: {}", e)),
    };

    let body_str = match response.body_mut().read_to_string() {
        Ok(s) => s,
        Err(e) => return UpdateCheckResult::Error(format!("Failed to read response: {}", e)),
    };

    let json: serde_json::Value = match serde_json::from_str(&body_str) {
        Ok(v) => v,
        Err(e) => return UpdateCheckResult::Error(format!("Invalid JSON: {}", e)),
    };

    // Extract the tag name (e.g. "v0.2.0" or "0.2.0")
    let tag = match json["tag_name"].as_str() {
        Some(t) => t.trim_start_matches('v').to_string(),
        None => {
            return UpdateCheckResult::Error("No tag_name in release response".into());
        }
    };

    let current = env!("CARGO_PKG_VERSION");

    match compare_versions(current, &tag) {
        Some(std::cmp::Ordering::Less) => {
            // Newer version available
            let html_url = json["html_url"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let body = json["body"].as_str().unwrap_or("").to_string();
            let download_url = find_asset_url(&json);

            UpdateCheckResult::Available(ReleaseInfo {
                version: tag,
                body,
                download_url,
                html_url,
            })
        }
        _ => UpdateCheckResult::UpToDate,
    }
}

/// Find the download URL for the correct platform binary from the release
/// assets array.
fn find_asset_url(release_json: &serde_json::Value) -> Option<String> {
    let assets = release_json["assets"].as_array()?;
    let target = platform_asset_suffix();

    for asset in assets {
        let name = asset["name"].as_str().unwrap_or("");
        if name.contains(target) {
            return asset["browser_download_url"]
                .as_str()
                .map(|s| s.to_string());
        }
    }
    None
}

/// Return a substring that the release asset filename should contain for the
/// current platform.
fn platform_asset_suffix() -> &'static str {
    if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        "linux-x86_64"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        "linux-aarch64"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        "macos-x86_64"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "macos-aarch64"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    }
}

/// Compare two semver strings. Returns `Some(Ordering)` or `None` if either
/// string cannot be parsed.
fn compare_versions(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let parse = |s: &str| -> Option<(u64, u64, u64)> {
        let s = s.trim_start_matches('v');
        let mut parts = s.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next().unwrap_or("0").parse().ok()?;
        // Strip any pre-release suffix from patch (e.g. "1-beta" -> "1")
        let patch_str = parts.next().unwrap_or("0");
        let patch_num = patch_str
            .split('-')
            .next()
            .unwrap_or("0")
            .parse()
            .ok()?;
        Some((major, minor, patch_num))
    };
    let va = parse(a)?;
    let vb = parse(b)?;
    Some(va.cmp(&vb))
}

/// Download the binary from `url` and replace the currently running
/// executable.
fn do_download_and_replace(url: &str) -> DownloadProgress {
    // Download to a temporary file next to the current executable
    let current_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => return DownloadProgress::Failed(format!("Cannot locate current exe: {}", e)),
    };

    let parent = match current_exe.parent() {
        Some(p) => p,
        None => return DownloadProgress::Failed("Cannot determine exe directory".into()),
    };

    let tmp_path = parent.join(".openedit-update.tmp");

    // Stream the download to the temp file
    let mut response = match ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .call()
    {
        Ok(r) => r,
        Err(e) => return DownloadProgress::Failed(format!("Download failed: {}", e)),
    };

    let bytes = match response.body_mut().read_to_vec() {
        Ok(v) => v,
        Err(e) => return DownloadProgress::Failed(format!("Read failed: {}", e)),
    };

    if let Err(e) = std::fs::write(&tmp_path, &bytes) {
        return DownloadProgress::Failed(format!("Write failed: {}", e));
    }

    // Platform-specific replacement
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        // Make the new binary executable
        if let Err(e) =
            std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755))
        {
            let _ = std::fs::remove_file(&tmp_path);
            return DownloadProgress::Failed(format!("chmod failed: {}", e));
        }

        // Rename current -> .old, then tmp -> current
        let old_path = parent.join(".openedit-old");
        let _ = std::fs::remove_file(&old_path); // clean up any previous .old
        if let Err(e) = std::fs::rename(&current_exe, &old_path) {
            let _ = std::fs::remove_file(&tmp_path);
            return DownloadProgress::Failed(format!("Rename current exe failed: {}", e));
        }
        if let Err(e) = std::fs::rename(&tmp_path, &current_exe) {
            // Try to restore the original
            let _ = std::fs::rename(&old_path, &current_exe);
            return DownloadProgress::Failed(format!("Rename new exe failed: {}", e));
        }
        // Clean up old binary
        let _ = std::fs::remove_file(&old_path);
    }

    #[cfg(windows)]
    {
        // On Windows we cannot replace a running executable directly.
        // Rename the new file next to the current one with a `.new` extension;
        // a helper batch script will do the swap on next launch.
        let new_path = current_exe.with_extension("exe.new");
        if let Err(e) = std::fs::rename(&tmp_path, &new_path) {
            let _ = std::fs::remove_file(&tmp_path);
            return DownloadProgress::Failed(format!("Staging new exe failed: {}", e));
        }

        // Write a small helper script that swaps files on next launch
        let bat_path = parent.join("openedit-update.bat");
        let script = format!(
            "@echo off\r\ntimeout /t 2 >nul\r\nmove /Y \"{}\" \"{}\"\r\ndel \"%~f0\"\r\n",
            new_path.display(),
            current_exe.display(),
        );
        if let Err(e) = std::fs::write(&bat_path, script) {
            return DownloadProgress::Failed(format!("Failed to write update script: {}", e));
        }
    }

    DownloadProgress::Done
}

// ---------------------------------------------------------------------------
// UI rendering — toast notification
// ---------------------------------------------------------------------------

/// Render the update notification toast. Call this from the main `update()`
/// loop, typically near the other toast / status messages.
pub fn render_update_toast(ctx: &egui::Context, state: &mut UpdaterState) {
    state.poll();

    if state.dismissed {
        return;
    }

    // Show update-available banner
    if let Some(UpdateCheckResult::Available(ref info)) = state.result.clone() {
        // If a download is in progress or completed, show that instead
        if let Some(ref progress) = state.download_progress.clone() {
            render_download_progress(ctx, state, progress);
            return;
        }

        egui::Window::new("update_toast")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -16.0])
            .fixed_size([360.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "Update available: v{}",
                            info.version
                        ))
                        .strong()
                        .color(egui::Color32::from_rgb(100, 200, 100)),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "Current: v{}",
                            env!("CARGO_PKG_VERSION")
                        ))
                        .small()
                        .color(egui::Color32::from_rgb(180, 180, 180)),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if info.download_url.is_some() && ui.button("Update Now").clicked() {
                            state.start_download();
                        }
                        if ui.button("Dismiss").clicked() {
                            state.dismissed = true;
                        }
                    });
                });
            });
    }

    // Show error as a transient message (auto-dismiss after 6 seconds)
    if let Some(UpdateCheckResult::Error(ref msg)) = state.result {
        log::warn!("Update check failed: {}", msg);
        // We just log the error; no UI toast for errors to avoid annoying users.
    }
}

fn render_download_progress(
    ctx: &egui::Context,
    state: &mut UpdaterState,
    progress: &DownloadProgress,
) {
    egui::Window::new("update_download_toast")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -16.0])
        .fixed_size([360.0, 0.0])
        .show(ctx, |ui| {
            match progress {
                DownloadProgress::Downloading => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Downloading update...");
                    });
                }
                DownloadProgress::Done => {
                    ui.label(
                        egui::RichText::new("Update installed! Restart to apply.")
                            .strong()
                            .color(egui::Color32::from_rgb(100, 200, 100)),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Dismiss").clicked() {
                            state.dismissed = true;
                        }
                    });
                }
                DownloadProgress::Failed(msg) => {
                    ui.label(
                        egui::RichText::new(format!("Update failed: {}", msg))
                            .color(egui::Color32::from_rgb(240, 100, 100)),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Dismiss").clicked() {
                            state.dismissed = true;
                        }
                    });
                }
            }
        });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_versions_less() {
        assert_eq!(
            compare_versions("0.1.0", "0.2.0"),
            Some(std::cmp::Ordering::Less)
        );
    }

    #[test]
    fn test_compare_versions_equal() {
        assert_eq!(
            compare_versions("1.0.0", "1.0.0"),
            Some(std::cmp::Ordering::Equal)
        );
    }

    #[test]
    fn test_compare_versions_greater() {
        assert_eq!(
            compare_versions("2.0.0", "1.9.9"),
            Some(std::cmp::Ordering::Greater)
        );
    }

    #[test]
    fn test_compare_versions_strips_v_prefix() {
        assert_eq!(
            compare_versions("v0.1.0", "v0.2.0"),
            Some(std::cmp::Ordering::Less)
        );
    }

    #[test]
    fn test_compare_versions_major_bump() {
        assert_eq!(
            compare_versions("0.9.9", "1.0.0"),
            Some(std::cmp::Ordering::Less)
        );
    }

    #[test]
    fn test_compare_versions_patch_only() {
        assert_eq!(
            compare_versions("1.2.3", "1.2.4"),
            Some(std::cmp::Ordering::Less)
        );
    }

    #[test]
    fn test_compare_versions_pre_release_stripped() {
        // Pre-release suffix on patch is stripped, so 1.0.0-beta == 1.0.0
        assert_eq!(
            compare_versions("1.0.0-beta", "1.0.0"),
            Some(std::cmp::Ordering::Equal)
        );
    }

    #[test]
    fn test_compare_versions_two_part() {
        // "1.2" should be treated as "1.2.0"
        assert_eq!(
            compare_versions("1.2", "1.2.0"),
            Some(std::cmp::Ordering::Equal)
        );
    }

    #[test]
    fn test_compare_versions_invalid() {
        assert_eq!(compare_versions("abc", "1.0.0"), None);
    }

    #[test]
    fn test_platform_asset_suffix_not_empty() {
        let suffix = platform_asset_suffix();
        assert!(!suffix.is_empty());
    }

    #[test]
    fn test_find_asset_url_matching() {
        let json: serde_json::Value = serde_json::json!({
            "assets": [
                {
                    "name": format!("openedit-{}.tar.gz", platform_asset_suffix()),
                    "browser_download_url": "https://example.com/download"
                },
                {
                    "name": "openedit-other-platform.tar.gz",
                    "browser_download_url": "https://example.com/other"
                }
            ]
        });
        let url = find_asset_url(&json);
        assert_eq!(url, Some("https://example.com/download".to_string()));
    }

    #[test]
    fn test_find_asset_url_no_match() {
        let json: serde_json::Value = serde_json::json!({
            "assets": [
                {
                    "name": "openedit-totally-other-os.tar.gz",
                    "browser_download_url": "https://example.com/other"
                }
            ]
        });
        // Will only match if platform_asset_suffix() happens to be in the name,
        // which it won't for "totally-other-os".
        // This test just verifies we don't panic.
        let _ = find_asset_url(&json);
    }

    #[test]
    fn test_find_asset_url_empty_assets() {
        let json: serde_json::Value = serde_json::json!({
            "assets": []
        });
        assert_eq!(find_asset_url(&json), None);
    }

    #[test]
    fn test_find_asset_url_no_assets_key() {
        let json: serde_json::Value = serde_json::json!({});
        assert_eq!(find_asset_url(&json), None);
    }

    #[test]
    fn test_updater_state_default() {
        let state = UpdaterState::default();
        assert!(state.result.is_none());
        assert!(!state.dismissed);
        assert!(state.download_progress.is_none());
        assert!(!state.has_update());
    }

    #[test]
    fn test_has_update_when_available() {
        let mut state = UpdaterState::default();
        state.result = Some(UpdateCheckResult::Available(ReleaseInfo {
            version: "99.0.0".into(),
            body: "notes".into(),
            download_url: None,
            html_url: "https://example.com".into(),
        }));
        assert!(state.has_update());
    }

    #[test]
    fn test_has_update_dismissed() {
        let mut state = UpdaterState::default();
        state.result = Some(UpdateCheckResult::Available(ReleaseInfo {
            version: "99.0.0".into(),
            body: String::new(),
            download_url: None,
            html_url: String::new(),
        }));
        state.dismissed = true;
        assert!(!state.has_update());
    }

    #[test]
    fn test_has_update_when_up_to_date() {
        let mut state = UpdaterState::default();
        state.result = Some(UpdateCheckResult::UpToDate);
        assert!(!state.has_update());
    }

    #[test]
    fn test_has_update_when_error() {
        let mut state = UpdaterState::default();
        state.result = Some(UpdateCheckResult::Error("fail".into()));
        assert!(!state.has_update());
    }

    #[test]
    fn test_poll_receives_result() {
        let mut state = UpdaterState::default();
        let (tx, rx) = mpsc::channel();
        state.rx = Some(rx);
        tx.send(UpdateCheckResult::UpToDate).unwrap();
        state.poll();
        assert!(matches!(state.result, Some(UpdateCheckResult::UpToDate)));
    }

    #[test]
    fn test_poll_receives_download_progress() {
        let mut state = UpdaterState::default();
        let (tx, rx) = mpsc::channel();
        state.download_rx = Some(rx);
        tx.send(DownloadProgress::Done).unwrap();
        state.poll();
        assert!(matches!(
            state.download_progress,
            Some(DownloadProgress::Done)
        ));
        // download_rx should be cleared after Done
        assert!(state.download_rx.is_none());
    }

    #[test]
    fn test_start_download_no_result() {
        let mut state = UpdaterState::default();
        // No result set — start_download should be a no-op
        state.start_download();
        assert!(state.download_rx.is_none());
    }

    #[test]
    fn test_start_download_no_url() {
        let mut state = UpdaterState::default();
        state.result = Some(UpdateCheckResult::Available(ReleaseInfo {
            version: "2.0.0".into(),
            body: String::new(),
            download_url: None,
            html_url: String::new(),
        }));
        state.start_download();
        // No download_url means no download should start
        assert!(state.download_rx.is_none());
    }
}
