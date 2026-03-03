/// Macro recording and playback system for the editor.
///
/// Records user actions (typed text, paste events, keyboard shortcuts) during a
/// recording session, and can replay them to repeat the same sequence of edits.
/// Supports named macro slots, save/load to disk as JSON, and multi-run playback.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A single recordable action within a macro.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MacroAction {
    /// Text typed by the user (from egui::Event::Text).
    InsertText(String),
    /// Text pasted from clipboard (from egui::Event::Paste).
    Paste(String),
    /// A keyboard shortcut or navigation key press.
    KeyAction {
        key: String,
        ctrl: bool,
        shift: bool,
        alt: bool,
    },
}

impl MacroAction {
    /// Convert a single macro action to a human-readable script line.
    pub fn to_script_line(&self) -> String {
        match self {
            MacroAction::InsertText(text) => {
                format!(
                    "type \"{}\"",
                    text.replace('\\', "\\\\").replace('"', "\\\"")
                )
            }
            MacroAction::Paste(text) => {
                format!(
                    "paste \"{}\"",
                    text.replace('\\', "\\\\")
                        .replace('"', "\\\"")
                        .replace('\n', "\\n")
                )
            }
            MacroAction::KeyAction {
                key,
                ctrl,
                shift,
                alt,
            } => {
                let mut parts = Vec::new();
                if *ctrl {
                    parts.push("ctrl");
                }
                if *shift {
                    parts.push("shift");
                }
                if *alt {
                    parts.push("alt");
                }
                parts.push(key);
                format!("key {}", parts.join("+"))
            }
        }
    }

    /// Parse a single script line into a MacroAction, returning None on invalid input.
    pub fn from_script_line(line: &str) -> Option<MacroAction> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        if let Some(rest) = line.strip_prefix("type ") {
            let text = parse_quoted_string(rest)?;
            Some(MacroAction::InsertText(text))
        } else if let Some(rest) = line.strip_prefix("paste ") {
            let text = parse_quoted_string(rest)?;
            Some(MacroAction::Paste(text))
        } else if let Some(rest) = line.strip_prefix("key ") {
            let rest = rest.trim();
            let mut ctrl = false;
            let mut shift = false;
            let mut alt = false;
            let parts: Vec<&str> = rest.split('+').collect();
            let key = parts.last()?.to_string();
            for &part in &parts[..parts.len() - 1] {
                match part {
                    "ctrl" => ctrl = true,
                    "shift" => shift = true,
                    "alt" => alt = true,
                    _ => {}
                }
            }
            Some(MacroAction::KeyAction {
                key,
                ctrl,
                shift,
                alt,
            })
        } else {
            None
        }
    }
}

/// Parse a double-quoted string with backslash escapes (\\, \", \n).
fn parse_quoted_string(s: &str) -> Option<String> {
    let s = s.trim();
    if !s.starts_with('"') || !s.ends_with('"') || s.len() < 2 {
        return None;
    }
    let inner = &s[1..s.len() - 1];
    let mut result = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some('n') => result.push('\n'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }
    Some(result)
}

