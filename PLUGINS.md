# OpenEdit Plugin API

This document describes how to create plugins for the OpenEdit text editor.

## Overview

OpenEdit has a trait-based plugin system. Plugins implement the `Plugin` trait
from `openedit-core` and are registered with the `PluginManager` at startup.
A plugin can:

- React to editor events (file open, save, close, text changes, startup/shutdown)
- Register commands that appear in the Command Palette
- Execute text transformations and other editor actions
- Read information about the current editor state

## Creating a Plugin

### 1. Implement the `Plugin` trait

```rust
use openedit_core::plugin::*;

struct MyPlugin;

impl Plugin for MyPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "com.example.my-plugin".into(),
            name: "My Plugin".into(),
            version: "1.0.0".into(),
            description: "Does something useful".into(),
        }
    }

    fn on_load(&mut self) -> anyhow::Result<()> {
        // Initialization logic
        Ok(())
    }

    fn on_unload(&mut self) {
        // Cleanup logic
    }

    fn on_event(&mut self, event: &EditorEvent) {
        match event {
            EditorEvent::FileSaved(path) => {
                // React to file saves
            }
            _ => {}
        }
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![PluginCommand {
            id: "my-plugin.do-thing".into(),
            label: "My Plugin: Do Thing".into(),
            shortcut: None,
        }]
    }

    fn execute_command(&mut self, command_id: &str, ctx: &PluginContext) -> PluginAction {
        match command_id {
            "my-plugin.do-thing" => {
                if let Some(text) = ctx.selected_text {
                    PluginAction::ReplaceSelection(text.to_uppercase())
                } else {
                    PluginAction::ShowMessage("No text selected".into())
                }
            }
            _ => PluginAction::None,
        }
    }
}
```

### 2. Register the plugin

In your application startup code:

```rust
let mut manager = PluginManager::new();
manager.register(Box::new(MyPlugin))?;
```

## Plugin Context (PluginContext)

When a plugin command is executed, it receives a `PluginContext` with read-only
information about the current editor state:

| Field            | Type                   | Description                                    |
|------------------|------------------------|------------------------------------------------|
| `active_text`    | `Option<&str>`         | Full text of the active document               |
| `active_path`    | `Option<&str>`         | Full file path of the active document          |
| `selection`      | `Option<(usize,usize)>`| Selection range as char offsets (start, end)   |
| `selected_text`  | `Option<&str>`         | The actual selected text content               |
| `cursor_line`    | `usize`                | Current cursor line (0-based)                  |
| `cursor_col`     | `usize`                | Current cursor column (0-based)                |
| `language`       | `Option<&str>`         | Detected file language (e.g. "rust", "python") |
| `line_count`     | `usize`                | Total number of lines in the document          |
| `file_name`      | `Option<&str>`         | Just the filename (e.g. "main.rs")             |
| `is_modified`    | `bool`                 | Whether the document has unsaved changes       |
| `tab_count`      | `usize`                | Number of open tabs                            |
| `editor_version` | `&str`                 | Editor version string                          |

The context implements `Default` so you can create one easily in tests:

```rust
let ctx = PluginContext {
    cursor_line: 5,
    language: Some("rust"),
    ..PluginContext::default()
};
```

## Plugin Actions (PluginAction)

Commands return a `PluginAction` telling the editor what to do:

| Variant                       | Description                                        |
|-------------------------------|----------------------------------------------------|
| `None`                        | Do nothing                                         |
| `ReplaceSelection(String)`    | Replace the current selection with text             |
| `ReplaceAll(String)`          | Replace the entire document content                |
| `InsertAtCursor(String)`      | Insert text at the current cursor position         |
| `ShowMessage(String)`         | Show a notification/message dialog                 |
| `OpenFile(String)`            | Open a file path in a new editor tab               |
| `RunCommand(String)`          | Execute a built-in editor command by ID            |
| `SetStatusMessage(String)`    | Show a temporary message in the status bar         |
| `Multiple(Vec<PluginAction>)` | Execute several actions in sequence                |

### Example: Multiple actions

```rust
PluginAction::Multiple(vec![
    PluginAction::InsertAtCursor("// Generated header\n".into()),
    PluginAction::SetStatusMessage("Header inserted".into()),
])
```

## Editor Events (EditorEvent)

Plugins can react to these events via `on_event()`:

| Variant                                            | Description                  |
|----------------------------------------------------|------------------------------|
| `FileOpened(String)`                               | A file was opened            |
| `FileSaved(String)`                                | A file was saved             |
| `FileClosed(String)`                               | A file tab was closed        |
| `TabChanged(Option<String>)`                       | Active tab changed           |
| `TextInserted { line, col, text }`                 | Text was inserted            |
| `TextDeleted { line, col, len }`                   | Text was deleted             |
| `Startup`                                          | Editor is starting up        |
| `Shutdown`                                         | Editor is shutting down      |

## Plugin Management UI

Users can manage plugins through the Command Palette:

1. Press `Ctrl+Shift+P` to open the Command Palette
2. Type "Plugins" and select **Plugins: Manage Plugins**

The plugin management panel shows:
- All registered plugins with name, version, and description
- Enable/disable toggle for each plugin
- A "Reload Plugins" button
- An "Open Plugins Folder" button to access the plugins directory

## Plugin Types

### Built-in Rust Plugins

These are compiled directly into the editor binary. They implement the
`Plugin` trait and are registered at startup. This is the primary and
recommended plugin type for performance and safety.

### Future: Script-based Plugins (Planned)

A future version may support script-based plugins using a directory structure:

```
~/.config/openedit/plugins/
  my-plugin/
    plugin.toml        # Plugin manifest
    main.lua           # Plugin script (or other scripting language)
```

With a manifest format like:

```toml
[plugin]
id = "com.example.my-plugin"
name = "My Plugin"
version = "1.0.0"
description = "A useful plugin"
entry = "main.lua"
```

## Example: Word Count Plugin

A complete example showing a plugin that counts words in the selection or
entire document:

```rust
use openedit_core::plugin::*;

struct WordCountPlugin;

impl Plugin for WordCountPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "builtin.word-count".into(),
            name: "Word Count".into(),
            version: "1.0.0".into(),
            description: "Count words in selection or document".into(),
        }
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![PluginCommand {
            id: "word-count.count".into(),
            label: "Word Count: Count Words".into(),
            shortcut: None,
        }]
    }

    fn execute_command(&mut self, command_id: &str, ctx: &PluginContext) -> PluginAction {
        if command_id != "word-count.count" {
            return PluginAction::None;
        }

        let text = ctx.selected_text
            .or(ctx.active_text)
            .unwrap_or("");

        let word_count = text.split_whitespace().count();
        let char_count = text.chars().count();
        let line_count = if text.is_empty() { 0 } else { text.lines().count() };

        let scope = if ctx.selected_text.is_some() {
            "Selection"
        } else {
            "Document"
        };

        PluginAction::Multiple(vec![
            PluginAction::ShowMessage(format!(
                "{}: {} words, {} characters, {} lines",
                scope, word_count, char_count, line_count,
            )),
            PluginAction::SetStatusMessage(format!("{} words", word_count)),
        ])
    }
}
```

Register it:

```rust
manager.register(Box::new(WordCountPlugin))?;
```

Then open the Command Palette and search for "Word Count" to use it.
