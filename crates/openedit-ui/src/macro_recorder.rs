/// Macro recording and playback system for the editor.
///
/// Records user actions (typed text, paste events, keyboard shortcuts) during a
/// recording session, and can replay them to repeat the same sequence of edits.

/// A single recordable action within a macro.
#[derive(Debug, Clone)]
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

/// Records and stores macros for later playback.
pub struct MacroRecorder {
    /// Whether currently recording user actions.
    pub recording: bool,
    /// Actions recorded during the current (or most recent) session.
    pub actions: Vec<MacroAction>,
    /// Named saved macros for later retrieval.
    pub saved_macros: Vec<(String, Vec<MacroAction>)>,
}

impl MacroRecorder {
    /// Create a new MacroRecorder with no recorded actions.
    pub fn new() -> Self {
        Self {
            recording: false,
            actions: Vec::new(),
            saved_macros: Vec::new(),
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

    /// Save the current recorded actions under the given name.
    pub fn save_macro(&mut self, name: String) {
        if !self.actions.is_empty() {
            self.saved_macros.push((name, self.actions.clone()));
        }
    }

    /// Get the last recorded macro actions (from the current/most recent session).
    pub fn last_recorded(&self) -> &[MacroAction] {
        &self.actions
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
    }

    #[test]
    fn test_save_empty_macro_does_nothing() {
        let mut rec = MacroRecorder::new();
        rec.save_macro("empty".to_string());
        assert!(rec.saved_macros.is_empty());
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
}