/// Convert a list of macro actions to a multi-line script string.
pub fn actions_to_script(actions: &[MacroAction]) -> String {
    actions
        .iter()
        .map(|a| a.to_script_line())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse a multi-line script string into a list of macro actions.
/// Blank lines and lines starting with '#' are skipped.
pub fn actions_from_script(script: &str) -> Vec<MacroAction> {
    script
        .lines()
        .filter_map(MacroAction::from_script_line)
        .collect()
}

/// Records and stores macros for later playback.
pub struct MacroRecorder {
    /// Whether currently recording user actions.
    pub recording: bool,
    /// Actions recorded during the current (or most recent) session.
    pub actions: Vec<MacroAction>,
    /// Named saved macros for later retrieval (in-memory).
    pub saved_macros: Vec<(String, Vec<MacroAction>)>,
    /// Named macro slots stored as a HashMap for quick lookup.
    pub named_macros: HashMap<String, Vec<MacroAction>>,
}

impl MacroRecorder {
    /// Create a new MacroRecorder with no recorded actions.
    pub fn new() -> Self {
        Self {
            recording: false,
            actions: Vec::new(),
            saved_macros: Vec::new(),
            named_macros: HashMap::new(),
        }
    }

    /// Start recording a new macro. Clears any previously recorded (unsaved) actions.
    pub fn start_recording(&mut self) {
        self.actions.clear();
        self.recording = true;
    }

    /// Stop recording the current macro.
    pub fn stop_recording(&mut self) {
        self.recording = false;
    }

    /// Record an action if currently recording.
    pub fn record_action(&mut self, action: MacroAction) {
        if self.recording {
            self.actions.push(action);
        }
    }

    /// Whether the recorder is currently active.
    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// Save the current recorded actions under the given name (in-memory).
    pub fn save_macro(&mut self, name: String) {
        if !self.actions.is_empty() {
            let actions = self.actions.clone();
            // Also store in named_macros HashMap
            self.named_macros.insert(name.clone(), actions.clone());
            self.saved_macros.push((name, actions));
        }
    }

    /// Get the last recorded macro actions (from the current/most recent session).
    pub fn last_recorded(&self) -> &[MacroAction] {
        &self.actions
    }

    /// Get a named macro's actions by name.
    pub fn get_named_macro(&self, name: &str) -> Option<&[MacroAction]> {
        self.named_macros.get(name).map(|v| v.as_slice())
    }

    /// Load a named macro into the current actions buffer (for playback).
    pub fn load_named_macro(&mut self, name: &str) -> bool {
        if let Some(actions) = self.named_macros.get(name) {
            self.actions = actions.clone();
            true
        } else {
            false
        }
    }

    /// Delete a named macro.
    pub fn delete_named_macro(&mut self, name: &str) {
        self.named_macros.remove(name);
        self.saved_macros.retain(|(n, _)| n != name);
    }

    /// Get a sorted list of all named macro names.
    pub fn macro_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.named_macros.keys().cloned().collect();
        names.sort();
        names
    }

    /// Returns the path to the macros JSON file in the platform config directory.
    pub fn macros_path() -> Option<PathBuf> {
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
        config_dir.map(|d| d.join("openedit").join("macros.json"))
    }

    /// Save all named macros to disk as JSON.
    pub fn save_macros_to_disk(&self) {
        let Some(path) = Self::macros_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let Ok(json) = serde_json::to_string_pretty(&self.named_macros) else {
            return;
        };
        let _ = std::fs::write(&path, json);
    }

    /// Load named macros from disk.
    pub fn load_macros_from_disk(&mut self) {
        let Some(path) = Self::macros_path() else {
            return;
        };
        let Ok(content) = std::fs::read_to_string(&path) else {
            return;
        };
        let Ok(macros) = serde_json::from_str::<HashMap<String, Vec<MacroAction>>>(&content) else {
            return;
        };
        // Merge loaded macros into named_macros and saved_macros
        for (name, actions) in macros {
            if !self.named_macros.contains_key(&name) {
                self.saved_macros.push((name.clone(), actions.clone()));
            }
            self.named_macros.insert(name, actions);
        }
    }
}

