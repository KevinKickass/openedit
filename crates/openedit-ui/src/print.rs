//! Print / Export-to-PDF functionality for OpenEdit.
//!
//! Generates a PDF from the current document using the `printpdf` crate.
//! Supports configurable page size, margins, font size, line numbers,
//! and optional syntax highlighting colors.

use crate::theme::SyntaxColors;
use openedit_core::syntax::HighlightSpan;
use printpdf::*;

// ── Paper sizes (width x height in mm) ──────────────────────────────

/// Supported paper sizes.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PaperSize {
    A4,
    Letter,
}

impl PaperSize {
    /// Width in mm.
    pub fn width_mm(&self) -> f32 {
        match self {
            PaperSize::A4 => 210.0,
            PaperSize::Letter => 215.9,
        }
    }

    /// Height in mm.
    pub fn height_mm(&self) -> f32 {
        match self {
            PaperSize::A4 => 297.0,
            PaperSize::Letter => 279.4,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PaperSize::A4 => "A4",
            PaperSize::Letter => "Letter",
        }
    }
}

// ── Print configuration ─────────────────────────────────────────────

/// Configuration for PDF export.
#[derive(Clone)]
pub struct PrintConfig {
    /// Include line numbers in the left margin.
    pub line_numbers: bool,
    /// Include syntax highlighting colors.
    pub syntax_highlighting: bool,
    /// Font size in points for the body text.
    pub font_size: f32,
    /// Paper size.
    pub paper_size: PaperSize,
    /// Page margins in mm (top, right, bottom, left).
    pub margins: (f32, f32, f32, f32),
}

impl Default for PrintConfig {
    fn default() -> Self {
        Self {
            line_numbers: true,
            syntax_highlighting: true,
            font_size: 10.0,
            paper_size: PaperSize::A4,
            margins: (15.0, 15.0, 15.0, 15.0), // top, right, bottom, left
        }
    }
}

// ── Print dialog state ──────────────────────────────────────────────

/// State for the print dialog UI.
pub struct PrintDialogState {
    /// Whether the dialog is open.
    pub open: bool,
    /// Print configuration.
    pub config: PrintConfig,
    /// If true, export PDF to file. If false, print via system.
    pub export_only: bool,
    /// Status / error message to show in the dialog.
    pub status_message: Option<String>,
}

impl Default for PrintDialogState {
    fn default() -> Self {
        Self {
            open: false,
            config: PrintConfig::default(),
            export_only: false,
            status_message: None,
        }
    }
}

// ── Page break calculation ──────────────────────────────────────────

/// Calculate the number of text lines that fit on a single page.
pub fn lines_per_page(config: &PrintConfig) -> usize {
    let page_height_pt = config.paper_size.height_mm() * (72.0 / 25.4);
    let top_margin_pt = config.margins.0 * (72.0 / 25.4);
    let bottom_margin_pt = config.margins.2 * (72.0 / 25.4);
    // Reserve space for the header (title) at the top of each page.
    let header_height_pt = config.font_size * 1.5 + 6.0;
    // Reserve space for the footer (page number).
    let footer_height_pt = config.font_size + 4.0;

    let usable_height =
        page_height_pt - top_margin_pt - bottom_margin_pt - header_height_pt - footer_height_pt;
    let line_height_pt = config.font_size * 1.3;

    (usable_height / line_height_pt).floor().max(1.0) as usize
}

/// Calculate the total number of pages needed.
pub fn total_pages(total_lines: usize, config: &PrintConfig) -> usize {
    let lpp = lines_per_page(config);
    if total_lines == 0 {
        return 1;
    }
    (total_lines + lpp - 1) / lpp
}

// ── PDF generation ──────────────────────────────────────────────────

/// Convert an egui `Color32` to a printpdf RGB `Color`.
fn egui_to_pdf_color(c: egui::Color32) -> Color {
    Color::Rgb(Rgb::new(
        c.r() as f32 / 255.0,
        c.g() as f32 / 255.0,
        c.b() as f32 / 255.0,
        None,
    ))
}

