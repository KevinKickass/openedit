use egui::Color32;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Parse a hex color string like "#RRGGBB" or "#RRGGBBAA" into a Color32.
/// Returns None if the string is not a valid hex color.
fn parse_hex_color(s: &str) -> Option<Color32> {
    let s = s.trim().strip_prefix('#')?;
    match s.len() {
        6 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some(Color32::from_rgb(r, g, b))
        }
        8 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            let a = u8::from_str_radix(&s[6..8], 16).ok()?;
            Some(Color32::from_rgba_premultiplied(r, g, b, a))
        }
        _ => None,
    }
}

/// Format a Color32 as a hex string "#RRGGBB" or "#RRGGBBAA" if alpha < 255.
fn color_to_hex(c: Color32) -> String {
    if c.a() == 255 {
        format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
    } else {
        format!("#{:02x}{:02x}{:02x}{:02x}", c.r(), c.g(), c.b(), c.a())
    }
}

// ── TOML theme file structs ──

/// TOML-serializable syntax color section.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyntaxColorsFile {
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub string: Option<String>,
    #[serde(default)]
    pub number: Option<String>,
    #[serde(default)]
    pub function: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub type_builtin: Option<String>,
    #[serde(default)]
    pub variable: Option<String>,
    #[serde(default)]
    pub variable_builtin: Option<String>,
    #[serde(default)]
    pub property: Option<String>,
    #[serde(default)]
    pub operator: Option<String>,
    #[serde(default)]
    pub punctuation: Option<String>,
    #[serde(default)]
    pub constant: Option<String>,
    #[serde(default)]
    pub constant_builtin: Option<String>,
    #[serde(default)]
    pub attribute: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub escape: Option<String>,
    #[serde(default)]
    pub function_macro: Option<String>,
}

/// TOML-serializable editor color section.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeColorsFile {
    #[serde(default)]
    pub background: Option<String>,
    #[serde(default)]
    pub foreground: Option<String>,
    #[serde(default)]
    pub gutter_bg: Option<String>,
    #[serde(default)]
    pub gutter_fg: Option<String>,
    #[serde(default)]
    pub current_line_bg: Option<String>,
    #[serde(default)]
    pub selection_bg: Option<String>,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub search_match_bg: Option<String>,
    #[serde(default)]
    pub search_current_match_bg: Option<String>,
    #[serde(default)]
    pub tab_active_bg: Option<String>,
    #[serde(default)]
    pub tab_inactive_bg: Option<String>,
    #[serde(default)]
    pub tab_text: Option<String>,
    #[serde(default)]
    pub status_bar_bg: Option<String>,
    #[serde(default)]
    pub status_bar_fg: Option<String>,
    #[serde(default)]
    pub modified_indicator: Option<String>,
}

/// A theme as stored in a TOML file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeFile {
    pub name: String,
    /// Optional base theme to inherit unset colors from (e.g. "Dark", "Light").
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default)]
    pub colors: ThemeColorsFile,
    #[serde(default)]
    pub syntax: SyntaxColorsFile,
}

impl ThemeFile {
    /// Convert this TOML theme file into an `EditorTheme`, falling back to the
    /// given base theme for any fields not specified.
    pub fn to_editor_theme(&self, base: &EditorTheme) -> EditorTheme {
        let c = &self.colors;
        let s = &self.syntax;

        let parse_or = |opt: &Option<String>, fallback: Color32| -> Color32 {
            opt.as_ref()
                .and_then(|s| parse_hex_color(s))
                .unwrap_or(fallback)
        };

        EditorTheme {
            name: self.name.clone(),
            background: parse_or(&c.background, base.background),
            foreground: parse_or(&c.foreground, base.foreground),
            gutter_bg: parse_or(&c.gutter_bg, base.gutter_bg),
            gutter_fg: parse_or(&c.gutter_fg, base.gutter_fg),
            current_line_bg: parse_or(&c.current_line_bg, base.current_line_bg),
            selection_bg: parse_or(&c.selection_bg, base.selection_bg),
            cursor_color: parse_or(&c.cursor, base.cursor_color),
            search_match_bg: parse_or(&c.search_match_bg, base.search_match_bg),
            search_current_match_bg: parse_or(
                &c.search_current_match_bg,
                base.search_current_match_bg,
            ),
            tab_active_bg: parse_or(&c.tab_active_bg, base.tab_active_bg),
            tab_inactive_bg: parse_or(&c.tab_inactive_bg, base.tab_inactive_bg),
            tab_text: parse_or(&c.tab_text, base.tab_text),
            status_bar_bg: parse_or(&c.status_bar_bg, base.status_bar_bg),
            status_bar_fg: parse_or(&c.status_bar_fg, base.status_bar_fg),
            modified_indicator: parse_or(&c.modified_indicator, base.modified_indicator),
            syntax_colors: SyntaxColors {
                keyword: parse_or(&s.keyword, base.syntax_colors.keyword),
                comment: parse_or(&s.comment, base.syntax_colors.comment),
                string: parse_or(&s.string, base.syntax_colors.string),
                number: parse_or(&s.number, base.syntax_colors.number),
                function: parse_or(&s.function, base.syntax_colors.function),
                r#type: parse_or(&s.r#type, base.syntax_colors.r#type),
                type_builtin: parse_or(&s.type_builtin, base.syntax_colors.type_builtin),
                variable: parse_or(&s.variable, base.syntax_colors.variable),
                variable_builtin: parse_or(
                    &s.variable_builtin,
                    base.syntax_colors.variable_builtin,
                ),
                property: parse_or(&s.property, base.syntax_colors.property),
                operator: parse_or(&s.operator, base.syntax_colors.operator),
                punctuation: parse_or(&s.punctuation, base.syntax_colors.punctuation),
                constant: parse_or(&s.constant, base.syntax_colors.constant),
                constant_builtin: parse_or(
                    &s.constant_builtin,
                    base.syntax_colors.constant_builtin,
                ),
                attribute: parse_or(&s.attribute, base.syntax_colors.attribute),
                tag: parse_or(&s.tag, base.syntax_colors.tag),
                escape: parse_or(&s.escape, base.syntax_colors.escape),
                function_macro: parse_or(&s.function_macro, base.syntax_colors.function_macro),
            },
        }
    }

