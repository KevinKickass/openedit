use egui::Color32;

/// Editor color theme.
#[derive(Debug, Clone)]
pub struct EditorTheme {
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

/// Colors for syntax highlighting, inspired by VS Code Dark+.
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
            keyword: Color32::from_rgb(197, 134, 192),       // purple
            comment: Color32::from_rgb(106, 153, 85),        // green
            string: Color32::from_rgb(206, 145, 120),        // orange
            number: Color32::from_rgb(181, 206, 168),        // light green
            function: Color32::from_rgb(220, 220, 170),      // yellow
            r#type: Color32::from_rgb(78, 201, 176),         // teal
            type_builtin: Color32::from_rgb(78, 201, 176),   // teal
            variable: Color32::from_rgb(156, 220, 254),      // light blue
            variable_builtin: Color32::from_rgb(86, 156, 214), // blue
            property: Color32::from_rgb(156, 220, 254),      // light blue
            operator: Color32::from_rgb(212, 212, 212),      // white
            punctuation: Color32::from_rgb(212, 212, 212),   // white
            constant: Color32::from_rgb(100, 150, 224),      // blue
            constant_builtin: Color32::from_rgb(86, 156, 214), // blue
            attribute: Color32::from_rgb(220, 220, 170),     // yellow
            tag: Color32::from_rgb(86, 156, 214),            // blue
            escape: Color32::from_rgb(215, 186, 125),        // gold
            function_macro: Color32::from_rgb(220, 220, 170), // yellow
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
            0 => Some(self.attribute),           // "attribute"
            1 => Some(self.constant_builtin),    // "boolean"
            2 => Some(self.comment),             // "comment"
            3 => Some(self.constant),            // "constant"
            4 => Some(self.constant_builtin),    // "constant.builtin"
            5 => Some(self.r#type),              // "constructor"
            6 => Some(self.string),              // "embedded"
            7 => Some(self.escape),              // "escape"
            8 => Some(self.function),            // "function"
            9 => Some(self.function),            // "function.builtin"
            10 => Some(self.function_macro),     // "function.macro"
            11 => Some(self.keyword),            // "keyword"
            12 => Some(self.variable),           // "label"
            13 => Some(self.variable),           // "module"
            14 => Some(self.number),             // "number"
            15 => Some(self.operator),           // "operator"
            16 => Some(self.property),           // "property"
            17 => Some(self.punctuation),        // "punctuation"
            18 => Some(self.punctuation),        // "punctuation.bracket"
            19 => Some(self.punctuation),        // "punctuation.delimiter"
            20 => Some(self.string),             // "string"
            21 => Some(self.escape),             // "string.escape"
            22 => Some(self.string),             // "string.special"
            23 => Some(self.tag),                // "tag"
            24 => Some(self.r#type),             // "type"
            25 => Some(self.type_builtin),       // "type.builtin"
            26 => Some(self.variable),           // "variable"
            27 => Some(self.variable_builtin),   // "variable.builtin"
            28 => Some(self.variable),           // "variable.parameter"
            _ => None,
        }
    }
}

impl EditorTheme {
    pub fn dark() -> Self {
        Self {
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
}

impl Default for EditorTheme {
    fn default() -> Self {
        Self::dark()
    }
}