/// Width of the line-number gutter in mm, based on the number of total lines.
fn gutter_width_mm(total_lines: usize, font_size_pt: f32) -> f32 {
    let digits = format!("{}", total_lines).len().max(3);
    // Courier: average glyph width ~ 0.6 * font_size in points.
    // Convert to mm: pt / (72/25.4) = pt * 25.4/72
    let char_width_mm = font_size_pt * 0.6 * 25.4 / 72.0;
    // digits + 2 chars padding
    (digits as f32 + 2.0) * char_width_mm
}

/// Generate a PDF document from source lines.
///
/// `title` is displayed as the header on each page.
/// `lines` are the source text lines (without trailing newlines).
/// `highlight_spans` is an optional per-line list of syntax highlight spans.
/// `syntax_colors` provides the color mapping for highlight indices.
pub fn generate_pdf(
    title: &str,
    lines: &[String],
    highlight_spans: Option<&Vec<Vec<HighlightSpan>>>,
    syntax_colors: Option<&SyntaxColors>,
    config: &PrintConfig,
) -> Vec<u8> {
    let mut doc = PdfDocument::new(title);
    let mut warnings = Vec::new();

    let page_w = config.paper_size.width_mm();
    let page_h = config.paper_size.height_mm();
    let (margin_top, _margin_right, margin_bottom, margin_left) = config.margins;

    let font_size = Pt(config.font_size);
    let line_height_pt = config.font_size * 1.3;
    let lpp = lines_per_page(config);

    let header_height_pt = config.font_size * 1.5 + 6.0;

    let gutter_w = if config.line_numbers {
        gutter_width_mm(lines.len(), config.font_size)
    } else {
        0.0
    };

    let total_lines = lines.len().max(1);
    let num_pages = total_pages(total_lines, config);

    let mut pages = Vec::new();

    for page_idx in 0..num_pages {
        let start_line = page_idx * lpp;
        let end_line = (start_line + lpp).min(lines.len());

        let mut ops: Vec<Op> = Vec::new();

        // ── Header: title ──
        let header_y_mm = page_h - margin_top;
        ops.push(Op::SetFillColor {
            col: Color::Rgb(Rgb::new(0.2, 0.2, 0.2, None)),
        });
        ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(margin_left), Mm(header_y_mm)),
        });
        ops.push(Op::SetLineHeight {
            lh: Pt(config.font_size * 1.5),
        });
        ops.push(Op::WriteTextBuiltinFont {
            items: vec![TextItem::Text(title.to_string())],
            font: BuiltinFont::CourierBold,
        });

        // ── Body text ──
        let body_start_y_mm = page_h - margin_top - (header_height_pt * 25.4 / 72.0);
        let text_x_mm = margin_left + gutter_w;

        for (i, line_offset) in (start_line..end_line).enumerate() {
            let y_mm = body_start_y_mm - (i as f32) * (line_height_pt * 25.4 / 72.0);

            // Draw line number
            if config.line_numbers {
                let line_num_str = format!(
                    "{:>width$}  ",
                    line_offset + 1,
                    width = format!("{}", total_lines).len().max(3)
                );
                ops.push(Op::SetFillColor {
                    col: Color::Rgb(Rgb::new(0.5, 0.5, 0.5, None)),
                });
                ops.push(Op::SetTextCursor {
                    pos: Point::new(Mm(margin_left), Mm(y_mm)),
                });
                ops.push(Op::SetLineHeight {
                    lh: Pt(line_height_pt),
                });
                ops.push(Op::SetFontSizeBuiltinFont {
                    size: font_size,
                    font: BuiltinFont::Courier,
                });
                ops.push(Op::WriteTextBuiltinFont {
                    items: vec![TextItem::Text(line_num_str)],
                    font: BuiltinFont::Courier,
                });
            }

            // Draw text content
            let line_text = if line_offset < lines.len() {
                &lines[line_offset]
            } else {
                ""
            };

            // Replace tabs with spaces for PDF rendering
            let line_text = line_text.replace('\t', "    ");

            let use_highlighting =
                config.syntax_highlighting && highlight_spans.is_some() && syntax_colors.is_some();

            if use_highlighting {
                let spans_list = highlight_spans.unwrap();
                let colors = syntax_colors.unwrap();

                if line_offset < spans_list.len() && !spans_list[line_offset].is_empty() {
                    // Render with syntax-highlighted segments
                    render_highlighted_line(
                        &mut ops,
                        &line_text,
                        &spans_list[line_offset],
                        colors,
                        text_x_mm,
                        y_mm,
                        config.font_size,
                        line_height_pt,
                    );
                } else {
                    // No spans for this line, render as plain text
                    render_plain_line(
                        &mut ops,
                        &line_text,
                        text_x_mm,
                        y_mm,
                        config.font_size,
                        line_height_pt,
                    );
                }
            } else {
                render_plain_line(
                    &mut ops,
                    &line_text,
                    text_x_mm,
                    y_mm,
                    config.font_size,
                    line_height_pt,
                );
            }
        }

        // ── Footer: page number ──
        let footer_y_mm = margin_bottom;
        let page_label = format!("Page {} of {}", page_idx + 1, num_pages);
        ops.push(Op::SetFillColor {
            col: Color::Rgb(Rgb::new(0.4, 0.4, 0.4, None)),
        });
        ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(page_w / 2.0 - 10.0), Mm(footer_y_mm)),
        });
        ops.push(Op::SetLineHeight {
            lh: Pt(config.font_size),
        });
        ops.push(Op::SetFontSizeBuiltinFont {
            size: Pt(config.font_size * 0.85),
            font: BuiltinFont::Courier,
        });
        ops.push(Op::WriteTextBuiltinFont {
            items: vec![TextItem::Text(page_label)],
            font: BuiltinFont::Courier,
        });

        pages.push(PdfPage::new(Mm(page_w), Mm(page_h), ops));
    }

    doc.with_pages(pages);
    doc.save(&PdfSaveOptions::default(), &mut warnings)
}