    /// Create a `ThemeFile` from an existing `EditorTheme` (for export).
    pub fn from_editor_theme(theme: &EditorTheme) -> Self {
        let sc = &theme.syntax_colors;
        ThemeFile {
            name: theme.name.clone(),
            base: None,
            colors: ThemeColorsFile {
                background: Some(color_to_hex(theme.background)),
                foreground: Some(color_to_hex(theme.foreground)),
                gutter_bg: Some(color_to_hex(theme.gutter_bg)),
                gutter_fg: Some(color_to_hex(theme.gutter_fg)),
                current_line_bg: Some(color_to_hex(theme.current_line_bg)),
                selection_bg: Some(color_to_hex(theme.selection_bg)),
                cursor: Some(color_to_hex(theme.cursor_color)),
                search_match_bg: Some(color_to_hex(theme.search_match_bg)),
                search_current_match_bg: Some(color_to_hex(theme.search_current_match_bg)),
                tab_active_bg: Some(color_to_hex(theme.tab_active_bg)),
                tab_inactive_bg: Some(color_to_hex(theme.tab_inactive_bg)),
                tab_text: Some(color_to_hex(theme.tab_text)),
                status_bar_bg: Some(color_to_hex(theme.status_bar_bg)),
                status_bar_fg: Some(color_to_hex(theme.status_bar_fg)),
                modified_indicator: Some(color_to_hex(theme.modified_indicator)),
            },
            syntax: SyntaxColorsFile {
                keyword: Some(color_to_hex(sc.keyword)),
                comment: Some(color_to_hex(sc.comment)),
                string: Some(color_to_hex(sc.string)),
                number: Some(color_to_hex(sc.number)),
                function: Some(color_to_hex(sc.function)),
                r#type: Some(color_to_hex(sc.r#type)),
                type_builtin: Some(color_to_hex(sc.type_builtin)),
                variable: Some(color_to_hex(sc.variable)),
                variable_builtin: Some(color_to_hex(sc.variable_builtin)),
                property: Some(color_to_hex(sc.property)),
                operator: Some(color_to_hex(sc.operator)),
                punctuation: Some(color_to_hex(sc.punctuation)),
                constant: Some(color_to_hex(sc.constant)),
                constant_builtin: Some(color_to_hex(sc.constant_builtin)),
                attribute: Some(color_to_hex(sc.attribute)),
                tag: Some(color_to_hex(sc.tag)),
                escape: Some(color_to_hex(sc.escape)),
                function_macro: Some(color_to_hex(sc.function_macro)),
            },
        }
    }
}

// ── Theme Registry ──

/// Manages all available themes (built-in + user-loaded).
pub struct ThemeRegistry {
    /// All available themes, both built-in and user-loaded.
    themes: Vec<EditorTheme>,
    /// Names of themes loaded from user TOML files (for distinguishing from built-in).
    user_theme_names: Vec<String>,
}

impl ThemeRegistry {
    /// Create a new registry with all built-in themes, then scan the user themes directory.
    pub fn new() -> Self {
        let mut registry = Self {
            themes: Self::built_in_themes(),
            user_theme_names: Vec::new(),
        };
        registry.load_user_themes();
        registry
    }

    /// All the built-in themes.
    fn built_in_themes() -> Vec<EditorTheme> {
        vec![
            EditorTheme::dark(),
            EditorTheme::light(),
            EditorTheme::monokai(),
            EditorTheme::dracula(),
            EditorTheme::solarized_dark(),
            EditorTheme::solarized_light(),
            EditorTheme::nord(),
            EditorTheme::one_dark(),
            EditorTheme::gruvbox(),
            EditorTheme::tokyo_night(),
        ]
    }

