use egui::{self, Pos2, Rect, Ui, Vec2};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::theme::EditorTheme;

/// Render a markdown preview panel for the given source text.
pub fn render_markdown_preview(
    ui: &mut Ui,
    source: &str,
    theme: &EditorTheme,
    scroll_offset: &mut f32,
) {
    let rect = ui.available_rect_before_wrap();

    // Background
    ui.painter().rect_filled(rect, 0.0, theme.background);

    // Parse markdown
    let options = Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(source, options);

    // Collect events into renderable blocks
    let blocks = parse_to_blocks(parser);

    // Calculate total content height for scrolling
    let line_height = 18.0f32;
    let padding = 16.0f32;
    let content_width = rect.width() - padding * 2.0;

    // Handle scroll
    let _scroll_response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
    let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
    if scroll_delta != 0.0 {
        *scroll_offset = (*scroll_offset - scroll_delta).max(0.0);
    }

    // Render blocks
    let mut y = rect.top() + padding - *scroll_offset;
    let clip_rect = rect;

    for block in &blocks {
        if y > clip_rect.bottom() + 100.0 {
            break; // past viewport
        }

        match block {
            MdBlock::Heading(level, text) => {
                let font_size = match level {
                    1 => 28.0,
                    2 => 24.0,
                    3 => 20.0,
                    4 => 18.0,
                    _ => 16.0,
                };
                let font = egui::FontId::proportional(font_size);

                if y + font_size > clip_rect.top() {
                    ui.painter().text(
                        Pos2::new(rect.left() + padding, y),
                        egui::Align2::LEFT_TOP,
                        text,
                        font.clone(),
                        theme.foreground,
                    );
                }
                y += font_size + 8.0;

                // Underline for h1 and h2
                if *level <= 2 && y > clip_rect.top() {
                    let line_y = y - 4.0;
                    ui.painter().line_segment(
                        [
                            Pos2::new(rect.left() + padding, line_y),
                            Pos2::new(rect.left() + padding + content_width, line_y),
                        ],
                        egui::Stroke::new(1.0, theme.gutter_fg),
                    );
                }
                y += 4.0;
            }
            MdBlock::Paragraph(text) => {
                let font = egui::FontId::proportional(14.0);
                // Simple word-wrap rendering
                let wrapped = wrap_text(text, content_width, 14.0 * 0.6);
                for line in &wrapped {
                    if y + line_height > clip_rect.top() && y < clip_rect.bottom() {
                        ui.painter().text(
                            Pos2::new(rect.left() + padding, y),
                            egui::Align2::LEFT_TOP,
                            line,
                            font.clone(),
                            theme.foreground,
                        );
                    }
                    y += line_height;
                }
                y += 8.0;
            }
            MdBlock::CodeBlock(text) => {
                let font = egui::FontId::monospace(13.0);
                let code_lines: Vec<&str> = text.lines().collect();
                let block_height = code_lines.len() as f32 * line_height + 16.0;

                if y + block_height > clip_rect.top() && y < clip_rect.bottom() {
                    // Code block background
                    let code_rect = Rect::from_min_size(
                        Pos2::new(rect.left() + padding, y),
                        Vec2::new(content_width, block_height),
                    );
                    ui.painter().rect_filled(code_rect, 4.0, theme.current_line_bg);

                    let code_y = y + 8.0;
                    for (i, line) in code_lines.iter().enumerate() {
                        ui.painter().text(
                            Pos2::new(rect.left() + padding + 8.0, code_y + i as f32 * line_height),
                            egui::Align2::LEFT_TOP,
                            line,
                            font.clone(),
                            theme.foreground,
                        );
                    }
                }
                y += block_height + 8.0;
            }
            MdBlock::ListItem(text, ordered, index) => {
                let font = egui::FontId::proportional(14.0);
                let bullet = if *ordered {
                    format!("{}. ", index)
                } else {
                    "\u{2022} ".to_string()
                };
                let full_text = format!("{}{}", bullet, text);

                if y + line_height > clip_rect.top() && y < clip_rect.bottom() {
                    ui.painter().text(
                        Pos2::new(rect.left() + padding + 16.0, y),
                        egui::Align2::LEFT_TOP,
                        &full_text,
                        font,
                        theme.foreground,
                    );
                }
                y += line_height;
            }
            MdBlock::HorizontalRule => {
                if y > clip_rect.top() && y < clip_rect.bottom() {
                    ui.painter().line_segment(
                        [
                            Pos2::new(rect.left() + padding, y + 8.0),
                            Pos2::new(rect.left() + padding + content_width, y + 8.0),
                        ],
                        egui::Stroke::new(1.0, theme.gutter_fg),
                    );
                }
                y += 16.0;
            }
            MdBlock::BlockQuote(text) => {
                let font = egui::FontId::proportional(14.0);
                let wrapped = wrap_text(text, content_width - 20.0, 14.0 * 0.6);
                let block_height = wrapped.len() as f32 * line_height;

                if y + block_height > clip_rect.top() && y < clip_rect.bottom() {
                    // Quote bar
                    ui.painter().line_segment(
                        [
                            Pos2::new(rect.left() + padding + 4.0, y),
                            Pos2::new(rect.left() + padding + 4.0, y + block_height),
                        ],
                        egui::Stroke::new(3.0, theme.gutter_fg),
                    );

                    for (i, line) in wrapped.iter().enumerate() {
                        ui.painter().text(
                            Pos2::new(rect.left() + padding + 16.0, y + i as f32 * line_height),
                            egui::Align2::LEFT_TOP,
                            line,
                            font.clone(),
                            theme.gutter_fg,
                        );
                    }
                }
                y += block_height + 8.0;
            }
        }
    }

    // Clamp scroll
    let content_height = y + *scroll_offset - rect.top();
    let max_scroll = (content_height - rect.height()).max(0.0);
    *scroll_offset = scroll_offset.min(max_scroll);
}

