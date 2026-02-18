//! Trait-based plugin API for OpenEdit extensibility.
//!
//! Plugins can hook into editor events, add menu items, transform text,
//! and register custom commands for the command palette.

use std::any::Any;
use std::collections::HashMap;

/// Metadata describing a plugin.
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Unique identifier (e.g. "com.example.myplugin").
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Version string (semver recommended).
    pub version: String,
    /// Short description.
    pub description: String,
}

/// Events that plugins can react to.
#[derive(Debug, Clone)]
pub enum EditorEvent {
    /// A file was opened. Contains the file path.
    FileOpened(String),
    /// A file was saved.
    FileSaved(String),
    /// A file tab was closed.
    FileClosed(String),
    /// The active tab changed. Contains the new file path (if any).
    TabChanged(Option<String>),
    /// Text was inserted at (line, col) with given content.
    TextInserted { line: usize, col: usize, text: String },
    /// Text was deleted at (line, col) with given length.
    TextDeleted { line: usize, col: usize, len: usize },
    /// Editor is starting up.
    Startup,
    /// Editor is shutting down.
    Shutdown,
}

/// A command that a plugin exposes to the command palette.
#[derive(Debug, Clone)]
pub struct PluginCommand {
    /// Unique command id (e.g. "myplugin.do_thing").
    pub id: String,
    /// Display name in command palette.
    pub label: String,
    /// Optional keyboard shortcut hint (display only).
    pub shortcut: Option<String>,
}

/// Context passed to plugin methods, providing access to editor state.
pub struct PluginContext<'a> {
    /// The full text of the active document (if any).
    pub active_text: Option<&'a str>,
    /// Path of the active document (if any).
    pub active_path: Option<&'a str>,
    /// Current selection range: (start_offset, end_offset) in chars.
    pub selection: Option<(usize, usize)>,
}

/// Result of a plugin text transformation.
pub enum PluginAction {
    /// Do nothing.
    None,
    /// Replace the current selection (or full text if no selection) with this text.
    ReplaceSelection(String),
    /// Replace the entire document text.
    ReplaceAll(String),
    /// Insert text at cursor position.
    InsertAtCursor(String),
    /// Show a message/notification to the user.
    ShowMessage(String),
}

/// The core plugin trait. Implement this to create an OpenEdit plugin.
pub trait Plugin: Any + Send {
    /// Return plugin metadata.
    fn info(&self) -> PluginInfo;