    /// Returns the path to the user themes directory.
    pub fn themes_dir() -> Option<PathBuf> {
        let config_dir = if cfg!(target_os = "windows") {
            std::env::var("APPDATA").ok().map(PathBuf::from)
        } else if cfg!(target_os = "macos") {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
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
        config_dir.map(|d| d.join("openedit").join("themes"))
    }

    /// Ensure the themes directory exists, creating it if necessary.
    /// Returns the path on success.
    pub fn ensure_themes_dir() -> Option<PathBuf> {
        let dir = Self::themes_dir()?;
        if !dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&dir) {
                log::error!("Failed to create themes directory {}: {}", dir.display(), e);
                return None;
            }
        }
        Some(dir)
    }

    /// Scan the user themes directory and load all .toml theme files.
    /// User themes with the same name as a built-in theme will override it.
    pub fn load_user_themes(&mut self) {
        self.user_theme_names.clear();

        let Some(dir) = Self::themes_dir() else {
            return;
        };

        if !dir.exists() {
            return;
        }

        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) => {
                log::warn!("Failed to read themes directory {}: {}", dir.display(), e);
                return;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }

            match Self::load_theme_file(&path) {
                Ok(theme) => {
                    log::info!("Loaded user theme '{}' from {}", theme.name, path.display());
                    self.user_theme_names.push(theme.name.clone());

                    // If a built-in theme has the same name, replace it; otherwise add.
                    if let Some(pos) = self.themes.iter().position(|t| t.name == theme.name) {
                        self.themes[pos] = theme;
                    } else {
                        self.themes.push(theme);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to load theme from {}: {}", path.display(), e);
                }
            }
        }
    }

    /// Load and parse a single TOML theme file.
    fn load_theme_file(path: &std::path::Path) -> Result<EditorTheme, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("read error: {}", e))?;

        let theme_file: ThemeFile =
            toml::from_str(&content).map_err(|e| format!("parse error: {}", e))?;

        // Determine the base theme (default is Dark).
        let base_name = theme_file.base.as_deref().unwrap_or("Dark");
        let base = EditorTheme::by_name(base_name);

        Ok(theme_file.to_editor_theme(&base))
    }

    /// Reload all user themes from disk (re-scan the themes directory).
    pub fn reload(&mut self) {
        // Reset to built-in themes, then re-add user themes.
        self.themes = Self::built_in_themes();
        self.load_user_themes();
    }

    /// Get a theme by name from the registry.
    /// Falls back to the built-in `by_name` if not found in the registry.
    pub fn get(&self, name: &str) -> EditorTheme {
        if let Some(t) = self.themes.iter().find(|t| t.name == name) {
            return t.clone();
        }
        // Also check config-name format (lowercase with underscores)
        if let Some(t) = self.themes.iter().find(|t| t.config_name() == name) {
            return t.clone();
        }
        EditorTheme::by_name(name)
    }

    /// All available theme names (both built-in and user).
    pub fn all_names(&self) -> Vec<String> {
        self.themes.iter().map(|t| t.name.clone()).collect()
    }

    /// Whether a given theme is user-defined (loaded from TOML).
    pub fn is_user_theme(&self, name: &str) -> bool {
        self.user_theme_names.iter().any(|n| n == name)
    }

    /// Export the given theme as a TOML file in the themes directory.
    /// Returns the path written, or an error message.
    pub fn export_theme(theme: &EditorTheme) -> Result<PathBuf, String> {
        let dir = Self::ensure_themes_dir()
            .ok_or_else(|| "Could not determine themes directory".to_string())?;

        let theme_file = ThemeFile::from_editor_theme(theme);
        let toml_content =
            toml::to_string_pretty(&theme_file).map_err(|e| format!("serialize error: {}", e))?;

        // Generate a filename from the theme name.
        let filename = theme
            .name
            .to_lowercase()
            .replace(' ', "_")
            .replace(|c: char| !c.is_alphanumeric() && c != '_', "");
        let path = dir.join(format!("{}.toml", filename));

        std::fs::write(&path, toml_content).map_err(|e| format!("write error: {}", e))?;

        log::info!("Exported theme '{}' to {}", theme.name, path.display());
        Ok(path)
    }

    /// Open the themes directory in the system file manager.
    pub fn open_themes_folder() {
        let Some(dir) = Self::ensure_themes_dir() else {
            log::warn!("Could not determine themes directory");
            return;
        };

        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(&dir).spawn();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer").arg(&dir).spawn();
        }
    }
}

impl Default for ThemeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Editor color theme.
#[derive(Debug, Clone)]
pub struct EditorTheme {
    pub name: String,
    pub background: Color32,
    pub foreground: Color32,
    pub gutter_bg: Color32,
    pub gutter_fg: Color32,
    pub current_line_bg: Color32,
    pub selection_bg: Color32,
    pub cursor_color: Color32,
    pub search_match_bg: Color32,
    pub search_current_match_bg: Color32,
    pub tab_active_bg: Color32,
    pub tab_inactive_bg: Color32,
    pub tab_text: Color32,
    pub status_bar_bg: Color32,
    pub status_bar_fg: Color32,
    pub modified_indicator: Color32,
    /// Syntax highlight colors indexed by highlight name.
    pub syntax_colors: SyntaxColors,
}

/// Colors for syntax highlighting.
#[derive(Debug, Clone)]
pub struct SyntaxColors {
    pub keyword: Color32,
    pub comment: Color32,
    pub string: Color32,
    pub number: Color32,
    pub function: Color32,
    pub r#type: Color32,
    pub type_builtin: Color32,
    pub variable: Color32,
    pub variable_builtin: Color32,
    pub property: Color32,
    pub operator: Color32,
    pub punctuation: Color32,
    pub constant: Color32,
    pub constant_builtin: Color32,
    pub attribute: Color32,
    pub tag: Color32,
    pub escape: Color32,
    pub function_macro: Color32,
}

impl SyntaxColors {
    pub fn dark() -> Self {
        Self {
            keyword: Color32::from_rgb(197, 134, 192),
            comment: Color32::from_rgb(106, 153, 85),
            string: Color32::from_rgb(206, 145, 120),
            number: Color32::from_rgb(181, 206, 168),
            function: Color32::from_rgb(220, 220, 170),
            r#type: Color32::from_rgb(78, 201, 176),
            type_builtin: Color32::from_rgb(78, 201, 176),
            variable: Color32::from_rgb(156, 220, 254),
            variable_builtin: Color32::from_rgb(86, 156, 214),
            property: Color32::from_rgb(156, 220, 254),
            operator: Color32::from_rgb(212, 212, 212),
            punctuation: Color32::from_rgb(212, 212, 212),
            constant: Color32::from_rgb(100, 150, 224),
            constant_builtin: Color32::from_rgb(86, 156, 214),
            attribute: Color32::from_rgb(220, 220, 170),
            tag: Color32::from_rgb(86, 156, 214),
            escape: Color32::from_rgb(215, 186, 125),
            function_macro: Color32::from_rgb(220, 220, 170),
        }
    }

    pub fn light() -> Self {
        Self {
            keyword: Color32::from_rgb(175, 0, 219),
            comment: Color32::from_rgb(0, 128, 0),
            string: Color32::from_rgb(163, 21, 21),
            number: Color32::from_rgb(9, 134, 88),
            function: Color32::from_rgb(121, 94, 38),
            r#type: Color32::from_rgb(38, 127, 153),
            type_builtin: Color32::from_rgb(38, 127, 153),
            variable: Color32::from_rgb(0, 16, 128),
            variable_builtin: Color32::from_rgb(0, 0, 255),
            property: Color32::from_rgb(0, 16, 128),
            operator: Color32::from_rgb(0, 0, 0),
            punctuation: Color32::from_rgb(0, 0, 0),
            constant: Color32::from_rgb(0, 0, 255),
            constant_builtin: Color32::from_rgb(0, 0, 255),
            attribute: Color32::from_rgb(121, 94, 38),
            tag: Color32::from_rgb(128, 0, 0),
            escape: Color32::from_rgb(238, 160, 0),
            function_macro: Color32::from_rgb(121, 94, 38),
        }
    }