/// Render a single line of plain text (no syntax highlighting).
fn render_plain_line(
    ops: &mut Vec<Op>,
    text: &str,
    x_mm: f32,
    y_mm: f32,
    font_size: f32,
    line_height_pt: f32,
) {
    ops.push(Op::SetFillColor {
        col: Color::Rgb(Rgb::new(0.1, 0.1, 0.1, None)),
    });
    ops.push(Op::SetTextCursor {
        pos: Point::new(Mm(x_mm), Mm(y_mm)),
    });
    ops.push(Op::SetLineHeight {
        lh: Pt(line_height_pt),
    });
    ops.push(Op::SetFontSizeBuiltinFont {
        size: Pt(font_size),
        font: BuiltinFont::Courier,
    });
    ops.push(Op::WriteTextBuiltinFont {
        items: vec![TextItem::Text(text.to_string())],
        font: BuiltinFont::Courier,
    });
}

/// Render a single line with syntax highlighting.
///
/// Splits the line into segments based on highlight spans and colors each
/// segment according to the theme's syntax colors.
fn render_highlighted_line(
    ops: &mut Vec<Op>,
    text: &str,
    spans: &[HighlightSpan],
    syntax_colors: &SyntaxColors,
    x_mm: f32,
    y_mm: f32,
    font_size: f32,
    line_height_pt: f32,
) {
    // Build colored segments from spans
    let chars: Vec<char> = text.chars().collect();
    let text_len = chars.len();
    if text_len == 0 {
        return;
    }

    // Build a list of (start_col, end_col, color_option) covering the whole line
    let mut segments: Vec<(usize, usize, Option<egui::Color32>)> = Vec::new();
    let mut pos = 0;

    // Sort spans by start column
    let mut sorted_spans: Vec<&HighlightSpan> = spans.iter().collect();
    sorted_spans.sort_by_key(|s| s.start_col);

    for span in &sorted_spans {
        let start = span.start_col.min(text_len);
        let end = span.end_col.min(text_len);
        if start >= end {
            continue;
        }
        // Gap before this span (unhighlighted text)
        if pos < start {
            segments.push((pos, start, None));
        }
        let color = syntax_colors.color_for_highlight(span.highlight_idx);
        segments.push((start, end, color));
        pos = end;
    }
    // Remaining text after last span
    if pos < text_len {
        segments.push((pos, text_len, None));
    }

    // If no segments were produced, render the whole line plain
    if segments.is_empty() {
        render_plain_line(ops, text, x_mm, y_mm, font_size, line_height_pt);
        return;
    }

    // Approximate char width in mm for Courier at the given font size.
    // Courier is a monospace font; width ~ 0.6 * font_size in points.
    let char_width_mm = font_size * 0.6 * 25.4 / 72.0;

    for (start, end, color_opt) in &segments {
        let segment_text: String = chars[*start..*end].iter().collect();
        let seg_x_mm = x_mm + (*start as f32) * char_width_mm;

        let pdf_color = match color_opt {
            Some(c) => egui_to_pdf_color(*c),
            None => Color::Rgb(Rgb::new(0.1, 0.1, 0.1, None)),
        };

        ops.push(Op::SetFillColor { col: pdf_color });
        ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(seg_x_mm), Mm(y_mm)),
        });
        ops.push(Op::SetLineHeight {
            lh: Pt(line_height_pt),
        });
        ops.push(Op::SetFontSizeBuiltinFont {
            size: Pt(font_size),
            font: BuiltinFont::Courier,
        });
        ops.push(Op::WriteTextBuiltinFont {
            items: vec![TextItem::Text(segment_text)],
            font: BuiltinFont::Courier,
        });
    }
}