    /// Called when the plugin is loaded. Return Ok(()) to succeed.
    fn on_load(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Called when the plugin is unloaded.
    fn on_unload(&mut self) {}

    /// React to an editor event.
    fn on_event(&mut self, _event: &EditorEvent) {}

    /// Return the list of commands this plugin provides.
    fn commands(&self) -> Vec<PluginCommand> {
        Vec::new()
    }

    /// Execute a command by id. Return an action for the editor to perform.
    fn execute_command(&mut self, _command_id: &str, _ctx: &PluginContext) -> PluginAction {
        PluginAction::None
    }
}

/// Manages registered plugins.
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    enabled: HashMap<String, bool>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            enabled: HashMap::new(),
        }
    }

    /// Register a plugin. Calls `on_load()`.
    pub fn register(&mut self, mut plugin: Box<dyn Plugin>) -> anyhow::Result<()> {
        let id = plugin.info().id.clone();
        plugin.on_load()?;
        self.enabled.insert(id, true);
        self.plugins.push(plugin);
        Ok(())
    }

    /// Unregister a plugin by id.
    pub fn unregister(&mut self, id: &str) {
        if let Some(pos) = self.plugins.iter().position(|p| p.info().id == id) {
            self.plugins[pos].on_unload();
            self.plugins.remove(pos);
            self.enabled.remove(id);
        }
    }

    /// Enable or disable a plugin.
    pub fn set_enabled(&mut self, id: &str, enabled: bool) {
        if let Some(v) = self.enabled.get_mut(id) {
            *v = enabled;
        }
    }

    /// Check if a plugin is enabled.
    pub fn is_enabled(&self, id: &str) -> bool {
        self.enabled.get(id).copied().unwrap_or(false)
    }

    /// Broadcast an event to all enabled plugins.
    pub fn broadcast_event(&mut self, event: &EditorEvent) {
        for i in 0..self.plugins.len() {
            let id = self.plugins[i].info().id.clone();
            if self.enabled.get(&id).copied().unwrap_or(false) {
                self.plugins[i].on_event(event);
            }
        }
    }

    /// Collect all commands from enabled plugins.
    pub fn all_commands(&self) -> Vec<(String, PluginCommand)> {
        let mut cmds = Vec::new();
        for plugin in &self.plugins {
            let info = plugin.info();
            if self.is_enabled(&info.id) {
                for cmd in plugin.commands() {
                    cmds.push((info.id.clone(), cmd));
                }
            }
        }
        cmds
    }

    /// Execute a command on the appropriate plugin.
    pub fn execute_command(&mut self, plugin_id: &str, command_id: &str, ctx: &PluginContext) -> PluginAction {
        for i in 0..self.plugins.len() {
            let id = self.plugins[i].info().id.clone();
            if id == plugin_id && self.enabled.get(&id).copied().unwrap_or(false) {
                return self.plugins[i].execute_command(command_id, ctx);
            }
        }
        PluginAction::None
    }

    /// List all registered plugins with their info and enabled state.
    pub fn list(&self) -> Vec<(PluginInfo, bool)> {
        self.plugins
            .iter()
            .map(|p| {
                let info = p.info();
                let enabled = self.is_enabled(&info.id);
                (info, enabled)
            })
            .collect()
    }

    /// Shutdown all plugins.
    pub fn shutdown(&mut self) {
        let event = EditorEvent::Shutdown;
        self.broadcast_event(&event);
        for plugin in &mut self.plugins {
            plugin.on_unload();
        }
        self.plugins.clear();
        self.enabled.clear();
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        for plugin in &mut self.plugins {
            plugin.on_unload();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin {
        loaded: bool,
        events: Vec<String>,
    }

    impl TestPlugin {
        fn new() -> Self {
            Self { loaded: false, events: Vec::new() }
        }
    }

    impl Plugin for TestPlugin {
        fn info(&self) -> PluginInfo {
            PluginInfo {
                id: "test.plugin".into(),
                name: "Test Plugin".into(),
                version: "0.1.0".into(),
                description: "A test plugin".into(),
            }
        }

        fn on_load(&mut self) -> anyhow::Result<()> {
            self.loaded = true;
            Ok(())
        }

        fn on_event(&mut self, event: &EditorEvent) {
            self.events.push(format!("{:?}", event));
        }

        fn commands(&self) -> Vec<PluginCommand> {
            vec![PluginCommand {
                id: "test.hello".into(),
                label: "Say Hello".into(),
                shortcut: None,
            }]
        }

        fn execute_command(&mut self, command_id: &str, _ctx: &PluginContext) -> PluginAction {
            if command_id == "test.hello" {
                PluginAction::ShowMessage("Hello from plugin!".into())
            } else {
                PluginAction::None
            }
        }
    }

    #[test]
    fn test_register_and_list() {
        let mut mgr = PluginManager::new();
        mgr.register(Box::new(TestPlugin::new())).unwrap();
        let list = mgr.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0.id, "test.plugin");
        assert!(list[0].1); // enabled
    }

    #[test]
    fn test_disable_plugin() {
        let mut mgr = PluginManager::new();
        mgr.register(Box::new(TestPlugin::new())).unwrap();
        mgr.set_enabled("test.plugin", false);
        assert!(!mgr.is_enabled("test.plugin"));
        assert!(mgr.all_commands().is_empty());
    }

    #[test]
    fn test_commands() {
        let mut mgr = PluginManager::new();
        mgr.register(Box::new(TestPlugin::new())).unwrap();
        let cmds = mgr.all_commands();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].1.id, "test.hello");
    }

    #[test]
    fn test_execute_command() {
        let mut mgr = PluginManager::new();
        mgr.register(Box::new(TestPlugin::new())).unwrap();
        let ctx = PluginContext { active_text: None, active_path: None, selection: None };
        let action = mgr.execute_command("test.plugin", "test.hello", &ctx);
        match action {
            PluginAction::ShowMessage(msg) => assert_eq!(msg, "Hello from plugin!"),
            _ => panic!("Expected ShowMessage"),
        }
    }

    #[test]
    fn test_unregister() {
        let mut mgr = PluginManager::new();
        mgr.register(Box::new(TestPlugin::new())).unwrap();
        mgr.unregister("test.plugin");
        assert!(mgr.list().is_empty());
    }
}