    /// Get color for a highlight index (matching HIGHLIGHT_NAMES order in syntax.rs).
    pub fn color_for_highlight(&self, idx: usize) -> Option<Color32> {
        match idx {
            0 => Some(self.attribute),
            1 => Some(self.constant_builtin),
            2 => Some(self.comment),
            3 => Some(self.constant),
            4 => Some(self.constant_builtin),
            5 => Some(self.r#type),
            6 => Some(self.string),
            7 => Some(self.escape),
            8 => Some(self.function),
            9 => Some(self.function),
            10 => Some(self.function_macro),
            11 => Some(self.keyword),
            12 => Some(self.variable),
            13 => Some(self.variable),
            14 => Some(self.number),
            15 => Some(self.operator),
            16 => Some(self.property),
            17 => Some(self.punctuation),
            18 => Some(self.punctuation),
            19 => Some(self.punctuation),
            20 => Some(self.string),
            21 => Some(self.escape),
            22 => Some(self.string),
            23 => Some(self.tag),
            24 => Some(self.r#type),
            25 => Some(self.type_builtin),
            26 => Some(self.variable),
            27 => Some(self.variable_builtin),
            28 => Some(self.variable),
            _ => None,
        }
    }
}

impl EditorTheme {
    pub fn dark() -> Self {
        Self {
            name: "Dark".to_string(),
            background: Color32::from_rgb(30, 30, 30),
            foreground: Color32::from_rgb(212, 212, 212),
            gutter_bg: Color32::from_rgb(30, 30, 30),
            gutter_fg: Color32::from_rgb(110, 110, 110),
            current_line_bg: Color32::from_rgb(40, 40, 40),
            selection_bg: Color32::from_rgba_premultiplied(70, 130, 180, 100),
            cursor_color: Color32::from_rgb(212, 212, 212),
            search_match_bg: Color32::from_rgba_premultiplied(180, 150, 50, 80),
            search_current_match_bg: Color32::from_rgba_premultiplied(220, 180, 50, 120),
            tab_active_bg: Color32::from_rgb(30, 30, 30),
            tab_inactive_bg: Color32::from_rgb(45, 45, 45),
            tab_text: Color32::from_rgb(200, 200, 200),
            status_bar_bg: Color32::from_rgb(0, 122, 204),
            status_bar_fg: Color32::WHITE,
            modified_indicator: Color32::from_rgb(255, 255, 255),
            syntax_colors: SyntaxColors::dark(),
        }
    }

    pub fn light() -> Self {
        Self {
            name: "Light".to_string(),
            background: Color32::from_rgb(255, 255, 255),
            foreground: Color32::from_rgb(0, 0, 0),
            gutter_bg: Color32::from_rgb(245, 245, 245),
            gutter_fg: Color32::from_rgb(140, 140, 140),
            current_line_bg: Color32::from_rgb(255, 255, 228),
            selection_bg: Color32::from_rgba_premultiplied(173, 214, 255, 100),
            cursor_color: Color32::from_rgb(0, 0, 0),
            search_match_bg: Color32::from_rgba_premultiplied(255, 200, 0, 80),
            search_current_match_bg: Color32::from_rgba_premultiplied(255, 200, 0, 140),
            tab_active_bg: Color32::from_rgb(255, 255, 255),
            tab_inactive_bg: Color32::from_rgb(236, 236, 236),
            tab_text: Color32::from_rgb(60, 60, 60),
            status_bar_bg: Color32::from_rgb(0, 122, 204),
            status_bar_fg: Color32::WHITE,
            modified_indicator: Color32::from_rgb(0, 0, 0),
            syntax_colors: SyntaxColors::light(),
        }
    }

    pub fn monokai() -> Self {
        Self {
            name: "Monokai".to_string(),
            background: Color32::from_rgb(39, 40, 34),
            foreground: Color32::from_rgb(248, 248, 242),
            gutter_bg: Color32::from_rgb(39, 40, 34),
            gutter_fg: Color32::from_rgb(144, 144, 138),
            current_line_bg: Color32::from_rgb(62, 61, 50),
            selection_bg: Color32::from_rgba_premultiplied(73, 72, 62, 160),
            cursor_color: Color32::from_rgb(248, 248, 240),
            search_match_bg: Color32::from_rgba_premultiplied(225, 200, 50, 80),
            search_current_match_bg: Color32::from_rgba_premultiplied(225, 200, 50, 140),
            tab_active_bg: Color32::from_rgb(39, 40, 34),
            tab_inactive_bg: Color32::from_rgb(30, 31, 28),
            tab_text: Color32::from_rgb(248, 248, 242),
            status_bar_bg: Color32::from_rgb(73, 72, 62),
            status_bar_fg: Color32::from_rgb(248, 248, 242),
            modified_indicator: Color32::from_rgb(166, 226, 46),
            syntax_colors: SyntaxColors {
                keyword: Color32::from_rgb(249, 38, 114),
                comment: Color32::from_rgb(117, 113, 94),
                string: Color32::from_rgb(230, 219, 116),
                number: Color32::from_rgb(174, 129, 255),
                function: Color32::from_rgb(166, 226, 46),
                r#type: Color32::from_rgb(102, 217, 239),
                type_builtin: Color32::from_rgb(102, 217, 239),
                variable: Color32::from_rgb(248, 248, 242),
                variable_builtin: Color32::from_rgb(253, 151, 31),
                property: Color32::from_rgb(248, 248, 242),
                operator: Color32::from_rgb(249, 38, 114),
                punctuation: Color32::from_rgb(248, 248, 242),
                constant: Color32::from_rgb(174, 129, 255),
                constant_builtin: Color32::from_rgb(174, 129, 255),
                attribute: Color32::from_rgb(166, 226, 46),
                tag: Color32::from_rgb(249, 38, 114),
                escape: Color32::from_rgb(174, 129, 255),
                function_macro: Color32::from_rgb(166, 226, 46),
            },
        }
    }

    pub fn dracula() -> Self {
        Self {
            name: "Dracula".to_string(),
            background: Color32::from_rgb(40, 42, 54),
            foreground: Color32::from_rgb(248, 248, 242),
            gutter_bg: Color32::from_rgb(40, 42, 54),
            gutter_fg: Color32::from_rgb(110, 114, 131),
            current_line_bg: Color32::from_rgb(68, 71, 90),
            selection_bg: Color32::from_rgba_premultiplied(68, 71, 90, 200),
            cursor_color: Color32::from_rgb(248, 248, 242),
            search_match_bg: Color32::from_rgba_premultiplied(241, 250, 140, 80),
            search_current_match_bg: Color32::from_rgba_premultiplied(241, 250, 140, 140),
            tab_active_bg: Color32::from_rgb(68, 71, 90),
            tab_inactive_bg: Color32::from_rgb(33, 34, 44),
            tab_text: Color32::from_rgb(248, 248, 242),
            status_bar_bg: Color32::from_rgb(98, 114, 164),
            status_bar_fg: Color32::from_rgb(248, 248, 242),
            modified_indicator: Color32::from_rgb(80, 250, 123),
            syntax_colors: SyntaxColors {
                keyword: Color32::from_rgb(255, 121, 198),
                comment: Color32::from_rgb(98, 114, 164),
                string: Color32::from_rgb(241, 250, 140),
                number: Color32::from_rgb(189, 147, 249),
                function: Color32::from_rgb(80, 250, 123),
                r#type: Color32::from_rgb(139, 233, 253),
                type_builtin: Color32::from_rgb(139, 233, 253),
                variable: Color32::from_rgb(248, 248, 242),
                variable_builtin: Color32::from_rgb(255, 184, 108),
                property: Color32::from_rgb(248, 248, 242),
                operator: Color32::from_rgb(255, 121, 198),
                punctuation: Color32::from_rgb(248, 248, 242),
                constant: Color32::from_rgb(189, 147, 249),
                constant_builtin: Color32::from_rgb(189, 147, 249),
                attribute: Color32::from_rgb(80, 250, 123),
                tag: Color32::from_rgb(255, 121, 198),
                escape: Color32::from_rgb(255, 184, 108),
                function_macro: Color32::from_rgb(80, 250, 123),
            },
        }
    }

    pub fn solarized_dark() -> Self {
        Self {
            name: "Solarized Dark".to_string(),
            background: Color32::from_rgb(0, 43, 54),
            foreground: Color32::from_rgb(131, 148, 150),
            gutter_bg: Color32::from_rgb(0, 43, 54),
            gutter_fg: Color32::from_rgb(88, 110, 117),
            current_line_bg: Color32::from_rgb(7, 54, 66),
            selection_bg: Color32::from_rgba_premultiplied(7, 54, 66, 200),
            cursor_color: Color32::from_rgb(131, 148, 150),
            search_match_bg: Color32::from_rgba_premultiplied(181, 137, 0, 80),
            search_current_match_bg: Color32::from_rgba_premultiplied(181, 137, 0, 140),
            tab_active_bg: Color32::from_rgb(7, 54, 66),
            tab_inactive_bg: Color32::from_rgb(0, 43, 54),
            tab_text: Color32::from_rgb(147, 161, 161),
            status_bar_bg: Color32::from_rgb(7, 54, 66),
            status_bar_fg: Color32::from_rgb(147, 161, 161),
            modified_indicator: Color32::from_rgb(181, 137, 0),
            syntax_colors: SyntaxColors {
                keyword: Color32::from_rgb(133, 153, 0),
                comment: Color32::from_rgb(88, 110, 117),
                string: Color32::from_rgb(42, 161, 152),
                number: Color32::from_rgb(211, 54, 130),
                function: Color32::from_rgb(38, 139, 210),
                r#type: Color32::from_rgb(181, 137, 0),
                type_builtin: Color32::from_rgb(181, 137, 0),
                variable: Color32::from_rgb(131, 148, 150),
                variable_builtin: Color32::from_rgb(203, 75, 22),
                property: Color32::from_rgb(131, 148, 150),
                operator: Color32::from_rgb(133, 153, 0),
                punctuation: Color32::from_rgb(131, 148, 150),
                constant: Color32::from_rgb(42, 161, 152),
                constant_builtin: Color32::from_rgb(211, 54, 130),
                attribute: Color32::from_rgb(108, 113, 196),
                tag: Color32::from_rgb(38, 139, 210),
                escape: Color32::from_rgb(203, 75, 22),
                function_macro: Color32::from_rgb(38, 139, 210),
            },
        }
    }

    pub fn solarized_light() -> Self {
        Self {
            name: "Solarized Light".to_string(),
            background: Color32::from_rgb(253, 246, 227),
            foreground: Color32::from_rgb(101, 123, 131),
            gutter_bg: Color32::from_rgb(238, 232, 213),
            gutter_fg: Color32::from_rgb(147, 161, 161),
            current_line_bg: Color32::from_rgb(238, 232, 213),
            selection_bg: Color32::from_rgba_premultiplied(238, 232, 213, 200),
            cursor_color: Color32::from_rgb(101, 123, 131),
            search_match_bg: Color32::from_rgba_premultiplied(181, 137, 0, 80),
            search_current_match_bg: Color32::from_rgba_premultiplied(181, 137, 0, 140),
            tab_active_bg: Color32::from_rgb(253, 246, 227),
            tab_inactive_bg: Color32::from_rgb(238, 232, 213),
            tab_text: Color32::from_rgb(88, 110, 117),
            status_bar_bg: Color32::from_rgb(238, 232, 213),
            status_bar_fg: Color32::from_rgb(88, 110, 117),
            modified_indicator: Color32::from_rgb(181, 137, 0),
            syntax_colors: SyntaxColors {
                keyword: Color32::from_rgb(133, 153, 0),
                comment: Color32::from_rgb(147, 161, 161),
                string: Color32::from_rgb(42, 161, 152),
                number: Color32::from_rgb(211, 54, 130),
                function: Color32::from_rgb(38, 139, 210),
                r#type: Color32::from_rgb(181, 137, 0),
                type_builtin: Color32::from_rgb(181, 137, 0),
                variable: Color32::from_rgb(101, 123, 131),
                variable_builtin: Color32::from_rgb(203, 75, 22),
                property: Color32::from_rgb(101, 123, 131),
                operator: Color32::from_rgb(133, 153, 0),
                punctuation: Color32::from_rgb(101, 123, 131),
                constant: Color32::from_rgb(42, 161, 152),
                constant_builtin: Color32::from_rgb(211, 54, 130),
                attribute: Color32::from_rgb(108, 113, 196),
                tag: Color32::from_rgb(38, 139, 210),
                escape: Color32::from_rgb(203, 75, 22),
                function_macro: Color32::from_rgb(38, 139, 210),
            },
        }
    }

    pub fn nord() -> Self {
        Self {
            name: "Nord".to_string(),
            background: Color32::from_rgb(46, 52, 64),
            foreground: Color32::from_rgb(216, 222, 233),
            gutter_bg: Color32::from_rgb(46, 52, 64),
            gutter_fg: Color32::from_rgb(76, 86, 106),
            current_line_bg: Color32::from_rgb(59, 66, 82),
            selection_bg: Color32::from_rgba_premultiplied(67, 76, 94, 200),
            cursor_color: Color32::from_rgb(216, 222, 233),
            search_match_bg: Color32::from_rgba_premultiplied(235, 203, 139, 80),
            search_current_match_bg: Color32::from_rgba_premultiplied(235, 203, 139, 140),
            tab_active_bg: Color32::from_rgb(59, 66, 82),
            tab_inactive_bg: Color32::from_rgb(46, 52, 64),
            tab_text: Color32::from_rgb(216, 222, 233),
            status_bar_bg: Color32::from_rgb(59, 66, 82),
            status_bar_fg: Color32::from_rgb(216, 222, 233),
            modified_indicator: Color32::from_rgb(163, 190, 140),
            syntax_colors: SyntaxColors {
                keyword: Color32::from_rgb(129, 161, 193),
                comment: Color32::from_rgb(97, 110, 136),
                string: Color32::from_rgb(163, 190, 140),
                number: Color32::from_rgb(180, 142, 173),
                function: Color32::from_rgb(136, 192, 208),
                r#type: Color32::from_rgb(235, 203, 139),
                type_builtin: Color32::from_rgb(235, 203, 139),
                variable: Color32::from_rgb(216, 222, 233),
                variable_builtin: Color32::from_rgb(208, 135, 112),
                property: Color32::from_rgb(216, 222, 233),
                operator: Color32::from_rgb(129, 161, 193),
                punctuation: Color32::from_rgb(216, 222, 233),
                constant: Color32::from_rgb(180, 142, 173),
                constant_builtin: Color32::from_rgb(180, 142, 173),
                attribute: Color32::from_rgb(143, 188, 187),
                tag: Color32::from_rgb(129, 161, 193),
                escape: Color32::from_rgb(208, 135, 112),
                function_macro: Color32::from_rgb(136, 192, 208),
            },
        }
    }

    pub fn one_dark() -> Self {
        Self {
            name: "One Dark".to_string(),
            background: Color32::from_rgb(40, 44, 52),
            foreground: Color32::from_rgb(171, 178, 191),
            gutter_bg: Color32::from_rgb(40, 44, 52),
            gutter_fg: Color32::from_rgb(76, 82, 99),
            current_line_bg: Color32::from_rgb(44, 49, 58),
            selection_bg: Color32::from_rgba_premultiplied(62, 68, 81, 200),
            cursor_color: Color32::from_rgb(82, 139, 255),
            search_match_bg: Color32::from_rgba_premultiplied(229, 192, 123, 80),
            search_current_match_bg: Color32::from_rgba_premultiplied(229, 192, 123, 140),
            tab_active_bg: Color32::from_rgb(40, 44, 52),
            tab_inactive_bg: Color32::from_rgb(33, 37, 43),
            tab_text: Color32::from_rgb(171, 178, 191),
            status_bar_bg: Color32::from_rgb(33, 37, 43),
            status_bar_fg: Color32::from_rgb(171, 178, 191),
            modified_indicator: Color32::from_rgb(152, 195, 121),
            syntax_colors: SyntaxColors {
                keyword: Color32::from_rgb(198, 120, 221),
                comment: Color32::from_rgb(92, 99, 112),
                string: Color32::from_rgb(152, 195, 121),
                number: Color32::from_rgb(209, 154, 102),
                function: Color32::from_rgb(97, 175, 239),
                r#type: Color32::from_rgb(229, 192, 123),
                type_builtin: Color32::from_rgb(229, 192, 123),
                variable: Color32::from_rgb(224, 108, 117),
                variable_builtin: Color32::from_rgb(224, 108, 117),
                property: Color32::from_rgb(224, 108, 117),
                operator: Color32::from_rgb(86, 182, 194),
                punctuation: Color32::from_rgb(171, 178, 191),
                constant: Color32::from_rgb(209, 154, 102),
                constant_builtin: Color32::from_rgb(209, 154, 102),
                attribute: Color32::from_rgb(209, 154, 102),
                tag: Color32::from_rgb(224, 108, 117),
                escape: Color32::from_rgb(86, 182, 194),
                function_macro: Color32::from_rgb(97, 175, 239),
            },
        }
    }

    pub fn gruvbox() -> Self {
        Self {
            name: "Gruvbox".to_string(),
            background: Color32::from_rgb(40, 40, 40),
            foreground: Color32::from_rgb(235, 219, 178),
            gutter_bg: Color32::from_rgb(40, 40, 40),
            gutter_fg: Color32::from_rgb(124, 111, 100),
            current_line_bg: Color32::from_rgb(50, 48, 47),
            selection_bg: Color32::from_rgba_premultiplied(80, 73, 69, 200),
            cursor_color: Color32::from_rgb(235, 219, 178),
            search_match_bg: Color32::from_rgba_premultiplied(250, 189, 47, 80),
            search_current_match_bg: Color32::from_rgba_premultiplied(250, 189, 47, 140),
            tab_active_bg: Color32::from_rgb(50, 48, 47),
            tab_inactive_bg: Color32::from_rgb(40, 40, 40),
            tab_text: Color32::from_rgb(235, 219, 178),
            status_bar_bg: Color32::from_rgb(80, 73, 69),
            status_bar_fg: Color32::from_rgb(235, 219, 178),
            modified_indicator: Color32::from_rgb(184, 187, 38),
            syntax_colors: SyntaxColors {
                keyword: Color32::from_rgb(251, 73, 52),
                comment: Color32::from_rgb(146, 131, 116),
                string: Color32::from_rgb(184, 187, 38),
                number: Color32::from_rgb(211, 134, 155),
                function: Color32::from_rgb(184, 187, 38),
                r#type: Color32::from_rgb(250, 189, 47),
                type_builtin: Color32::from_rgb(250, 189, 47),
                variable: Color32::from_rgb(131, 165, 152),
                variable_builtin: Color32::from_rgb(254, 128, 25),
                property: Color32::from_rgb(131, 165, 152),
                operator: Color32::from_rgb(235, 219, 178),
                punctuation: Color32::from_rgb(235, 219, 178),
                constant: Color32::from_rgb(211, 134, 155),
                constant_builtin: Color32::from_rgb(211, 134, 155),
                attribute: Color32::from_rgb(142, 192, 124),
                tag: Color32::from_rgb(251, 73, 52),
                escape: Color32::from_rgb(254, 128, 25),
                function_macro: Color32::from_rgb(142, 192, 124),
            },
        }
    }

    pub fn tokyo_night() -> Self {
        Self {
            name: "Tokyo Night".to_string(),
            background: Color32::from_rgb(26, 27, 38),
            foreground: Color32::from_rgb(169, 177, 214),
            gutter_bg: Color32::from_rgb(26, 27, 38),
            gutter_fg: Color32::from_rgb(59, 66, 97),
            current_line_bg: Color32::from_rgb(41, 46, 66),
            selection_bg: Color32::from_rgba_premultiplied(51, 59, 91, 200),
            cursor_color: Color32::from_rgb(192, 202, 245),
            search_match_bg: Color32::from_rgba_premultiplied(224, 175, 104, 80),
            search_current_match_bg: Color32::from_rgba_premultiplied(224, 175, 104, 140),
            tab_active_bg: Color32::from_rgb(41, 46, 66),
            tab_inactive_bg: Color32::from_rgb(26, 27, 38),
            tab_text: Color32::from_rgb(169, 177, 214),
            status_bar_bg: Color32::from_rgb(36, 40, 59),
            status_bar_fg: Color32::from_rgb(169, 177, 214),
            modified_indicator: Color32::from_rgb(158, 206, 106),
            syntax_colors: SyntaxColors {
                keyword: Color32::from_rgb(157, 124, 216),
                comment: Color32::from_rgb(86, 95, 137),
                string: Color32::from_rgb(158, 206, 106),
                number: Color32::from_rgb(255, 158, 100),
                function: Color32::from_rgb(125, 207, 255),
                r#type: Color32::from_rgb(42, 195, 222),
                type_builtin: Color32::from_rgb(42, 195, 222),
                variable: Color32::from_rgb(192, 202, 245),
                variable_builtin: Color32::from_rgb(255, 117, 127),
                property: Color32::from_rgb(115, 218, 202),
                operator: Color32::from_rgb(137, 221, 255),
                punctuation: Color32::from_rgb(169, 177, 214),
                constant: Color32::from_rgb(255, 158, 100),
                constant_builtin: Color32::from_rgb(255, 158, 100),
                attribute: Color32::from_rgb(224, 175, 104),
                tag: Color32::from_rgb(255, 117, 127),
                escape: Color32::from_rgb(137, 221, 255),
                function_macro: Color32::from_rgb(125, 207, 255),
            },
        }
    }

    /// Get a built-in theme by name.
    pub fn by_name(name: &str) -> Self {
        match name {
            "dark" | "Dark" => Self::dark(),
            "light" | "Light" => Self::light(),
            "monokai" | "Monokai" => Self::monokai(),
            "dracula" | "Dracula" => Self::dracula(),
            "solarized_dark" | "Solarized Dark" => Self::solarized_dark(),
            "solarized_light" | "Solarized Light" => Self::solarized_light(),
            "nord" | "Nord" => Self::nord(),
            "one_dark" | "One Dark" => Self::one_dark(),
            "gruvbox" | "Gruvbox" => Self::gruvbox(),
            "tokyo_night" | "Tokyo Night" => Self::tokyo_night(),
            _ => Self::dark(),
        }
    }

    /// All available built-in theme names.
    pub fn all_names() -> &'static [&'static str] {
        &[
            "Dark",
            "Light",
            "Monokai",
            "Dracula",
            "Solarized Dark",
            "Solarized Light",
            "Nord",
            "One Dark",
            "Gruvbox",
            "Tokyo Night",
        ]
    }

    /// Get the config-friendly name (lowercase, underscored).
    pub fn config_name(&self) -> &str {
        match self.name.as_str() {
            "Dark" => "dark",
            "Light" => "light",
            "Monokai" => "monokai",
            "Dracula" => "dracula",
            "Solarized Dark" => "solarized_dark",
            "Solarized Light" => "solarized_light",
            "Nord" => "nord",
            "One Dark" => "one_dark",
            "Gruvbox" => "gruvbox",
            "Tokyo Night" => "tokyo_night",
            // For custom themes the name IS the config name
            _ => &self.name,
        }
    }
}

impl Default for EditorTheme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_themes_constructible() {
        for name in EditorTheme::all_names() {
            let theme = EditorTheme::by_name(name);
            assert_eq!(theme.name, *name);
        }
    }

