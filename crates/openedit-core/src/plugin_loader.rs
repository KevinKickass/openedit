//! Plugin manifest parsing, directory-based loading, and script plugin execution.
//!
//! Plugins live in `~/.config/openedit/plugins/<plugin-name>/` with a `plugin.toml` manifest.
//! Each plugin can define commands that either run shell scripts (receiving selection/document
//! on stdin, returning replacement text on stdout) or map to sequences of built-in editor commands.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use crate::plugin::{Plugin, PluginAction, PluginCommand, PluginContext, PluginInfo};

// ---------------------------------------------------------------------------
// Manifest types (parsed from plugin.toml)
// ---------------------------------------------------------------------------

/// Top-level manifest structure: `[plugin]` table in `plugin.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginManifest {
    pub plugin: PluginManifestInner,
}

/// The `[plugin]` table contents.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginManifestInner {
    /// Unique plugin identifier, e.g. "sort-json-keys".
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// SemVer version string.
    pub version: String,
    /// Short description of what this plugin does.
    #[serde(default)]
    pub description: String,
    /// Author name(s).
    #[serde(default)]
    pub author: String,
    /// Minimum editor version required (informational, not enforced yet).
    #[serde(default)]
    pub min_editor_version: String,
    /// Commands this plugin provides.
    #[serde(default)]
    pub commands: Vec<ManifestCommand>,
}

/// A command declared in the plugin manifest.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ManifestCommand {
    /// Unique command id, e.g. "sort-json-keys.sort".
    pub id: String,
    /// Display label in the command palette.
    pub label: String,
    /// Optional keyboard shortcut hint (display only).
    #[serde(default)]
    pub shortcut: Option<String>,
    /// Action type: "script" for shell script execution, "builtin" for a built-in action.
    #[serde(default = "default_action_type")]
    pub action: String,
    /// For script actions: the script filename relative to the plugin directory.
    #[serde(default)]
    pub script: Option<String>,
    /// What to pass on stdin: "selection", "document", or "none".
    #[serde(default = "default_input")]
    pub input: String,
    /// How to handle stdout: "replace_selection", "replace_all", "insert", "message".
    #[serde(default = "default_output")]
    pub output: String,
}

fn default_action_type() -> String {
    "script".to_string()
}

fn default_input() -> String {
    "selection".to_string()
}

fn default_output() -> String {
    "replace_selection".to_string()
}

// ---------------------------------------------------------------------------
// Plugin enabled/disabled state persistence
// ---------------------------------------------------------------------------

/// Serialized state for plugin enable/disable, saved to `plugins.json`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PluginStates {
    /// Map from plugin id to enabled state.
    #[serde(default)]
    pub enabled: HashMap<String, bool>,
}

impl PluginStates {
    /// Load from the standard path. Returns default if file doesn't exist or can't be parsed.
    pub fn load() -> Self {
        if let Some(path) = plugin_states_path() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(states) = serde_json::from_str(&data) {
                    return states;
                }
            }
        }
        Self::default()
    }

    /// Save to the standard path.
    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(path) = plugin_states_path() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let data = serde_json::to_string_pretty(self)?;
            std::fs::write(&path, data)?;
        }
        Ok(())
    }
}

/// Returns the path to `plugins.json` in the platform config directory.
fn plugin_states_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("plugins.json"))
}

// ---------------------------------------------------------------------------
// Manifest parsing
// ---------------------------------------------------------------------------

/// Parse a `plugin.toml` manifest from a string.
pub fn parse_manifest(toml_str: &str) -> anyhow::Result<PluginManifest> {
    let manifest: PluginManifest = toml::from_str(toml_str)?;

    // Validate required fields
    if manifest.plugin.id.is_empty() {
        anyhow::bail!("plugin.id is required");
    }
    if manifest.plugin.name.is_empty() {
        anyhow::bail!("plugin.name is required");
    }
    if manifest.plugin.version.is_empty() {
        anyhow::bail!("plugin.version is required");
    }

    // Validate commands
    for cmd in &manifest.plugin.commands {
        if cmd.id.is_empty() {
            anyhow::bail!("command id is required");
        }
        if cmd.label.is_empty() {
            anyhow::bail!("command label is required for command '{}'", cmd.id);
        }
        match cmd.action.as_str() {
            "script" => {
                if cmd.script.is_none() || cmd.script.as_ref().is_none_or(|s| s.is_empty()) {
                    anyhow::bail!(
                        "command '{}' has action=script but no script file specified",
                        cmd.id
                    );
                }
            }
            "builtin" | "message" => {}
            other => {
                anyhow::bail!("unknown action type '{}' for command '{}'", other, cmd.id);
            }
        }
        // Validate input/output values
        match cmd.input.as_str() {
            "selection" | "document" | "none" => {}
            other => anyhow::bail!("unknown input type '{}' for command '{}'", other, cmd.id),
        }
        match cmd.output.as_str() {
            "replace_selection" | "replace_all" | "insert" | "message" => {}
            other => anyhow::bail!("unknown output type '{}' for command '{}'", other, cmd.id),
        }
    }

    Ok(manifest)
}

