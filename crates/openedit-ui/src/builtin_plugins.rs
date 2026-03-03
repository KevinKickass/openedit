//! Built-in plugins that demonstrate the plugin system.

use openedit_core::{EditorEvent, Plugin, PluginAction, PluginCommand, PluginContext, PluginInfo};

/// Plugin that counts words in the active document.
///
/// Tracks word count on tab change and provides a command to display it.
pub struct WordCounterPlugin {
    word_count: usize,
}

impl WordCounterPlugin {
    pub fn new() -> Self {
        Self { word_count: 0 }
    }

    fn count_words(text: &str) -> usize {
        text.split_whitespace().count()
    }
}

impl Plugin for WordCounterPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "builtin.word_counter".into(),
            name: "Word Counter".into(),
            version: "1.0.0".into(),
            description: "Counts words in the active document".into(),
        }
    }

    fn on_event(&mut self, event: &EditorEvent) {
        if let EditorEvent::TabChanged(Some(_)) = event {
            // Word count will be updated when the command is executed
            // since we need the PluginContext to access the text.
        }
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![PluginCommand {
            id: "builtin.word_counter.show_count".into(),
            label: "Word Counter: Show Word Count".into(),
            shortcut: None,
        }]
    }

    fn execute_command(&mut self, command_id: &str, ctx: &PluginContext) -> PluginAction {
        if command_id == "builtin.word_counter.show_count" {
            if let Some(text) = ctx.active_text {
                self.word_count = Self::count_words(text);
                let char_count = text.chars().count();
                let line_count = text.lines().count().max(1);
                PluginAction::ShowMessage(format!(
                    "Words: {}  |  Characters: {}  |  Lines: {}",
                    self.word_count, char_count, line_count
                ))
            } else {
                PluginAction::ShowMessage("No active document".into())
            }
        } else {
            PluginAction::None
        }
    }
}

/// Plugin that inserts lorem ipsum placeholder text at the cursor.
pub struct LoremIpsumPlugin;

impl LoremIpsumPlugin {
    pub fn new() -> Self {
        Self
    }
}

const LOREM_SHORT: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit.";

const LOREM_PARAGRAPH: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, \
quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute \
irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. \
Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit \
anim id est laborum.";

impl Plugin for LoremIpsumPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "builtin.lorem_ipsum".into(),
            name: "Lorem Ipsum".into(),
            version: "1.0.0".into(),
            description: "Insert lorem ipsum placeholder text".into(),
        }
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![
            PluginCommand {
                id: "builtin.lorem_ipsum.insert_sentence".into(),
                label: "Lorem Ipsum: Insert Sentence".into(),
                shortcut: None,
            },
            PluginCommand {
                id: "builtin.lorem_ipsum.insert_paragraph".into(),
                label: "Lorem Ipsum: Insert Paragraph".into(),
                shortcut: None,
            },
        ]
    }

    fn execute_command(&mut self, command_id: &str, _ctx: &PluginContext) -> PluginAction {
        match command_id {
            "builtin.lorem_ipsum.insert_sentence" => {
                PluginAction::InsertAtCursor(LOREM_SHORT.into())
            }
            "builtin.lorem_ipsum.insert_paragraph" => {
                PluginAction::InsertAtCursor(LOREM_PARAGRAPH.into())
            }
            _ => PluginAction::None,
        }
    }
}

/// Plugin that provides a timestamp insertion command.
pub struct TimestampPlugin;

