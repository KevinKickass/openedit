pub mod buffer;
pub mod cursor;
pub mod diff;
pub mod document;
pub mod edit;
pub mod encoding;
pub mod folding;
pub mod line_ending;
pub mod plugin;
pub mod plugin_loader;
pub mod search;
pub mod selection;
pub mod syntax;
pub mod undo;

pub use buffer::Buffer;
pub use cursor::{Cursor, Position};
pub use document::Document;
pub use encoding::Encoding;
pub use folding::FoldingState;
pub use line_ending::LineEnding;
pub use plugin::{Plugin, PluginManager, PluginAction, PluginContext, PluginCommand, PluginInfo, EditorEvent};
pub use plugin_loader::{
    PluginManifest, ScriptPlugin, PluginStates,
    parse_manifest, parse_manifest_file,
    scan_plugins, scan_plugins_from_dir,
    load_plugins, load_plugins_from_dir,
    save_plugin_states, plugins_dir, config_dir as plugin_config_dir,
};
pub use selection::Selection;
