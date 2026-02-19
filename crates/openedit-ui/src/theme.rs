use egui::Color32;

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

    /// Get a theme by name.
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

    /// All available theme names.
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
            _ => "dark",
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
}
