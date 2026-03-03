//! Bracket pair colorization — different colors for nested brackets.

use egui::Color32;

/// Colors for nested bracket pairs (cycles through these).
const BRACKET_COLORS: [Color32; 6] = [
    Color32::from_rgb(255, 215, 0),   // Gold
    Color32::from_rgb(218, 112, 214), // Orchid
    Color32::from_rgb(0, 191, 255),   // Deep sky blue
    Color32::from_rgb(255, 165, 0),   // Orange
    Color32::from_rgb(50, 205, 50),   // Lime green
    Color32::from_rgb(255, 105, 180), // Hot pink
];

/// A bracket with its position and nesting depth.
#[derive(Debug, Clone)]
pub struct ColoredBracket {
    pub col: usize,
    pub depth: usize,
    pub color: Color32,
}

/// Compute bracket colors for a single line given the starting depth.
/// Returns the list of colored brackets and the depth at end of line.
pub fn colorize_brackets_line(line: &str, start_depth: usize) -> (Vec<ColoredBracket>, usize) {
    let mut depth = start_depth;
    let mut brackets = Vec::new();

    for (col, ch) in line.chars().enumerate() {
        match ch {
            '(' | '[' | '{' => {
                brackets.push(ColoredBracket {
                    col,
                    depth,
                    color: BRACKET_COLORS[depth % BRACKET_COLORS.len()],
                });
                depth += 1;
            }
            ')' | ']' | '}' => {
                depth = depth.saturating_sub(1);
                brackets.push(ColoredBracket {
                    col,
                    depth,
                    color: BRACKET_COLORS[depth % BRACKET_COLORS.len()],
                });
            }
            _ => {}
        }
    }

    (brackets, depth)
}

/// Compute bracket colors for all visible lines.
/// `lines` is a slice of (line_index, line_text) pairs in document order.
/// `preceding_depth` is the bracket depth at the start of the first visible line
/// (computed by scanning all lines before).
pub fn colorize_brackets(
    all_lines: &[&str],
    visible_start: usize,
    visible_end: usize,
) -> Vec<(usize, Vec<ColoredBracket>)> {
    // Compute depth at start of visible region by scanning preceding lines
    let mut depth = 0usize;
    for line in all_lines.iter().take(visible_start) {
        for ch in line.chars() {
            match ch {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }

    let mut result = Vec::new();
    for (line_idx, line) in all_lines
        .iter()
        .enumerate()
        .take(visible_end.min(all_lines.len()))
        .skip(visible_start)
    {
        let (brackets, new_depth) = colorize_brackets_line(line, depth);
        if !brackets.is_empty() {
            result.push((line_idx, brackets));
        }
        depth = new_depth;
    }

    result
}

/// Render bracket colorization overlays for a line.
pub fn render_bracket_colors(
    ui: &mut egui::Ui,
    brackets: &[ColoredBracket],
    text_left: f32,
    y: f32,
    char_width: f32,
    font_id: &egui::FontId,
    col_offset: usize,
    line_text: &str,
) {
    let chars: Vec<char> = line_text.chars().collect();

    for bracket in brackets {
        if bracket.col < col_offset {
            continue;
        }
        let screen_col = bracket.col - col_offset;
        let x = text_left + 4.0 + screen_col as f32 * char_width;

        // Get the actual bracket character
        let ch = if bracket.col < chars.len() {
            chars[bracket.col].to_string()
        } else {
            continue;
        };

        // Draw the bracket with its depth-based color
        ui.painter().text(
            egui::Pos2::new(x, y),
            egui::Align2::LEFT_TOP,
            &ch,
            font_id.clone(),
            bracket.color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colorize_brackets_line_empty() {
        let (brackets, depth) = colorize_brackets_line("", 0);
        assert!(brackets.is_empty());
        assert_eq!(depth, 0);
    }

    #[test]
    fn test_colorize_brackets_line_nested() {
        let (brackets, depth) = colorize_brackets_line("(())", 0);
        assert_eq!(brackets.len(), 4);
        assert_eq!(brackets[0].depth, 0); // (
        assert_eq!(brackets[1].depth, 1); // (
        assert_eq!(brackets[2].depth, 1); // )
        assert_eq!(brackets[3].depth, 0); // )
        assert_eq!(depth, 0);
    }

    #[test]
    fn test_colorize_brackets_line_unmatched() {
        let (brackets, depth) = colorize_brackets_line("((", 0);
        assert_eq!(brackets.len(), 2);
        assert_eq!(depth, 2);
    }

    #[test]
    fn test_colorize_brackets_with_preceding() {
        let lines = vec!["fn main() {", "    let x = (1 + 2);", "}"];
        let result = colorize_brackets(&lines, 1, 2);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_colorize_brackets_colors_cycle() {
        let (brackets, _) = colorize_brackets_line("((((((((", 0);
        assert_eq!(brackets.len(), 8);
        // Colors should cycle through BRACKET_COLORS
        assert_eq!(brackets[0].color, BRACKET_COLORS[0]);
        assert_eq!(brackets[6].color, BRACKET_COLORS[0]); // cycles
    }
}