    #[test]
    fn test_theme_count() {
        assert_eq!(EditorTheme::all_names().len(), 10);
    }

    #[test]
    fn test_config_name_roundtrip() {
        for name in EditorTheme::all_names() {
            let theme = EditorTheme::by_name(name);
            let config = theme.config_name();
            let theme2 = EditorTheme::by_name(config);
            assert_eq!(theme.name, theme2.name);
        }
    }

    #[test]
    fn test_syntax_color_lookup() {
        let colors = SyntaxColors::dark();
        assert!(colors.color_for_highlight(0).is_some());
        assert!(colors.color_for_highlight(11).is_some()); // keyword
        assert!(colors.color_for_highlight(29).is_none()); // out of range
    }

    #[test]
    fn test_parse_hex_color_rgb() {
        let c = parse_hex_color("#1e1e1e").unwrap();
        assert_eq!(c, Color32::from_rgb(0x1e, 0x1e, 0x1e));
    }

    #[test]
    fn test_parse_hex_color_rgba() {
        let c = parse_hex_color("#46829664").unwrap();
        assert_eq!(c, Color32::from_rgba_premultiplied(0x46, 0x82, 0x96, 0x64));
    }

    #[test]
    fn test_parse_hex_color_uppercase() {
        let c = parse_hex_color("#AABBCC").unwrap();
        assert_eq!(c, Color32::from_rgb(0xAA, 0xBB, 0xCC));
    }