impl Default for MacroRecorder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_recorder_not_recording() {
        let rec = MacroRecorder::new();
        assert!(!rec.is_recording());
        assert!(rec.actions.is_empty());
        assert!(rec.saved_macros.is_empty());
        assert!(rec.named_macros.is_empty());
    }

    #[test]
    fn test_start_stop_recording() {
        let mut rec = MacroRecorder::new();
        rec.start_recording();
        assert!(rec.is_recording());
        rec.stop_recording();
        assert!(!rec.is_recording());
    }

    #[test]
    fn test_record_action_while_recording() {
        let mut rec = MacroRecorder::new();
        rec.start_recording();
        rec.record_action(MacroAction::InsertText("hello".to_string()));
        rec.record_action(MacroAction::KeyAction {
            key: "Enter".to_string(),
            ctrl: false,
            shift: false,
            alt: false,
        });
        assert_eq!(rec.actions.len(), 2);
    }

    #[test]
    fn test_record_action_while_not_recording() {
        let mut rec = MacroRecorder::new();
        rec.record_action(MacroAction::InsertText("hello".to_string()));
        assert!(rec.actions.is_empty());
    }

    #[test]
    fn test_start_recording_clears_previous() {
        let mut rec = MacroRecorder::new();
        rec.start_recording();
        rec.record_action(MacroAction::InsertText("first".to_string()));
        rec.stop_recording();
        assert_eq!(rec.actions.len(), 1);

        rec.start_recording();
        assert!(rec.actions.is_empty());
    }

    #[test]
    fn test_save_macro() {
        let mut rec = MacroRecorder::new();
        rec.start_recording();
        rec.record_action(MacroAction::InsertText("test".to_string()));
        rec.stop_recording();

        rec.save_macro("my_macro".to_string());
        assert_eq!(rec.saved_macros.len(), 1);
        assert_eq!(rec.saved_macros[0].0, "my_macro");
        assert_eq!(rec.saved_macros[0].1.len(), 1);
        assert!(rec.named_macros.contains_key("my_macro"));
    }

    #[test]
    fn test_save_empty_macro_does_nothing() {
        let mut rec = MacroRecorder::new();
        rec.save_macro("empty".to_string());
        assert!(rec.saved_macros.is_empty());
        assert!(rec.named_macros.is_empty());
    }

    #[test]
    fn test_last_recorded() {
        let mut rec = MacroRecorder::new();
        assert!(rec.last_recorded().is_empty());

        rec.start_recording();
        rec.record_action(MacroAction::Paste("pasted".to_string()));
        rec.stop_recording();

        let recorded = rec.last_recorded();
        assert_eq!(recorded.len(), 1);
        match &recorded[0] {
            MacroAction::Paste(text) => assert_eq!(text, "pasted"),
            _ => panic!("Expected Paste action"),
        }
    }

    #[test]
    fn test_named_macros() {
        let mut rec = MacroRecorder::new();
        rec.start_recording();
        rec.record_action(MacroAction::InsertText("hello".to_string()));
        rec.stop_recording();
        rec.save_macro("greeting".to_string());

        assert_eq!(rec.macro_names(), vec!["greeting".to_string()]);
        assert!(rec.get_named_macro("greeting").is_some());
        assert_eq!(rec.get_named_macro("greeting").unwrap().len(), 1);
    }

    #[test]
    fn test_load_named_macro() {
        let mut rec = MacroRecorder::new();
        rec.start_recording();
        rec.record_action(MacroAction::InsertText("hello".to_string()));
        rec.stop_recording();
        rec.save_macro("greeting".to_string());

        // Clear current actions
        rec.start_recording();
        rec.stop_recording();
        assert!(rec.actions.is_empty());

        // Load named macro
        assert!(rec.load_named_macro("greeting"));
        assert_eq!(rec.actions.len(), 1);
    }

    #[test]
    fn test_load_nonexistent_macro() {
        let mut rec = MacroRecorder::new();
        assert!(!rec.load_named_macro("nonexistent"));
    }

    #[test]
    fn test_delete_named_macro() {
        let mut rec = MacroRecorder::new();
        rec.start_recording();
        rec.record_action(MacroAction::InsertText("hello".to_string()));
        rec.stop_recording();
        rec.save_macro("greeting".to_string());

        rec.delete_named_macro("greeting");
        assert!(rec.named_macros.is_empty());
        assert!(rec.saved_macros.is_empty());
    }

    #[test]
    fn test_macro_names_sorted() {
        let mut rec = MacroRecorder::new();
        rec.start_recording();
        rec.record_action(MacroAction::InsertText("a".to_string()));
        rec.stop_recording();
        rec.save_macro("zebra".to_string());

        rec.start_recording();
        rec.record_action(MacroAction::InsertText("b".to_string()));
        rec.stop_recording();
        rec.save_macro("alpha".to_string());

        let names = rec.macro_names();
        assert_eq!(names, vec!["alpha".to_string(), "zebra".to_string()]);
    }

    #[test]
    fn test_serialize_deserialize_macro_action() {
        let actions = vec![
            MacroAction::InsertText("hello".to_string()),
            MacroAction::Paste("world".to_string()),
            MacroAction::KeyAction {
                key: "Enter".to_string(),
                ctrl: false,
                shift: true,
                alt: false,
            },
        ];

        let json = serde_json::to_string(&actions).unwrap();
        let deserialized: Vec<MacroAction> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 3);

        match &deserialized[0] {
            MacroAction::InsertText(t) => assert_eq!(t, "hello"),
            _ => panic!("Expected InsertText"),
        }
        match &deserialized[1] {
            MacroAction::Paste(t) => assert_eq!(t, "world"),
            _ => panic!("Expected Paste"),
        }
        match &deserialized[2] {
            MacroAction::KeyAction {
                key,
                ctrl,
                shift,
                alt,
            } => {
                assert_eq!(key, "Enter");
                assert!(!ctrl);
                assert!(shift);
                assert!(!alt);
            }
            _ => panic!("Expected KeyAction"),
        }
    }

    #[test]
    fn test_save_load_macros_disk_roundtrip() {
        // Use a temp directory to avoid polluting user config
        let tmp = std::env::temp_dir().join("openedit_test_macros");
        let _ = std::fs::create_dir_all(&tmp);
        let path = tmp.join("macros.json");

        let mut macros = HashMap::new();
        macros.insert(
            "test_macro".to_string(),
            vec![
                MacroAction::InsertText("abc".to_string()),
                MacroAction::KeyAction {
                    key: "Enter".to_string(),
                    ctrl: false,
                    shift: false,
                    alt: false,
                },
            ],
        );

        // Write directly
        let json = serde_json::to_string_pretty(&macros).unwrap();
        std::fs::write(&path, &json).unwrap();

        // Read back
        let content = std::fs::read_to_string(&path).unwrap();
        let loaded: HashMap<String, Vec<MacroAction>> = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(loaded.contains_key("test_macro"));
        assert_eq!(loaded["test_macro"].len(), 2);

        // Cleanup
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&tmp);
    }

    #[test]
    fn test_script_roundtrip() {
        let actions = vec![
            MacroAction::InsertText("hello world".to_string()),
            MacroAction::Paste("line1\nline2".to_string()),
            MacroAction::KeyAction {
                key: "Enter".to_string(),
                ctrl: false,
                shift: false,
                alt: false,
            },
            MacroAction::KeyAction {
                key: "S".to_string(),
                ctrl: true,
                shift: false,
                alt: false,
            },
            MacroAction::KeyAction {
                key: "K".to_string(),
                ctrl: true,
                shift: true,
                alt: false,
            },
            MacroAction::InsertText("has \"quotes\" and \\backslash".to_string()),
        ];

        let script = actions_to_script(&actions);
        let parsed = actions_from_script(&script);

        assert_eq!(parsed.len(), actions.len());

        // Verify round-trip by converting back to script
        let script2 = actions_to_script(&parsed);
        assert_eq!(script, script2);
    }

    #[test]
    fn test_script_line_type() {
        let action = MacroAction::InsertText("hello".to_string());
        assert_eq!(action.to_script_line(), "type \"hello\"");
        let parsed = MacroAction::from_script_line("type \"hello\"").unwrap();
        match parsed {
            MacroAction::InsertText(t) => assert_eq!(t, "hello"),
            _ => panic!("Expected InsertText"),
        }
    }

    #[test]
    fn test_script_line_paste() {
        let action = MacroAction::Paste("clip".to_string());
        assert_eq!(action.to_script_line(), "paste \"clip\"");
        let parsed = MacroAction::from_script_line("paste \"clip\"").unwrap();
        match parsed {
            MacroAction::Paste(t) => assert_eq!(t, "clip"),
            _ => panic!("Expected Paste"),
        }
    }

    #[test]
    fn test_script_line_key_simple() {
        let action = MacroAction::KeyAction {
            key: "Enter".to_string(),
            ctrl: false,
            shift: false,
            alt: false,
        };
        assert_eq!(action.to_script_line(), "key Enter");
    }

    #[test]
    fn test_script_line_key_modifiers() {
        let action = MacroAction::KeyAction {
            key: "S".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
        };
        assert_eq!(action.to_script_line(), "key ctrl+shift+S");

        let parsed = MacroAction::from_script_line("key ctrl+shift+S").unwrap();
        match parsed {
            MacroAction::KeyAction {
                key,
                ctrl,
                shift,
                alt,
            } => {
                assert_eq!(key, "S");
                assert!(ctrl);
                assert!(shift);
                assert!(!alt);
            }
            _ => panic!("Expected KeyAction"),
        }
    }

    #[test]
    fn test_script_blank_and_comment_lines_skipped() {
        let script = "type \"a\"\n\n# comment\nkey Enter\n";
        let parsed = actions_from_script(script);
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn test_script_invalid_lines_skipped() {
        let script = "type \"a\"\ninvalid line\nkey Enter\n";
        let parsed = actions_from_script(script);
        assert_eq!(parsed.len(), 2);
    }
}