/// Parse a `plugin.toml` manifest from a file path.
pub fn parse_manifest_file(path: &Path) -> anyhow::Result<PluginManifest> {
    let content = std::fs::read_to_string(path)?;
    parse_manifest(&content)
}

// ---------------------------------------------------------------------------
// ScriptPlugin: implements Plugin trait via shell scripts
// ---------------------------------------------------------------------------

/// A plugin backed by shell scripts defined in its manifest.
pub struct ScriptPlugin {
    manifest: PluginManifest,
    /// The directory containing this plugin's files.
    plugin_dir: PathBuf,
}

impl ScriptPlugin {
    /// Create a new ScriptPlugin from a parsed manifest and its directory.
    pub fn new(manifest: PluginManifest, plugin_dir: PathBuf) -> Self {
        Self {
            manifest,
            plugin_dir,
        }
    }

    /// Run a shell script from this plugin's directory.
    fn run_script(&self, script_name: &str, input: &str) -> anyhow::Result<String> {
        let script_path = self.plugin_dir.join(script_name);
        if !script_path.exists() {
            anyhow::bail!("script not found: {}", script_path.display());
        }

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(script_path.to_string_lossy().as_ref())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&self.plugin_dir)
            .spawn()?;

        if let Some(ref mut stdin) = child.stdin {
            stdin.write_all(input.as_bytes())?;
        }
        // Close stdin so the child process knows we're done
        drop(child.stdin.take());

        let output = child.wait_with_output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "script '{}' failed with status {}: {}",
                script_name,
                output.status,
                stderr.trim()
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

impl Plugin for ScriptPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: self.manifest.plugin.id.clone(),
            name: self.manifest.plugin.name.clone(),
            version: self.manifest.plugin.version.clone(),
            description: self.manifest.plugin.description.clone(),
        }
    }

    fn on_load(&mut self) -> anyhow::Result<()> {
        log::info!(
            "Loaded script plugin: {} v{} from {}",
            self.manifest.plugin.name,
            self.manifest.plugin.version,
            self.plugin_dir.display()
        );
        Ok(())
    }

    fn commands(&self) -> Vec<PluginCommand> {
        self.manifest
            .plugin
            .commands
            .iter()
            .map(|cmd| PluginCommand {
                id: cmd.id.clone(),
                label: cmd.label.clone(),
                shortcut: cmd.shortcut.clone(),
            })
            .collect()
    }

    fn execute_command(&mut self, command_id: &str, ctx: &PluginContext) -> PluginAction {
        let cmd = match self
            .manifest
            .plugin
            .commands
            .iter()
            .find(|c| c.id == command_id)
        {
            Some(c) => c.clone(),
            None => return PluginAction::None,
        };

        match cmd.action.as_str() {
            "script" => {
                let script_name = match &cmd.script {
                    Some(s) => s.clone(),
                    None => return PluginAction::ShowMessage("No script specified".to_string()),
                };

                // Determine input
                let input = match cmd.input.as_str() {
                    "selection" => {
                        if let (Some(text), Some((start, end))) = (ctx.active_text, ctx.selection) {
                            // Extract selected text from char offsets
                            text.chars().skip(start).take(end - start).collect()
                        } else {
                            String::new()
                        }
                    }
                    "document" => ctx.active_text.unwrap_or("").to_string(),
                    _ => String::new(),
                };

                // Run script
                match self.run_script(&script_name, &input) {
                    Ok(output) => match cmd.output.as_str() {
                        "replace_selection" => PluginAction::ReplaceSelection(output),
                        "replace_all" => PluginAction::ReplaceAll(output),
                        "insert" => PluginAction::InsertAtCursor(output),
                        "message" => PluginAction::ShowMessage(output),
                        _ => PluginAction::None,
                    },
                    Err(e) => PluginAction::ShowMessage(format!("Plugin error: {}", e)),
                }
            }
            "message" => PluginAction::ShowMessage(format!(
                "{} v{}",
                self.manifest.plugin.name, self.manifest.plugin.version
            )),
            _ => PluginAction::None,
        }
    }
}