    #[test]
    fn test_parse_hex_color_invalid() {
        assert!(parse_hex_color("not-a-color").is_none());
        assert!(parse_hex_color("#xyz").is_none());
        assert!(parse_hex_color("#12345").is_none());
    }

    #[test]
    fn test_color_to_hex_rgb() {
        let s = color_to_hex(Color32::from_rgb(0x1e, 0x1e, 0x1e));
        assert_eq!(s, "#1e1e1e");
    }

    #[test]
    fn test_color_to_hex_rgba() {
        // Use from_rgba_premultiplied directly so values are exact (no conversion).
        let c = Color32::from_rgba_premultiplied(70, 130, 180, 100);
        let s = color_to_hex(c);
        // Verify it starts with # and has 8 hex digits (alpha < 255)
        assert!(s.starts_with('#'));
        assert_eq!(s.len(), 9); // "#" + 8 hex chars
        // Round-trip: parse back and compare
        let parsed = parse_hex_color(&s).unwrap();
        assert_eq!(parsed, c);
    }

    #[test]
    fn test_theme_file_roundtrip() {
        let dark = EditorTheme::dark();
        let tf = ThemeFile::from_editor_theme(&dark);
        let toml_str = toml::to_string_pretty(&tf).unwrap();
        let parsed: ThemeFile = toml::from_str(&toml_str).unwrap();
        let restored = parsed.to_editor_theme(&EditorTheme::dark());
        assert_eq!(restored.name, dark.name);
        assert_eq!(restored.background, dark.background);
        assert_eq!(restored.foreground, dark.foreground);
        assert_eq!(restored.syntax_colors.keyword, dark.syntax_colors.keyword);
    }