// ── System print (open PDF with default viewer) ─────────────────────

/// Open a file with the system's default application.
/// On Linux: `xdg-open`, on macOS: `open`, on Windows: `cmd /c start`.
pub fn open_with_system(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open with xdg-open: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open with open: {}", e))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Failed to open with start: {}", e))?;
    }
    Ok(())
}

// ── Print dialog UI ─────────────────────────────────────────────────

/// Render the print dialog. Returns `Some(true)` if the user clicked Print/Export,
/// `Some(false)` if cancelled, or `None` if the dialog is still open.
pub fn render_print_dialog(ctx: &egui::Context, state: &mut PrintDialogState) -> Option<bool> {
    if !state.open {
        return None;
    }

    let mut result = None;
    let mut open = state.open;

    let title = if state.export_only {
        "Export to PDF"
    } else {
        "Print"
    };

    egui::Window::new(title)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([320.0, 280.0])
        .show(ctx, |ui| {
            ui.heading(title);
            ui.add_space(8.0);

            // Paper size
            ui.horizontal(|ui| {
                ui.label("Paper size:");
                if ui
                    .selectable_label(state.config.paper_size == PaperSize::A4, "A4")
                    .clicked()
                {
                    state.config.paper_size = PaperSize::A4;
                }
                if ui
                    .selectable_label(state.config.paper_size == PaperSize::Letter, "Letter")
                    .clicked()
                {
                    state.config.paper_size = PaperSize::Letter;
                }
            });

            ui.add_space(4.0);

            // Font size
            ui.horizontal(|ui| {
                ui.label("Font size:");
                ui.add(
                    egui::DragValue::new(&mut state.config.font_size)
                        .range(6.0..=24.0)
                        .speed(0.5)
                        .suffix(" pt"),
                );
            });

            ui.add_space(4.0);

            // Checkboxes
            ui.checkbox(&mut state.config.line_numbers, "Include line numbers");
            ui.checkbox(
                &mut state.config.syntax_highlighting,
                "Include syntax highlighting",
            );

            ui.add_space(8.0);

            // Status message
            if let Some(ref msg) = state.status_message {
                ui.colored_label(egui::Color32::from_rgb(200, 80, 80), msg);
                ui.add_space(4.0);
            }

            ui.separator();
            ui.add_space(4.0);

            // Buttons
            ui.horizontal(|ui| {
                let button_label = if state.export_only {
                    "Export PDF"
                } else {
                    "Print"
                };
                if ui.button(button_label).clicked() {
                    result = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    result = Some(false);
                    state.open = false;
                }
            });
        });

    state.open = open;
    if !open {
        // Window close button was clicked
        result = Some(false);
    }
    result
}

