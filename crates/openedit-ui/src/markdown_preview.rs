use egui::{self, Color32, Pos2, Rect, Ui, Vec2};
use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::mermaid::{self, MermaidColors};
use crate::theme::EditorTheme;

// ────────────────────────────────────────────────────────────────────────────
// Rich inline span model
// ────────────────────────────────────────────────────────────────────────────

/// A span of inline text with formatting metadata.
#[derive(Debug, Clone)]
struct RichSpan {
    text: String,
    bold: bool,
    italic: bool,
    strikethrough: bool,
    code: bool,
    /// If Some, this span is a link with the given URL.
    link_url: Option<String>,
}

impl RichSpan {
    fn plain(text: &str) -> Self {
        Self {
            text: text.to_string(),
            bold: false,
            italic: false,
            strikethrough: false,
            code: false,
            link_url: None,
        }
    }

    /// Flatten the text content of a slice of spans.
    fn flat_text(spans: &[RichSpan]) -> String {
        spans.iter().map(|s| s.text.as_str()).collect()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Block model
// ────────────────────────────────────────────────────────────────────────────

/// Simplified markdown block types for rendering.
enum MdBlock {
    Heading(usize, Vec<RichSpan>),
    Paragraph(Vec<RichSpan>),
    CodeBlock(String),
    MermaidDiagram(mermaid::MermaidDiagram),
    ListItem {
        spans: Vec<RichSpan>,
        ordered: bool,
        index: usize,
        depth: usize,
    },
    TaskListItem {
        spans: Vec<RichSpan>,
        checked: bool,
        depth: usize,
    },
    HorizontalRule,
    BlockQuote(Vec<RichSpan>),
    Table {
        header: Vec<Vec<RichSpan>>,
        rows: Vec<Vec<Vec<RichSpan>>>,
        alignments: Vec<Alignment>,
    },
    Image {
        alt: String,
        url: String,
        title: String,
    },
}

// ────────────────────────────────────────────────────────────────────────────
// Rendering
// ────────────────────────────────────────────────────────────────────────────

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
    let options =
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(source, options);
    let blocks = parse_to_blocks(parser);

    let line_height = 18.0f32;
    let padding = 16.0f32;
    let content_width = rect.width() - padding * 2.0;

    // Handle scroll
    let _scroll_response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
    let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
    if scroll_delta != 0.0 {
        *scroll_offset = (*scroll_offset - scroll_delta).max(0.0);
    }

    let link_color = theme.syntax_colors.function;
    let code_bg = theme.current_line_bg;
    let inline_code_bg = Color32::from_rgba_premultiplied(
        theme.current_line_bg.r(),
        theme.current_line_bg.g(),
        theme.current_line_bg.b(),
        180,
    );

    // Render blocks
    let mut y = rect.top() + padding - *scroll_offset;
    let clip_rect = rect;

    for block in &blocks {
        if y > clip_rect.bottom() + 100.0 {
            break;
        }

        match block {
            // ── Headings ────────────────────────────────────────────
            MdBlock::Heading(level, spans) => {
                let font_size = match level {
                    1 => 28.0,
                    2 => 24.0,
                    3 => 20.0,
                    4 => 18.0,
                    _ => 16.0,
                };

                if y + font_size > clip_rect.top() {
                    let x = rect.left() + padding;
                    render_rich_spans(
                        ui,
                        spans,
                        x,
                        y,
                        font_size,
                        true, // headings are always bold
                        theme.foreground,
                        link_color,
                        inline_code_bg,
                        content_width,
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

            // ── Paragraphs (with rich inline formatting) ────────────
            MdBlock::Paragraph(spans) => {
                let font_size = 14.0;
                let rendered_height = estimate_rich_height(spans, content_width, font_size);

                if y + rendered_height > clip_rect.top() && y < clip_rect.bottom() {
                    let x = rect.left() + padding;
                    let h = render_rich_spans_wrapped(
                        ui,
                        spans,
                        x,
                        y,
                        font_size,
                        theme.foreground,
                        link_color,
                        inline_code_bg,
                        content_width,
                        line_height,
                    );
                    y += h;
                } else {
                    y += rendered_height;
                }
                y += 8.0;
            }

            // ── Code blocks ─────────────────────────────────────────
            MdBlock::CodeBlock(text) => {
                let font = egui::FontId::monospace(13.0);
                let code_lines: Vec<&str> = text.lines().collect();
                let block_height = code_lines.len() as f32 * line_height + 16.0;

                if y + block_height > clip_rect.top() && y < clip_rect.bottom() {
                    let code_rect = Rect::from_min_size(
                        Pos2::new(rect.left() + padding, y),
                        Vec2::new(content_width, block_height),
                    );
                    // Dark background with rounded corners
                    ui.painter().rect_filled(code_rect, 4.0, code_bg);
                    // Subtle border
                    ui.painter().rect_stroke(
                        code_rect,
                        4.0,
                        egui::Stroke::new(1.0, theme.gutter_fg),
                    );

                    let code_y = y + 8.0;
                    for (i, line) in code_lines.iter().enumerate() {
                        ui.painter().text(
                            Pos2::new(
                                rect.left() + padding + 8.0,
                                code_y + i as f32 * line_height,
                            ),
                            egui::Align2::LEFT_TOP,
                            line,
                            font.clone(),
                            theme.foreground,
                        );
                    }
                }
                y += block_height + 8.0;
            }

            // ── Mermaid diagrams ────────────────────────────────────
            MdBlock::MermaidDiagram(diagram) => {
                let mermaid_colors = MermaidColors {
                    node_fill: theme.current_line_bg,
                    node_stroke: theme.syntax_colors.keyword,
                    node_text: theme.foreground,
                    edge_color: theme.gutter_fg,
                    label_color: theme.foreground,
                    background: Color32::from_rgba_premultiplied(
                        theme.current_line_bg.r(),
                        theme.current_line_bg.g(),
                        theme.current_line_bg.b(),
                        200,
                    ),
                    participant_fill: theme.tab_active_bg,
                    arrow_color: theme.syntax_colors.function,
                };

                let height = mermaid::render_mermaid(
                    ui.painter(),
                    diagram,
                    Pos2::new(rect.left() + padding, y),
                    content_width,
                    &mermaid_colors,
                );
                y += height + 8.0;
            }

            // ── List items ──────────────────────────────────────────
            MdBlock::ListItem {
                spans,
                ordered,
                index,
                depth,
            } => {
                let font_size = 14.0;
                let indent = 16.0 + (*depth as f32) * 20.0;
                let bullet = if *ordered {
                    format!("{}. ", index)
                } else {
                    match depth % 3 {
                        0 => "\u{2022} ".to_string(), // bullet
                        1 => "\u{25E6} ".to_string(), // white bullet
                        _ => "\u{25AA} ".to_string(), // small black square
                    }
                };
                let bullet_font = egui::FontId::proportional(font_size);
                let avail = content_width - indent;

                if y + line_height > clip_rect.top() && y < clip_rect.bottom() {
                    let bx = rect.left() + padding + indent;
                    // Draw the bullet/number
                    let bullet_galley = ui.painter().layout_no_wrap(
                        bullet.clone(),
                        bullet_font,
                        theme.foreground,
                    );
                    let bullet_width = bullet_galley.rect.width();
                    ui.painter()
                        .galley(Pos2::new(bx, y), bullet_galley, theme.foreground);

                    // Draw the rich text after the bullet
                    let h = render_rich_spans_wrapped(
                        ui,
                        spans,
                        bx + bullet_width,
                        y,
                        font_size,
                        theme.foreground,
                        link_color,
                        inline_code_bg,
                        avail - bullet_width,
                        line_height,
                    );
                    y += h.max(line_height);
                } else {
                    y += line_height;
                }
            }

            // ── Task list items ─────────────────────────────────────
            MdBlock::TaskListItem {
                spans,
                checked,
                depth,
            } => {
                let font_size = 14.0;
                let indent = 16.0 + (*depth as f32) * 20.0;
                let checkbox = if *checked { "\u{2611} " } else { "\u{2610} " };
                let checkbox_font = egui::FontId::proportional(font_size);
                let avail = content_width - indent;

                if y + line_height > clip_rect.top() && y < clip_rect.bottom() {
                    let bx = rect.left() + padding + indent;
                    let cb_galley = ui.painter().layout_no_wrap(
                        checkbox.to_string(),
                        checkbox_font,
                        theme.foreground,
                    );
                    let cb_width = cb_galley.rect.width();
                    ui.painter()
                        .galley(Pos2::new(bx, y), cb_galley, theme.foreground);

                    let text_color = if *checked {
                        theme.gutter_fg // dimmed for checked items
                    } else {
                        theme.foreground
                    };
                    let h = render_rich_spans_wrapped(
                        ui,
                        spans,
                        bx + cb_width,
                        y,
                        font_size,
                        text_color,
                        link_color,
                        inline_code_bg,
                        avail - cb_width,
                        line_height,
                    );
                    y += h.max(line_height);
                } else {
                    y += line_height;
                }
            }

            // ── Horizontal rule ─────────────────────────────────────
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
                y += 24.0;
            }

            // ── Block quotes ────────────────────────────────────────
            MdBlock::BlockQuote(spans) => {
                let font_size = 14.0;
                let bar_x = rect.left() + padding + 4.0;
                let text_x = rect.left() + padding + 16.0;
                let avail = content_width - 20.0;

                let rendered_height = estimate_rich_height(spans, avail, font_size);

                if y + rendered_height > clip_rect.top() && y < clip_rect.bottom() {
                    // Quote background
                    let bg_rect = Rect::from_min_size(
                        Pos2::new(rect.left() + padding, y - 2.0),
                        Vec2::new(content_width, rendered_height + 4.0),
                    );
                    let quote_bg = Color32::from_rgba_premultiplied(
                        theme.gutter_fg.r(),
                        theme.gutter_fg.g(),
                        theme.gutter_fg.b(),
                        30,
                    );
                    ui.painter().rect_filled(bg_rect, 2.0, quote_bg);

                    // Left bar
                    ui.painter().line_segment(
                        [
                            Pos2::new(bar_x, y - 2.0),
                            Pos2::new(bar_x, y + rendered_height + 2.0),
                        ],
                        egui::Stroke::new(3.0, theme.gutter_fg),
                    );

                    let h = render_rich_spans_wrapped(
                        ui,
                        spans,
                        text_x,
                        y,
                        font_size,
                        theme.gutter_fg,
                        link_color,
                        inline_code_bg,
                        avail,
                        line_height,
                    );
                    y += h;
                } else {
                    y += rendered_height;
                }
                y += 8.0;
            }

            // ── Tables ──────────────────────────────────────────────
            MdBlock::Table {
                header,
                rows,
                alignments,
            } => {
                let font_size = 13.0;
                let cell_pad = 8.0;
                let num_cols = header.len().max(1);
                let col_width = (content_width - cell_pad * 2.0) / num_cols as f32;
                let row_height = line_height + cell_pad;
                let total_rows = 1 + rows.len();
                let table_height = total_rows as f32 * row_height + 2.0;

                if y + table_height > clip_rect.top() && y < clip_rect.bottom() {
                    let table_x = rect.left() + padding;
                    let table_rect = Rect::from_min_size(
                        Pos2::new(table_x, y),
                        Vec2::new(content_width, table_height),
                    );

                    // Table border
                    ui.painter().rect_stroke(
                        table_rect,
                        2.0,
                        egui::Stroke::new(1.0, theme.gutter_fg),
                    );

                    // Header background
                    let header_rect = Rect::from_min_size(
                        Pos2::new(table_x, y),
                        Vec2::new(content_width, row_height),
                    );
                    ui.painter().rect_filled(header_rect, 0.0, code_bg);

                    // Render header cells
                    for (col_idx, cell_spans) in header.iter().enumerate() {
                        let cx =
                            table_x + cell_pad + col_idx as f32 * col_width;
                        let text = RichSpan::flat_text(cell_spans);
                        let align = alignments.get(col_idx).copied().unwrap_or(Alignment::None);
                        let font = egui::FontId::proportional(font_size);
                        let text_pos = aligned_text_pos(
                            cx,
                            y + cell_pad / 2.0,
                            &text,
                            col_width - cell_pad,
                            align,
                            font_size,
                        );
                        // Bold header text
                        let galley = ui.painter().layout_no_wrap(
                            text,
                            font,
                            theme.foreground,
                        );
                        ui.painter()
                            .galley(text_pos, galley, theme.foreground);
                    }

                    // Header bottom line
                    let hline_y = y + row_height;
                    ui.painter().line_segment(
                        [
                            Pos2::new(table_x, hline_y),
                            Pos2::new(table_x + content_width, hline_y),
                        ],
                        egui::Stroke::new(1.0, theme.gutter_fg),
                    );

                    // Render body rows
                    for (row_idx, row) in rows.iter().enumerate() {
                        let ry = y + (row_idx + 1) as f32 * row_height;

                        // Alternating row background
                        if row_idx % 2 == 1 {
                            let alt_rect = Rect::from_min_size(
                                Pos2::new(table_x, ry),
                                Vec2::new(content_width, row_height),
                            );
                            let alt_bg = Color32::from_rgba_premultiplied(
                                theme.current_line_bg.r(),
                                theme.current_line_bg.g(),
                                theme.current_line_bg.b(),
                                80,
                            );
                            ui.painter().rect_filled(alt_rect, 0.0, alt_bg);
                        }

                        for (col_idx, cell_spans) in row.iter().enumerate() {
                            let cx = table_x + cell_pad + col_idx as f32 * col_width;
                            let text = RichSpan::flat_text(cell_spans);
                            let align =
                                alignments.get(col_idx).copied().unwrap_or(Alignment::None);
                            let font = egui::FontId::proportional(font_size);
                            let text_pos = aligned_text_pos(
                                cx,
                                ry + cell_pad / 2.0,
                                &text,
                                col_width - cell_pad,
                                align,
                                font_size,
                            );
                            let galley = ui.painter().layout_no_wrap(
                                text,
                                font,
                                theme.foreground,
                            );
                            ui.painter()
                                .galley(text_pos, galley, theme.foreground);
                        }

                        // Row bottom line
                        let rline_y = ry + row_height;
                        ui.painter().line_segment(
                            [
                                Pos2::new(table_x, rline_y),
                                Pos2::new(table_x + content_width, rline_y),
                            ],
                            egui::Stroke::new(0.5, theme.gutter_fg),
                        );
                    }

                    // Vertical column separators
                    for col_idx in 1..num_cols {
                        let cx = table_x + col_idx as f32 * col_width;
                        ui.painter().line_segment(
                            [
                                Pos2::new(cx, y),
                                Pos2::new(cx, y + table_height),
                            ],
                            egui::Stroke::new(0.5, theme.gutter_fg),
                        );
                    }
                }
                y += table_height + 8.0;
            }

            // ── Images ──────────────────────────────────────────────
            MdBlock::Image { alt, url, title } => {
                let font_size = 13.0;
                let label = if !alt.is_empty() {
                    format!("[Image: {}]", alt)
                } else if !title.is_empty() {
                    format!("[Image: {}]", title)
                } else {
                    format!("[Image: {}]", url)
                };

                if y + line_height > clip_rect.top() && y < clip_rect.bottom() {
                    // Image placeholder background
                    let img_rect = Rect::from_min_size(
                        Pos2::new(rect.left() + padding, y),
                        Vec2::new(content_width, line_height + 8.0),
                    );
                    ui.painter().rect_filled(img_rect, 4.0, code_bg);
                    ui.painter().rect_stroke(
                        img_rect,
                        4.0,
                        egui::Stroke::new(1.0, theme.gutter_fg),
                    );

                    let font = egui::FontId::proportional(font_size);
                    ui.painter().text(
                        Pos2::new(rect.left() + padding + 8.0, y + 4.0),
                        egui::Align2::LEFT_TOP,
                        &label,
                        font,
                        link_color,
                    );
                }
                y += line_height + 16.0;
            }
        }
    }

    // Clamp scroll
    let content_height = y + *scroll_offset - rect.top();
    let max_scroll = (content_height - rect.height()).max(0.0);
    *scroll_offset = scroll_offset.min(max_scroll);
}

// ────────────────────────────────────────────────────────────────────────────
// Rich span rendering helpers
// ────────────────────────────────────────────────────────────────────────────

/// Render a sequence of rich spans on a single line (no wrapping).
/// Used for headings.
fn render_rich_spans(
    ui: &Ui,
    spans: &[RichSpan],
    x: f32,
    y: f32,
    font_size: f32,
    force_bold: bool,
    default_color: Color32,
    link_color: Color32,
    inline_code_bg: Color32,
    _max_width: f32,
) {
    let mut cx = x;
    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        let font = if span.code {
            egui::FontId::monospace(font_size * 0.9)
        } else {
            egui::FontId::proportional(font_size)
        };

        let color = if span.link_url.is_some() {
            link_color
        } else {
            default_color
        };

        // Build styled text via galley
        let mut text = span.text.clone();
        if span.strikethrough {
            // Approximate strikethrough with Unicode combining chars is ugly;
            // we draw a line over it after painting.
        }

        let galley = ui.painter().layout_no_wrap(text.clone(), font.clone(), color);
        let gw = galley.rect.width();

        // Inline code background
        if span.code {
            let bg_rect = Rect::from_min_size(
                Pos2::new(cx - 2.0, y),
                Vec2::new(gw + 4.0, font_size + 2.0),
            );
            ui.painter().rect_filled(bg_rect, 3.0, inline_code_bg);
        }

        ui.painter().galley(Pos2::new(cx, y), galley, color);

        // Draw bold by painting again with a 1px offset (egui doesn't have bold fonts)
        if span.bold || force_bold {
            let galley2 =
                ui.painter()
                    .layout_no_wrap(text.clone(), font.clone(), color);
            ui.painter()
                .galley(Pos2::new(cx + 0.5, y), galley2, color);
        }

        // Italic: draw with a slight shear (approximation)
        // egui doesn't support italic natively in all backends, so we use the
        // italics field on FontId — but egui's default fonts do have italic
        // variants for proportional text. We indicate italic via visual cue:
        // we could skip this since egui may not have italic glyphs; instead
        // we render italic text with a subtle color shift.
        if span.italic && !span.bold {
            // Already rendered above; italic is a best-effort visual hint
        }

        // Strikethrough line
        if span.strikethrough {
            let mid_y = y + font_size / 2.0;
            ui.painter().line_segment(
                [Pos2::new(cx, mid_y), Pos2::new(cx + gw, mid_y)],
                egui::Stroke::new(1.0, color),
            );
        }

        // Underline for links
        if span.link_url.is_some() {
            let ul_y = y + font_size + 1.0;
            ui.painter().line_segment(
                [Pos2::new(cx, ul_y), Pos2::new(cx + gw, ul_y)],
                egui::Stroke::new(1.0, link_color),
            );
        }

        // Suppress the unused variable warning
        let _ = &mut text;

        cx += gw;
    }
}

/// Render rich spans with word wrapping. Returns total height consumed.
fn render_rich_spans_wrapped(
    ui: &Ui,
    spans: &[RichSpan],
    x: f32,
    y: f32,
    font_size: f32,
    default_color: Color32,
    link_color: Color32,
    inline_code_bg: Color32,
    max_width: f32,
    line_height: f32,
) -> f32 {
    // Flatten spans into "words" that carry formatting, then wrap.
    let words = spans_to_words(spans);
    let char_width = font_size * 0.55;

    let mut cx = 0.0f32;
    let mut cy = y;

    for word in &words {
        let font = if word.code {
            egui::FontId::monospace(font_size * 0.9)
        } else {
            egui::FontId::proportional(font_size)
        };

        let color = if word.link_url.is_some() {
            link_color
        } else {
            default_color
        };

        // Estimate width
        let est_width = word.text.len() as f32 * char_width;

        // Wrap if needed
        if cx > 0.0 && cx + est_width > max_width {
            cx = 0.0;
            cy += line_height;
        }

        let galley = ui.painter().layout_no_wrap(word.text.clone(), font.clone(), color);
        let gw = galley.rect.width();

        // Inline code background
        if word.code {
            let bg_rect = Rect::from_min_size(
                Pos2::new(x + cx - 2.0, cy),
                Vec2::new(gw + 4.0, line_height),
            );
            ui.painter().rect_filled(bg_rect, 3.0, inline_code_bg);
        }

        ui.painter()
            .galley(Pos2::new(x + cx, cy), galley, color);

        // Bold: overlay slightly offset
        if word.bold {
            let galley2 =
                ui.painter()
                    .layout_no_wrap(word.text.clone(), font.clone(), color);
            ui.painter()
                .galley(Pos2::new(x + cx + 0.5, cy), galley2, color);
        }

        // Strikethrough
        if word.strikethrough {
            let mid_y = cy + font_size / 2.0;
            ui.painter().line_segment(
                [
                    Pos2::new(x + cx, mid_y),
                    Pos2::new(x + cx + gw, mid_y),
                ],
                egui::Stroke::new(1.0, color),
            );
        }

        // Underline for links
        if word.link_url.is_some() {
            let ul_y = cy + font_size + 1.0;
            ui.painter().line_segment(
                [
                    Pos2::new(x + cx, ul_y),
                    Pos2::new(x + cx + gw, ul_y),
                ],
                egui::Stroke::new(1.0, link_color),
            );
        }

        cx += gw;
    }

    (cy - y) + line_height
}

/// Break rich spans into word-level chunks preserving formatting.
fn spans_to_words(spans: &[RichSpan]) -> Vec<RichSpan> {
    let mut words = Vec::new();
    for span in spans {
        if span.code {
            // Keep inline code as a single token (don't split on spaces)
            words.push(span.clone());
            continue;
        }

        let parts: Vec<&str> = span.text.split_inclusive(char::is_whitespace).collect();
        for part in parts {
            if part.is_empty() {
                continue;
            }
            words.push(RichSpan {
                text: part.to_string(),
                bold: span.bold,
                italic: span.italic,
                strikethrough: span.strikethrough,
                code: span.code,
                link_url: span.link_url.clone(),
            });
        }
    }
    words
}

/// Estimate height needed for wrapped rich spans.
fn estimate_rich_height(spans: &[RichSpan], max_width: f32, font_size: f32) -> f32 {
    let char_width = font_size * 0.55;
    let line_height = 18.0;
    let mut cx = 0.0f32;
    let mut lines = 1.0f32;

    for span in spans {
        if span.code {
            let w = span.text.len() as f32 * char_width * 0.85;
            if cx > 0.0 && cx + w > max_width {
                cx = 0.0;
                lines += 1.0;
            }
            cx += w;
            continue;
        }
        for part in span.text.split_inclusive(char::is_whitespace) {
            let w = part.len() as f32 * char_width;
            if cx > 0.0 && cx + w > max_width {
                cx = 0.0;
                lines += 1.0;
            }
            cx += w;
        }
    }
    lines * line_height
}

/// Compute text position respecting table column alignment.
fn aligned_text_pos(
    x: f32,
    y: f32,
    text: &str,
    col_width: f32,
    alignment: Alignment,
    font_size: f32,
) -> Pos2 {
    let char_width = font_size * 0.55;
    let text_width = text.len() as f32 * char_width;
    match alignment {
        Alignment::Center => Pos2::new(x + (col_width - text_width) / 2.0, y),
        Alignment::Right => Pos2::new(x + col_width - text_width, y),
        _ => Pos2::new(x, y),
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Parser: pulldown-cmark events -> MdBlock
// ────────────────────────────────────────────────────────────────────────────

fn parse_to_blocks(parser: Parser) -> Vec<MdBlock> {
    let mut blocks = Vec::new();
    let mut span_stack: Vec<RichSpan> = Vec::new(); // accumulated spans for current block

    // Inline style state (nested)
    let mut bold_depth: usize = 0;
    let mut italic_depth: usize = 0;
    let mut strikethrough_depth: usize = 0;
    let mut link_stack: Vec<String> = Vec::new(); // nested link URLs

    // Block context
    let mut in_heading: Option<usize> = None;
    let mut _in_paragraph = false;
    let mut in_code_block = false;
    let mut in_mermaid_block = false;
    let mut code_text = String::new();
    let mut in_block_quote = false;
    let mut list_stack: Vec<ListCtx> = Vec::new(); // for nested lists
    let mut in_list_item = false;
    let mut task_list_checked: Option<bool> = None;

    // Table state
    let mut _in_table = false;
    let mut table_alignments: Vec<Alignment> = Vec::new();
    let mut table_header: Vec<Vec<RichSpan>> = Vec::new();
    let mut table_rows: Vec<Vec<Vec<RichSpan>>> = Vec::new();
    let mut table_current_row: Vec<Vec<RichSpan>> = Vec::new();
    let mut in_table_head = false;
    let mut in_table_cell = false;

    // Image state
    let mut in_image = false;
    let mut image_url = String::new();
    let mut image_title = String::new();

    for event in parser {
        match event {
            // ── Headings ────────────────────────────────────────
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
                span_stack.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(level) = in_heading.take() {
                    blocks.push(MdBlock::Heading(level, std::mem::take(&mut span_stack)));
                }
            }

            // ── Paragraphs ──────────────────────────────────────
            Event::Start(Tag::Paragraph) => {
                if !in_table_cell && !in_image {
                    _in_paragraph = true;
                    span_stack.clear();
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if in_image {
                    // Paragraph inside image tag — ignore; image handled separately
                } else if in_table_cell {
                    // Paragraph inside table cell — just continue accumulating
                } else if in_block_quote {
                    blocks.push(MdBlock::BlockQuote(std::mem::take(&mut span_stack)));
                } else if in_list_item {
                    let depth = list_stack.len().saturating_sub(1);
                    if let Some(checked) = task_list_checked.take() {
                        blocks.push(MdBlock::TaskListItem {
                            spans: std::mem::take(&mut span_stack),
                            checked,
                            depth,
                        });
                    } else {
                        let (ordered, index) = list_stack
                            .last()
                            .map(|l| (l.ordered, l.index))
                            .unwrap_or((false, 0));
                        blocks.push(MdBlock::ListItem {
                            spans: std::mem::take(&mut span_stack),
                            ordered,
                            index,
                            depth,
                        });
                    }
                } else {
                    blocks.push(MdBlock::Paragraph(std::mem::take(&mut span_stack)));
                }
                _in_paragraph = false;
            }

            // ── Code blocks ─────────────────────────────────────
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                in_mermaid_block = matches!(&kind, CodeBlockKind::Fenced(lang) if lang.as_ref().trim().eq_ignore_ascii_case("mermaid"));
                code_text.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                if in_mermaid_block {
                    let diagram = mermaid::parse_mermaid(&code_text);
                    blocks.push(MdBlock::MermaidDiagram(diagram));
                    in_mermaid_block = false;
                } else {
                    blocks.push(MdBlock::CodeBlock(std::mem::take(&mut code_text)));
                }
            }

            // ── Lists ───────────────────────────────────────────
            Event::Start(Tag::List(start)) => {
                let ordered = start.is_some();
                let index = start.unwrap_or(0) as usize;
                list_stack.push(ListCtx { ordered, index });
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
            }
            Event::Start(Tag::Item) => {
                in_list_item = true;
                task_list_checked = None;
                if let Some(ctx) = list_stack.last_mut() {
                    if ctx.ordered {
                        ctx.index += 1;
                    }
                }
                span_stack.clear();
            }
            Event::End(TagEnd::Item) => {
                // If the item had no inner paragraph, flush accumulated spans now
                if !span_stack.is_empty() {
                    let depth = list_stack.len().saturating_sub(1);
                    if let Some(checked) = task_list_checked.take() {
                        blocks.push(MdBlock::TaskListItem {
                            spans: std::mem::take(&mut span_stack),
                            checked,
                            depth,
                        });
                    } else {
                        let (ordered, index) = list_stack
                            .last()
                            .map(|l| (l.ordered, l.index))
                            .unwrap_or((false, 0));
                        blocks.push(MdBlock::ListItem {
                            spans: std::mem::take(&mut span_stack),
                            ordered,
                            index,
                            depth,
                        });
                    }
                }
                in_list_item = false;
                task_list_checked = None;
            }

            // ── Block quotes ────────────────────────────────────
            Event::Start(Tag::BlockQuote(_)) => {
                in_block_quote = true;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                // Flush any remaining spans as a block quote
                if !span_stack.is_empty() {
                    blocks.push(MdBlock::BlockQuote(std::mem::take(&mut span_stack)));
                }
                in_block_quote = false;
            }

            // ── Inline styles ───────────────────────────────────
            Event::Start(Tag::Strong) => {
                bold_depth += 1;
            }
            Event::End(TagEnd::Strong) => {
                bold_depth = bold_depth.saturating_sub(1);
            }
            Event::Start(Tag::Emphasis) => {
                italic_depth += 1;
            }
            Event::End(TagEnd::Emphasis) => {
                italic_depth = italic_depth.saturating_sub(1);
            }
            Event::Start(Tag::Strikethrough) => {
                strikethrough_depth += 1;
            }
            Event::End(TagEnd::Strikethrough) => {
                strikethrough_depth = strikethrough_depth.saturating_sub(1);
            }

            // ── Links ───────────────────────────────────────────
            Event::Start(Tag::Link { dest_url, .. }) => {
                link_stack.push(dest_url.to_string());
            }
            Event::End(TagEnd::Link) => {
                link_stack.pop();
            }

            // ── Images ──────────────────────────────────────────
            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                in_image = true;
                image_url = dest_url.to_string();
                image_title = title.to_string();
                span_stack.clear();
            }
            Event::End(TagEnd::Image) => {
                let alt = RichSpan::flat_text(&span_stack);
                blocks.push(MdBlock::Image {
                    alt,
                    url: std::mem::take(&mut image_url),
                    title: std::mem::take(&mut image_title),
                });
                span_stack.clear();
                in_image = false;
            }

            // ── Tables ──────────────────────────────────────────
            Event::Start(Tag::Table(aligns)) => {
                _in_table = true;
                table_alignments = aligns;
                table_header.clear();
                table_rows.clear();
            }
            Event::End(TagEnd::Table) => {
                blocks.push(MdBlock::Table {
                    header: std::mem::take(&mut table_header),
                    rows: std::mem::take(&mut table_rows),
                    alignments: std::mem::take(&mut table_alignments),
                });
                _in_table = false;
            }
            Event::Start(Tag::TableHead) => {
                in_table_head = true;
                table_current_row.clear();
            }
            Event::End(TagEnd::TableHead) => {
                table_header = std::mem::take(&mut table_current_row);
                in_table_head = false;
            }
            Event::Start(Tag::TableRow) => {
                table_current_row.clear();
            }
            Event::End(TagEnd::TableRow) => {
                if !in_table_head {
                    table_rows.push(std::mem::take(&mut table_current_row));
                }
            }
            Event::Start(Tag::TableCell) => {
                in_table_cell = true;
                span_stack.clear();
            }
            Event::End(TagEnd::TableCell) => {
                table_current_row.push(std::mem::take(&mut span_stack));
                in_table_cell = false;
            }

            // ── Text content ────────────────────────────────────
            Event::Text(text) => {
                if in_code_block {
                    code_text.push_str(&text);
                } else {
                    let link = link_stack.last().cloned();
                    span_stack.push(RichSpan {
                        text: text.to_string(),
                        bold: bold_depth > 0,
                        italic: italic_depth > 0,
                        strikethrough: strikethrough_depth > 0,
                        code: false,
                        link_url: link,
                    });
                }
            }

            // ── Inline code ─────────────────────────────────────
            Event::Code(text) => {
                if in_code_block {
                    code_text.push_str(&text);
                } else {
                    span_stack.push(RichSpan {
                        text: text.to_string(),
                        bold: bold_depth > 0,
                        italic: italic_depth > 0,
                        strikethrough: strikethrough_depth > 0,
                        code: true,
                        link_url: link_stack.last().cloned(),
                    });
                }
            }

            Event::SoftBreak => {
                if in_code_block {
                    code_text.push(' ');
                } else {
                    span_stack.push(RichSpan::plain(" "));
                }
            }
            Event::HardBreak => {
                if in_code_block {
                    code_text.push('\n');
                } else {
                    span_stack.push(RichSpan::plain("\n"));
                }
            }

            Event::Rule => {
                blocks.push(MdBlock::HorizontalRule);
            }

            Event::TaskListMarker(checked) => {
                task_list_checked = Some(checked);
            }

            _ => {}
        }
    }

    blocks
}

struct ListCtx {
    ordered: bool,
    index: usize,
}

// ────────────────────────────────────────────────────────────────────────────
// Simple word wrap (kept for any remaining plain-text usage)
// ────────────────────────────────────────────────────────────────────────────

/// Simple word wrap: break text into lines that fit within width.
#[allow(dead_code)]
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