// ---------------------------------------------------------------------------
// Directory scanning and plugin loading
// ---------------------------------------------------------------------------

/// Returns the platform-appropriate config directory for OpenEdit.
pub fn config_dir() -> Option<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
    } else {
        // Linux and other Unix
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".config"))
            })
    };
    base.map(|d| d.join("openedit"))
}

/// Returns the directory where plugins are stored.
pub fn plugins_dir() -> Option<PathBuf> {
    config_dir().map(|d| d.join("plugins"))
}

/// Scan the plugins directory and load all valid plugins.
///
/// Returns a list of (ScriptPlugin, errors) -- valid plugins are returned,
/// and any errors from broken plugin directories are collected.
pub fn scan_plugins() -> (Vec<ScriptPlugin>, Vec<String>) {
    scan_plugins_from_dir(plugins_dir())
}

/// Scan a specific directory for plugins. Useful for testing with a custom path.
pub fn scan_plugins_from_dir(dir: Option<PathBuf>) -> (Vec<ScriptPlugin>, Vec<String>) {
    let mut loaded = Vec::new();
    let mut errors = Vec::new();

    let dir = match dir {
        Some(d) => d,
        None => {
            errors.push("Could not determine plugins directory".to_string());
            return (loaded, errors);
        }
    };

    if !dir.exists() {
        // No plugins directory yet -- that's fine, not an error
        return (loaded, errors);
    }

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(e) => {
            errors.push(format!("Failed to read plugins directory: {}", e));
            return (loaded, errors);
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                errors.push(format!("Failed to read directory entry: {}", e));
                continue;
            }
        };

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("plugin.toml");
        if !manifest_path.exists() {
            let dir_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            errors.push(format!(
                "Plugin directory '{}' has no plugin.toml, skipping",
                dir_name
            ));
            continue;
        }

        match parse_manifest_file(&manifest_path) {
            Ok(manifest) => {
                let plugin = ScriptPlugin::new(manifest, path);
                loaded.push(plugin);
            }
            Err(e) => {
                let dir_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                errors.push(format!("Failed to parse plugin '{}': {}", dir_name, e));
            }
        }
    }

    (loaded, errors)
}

/// Load plugins from the standard directory and register them with a PluginManager.
///
/// Also loads and applies saved enabled/disabled states from `plugins.json`.
/// Returns any warnings/errors encountered during loading.
pub fn load_plugins(manager: &mut crate::plugin::PluginManager) -> Vec<String> {
    let mut warnings = Vec::new();

    // Load saved plugin states
    let states = PluginStates::load();

    // Scan for plugins
    let (plugins, scan_errors) = scan_plugins();
    warnings.extend(scan_errors);

    // Register each plugin
    for plugin in plugins {
        let id = plugin.manifest.plugin.id.clone();
        match manager.register(Box::new(plugin)) {
            Ok(()) => {
                // Apply saved enabled/disabled state
                if let Some(&enabled) = states.enabled.get(&id) {
                    manager.set_enabled(&id, enabled);
                }
                log::info!("Registered plugin: {}", id);
            }
            Err(e) => {
                warnings.push(format!("Failed to register plugin '{}': {}", id, e));
            }
        }
    }

    warnings
}

/// Load plugins from a specific directory and register them with a PluginManager.
/// Useful for testing.
pub fn load_plugins_from_dir(
    manager: &mut crate::plugin::PluginManager,
    dir: &Path,
) -> Vec<String> {
    let mut warnings = Vec::new();

    let (plugins, scan_errors) = scan_plugins_from_dir(Some(dir.to_path_buf()));
    warnings.extend(scan_errors);

    for plugin in plugins {
        let id = plugin.manifest.plugin.id.clone();
        match manager.register(Box::new(plugin)) {
            Ok(()) => {
                log::info!("Registered plugin: {}", id);
            }
            Err(e) => {
                warnings.push(format!("Failed to register plugin '{}': {}", id, e));
            }
        }
    }

    warnings
}