/// Simplified markdown block types for rendering.
enum MdBlock {
    Heading(usize, String),     // level, text
    Paragraph(String),
    CodeBlock(String),
    ListItem(String, bool, usize), // text, is_ordered, index
    HorizontalRule,
    BlockQuote(String),
}

fn parse_to_blocks(parser: Parser) -> Vec<MdBlock> {
    let mut blocks = Vec::new();
    let mut current_text = String::new();
    let mut in_heading: Option<usize> = None;
    let mut _in_code_block = false;
    let mut in_list_item = false;
    let mut is_ordered_list = false;
    let mut list_index: usize = 0;
    let mut in_block_quote = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let lvl = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                in_heading = Some(lvl);
                current_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(level) = in_heading.take() {
                    blocks.push(MdBlock::Heading(level, current_text.clone()));
                    current_text.clear();
                }
            }
            Event::Start(Tag::Paragraph) => {
                current_text.clear();
            }
            Event::End(TagEnd::Paragraph) => {
                if in_block_quote {
                    blocks.push(MdBlock::BlockQuote(current_text.clone()));
                } else if in_list_item {
                    blocks.push(MdBlock::ListItem(current_text.clone(), is_ordered_list, list_index));
                } else {
                    blocks.push(MdBlock::Paragraph(current_text.clone()));
                }
                current_text.clear();
            }
            Event::Start(Tag::CodeBlock(_)) => {
                _in_code_block = true;
                current_text.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                _in_code_block = false;
                blocks.push(MdBlock::CodeBlock(current_text.clone()));
                current_text.clear();
            }
            Event::Start(Tag::List(start)) => {
                is_ordered_list = start.is_some();
                list_index = start.unwrap_or(0) as usize;
            }
            Event::End(TagEnd::List(_)) => {
                is_ordered_list = false;
            }
            Event::Start(Tag::Item) => {
                in_list_item = true;
                if is_ordered_list {
                    list_index += 1;
                }
                current_text.clear();
            }
            Event::End(TagEnd::Item) => {
                in_list_item = false;
            }
            Event::Start(Tag::BlockQuote(_)) => {
                in_block_quote = true;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                in_block_quote = false;
            }
            Event::Text(text) => {
                current_text.push_str(&text);
            }
            Event::Code(text) => {
                current_text.push('`');
                current_text.push_str(&text);
                current_text.push('`');
            }
            Event::SoftBreak => {
                current_text.push(' ');
            }
            Event::HardBreak => {
                current_text.push('\n');
            }
            Event::Rule => {
                blocks.push(MdBlock::HorizontalRule);
            }
            _ => {}
        }
    }

    blocks
}

/// Simple word wrap: break text into lines that fit within width.
fn wrap_text(text: &str, max_width: f32, char_width: f32) -> Vec<String> {
    let max_chars = (max_width / char_width).floor().max(1.0) as usize;
    let mut lines = Vec::new();

    for input_line in text.lines() {
        if input_line.is_empty() {
            lines.push(String::new());
            continue;
        }
        let words: Vec<&str> = input_line.split_whitespace().collect();
        let mut current_line = String::new();

        for word in words {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= max_chars {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
