use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Persistent editor configuration, stored as TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    #[serde(default)]
    pub editor: EditorSection,
    #[serde(default)]
    pub ui: UiSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSection {
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_tab_size")]
    pub tab_size: u32,
    #[serde(default)]
    pub word_wrap: bool,
    #[serde(default)]
    pub show_whitespace: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSection {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_show_minimap")]
    pub show_minimap: bool,
    #[serde(default)]
    pub show_sidebar: bool,
}

fn default_font_size() -> f32 {
    13.0
}

fn default_tab_size() -> u32 {
    4
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_show_minimap() -> bool {
    true
}

impl Default for EditorSection {
    fn default() -> Self {
        Self {
            font_size: default_font_size(),
            tab_size: default_tab_size(),
            word_wrap: false,
            show_whitespace: false,
        }
    }
}

impl Default for UiSection {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            show_minimap: default_show_minimap(),
            show_sidebar: false,
        }
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            editor: EditorSection::default(),
            ui: UiSection::default(),
        }
    }
}

/// Returns the path to `config.toml` in the platform-appropriate config directory.
///
/// - Linux: `$XDG_CONFIG_HOME/openedit/config.toml` or `$HOME/.config/openedit/config.toml`
/// - macOS: `$HOME/Library/Application Support/openedit/config.toml`
/// - Windows: `%APPDATA%\openedit\config.toml`
pub fn config_path() -> Option<PathBuf> {
    let config_dir = if cfg!(target_os = "windows") {
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
    config_dir.map(|d| d.join("openedit").join("config.toml"))
}

/// Load the configuration from disk. Returns the default config if the file
/// does not exist or cannot be parsed.
pub fn load_config() -> EditorConfig {
    let Some(path) = config_path() else {
        log::warn!("Could not determine config directory; using defaults");
        return EditorConfig::default();
    };

    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<EditorConfig>(&content) {
            Ok(config) => {
                log::info!("Loaded config from {}", path.display());
                config
            }
            Err(e) => {
                log::warn!("Failed to parse {}: {}; using defaults", path.display(), e);
                EditorConfig::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            log::info!("No config file at {}; using defaults", path.display());
            EditorConfig::default()
        }
        Err(e) => {
            log::warn!("Failed to read {}: {}; using defaults", path.display(), e);
            EditorConfig::default()
        }
    }
}

/// Save the configuration to disk. Silently logs errors on failure.
pub fn save_config(config: &EditorConfig) {
    let Some(path) = config_path() else {
        log::warn!("Could not determine config directory; config not saved");
        return;
    };

    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            log::error!(
                "Failed to create config directory {}: {}",
                parent.display(),
                e
            );
            return;
        }
    }

    match toml::to_string_pretty(config) {
        Ok(content) => {
            if let Err(e) = std::fs::write(&path, content) {
                log::error!("Failed to write config to {}: {}", path.display(), e);
            } else {
                log::info!("Saved config to {}", path.display());
            }
        }
        Err(e) => {
            log::error!("Failed to serialize config: {}", e);
        }
    }
}