// ── Helper: collect lines from a document buffer ────────────────────

/// Extract lines from document text (splitting by newline, no trailing newlines).
pub fn text_to_lines(text: &str) -> Vec<String> {
    text.lines().map(|l| l.to_string()).collect()
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lines_per_page_a4_default() {
        let config = PrintConfig::default();
        let lpp = lines_per_page(&config);
        // A4 at 10pt with 15mm margins should give roughly 55-65 lines
        assert!(lpp > 40, "Expected >40 lines per page, got {}", lpp);
        assert!(lpp < 80, "Expected <80 lines per page, got {}", lpp);
    }

    #[test]
    fn test_lines_per_page_letter() {
        let config = PrintConfig {
            paper_size: PaperSize::Letter,
            ..Default::default()
        };
        let lpp = lines_per_page(&config);
        assert!(lpp > 40, "Expected >40 lines per page, got {}", lpp);
        assert!(lpp < 80, "Expected <80 lines per page, got {}", lpp);
    }

    #[test]
    fn test_lines_per_page_larger_font() {
        let small = PrintConfig {
            font_size: 8.0,
            ..Default::default()
        };
        let large = PrintConfig {
            font_size: 16.0,
            ..Default::default()
        };
        assert!(
            lines_per_page(&small) > lines_per_page(&large),
            "Smaller font should fit more lines"
        );
    }

    #[test]
    fn test_total_pages_empty() {
        let config = PrintConfig::default();
        assert_eq!(total_pages(0, &config), 1);
    }

    #[test]
    fn test_total_pages_one_page() {
        let config = PrintConfig::default();
        let lpp = lines_per_page(&config);
        assert_eq!(total_pages(lpp, &config), 1);
        assert_eq!(total_pages(1, &config), 1);
    }

    #[test]
    fn test_total_pages_multi() {
        let config = PrintConfig::default();
        let lpp = lines_per_page(&config);
        assert_eq!(total_pages(lpp + 1, &config), 2);
        assert_eq!(total_pages(lpp * 3, &config), 3);
        assert_eq!(total_pages(lpp * 3 + 1, &config), 4);
    }

    #[test]
    fn test_text_to_lines() {
        let text = "hello\nworld\nfoo";
        let lines = text_to_lines(text);
        assert_eq!(lines, vec!["hello", "world", "foo"]);
    }

    #[test]
    fn test_text_to_lines_empty() {
        let lines = text_to_lines("");
        assert!(lines.is_empty());
    }

    #[test]
    fn test_text_to_lines_trailing_newline() {
        let text = "a\nb\n";
        let lines = text_to_lines(text);
        assert_eq!(lines, vec!["a", "b"]);
    }

    #[test]
    fn test_generate_pdf_produces_bytes() {
        let lines = vec!["Hello, world!".to_string(), "Line 2".to_string()];
        let config = PrintConfig::default();
        let pdf = generate_pdf("Test Document", &lines, None, None, &config);
        // PDF files start with %PDF
        assert!(pdf.len() > 100, "PDF should have reasonable size");
        let header = String::from_utf8_lossy(&pdf[..5]);
        assert!(
            header.starts_with("%PDF"),
            "Should be a valid PDF header, got: {}",
            header
        );
    }

    #[test]
    fn test_generate_pdf_empty_document() {
        let lines: Vec<String> = vec![];
        let config = PrintConfig::default();
        let pdf = generate_pdf("Empty", &lines, None, None, &config);
        assert!(pdf.len() > 50, "Even empty PDF should have some bytes");
    }

    #[test]
    fn test_generate_pdf_multi_page() {
        let config = PrintConfig::default();
        let lpp = lines_per_page(&config);
        // Create enough lines to span multiple pages
        let lines: Vec<String> = (0..lpp * 3 + 5)
            .map(|i| format!("Line {}", i + 1))
            .collect();
        let pdf = generate_pdf("Multi Page", &lines, None, None, &config);
        assert!(pdf.len() > 100);
        let header = String::from_utf8_lossy(&pdf[..5]);
        assert!(header.starts_with("%PDF"));
    }

    #[test]
    fn test_generate_pdf_no_line_numbers() {
        let lines = vec!["Hello".to_string()];
        let config = PrintConfig {
            line_numbers: false,
            ..Default::default()
        };
        let pdf = generate_pdf("No Numbers", &lines, None, None, &config);
        assert!(pdf.len() > 50);
    }

    #[test]
    fn test_generate_pdf_with_tabs() {
        let lines = vec!["\tindented\t\tline".to_string()];
        let config = PrintConfig::default();
        let pdf = generate_pdf("Tabs", &lines, None, None, &config);
        assert!(pdf.len() > 50);
    }

    #[test]
    fn test_paper_size_dimensions() {
        assert_eq!(PaperSize::A4.width_mm(), 210.0);
        assert_eq!(PaperSize::A4.height_mm(), 297.0);
        assert_eq!(PaperSize::Letter.width_mm(), 215.9);
        assert_eq!(PaperSize::Letter.height_mm(), 279.4);
    }

    #[test]
    fn test_paper_size_label() {
        assert_eq!(PaperSize::A4.label(), "A4");
        assert_eq!(PaperSize::Letter.label(), "Letter");
    }

    #[test]
    fn test_gutter_width_scales_with_line_count() {
        let small = gutter_width_mm(10, 10.0);
        let large = gutter_width_mm(10000, 10.0);
        assert!(
            large > small,
            "Gutter should be wider for more lines: {} vs {}",
            large,
            small
        );
    }

    #[test]
    fn test_egui_to_pdf_color() {
        let white = egui_to_pdf_color(egui::Color32::WHITE);
        match white {
            Color::Rgb(rgb) => {
                assert!((rgb.r - 1.0).abs() < 0.01);
                assert!((rgb.g - 1.0).abs() < 0.01);
                assert!((rgb.b - 1.0).abs() < 0.01);
            }
            _ => panic!("Expected RGB color"),
        }

        let black = egui_to_pdf_color(egui::Color32::BLACK);
        match black {
            Color::Rgb(rgb) => {
                assert!((rgb.r).abs() < 0.01);
                assert!((rgb.g).abs() < 0.01);
                assert!((rgb.b).abs() < 0.01);
            }
            _ => panic!("Expected RGB color"),
        }
    }

    #[test]
    fn test_generate_pdf_with_highlight_spans() {
        use crate::theme::SyntaxColors;
        use openedit_core::syntax::HighlightSpan;

        let lines = vec!["fn main() {}".to_string(), "    let x = 42;".to_string()];
        let spans = vec![
            vec![HighlightSpan {
                start_col: 0,
                end_col: 2,
                highlight_idx: 11, // keyword
            }],
            vec![
                HighlightSpan {
                    start_col: 4,
                    end_col: 7,
                    highlight_idx: 11, // keyword (let)
                },
                HighlightSpan {
                    start_col: 12,
                    end_col: 14,
                    highlight_idx: 14, // number
                },
            ],
        ];
        let colors = SyntaxColors::dark();
        let config = PrintConfig::default();

        let pdf = generate_pdf("Highlighted", &lines, Some(&spans), Some(&colors), &config);
        assert!(pdf.len() > 100);
        let header = String::from_utf8_lossy(&pdf[..5]);
        assert!(header.starts_with("%PDF"));
    }

    #[test]
    fn test_default_print_config() {
        let config = PrintConfig::default();
        assert!(config.line_numbers);
        assert!(config.syntax_highlighting);
        assert_eq!(config.font_size, 10.0);
        assert_eq!(config.paper_size, PaperSize::A4);
        assert_eq!(config.margins, (15.0, 15.0, 15.0, 15.0));
    }

    #[test]
    fn test_lines_per_page_with_wide_margins() {
        let narrow = PrintConfig {
            margins: (10.0, 10.0, 10.0, 10.0),
            ..Default::default()
        };
        let wide = PrintConfig {
            margins: (30.0, 30.0, 30.0, 30.0),
            ..Default::default()
        };
        assert!(
            lines_per_page(&narrow) > lines_per_page(&wide),
            "Narrow margins should allow more lines"
        );
    }
}