    #[test]
    fn test_theme_file_partial_with_base() {
        // A theme file with only a few colors set should inherit the rest from base.
        let toml_str = r##"
name = "Custom Dark"
base = "Dark"

[colors]
background = "#000000"

[syntax]
keyword = "#ff0000"
"##;
        let tf: ThemeFile = toml::from_str(toml_str).unwrap();
        let dark = EditorTheme::dark();
        let theme = tf.to_editor_theme(&dark);
        assert_eq!(theme.name, "Custom Dark");
        assert_eq!(theme.background, Color32::from_rgb(0, 0, 0));
        assert_eq!(theme.syntax_colors.keyword, Color32::from_rgb(255, 0, 0));
        // Inherited from dark base:
        assert_eq!(theme.foreground, dark.foreground);
        assert_eq!(theme.syntax_colors.comment, dark.syntax_colors.comment);
    }

    #[test]
    fn test_theme_file_minimal() {
        // Minimal theme: name only, everything inherited from default Dark base.
        let toml_str = r##"
name = "Minimal"
"##;
        let tf: ThemeFile = toml::from_str(toml_str).unwrap();
        let dark = EditorTheme::dark();
        let theme = tf.to_editor_theme(&dark);
        assert_eq!(theme.name, "Minimal");
        assert_eq!(theme.background, dark.background);
    }

    #[test]
    fn test_theme_registry_builtins() {
        let registry = ThemeRegistry {
            themes: ThemeRegistry::built_in_themes(),
            user_theme_names: Vec::new(),
        };
        let names = registry.all_names();
        assert_eq!(names.len(), 10);
        assert!(names.contains(&"Dark".to_string()));
        assert!(names.contains(&"Tokyo Night".to_string()));
    }

    #[test]
    fn test_theme_registry_get_by_display_name() {
        let registry = ThemeRegistry {
            themes: ThemeRegistry::built_in_themes(),
            user_theme_names: Vec::new(),
        };
        let theme = registry.get("Monokai");
        assert_eq!(theme.name, "Monokai");
    }

    #[test]
    fn test_theme_registry_get_by_config_name() {
        let registry = ThemeRegistry {
            themes: ThemeRegistry::built_in_themes(),
            user_theme_names: Vec::new(),
        };
        let theme = registry.get("one_dark");
        assert_eq!(theme.name, "One Dark");
    }

    #[test]
    fn test_custom_theme_config_name() {
        let mut theme = EditorTheme::dark();
        theme.name = "My Cool Theme".to_string();
        assert_eq!(theme.config_name(), "My Cool Theme");
    }
}