impl TimestampPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for TimestampPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "builtin.timestamp".into(),
            name: "Insert Timestamp".into(),
            version: "1.0.0".into(),
            description: "Insert current date/time at cursor".into(),
        }
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![
            PluginCommand {
                id: "builtin.timestamp.insert_iso".into(),
                label: "Timestamp: Insert ISO 8601 Date/Time".into(),
                shortcut: None,
            },
            PluginCommand {
                id: "builtin.timestamp.insert_unix".into(),
                label: "Timestamp: Insert Unix Timestamp".into(),
                shortcut: None,
            },
        ]
    }

    fn execute_command(&mut self, command_id: &str, _ctx: &PluginContext) -> PluginAction {
        use std::time::{SystemTime, UNIX_EPOCH};
        match command_id {
            "builtin.timestamp.insert_iso" => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default();
                let secs = now.as_secs();
                // Simple ISO 8601-like format without external crate
                let days_since_epoch = secs / 86400;
                let time_of_day = secs % 86400;
                let hours = time_of_day / 3600;
                let minutes = (time_of_day % 3600) / 60;
                let seconds = time_of_day % 60;

                // Compute year/month/day from days since 1970-01-01
                let (year, month, day) = days_to_ymd(days_since_epoch as i64);

                let ts = format!(
                    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                    year, month, day, hours, minutes, seconds
                );
                PluginAction::InsertAtCursor(ts)
            }
            "builtin.timestamp.insert_unix" => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default();
                PluginAction::InsertAtCursor(now.as_secs().to_string())
            }
            _ => PluginAction::None,
        }
    }
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_ymd(mut days: i64) -> (i64, u32, u32) {
    // Algorithm from Howard Hinnant's date algorithms
    days += 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openedit_core::PluginContext;

    #[test]
    fn test_word_counter_plugin() {
        let mut plugin = WordCounterPlugin::new();
        assert_eq!(plugin.info().id, "builtin.word_counter");
        assert_eq!(plugin.commands().len(), 1);

        let text = "Hello world, this is a test.";
        let ctx = PluginContext {
            active_text: Some(text),
            ..PluginContext::default()
        };
        let action = plugin.execute_command("builtin.word_counter.show_count", &ctx);
        match action {
            PluginAction::ShowMessage(msg) => {
                assert!(msg.contains("Words: 6"), "Expected 'Words: 6' in: {}", msg);
            }
            _ => panic!("Expected ShowMessage"),
        }
    }

    #[test]
    fn test_word_counter_empty() {
        let mut plugin = WordCounterPlugin::new();
        let ctx = PluginContext::default();
        let action = plugin.execute_command("builtin.word_counter.show_count", &ctx);
        match action {
            PluginAction::ShowMessage(msg) => {
                assert!(msg.contains("No active document"));
            }
            _ => panic!("Expected ShowMessage"),
        }
    }

    #[test]
    fn test_lorem_ipsum_sentence() {
        let mut plugin = LoremIpsumPlugin::new();
        assert_eq!(plugin.info().id, "builtin.lorem_ipsum");
        assert_eq!(plugin.commands().len(), 2);

        let ctx = PluginContext::default();
        let action = plugin.execute_command("builtin.lorem_ipsum.insert_sentence", &ctx);
        match action {
            PluginAction::InsertAtCursor(text) => {
                assert!(text.starts_with("Lorem ipsum"));
            }
            _ => panic!("Expected InsertAtCursor"),
        }
    }

    #[test]
    fn test_lorem_ipsum_paragraph() {
        let mut plugin = LoremIpsumPlugin::new();
        let ctx = PluginContext::default();
        let action = plugin.execute_command("builtin.lorem_ipsum.insert_paragraph", &ctx);
        match action {
            PluginAction::InsertAtCursor(text) => {
                assert!(text.starts_with("Lorem ipsum"));
                assert!(text.contains("laborum"));
            }
            _ => panic!("Expected InsertAtCursor"),
        }
    }

    #[test]
    fn test_timestamp_plugin() {
        let mut plugin = TimestampPlugin::new();
        assert_eq!(plugin.info().id, "builtin.timestamp");
        assert_eq!(plugin.commands().len(), 2);

        let ctx = PluginContext::default();

        // Test ISO timestamp
        let action = plugin.execute_command("builtin.timestamp.insert_iso", &ctx);
        match action {
            PluginAction::InsertAtCursor(text) => {
                assert!(
                    text.contains("T"),
                    "Expected ISO format with T separator: {}",
                    text
                );
                assert!(text.ends_with("Z"), "Expected UTC Z suffix: {}", text);
            }
            _ => panic!("Expected InsertAtCursor"),
        }

        // Test Unix timestamp
        let action = plugin.execute_command("builtin.timestamp.insert_unix", &ctx);
        match action {
            PluginAction::InsertAtCursor(text) => {
                let ts: u64 = text.parse().expect("Expected numeric timestamp");
                assert!(ts > 1_700_000_000, "Timestamp seems too small: {}", ts);
            }
            _ => panic!("Expected InsertAtCursor"),
        }
    }

    #[test]
    fn test_days_to_ymd() {
        // 1970-01-01 = day 0
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
        // 2000-01-01 = day 10957
        assert_eq!(days_to_ymd(10957), (2000, 1, 1));
    }

    #[test]
    fn test_plugin_manager_with_builtins() {
        let mut mgr = openedit_core::PluginManager::new();
        mgr.register(Box::new(WordCounterPlugin::new())).unwrap();
        mgr.register(Box::new(LoremIpsumPlugin::new())).unwrap();
        mgr.register(Box::new(TimestampPlugin::new())).unwrap();

        let list = mgr.list();
        assert_eq!(list.len(), 3);

        let cmds = mgr.all_commands();
        // WordCounter: 1, LoremIpsum: 2, Timestamp: 2
        assert_eq!(cmds.len(), 5);

        // Execute a lorem ipsum command
        let ctx = PluginContext::default();
        let action = mgr.execute_command(
            "builtin.lorem_ipsum",
            "builtin.lorem_ipsum.insert_sentence",
            &ctx,
        );
        match action {
            PluginAction::InsertAtCursor(text) => {
                assert!(text.starts_with("Lorem ipsum"));
            }
            _ => panic!("Expected InsertAtCursor"),
        }
    }
}