/// Save the current enabled/disabled state of all plugins in the manager.
pub fn save_plugin_states(manager: &crate::plugin::PluginManager) -> anyhow::Result<()> {
    let mut states = PluginStates::default();
    for (info, enabled) in manager.list() {
        states.enabled.insert(info.id, enabled);
    }
    states.save()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_manifest() {
        let toml = r#"
[plugin]
id = "my-plugin"
name = "My Plugin"
version = "1.0.0"
description = "Does something useful"
author = "Someone"
min_editor_version = "0.1.0"
"#;
        let manifest = parse_manifest(toml).unwrap();
        assert_eq!(manifest.plugin.id, "my-plugin");
        assert_eq!(manifest.plugin.name, "My Plugin");
        assert_eq!(manifest.plugin.version, "1.0.0");
        assert_eq!(manifest.plugin.description, "Does something useful");
        assert_eq!(manifest.plugin.author, "Someone");
        assert_eq!(manifest.plugin.min_editor_version, "0.1.0");
        assert!(manifest.plugin.commands.is_empty());
    }

    #[test]
    fn test_parse_manifest_with_commands() {
        let toml = r#"
[plugin]
id = "sort-json-keys"
name = "Sort JSON Keys"
version = "1.0.0"
description = "Sorts JSON object keys alphabetically"

[[plugin.commands]]
id = "sort-json-keys.sort"
label = "Sort JSON Keys"
action = "script"
script = "sort_json.sh"
input = "selection"
output = "replace_selection"

[[plugin.commands]]
id = "sort-json-keys.info"
label = "Sort JSON Keys: About"
action = "message"
"#;
        let manifest = parse_manifest(toml).unwrap();
        assert_eq!(manifest.plugin.commands.len(), 2);

        let cmd0 = &manifest.plugin.commands[0];
        assert_eq!(cmd0.id, "sort-json-keys.sort");
        assert_eq!(cmd0.label, "Sort JSON Keys");
        assert_eq!(cmd0.action, "script");
        assert_eq!(cmd0.script.as_deref(), Some("sort_json.sh"));
        assert_eq!(cmd0.input, "selection");
        assert_eq!(cmd0.output, "replace_selection");

        let cmd1 = &manifest.plugin.commands[1];
        assert_eq!(cmd1.id, "sort-json-keys.info");
        assert_eq!(cmd1.action, "message");
    }

    #[test]
    fn test_parse_manifest_defaults() {
        let toml = r#"
[plugin]
id = "minimal"
name = "Minimal Plugin"
version = "0.1.0"

[[plugin.commands]]
id = "minimal.run"
label = "Run"
action = "script"
script = "run.sh"
"#;
        let manifest = parse_manifest(toml).unwrap();
        assert_eq!(manifest.plugin.description, "");
        assert_eq!(manifest.plugin.author, "");

        let cmd = &manifest.plugin.commands[0];
        assert_eq!(cmd.input, "selection"); // default
        assert_eq!(cmd.output, "replace_selection"); // default
        assert!(cmd.shortcut.is_none());
    }

    #[test]
    fn test_parse_manifest_missing_id() {
        let toml = r#"
[plugin]
id = ""
name = "Bad Plugin"
version = "1.0.0"
"#;
        let err = parse_manifest(toml).unwrap_err();
        assert!(err.to_string().contains("plugin.id is required"));
    }

    #[test]
    fn test_parse_manifest_missing_name() {
        let toml = r#"
[plugin]
id = "test"
name = ""
version = "1.0.0"
"#;
        let err = parse_manifest(toml).unwrap_err();
        assert!(err.to_string().contains("plugin.name is required"));
    }

    #[test]
    fn test_parse_manifest_missing_version() {
        let toml = r#"
[plugin]
id = "test"
name = "Test"
version = ""
"#;
        let err = parse_manifest(toml).unwrap_err();
        assert!(err.to_string().contains("plugin.version is required"));
    }

    #[test]
    fn test_parse_manifest_script_action_no_script() {
        let toml = r#"
[plugin]
id = "test"
name = "Test"
version = "1.0.0"

[[plugin.commands]]
id = "test.run"
label = "Run"
action = "script"
"#;
        let err = parse_manifest(toml).unwrap_err();
        assert!(err.to_string().contains("no script file specified"));
    }

    #[test]
    fn test_parse_manifest_unknown_action() {
        let toml = r#"
[plugin]
id = "test"
name = "Test"
version = "1.0.0"

[[plugin.commands]]
id = "test.run"
label = "Run"
action = "unknown_action"
"#;
        let err = parse_manifest(toml).unwrap_err();
        assert!(err.to_string().contains("unknown action type"));
    }

    #[test]
    fn test_parse_manifest_unknown_input() {
        let toml = r#"
[plugin]
id = "test"
name = "Test"
version = "1.0.0"

[[plugin.commands]]
id = "test.run"
label = "Run"
action = "script"
script = "run.sh"
input = "clipboard"
"#;
        let err = parse_manifest(toml).unwrap_err();
        assert!(err.to_string().contains("unknown input type"));
    }

    #[test]
    fn test_parse_manifest_unknown_output() {
        let toml = r#"
[plugin]
id = "test"
name = "Test"
version = "1.0.0"

[[plugin.commands]]
id = "test.run"
label = "Run"
action = "script"
script = "run.sh"
output = "clipboard"
"#;
        let err = parse_manifest(toml).unwrap_err();
        assert!(err.to_string().contains("unknown output type"));
    }

    #[test]
    fn test_parse_manifest_invalid_toml() {
        let toml = "this is not valid toml [[[";
        assert!(parse_manifest(toml).is_err());
    }

    #[test]
    fn test_parse_manifest_missing_plugin_table() {
        let toml = r#"
[something]
id = "test"
"#;
        assert!(parse_manifest(toml).is_err());
    }

    #[test]
    fn test_parse_manifest_command_missing_id() {
        let toml = r#"
[plugin]
id = "test"
name = "Test"
version = "1.0.0"

[[plugin.commands]]
id = ""
label = "Run"
action = "message"
"#;
        let err = parse_manifest(toml).unwrap_err();
        assert!(err.to_string().contains("command id is required"));
    }

    #[test]
    fn test_parse_manifest_command_missing_label() {
        let toml = r#"
[plugin]
id = "test"
name = "Test"
version = "1.0.0"

[[plugin.commands]]
id = "test.run"
label = ""
action = "message"
"#;
        let err = parse_manifest(toml).unwrap_err();
        assert!(err.to_string().contains("command label is required"));
    }

    #[test]
    fn test_script_plugin_info() {
        let manifest = parse_manifest(
            r#"
[plugin]
id = "test-script"
name = "Test Script"
version = "2.0.0"
description = "A test script plugin"
"#,
        )
        .unwrap();

        let plugin = ScriptPlugin::new(manifest, PathBuf::from("/tmp/test"));
        let info = plugin.info();
        assert_eq!(info.id, "test-script");
        assert_eq!(info.name, "Test Script");
        assert_eq!(info.version, "2.0.0");
        assert_eq!(info.description, "A test script plugin");
    }

    #[test]
    fn test_script_plugin_commands() {
        let manifest = parse_manifest(
            r#"
[plugin]
id = "test"
name = "Test"
version = "1.0.0"

[[plugin.commands]]
id = "test.greet"
label = "Greet"
shortcut = "Ctrl+Shift+G"
action = "script"
script = "greet.sh"

[[plugin.commands]]
id = "test.info"
label = "About"
action = "message"
"#,
        )
        .unwrap();

        let plugin = ScriptPlugin::new(manifest, PathBuf::from("/tmp/test"));
        let cmds = plugin.commands();
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].id, "test.greet");
        assert_eq!(cmds[0].label, "Greet");
        assert_eq!(cmds[0].shortcut.as_deref(), Some("Ctrl+Shift+G"));
        assert_eq!(cmds[1].id, "test.info");
        assert!(cmds[1].shortcut.is_none());
    }

    #[test]
    fn test_script_plugin_execute_message_action() {
        let manifest = parse_manifest(
            r#"
[plugin]
id = "test"
name = "Test Plugin"
version = "1.0.0"

[[plugin.commands]]
id = "test.info"
label = "About"
action = "message"
"#,
        )
        .unwrap();

        let mut plugin = ScriptPlugin::new(manifest, PathBuf::from("/tmp/test"));
        let ctx = PluginContext::default();
        let action = plugin.execute_command("test.info", &ctx);
        match action {
            PluginAction::ShowMessage(msg) => {
                assert!(msg.contains("Test Plugin"));
                assert!(msg.contains("1.0.0"));
            }
            _ => panic!("Expected ShowMessage"),
        }
    }

    #[test]
    fn test_script_plugin_execute_unknown_command() {
        let manifest = parse_manifest(
            r#"
[plugin]
id = "test"
name = "Test"
version = "1.0.0"
"#,
        )
        .unwrap();

        let mut plugin = ScriptPlugin::new(manifest, PathBuf::from("/tmp/test"));
        let ctx = PluginContext::default();
        let action = plugin.execute_command("nonexistent", &ctx);
        match action {
            PluginAction::None => {}
            _ => panic!("Expected None"),
        }
    }

    #[test]
    fn test_scan_plugins_empty_dir() {
        let dir = std::env::temp_dir().join("openedit_test_empty_plugins");
        let _ = std::fs::create_dir_all(&dir);
        let (plugins, errors) = scan_plugins_from_dir(Some(dir.clone()));
        assert!(plugins.is_empty());
        assert!(errors.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_plugins_nonexistent_dir() {
        let dir = PathBuf::from("/tmp/openedit_nonexistent_dir_12345");
        let (plugins, errors) = scan_plugins_from_dir(Some(dir));
        assert!(plugins.is_empty());
        assert!(errors.is_empty()); // non-existent is not an error, just empty
    }

    #[test]
    fn test_scan_plugins_none_dir() {
        let (plugins, errors) = scan_plugins_from_dir(None);
        assert!(plugins.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Could not determine"));
    }

    #[test]
    fn test_scan_plugins_with_valid_plugin() {
        let dir = std::env::temp_dir().join("openedit_test_scan_valid");
        let plugin_dir = dir.join("my-plugin");
        let _ = std::fs::create_dir_all(&plugin_dir);

        let manifest = r#"
[plugin]
id = "my-plugin"
name = "My Plugin"
version = "1.0.0"
description = "Test plugin"

[[plugin.commands]]
id = "my-plugin.hello"
label = "Say Hello"
action = "message"
"#;
        std::fs::write(plugin_dir.join("plugin.toml"), manifest).unwrap();

        let (plugins, errors) = scan_plugins_from_dir(Some(dir.clone()));
        assert_eq!(plugins.len(), 1);
        assert!(errors.is_empty());
        assert_eq!(plugins[0].manifest.plugin.id, "my-plugin");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_plugins_with_broken_manifest() {
        let dir = std::env::temp_dir().join("openedit_test_scan_broken");
        let plugin_dir = dir.join("broken-plugin");
        let _ = std::fs::create_dir_all(&plugin_dir);

        std::fs::write(plugin_dir.join("plugin.toml"), "not valid toml [[[").unwrap();

        let (plugins, errors) = scan_plugins_from_dir(Some(dir.clone()));
        assert!(plugins.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Failed to parse plugin"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_plugins_dir_without_manifest() {
        let dir = std::env::temp_dir().join("openedit_test_scan_no_manifest");
        let plugin_dir = dir.join("no-manifest-plugin");
        let _ = std::fs::create_dir_all(&plugin_dir);
        // Create a regular file, not a plugin.toml
        std::fs::write(plugin_dir.join("readme.txt"), "not a plugin").unwrap();

        let (plugins, errors) = scan_plugins_from_dir(Some(dir.clone()));
        assert!(plugins.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("no plugin.toml"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_plugins_from_dir() {
        let dir = std::env::temp_dir().join("openedit_test_load_plugins");
        let plugin_dir = dir.join("test-plugin");
        let _ = std::fs::create_dir_all(&plugin_dir);

        let manifest = r#"
[plugin]
id = "test-plugin"
name = "Test Plugin"
version = "1.0.0"

[[plugin.commands]]
id = "test-plugin.hello"
label = "Hello"
action = "message"
"#;
        std::fs::write(plugin_dir.join("plugin.toml"), manifest).unwrap();

        let mut mgr = crate::plugin::PluginManager::new();
        let warnings = load_plugins_from_dir(&mut mgr, &dir);
        assert!(warnings.is_empty());

        let list = mgr.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0.id, "test-plugin");
        assert!(list[0].1); // enabled by default

        let cmds = mgr.all_commands();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].1.id, "test-plugin.hello");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_script_plugin_execute_script() {
        // Create a temporary plugin with a real script
        let dir = std::env::temp_dir().join("openedit_test_script_exec");
        let _ = std::fs::create_dir_all(&dir);

        // Write a simple script that uppercases stdin
        let script = if cfg!(target_os = "windows") {
            // Skip on Windows
            let _ = std::fs::remove_dir_all(&dir);
            return;
        } else {
            "#!/bin/sh\ntr '[:lower:]' '[:upper:]'"
        };
        let script_path = dir.join("upper.sh");
        std::fs::write(&script_path, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let manifest = parse_manifest(
            r#"
[plugin]
id = "upper"
name = "Uppercase"
version = "1.0.0"

[[plugin.commands]]
id = "upper.run"
label = "Uppercase Selection"
action = "script"
script = "upper.sh"
input = "selection"
output = "replace_selection"
"#,
        )
        .unwrap();

        let mut plugin = ScriptPlugin::new(manifest, dir.clone());
        let ctx = PluginContext {
            active_text: Some("hello world, this is a test"),
            selection: Some((0, 11)), // "hello world"
            ..PluginContext::default()
        };

        let action = plugin.execute_command("upper.run", &ctx);
        match action {
            PluginAction::ReplaceSelection(text) => {
                assert_eq!(text, "HELLO WORLD");
            }
            other => panic!("Expected ReplaceSelection, got {:?}", other),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_script_plugin_execute_document_input() {
        let dir = std::env::temp_dir().join("openedit_test_script_doc_input");
        let _ = std::fs::create_dir_all(&dir);

        // Script that counts lines
        let script = if cfg!(target_os = "windows") {
            let _ = std::fs::remove_dir_all(&dir);
            return;
        } else {
            "#!/bin/sh\nwc -l | tr -d ' '"
        };
        let script_path = dir.join("count.sh");
        std::fs::write(&script_path, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let manifest = parse_manifest(
            r#"
[plugin]
id = "counter"
name = "Line Counter"
version = "1.0.0"

[[plugin.commands]]
id = "counter.count"
label = "Count Lines"
action = "script"
script = "count.sh"
input = "document"
output = "message"
"#,
        )
        .unwrap();

        let mut plugin = ScriptPlugin::new(manifest, dir.clone());
        let doc_text = "line1\nline2\nline3\n";
        let ctx = PluginContext {
            active_text: Some(doc_text),
            ..PluginContext::default()
        };

        let action = plugin.execute_command("counter.count", &ctx);
        match action {
            PluginAction::ShowMessage(msg) => {
                // wc -l should return 3 for 3 newlines
                assert_eq!(msg.trim(), "3");
            }
            other => panic!("Expected ShowMessage, got {:?}", other),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_script_plugin_missing_script_file() {
        let dir = std::env::temp_dir().join("openedit_test_script_missing");
        let _ = std::fs::create_dir_all(&dir);

        let manifest = parse_manifest(
            r#"
[plugin]
id = "missing"
name = "Missing Script"
version = "1.0.0"

[[plugin.commands]]
id = "missing.run"
label = "Run"
action = "script"
script = "nonexistent.sh"
input = "none"
output = "message"
"#,
        )
        .unwrap();

        let mut plugin = ScriptPlugin::new(manifest, dir.clone());
        let ctx = PluginContext::default();

        let action = plugin.execute_command("missing.run", &ctx);
        match action {
            PluginAction::ShowMessage(msg) => {
                assert!(msg.contains("error") || msg.contains("not found"));
            }
            _ => panic!("Expected ShowMessage with error"),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_plugin_states_serialize_deserialize() {
        let mut states = PluginStates::default();
        states.enabled.insert("plugin-a".to_string(), true);
        states.enabled.insert("plugin-b".to_string(), false);

        let json = serde_json::to_string(&states).unwrap();
        let loaded: PluginStates = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.enabled.get("plugin-a"), Some(&true));
        assert_eq!(loaded.enabled.get("plugin-b"), Some(&false));
    }

    #[test]
    fn test_plugin_states_save_and_load() {
        let dir = std::env::temp_dir().join("openedit_test_plugin_states");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("plugins.json");

        let mut states = PluginStates::default();
        states.enabled.insert("test-plugin".to_string(), false);

        let json = serde_json::to_string_pretty(&states).unwrap();
        std::fs::write(&path, &json).unwrap();

        let loaded: PluginStates =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.enabled.get("test-plugin"), Some(&false));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_parse_manifest_with_shortcut() {
        let toml = r#"
[plugin]
id = "test"
name = "Test"
version = "1.0.0"

[[plugin.commands]]
id = "test.run"
label = "Run"
shortcut = "Ctrl+Shift+R"
action = "message"
"#;
        let manifest = parse_manifest(toml).unwrap();
        assert_eq!(
            manifest.plugin.commands[0].shortcut.as_deref(),
            Some("Ctrl+Shift+R")
        );
    }

    #[test]
    fn test_parse_manifest_builtin_action() {
        let toml = r#"
[plugin]
id = "test"
name = "Test"
version = "1.0.0"

[[plugin.commands]]
id = "test.run"
label = "Run"
action = "builtin"
"#;
        let manifest = parse_manifest(toml).unwrap();
        assert_eq!(manifest.plugin.commands[0].action, "builtin");
    }

    #[test]
    fn test_multiple_plugins_load() {
        let dir = std::env::temp_dir().join("openedit_test_multi_load");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);

        // Plugin A
        let plugin_a_dir = dir.join("plugin-a");
        std::fs::create_dir_all(&plugin_a_dir).unwrap();
        std::fs::write(
            plugin_a_dir.join("plugin.toml"),
            r#"
[plugin]
id = "plugin-a"
name = "Plugin A"
version = "1.0.0"

[[plugin.commands]]
id = "plugin-a.greet"
label = "Greet A"
action = "message"
"#,
        )
        .unwrap();

        // Plugin B
        let plugin_b_dir = dir.join("plugin-b");
        std::fs::create_dir_all(&plugin_b_dir).unwrap();
        std::fs::write(
            plugin_b_dir.join("plugin.toml"),
            r#"
[plugin]
id = "plugin-b"
name = "Plugin B"
version = "2.0.0"

[[plugin.commands]]
id = "plugin-b.greet"
label = "Greet B"
action = "message"
"#,
        )
        .unwrap();

        let mut mgr = crate::plugin::PluginManager::new();
        let warnings = load_plugins_from_dir(&mut mgr, &dir);
        assert!(warnings.is_empty());

        let list = mgr.list();
        assert_eq!(list.len(), 2);

        let cmds = mgr.all_commands();
        assert_eq!(cmds.len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_skips_regular_files() {
        let dir = std::env::temp_dir().join("openedit_test_skip_files");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);

        // Create a regular file (not a directory) in the plugins dir
        std::fs::write(dir.join("not-a-plugin.txt"), "just a file").unwrap();

        let (plugins, errors) = scan_plugins_from_dir(Some(dir.clone()));
        assert!(plugins.is_empty());
        assert!(errors.is_empty()); // Regular files are silently skipped

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_script_plugin_insert_output() {
        let dir = std::env::temp_dir().join("openedit_test_insert_output");
        let _ = std::fs::create_dir_all(&dir);

        let script = "#!/bin/sh\necho 'inserted text'";
        let script_path = dir.join("insert.sh");
        std::fs::write(&script_path, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let manifest = parse_manifest(
            r#"
[plugin]
id = "insert-test"
name = "Insert Test"
version = "1.0.0"

[[plugin.commands]]
id = "insert-test.run"
label = "Insert"
action = "script"
script = "insert.sh"
input = "none"
output = "insert"
"#,
        )
        .unwrap();

        let mut plugin = ScriptPlugin::new(manifest, dir.clone());
        let ctx = PluginContext::default();

        let action = plugin.execute_command("insert-test.run", &ctx);
        match action {
            PluginAction::InsertAtCursor(text) => {
                assert_eq!(text.trim(), "inserted text");
            }
            other => panic!("Expected InsertAtCursor, got {:?}", other),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_script_plugin_replace_all_output() {
        let dir = std::env::temp_dir().join("openedit_test_replace_all");
        let _ = std::fs::create_dir_all(&dir);

        let script = "#!/bin/sh\necho 'new content'";
        let script_path = dir.join("replace.sh");
        std::fs::write(&script_path, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let manifest = parse_manifest(
            r#"
[plugin]
id = "replace-test"
name = "Replace Test"
version = "1.0.0"

[[plugin.commands]]
id = "replace-test.run"
label = "Replace All"
action = "script"
script = "replace.sh"
input = "document"
output = "replace_all"
"#,
        )
        .unwrap();

        let mut plugin = ScriptPlugin::new(manifest, dir.clone());
        let ctx = PluginContext {
            active_text: Some("old content"),
            ..PluginContext::default()
        };

        let action = plugin.execute_command("replace-test.run", &ctx);
        match action {
            PluginAction::ReplaceAll(text) => {
                assert_eq!(text.trim(), "new content");
            }
            other => panic!("Expected ReplaceAll, got {:?}", other),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
