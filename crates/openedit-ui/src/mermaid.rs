//! Simple Mermaid diagram parser and egui renderer.
//!
//! Supports:
//! - Flowcharts (`graph`/`flowchart` with LR/TD/TB/RL/BT directions)
//! - Sequence diagrams (`sequenceDiagram`)
//!
//! This is intentionally a minimal implementation covering the ~80% use-case,
//! not a full Mermaid specification parser.

use egui::{Color32, Pos2, Rect, Stroke, Vec2};

// ────────────────────────────────────────────────────────────────────────────
// Data types
// ────────────────────────────────────────────────────────────────────────────

/// Top-level parsed Mermaid diagram.
#[derive(Debug, Clone, PartialEq)]
pub enum MermaidDiagram {
    Flowchart(Flowchart),
    Sequence(SequenceDiagram),
    /// Could not parse – will be rendered as a code block with an error hint.
    Unknown(String),
}

// ── Flowchart ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowDirection {
    TopDown,   // TD / TB
    LeftRight, // LR
    RightLeft, // RL
    BottomTop, // BT
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeShape {
    Rect,     // [text]
    Round,    // (text)
    Diamond,  // {text}
    Stadium,  // ([text])
    Hexagon,  // {{text}}
    Cylinder, // [(text)]
    Default,  // plain id
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowNode {
    pub id: String,
    pub label: String,
    pub shape: NodeShape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeStyle {
    Solid,  // -->
    Dotted, // -.->
    Thick,  // ==>
    Plain,  // ---
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub style: EdgeStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Flowchart {
    pub direction: FlowDirection,
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
}

// ── Sequence ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrowKind {
    Solid,      // ->>
    Dashed,     // -->>
    SolidLine,  // ->
    DashedLine, // -->
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeqMessage {
    pub from: String,
    pub to: String,
    pub label: String,
    pub arrow: ArrowKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequenceDiagram {
    pub participants: Vec<String>,
    pub messages: Vec<SeqMessage>,
}

// ────────────────────────────────────────────────────────────────────────────
// Parser
// ────────────────────────────────────────────────────────────────────────────

/// Parse mermaid source text into a diagram.
pub fn parse_mermaid(src: &str) -> MermaidDiagram {
    let trimmed = src.trim();
    let first_line = trimmed.lines().next().unwrap_or("").trim();

    if first_line.starts_with("graph") || first_line.starts_with("flowchart") {
        match parse_flowchart(trimmed) {
            Some(fc) => MermaidDiagram::Flowchart(fc),
            None => MermaidDiagram::Unknown(src.to_string()),
        }
    } else if first_line == "sequenceDiagram" {
        match parse_sequence(trimmed) {
            Some(sd) => MermaidDiagram::Sequence(sd),
            None => MermaidDiagram::Unknown(src.to_string()),
        }
    } else {
        MermaidDiagram::Unknown(src.to_string())
    }
}

/// Parse the direction token from the first line of a graph/flowchart.
fn parse_direction(first_line: &str) -> FlowDirection {
    let tokens: Vec<&str> = first_line.split_whitespace().collect();
    if tokens.len() >= 2 {
        match tokens[1].to_uppercase().as_str() {
            "LR" => FlowDirection::LeftRight,
            "RL" => FlowDirection::RightLeft,
            "BT" => FlowDirection::BottomTop,
            "TD" | "TB" => FlowDirection::TopDown,
            _ => FlowDirection::TopDown,
        }
    } else {
        FlowDirection::TopDown
    }
}

/// Parse a flowchart from mermaid source.
fn parse_flowchart(src: &str) -> Option<Flowchart> {
    let mut lines = src.lines();
    let first_line = lines.next()?.trim();
    let direction = parse_direction(first_line);

    let mut nodes: Vec<FlowNode> = Vec::new();
    let mut edges: Vec<FlowEdge> = Vec::new();

    // Helper: ensure a node with given id exists; if not, add a default one.
    fn ensure_node(nodes: &mut Vec<FlowNode>, id: &str) {
        if !nodes.iter().any(|n| n.id == id) {
            nodes.push(FlowNode {
                id: id.to_string(),
                label: id.to_string(),
                shape: NodeShape::Default,
            });
        }
    }

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        // Skip subgraph / end / style / class directives
        if line.starts_with("subgraph")
            || line == "end"
            || line.starts_with("style ")
            || line.starts_with("class ")
            || line.starts_with("classDef ")
            || line.starts_with("click ")
            || line.starts_with("linkStyle ")
        {
            continue;
        }

        // Try to parse as an edge line: A --> B or A -->|label| B etc.
        if let Some(edge_result) = try_parse_edge_line(line) {
            for (from_id, from_node, to_id, to_node, edge) in edge_result {
                // Register / update nodes
                if let Some(n) = from_node {
                    update_or_insert_node(&mut nodes, n);
                } else {
                    ensure_node(&mut nodes, &from_id);
                }
                if let Some(n) = to_node {
                    update_or_insert_node(&mut nodes, n);
                } else {
                    ensure_node(&mut nodes, &to_id);
                }
                edges.push(edge);
            }
        } else if let Some(node) = try_parse_node_def(line) {
            // Standalone node definition: A[Label]
            update_or_insert_node(&mut nodes, node);
        }
    }

    if nodes.is_empty() {
        return None;
    }
    Some(Flowchart {
        direction,
        nodes,
        edges,
    })
}

/// Update an existing node (upgrading label/shape) or insert a new one.
fn update_or_insert_node(nodes: &mut Vec<FlowNode>, node: FlowNode) {
    if let Some(existing) = nodes.iter_mut().find(|n| n.id == node.id) {
        // Only upgrade from Default shape
        if (existing.shape == NodeShape::Default && node.shape != NodeShape::Default)
            || (existing.label == existing.id && node.label != node.id)
        {
            existing.label = node.label;
            existing.shape = node.shape;
        }
    } else {
        nodes.push(node);
    }
}

/// Attempt to parse a standalone node definition like `A[Some label]` or `B(Round)`.
fn try_parse_node_def(line: &str) -> Option<FlowNode> {
    // Must contain a bracket-like shape delimiter and NOT an arrow
    if contains_arrow(line) {
        return None;
    }
    parse_node_token(line.trim().trim_end_matches(';'))
}

/// Check if line contains an edge arrow.
fn contains_arrow(s: &str) -> bool {
    s.contains("-->")
        || s.contains("---")
        || s.contains("-.->")
        || s.contains("-.-")
        || s.contains("==>")
        || s.contains("===")
        || s.contains("-->")
}

/// Try to parse a line that contains edges (may be chained: A --> B --> C).
fn try_parse_edge_line(
    line: &str,
) -> Option<Vec<(String, Option<FlowNode>, String, Option<FlowNode>, FlowEdge)>> {
    let line = line.trim().trim_end_matches(';');

    // Find all arrow positions and split into segments.
    // Strategy: scan for arrow patterns, split on them, keeping track of styles and labels.
    let segments = split_on_arrows(line)?;
    if segments.len() < 2 {
        return None;
    }

    let mut results = Vec::new();
    for i in 0..segments.len() - 1 {
        let from_token = segments[i].token.trim();
        let to_token = segments[i + 1].token.trim();

        let from_node = parse_node_token(from_token);
        let to_node = parse_node_token(to_token);
        let from_id = from_node
            .as_ref()
            .map(|n| n.id.clone())
            .unwrap_or_else(|| from_token.to_string());
        let to_id = to_node
            .as_ref()
            .map(|n| n.id.clone())
            .unwrap_or_else(|| to_token.to_string());

        let edge = FlowEdge {
            from: from_id.clone(),
            to: to_id.clone(),
            label: segments[i].edge_label.clone(),
            style: segments[i].edge_style.clone(),
        };
        results.push((from_id, from_node, to_id, to_node, edge));
    }

    if results.is_empty() {
        None
    } else {
        Some(results)
    }
}

struct ArrowSegment {
    token: String,
    edge_style: EdgeStyle,
    edge_label: Option<String>,
}

/// Split a line on arrow operators, returning tokens between arrows and their styles/labels.
fn split_on_arrows(line: &str) -> Option<Vec<ArrowSegment>> {
    // We need to find arrow patterns while being careful about brackets.
    // arrows: -.-> ==> --> --- -.- ===
    // labels: -->|text| or -- text -->

    let mut segments: Vec<ArrowSegment> = Vec::new();
    let mut remaining = line;

    loop {
        // Find the next arrow in remaining
        if let Some((before, style, label, after)) = find_next_arrow(remaining) {
            let token = before.trim().to_string();
            if !token.is_empty() || !segments.is_empty() {
                if segments.is_empty() {
                    // First token, no edge yet
                    segments.push(ArrowSegment {
                        token,
                        edge_style: style,
                        edge_label: label,
                    });
                } else {
                    // Update the last segment's edge info and push this token
                    segments.push(ArrowSegment {
                        token,
                        edge_style: style,
                        edge_label: label,
                    });
                }
            }
            remaining = after;
        } else {
            // No more arrows – the rest is the final token
            let token = remaining.trim().to_string();
            if !token.is_empty() {
                segments.push(ArrowSegment {
                    token,
                    edge_style: EdgeStyle::Solid, // won't be used for last
                    edge_label: None,
                });
            }
            break;
        }
    }

    if segments.len() >= 2 {
        Some(segments)
    } else {
        None
    }
}

/// Find the next arrow in the string. Returns (before, style, label, after).
fn find_next_arrow(s: &str) -> Option<(&str, EdgeStyle, Option<String>, &str)> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    // Track bracket depth to avoid matching inside node definitions
    let mut bracket_depth: i32 = 0;

    let mut i = 0;
    while i < len {
        match bytes[i] {
            b'[' | b'(' | b'{' => bracket_depth += 1,
            b']' | b')' | b'}' => bracket_depth = (bracket_depth - 1).max(0),
            _ => {}
        }
        if bracket_depth > 0 {
            i += 1;
            continue;
        }

        // Try matching arrow patterns (longest first)
        // -.-> (dotted arrow)
        if i + 4 <= len && &s[i..i + 4] == "-.->" {
            let label = extract_pipe_label(&s[i + 4..]);
            let skip = 4 + label.as_ref().map(|l| l.len() + 2).unwrap_or(0);
            return Some((&s[..i], EdgeStyle::Dotted, label, &s[i + skip..]));
        }
        // -.- (dotted line, no arrow)
        if i + 3 <= len && &s[i..i + 3] == "-.-" && (i + 3 >= len || bytes[i + 3] != b'>') {
            let label = extract_pipe_label(&s[i + 3..]);
            let skip = 3 + label.as_ref().map(|l| l.len() + 2).unwrap_or(0);
            return Some((&s[..i], EdgeStyle::Dotted, label, &s[i + skip..]));
        }
        // ==> (thick arrow)
        if i + 3 <= len && &s[i..i + 3] == "==>" {
            let label = extract_pipe_label(&s[i + 3..]);
            let skip = 3 + label.as_ref().map(|l| l.len() + 2).unwrap_or(0);
            return Some((&s[..i], EdgeStyle::Thick, label, &s[i + skip..]));
        }
        // === (thick line)
        if i + 3 <= len && &s[i..i + 3] == "===" && (i + 3 >= len || bytes[i + 3] != b'>') {
            let label = extract_pipe_label(&s[i + 3..]);
            let skip = 3 + label.as_ref().map(|l| l.len() + 2).unwrap_or(0);
            return Some((&s[..i], EdgeStyle::Thick, label, &s[i + skip..]));
        }
        // --> (solid arrow) - check this is not part of --->
        if i + 3 <= len && &s[i..i + 3] == "-->" {
            let label = extract_pipe_label(&s[i + 3..]);
            let skip = 3 + label.as_ref().map(|l| l.len() + 2).unwrap_or(0);
            return Some((&s[..i], EdgeStyle::Solid, label, &s[i + skip..]));
        }
        // -- text --> pattern (edge label between dashes and arrow)
        if i + 2 <= len
            && &s[i..i + 2] == "--"
            && (i + 2 < len && bytes[i + 2] != b'-' && bytes[i + 2] != b'>')
        {
            // Check for "-- label -->" pattern
            if let Some(end_arrow) = s[i + 2..].find("-->") {
                let label_text = s[i + 2..i + 2 + end_arrow].trim().to_string();
                if !label_text.is_empty() {
                    let label = if label_text.is_empty() {
                        None
                    } else {
                        Some(label_text)
                    };
                    return Some((
                        &s[..i],
                        EdgeStyle::Solid,
                        label,
                        &s[i + 2 + end_arrow + 3..],
                    ));
                }
            }
        }
        // --- (plain line)
        if i + 3 <= len && &s[i..i + 3] == "---" && (i + 3 >= len || bytes[i + 3] != b'-') {
            let label = extract_pipe_label(&s[i + 3..]);
            let skip = 3 + label.as_ref().map(|l| l.len() + 2).unwrap_or(0);
            return Some((&s[..i], EdgeStyle::Plain, label, &s[i + skip..]));
        }

        i += 1;
    }
    None
}

/// Extract `|label|` from the start of a string (after an arrow).
fn extract_pipe_label(s: &str) -> Option<String> {
    let s = s.trim_start();
    if let Some(stripped) = s.strip_prefix('|') {
        if let Some(end) = stripped.find('|') {
            let label = stripped[..end].trim().to_string();
            if !label.is_empty() {
                return Some(label);
            }
        }
    }
    None
}

/// Parse a node token like `A`, `A[label]`, `A(label)`, `A{label}`, `A([label])`, etc.
fn parse_node_token(token: &str) -> Option<FlowNode> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }

    // Try various bracket patterns
    // Stadium: id([label])
    if let Some(idx) = token.find("([") {
        let id = token[..idx].trim().to_string();
        if let Some(end) = token.rfind("])") {
            let label = token[idx + 2..end].trim().to_string();
            return Some(FlowNode {
                id,
                label,
                shape: NodeShape::Stadium,
            });
        }
    }
    // Double brace: id{{label}}
    if let Some(idx) = token.find("{{") {
        let id = token[..idx].trim().to_string();
        if let Some(end) = token.rfind("}}") {
            let label = token[idx + 2..end].trim().to_string();
            return Some(FlowNode {
                id,
                label,
                shape: NodeShape::Hexagon,
            });
        }
    }
    // Cylinder: id[(label)]
    if let Some(idx) = token.find("[(") {
        let id = token[..idx].trim().to_string();
        if let Some(end) = token.rfind(")]") {
            let label = token[idx + 2..end].trim().to_string();
            return Some(FlowNode {
                id,
                label,
                shape: NodeShape::Cylinder,
            });
        }
    }
    // Rect: id[label]
    if let Some(idx) = token.find('[') {
        let id = token[..idx].trim().to_string();
        if let Some(end) = token.rfind(']') {
            let label = token[idx + 1..end].trim().to_string();
            return Some(FlowNode {
                id,
                label,
                shape: NodeShape::Rect,
            });
        }
    }
    // Round: id(label)
    if let Some(idx) = token.find('(') {
        let id = token[..idx].trim().to_string();
        if let Some(end) = token.rfind(')') {
            let label = token[idx + 1..end].trim().to_string();
            return Some(FlowNode {
                id,
                label,
                shape: NodeShape::Round,
            });
        }
    }
    // Diamond: id{label}
    if let Some(idx) = token.find('{') {
        let id = token[..idx].trim().to_string();
        if let Some(end) = token.rfind('}') {
            let label = token[idx + 1..end].trim().to_string();
            return Some(FlowNode {
                id,
                label,
                shape: NodeShape::Diamond,
            });
        }
    }

    // Plain id (no shape delimiters) - only valid if it looks like an identifier
    if token
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Some(FlowNode {
            id: token.to_string(),
            label: token.to_string(),
            shape: NodeShape::Default,
        });
    }

    None
}

// ── Sequence diagram parser ─────────────────────────────────────────────────

fn parse_sequence(src: &str) -> Option<SequenceDiagram> {
    let mut participants: Vec<String> = Vec::new();
    let mut messages: Vec<SeqMessage> = Vec::new();

    fn ensure_participant(participants: &mut Vec<String>, name: &str) {
        if !participants.iter().any(|p| p == name) {
            participants.push(name.to_string());
        }
    }

    for line in src.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // participant / actor declarations
        if line.starts_with("participant ") || line.starts_with("actor ") {
            let name = line
                .split_once(' ')
                .map(|x| x.1)
                .unwrap_or("")
                .trim()
                .trim_end_matches(';');
            // Handle "participant A as Alice" syntax
            let name = if let Some(idx) = name.find(" as ") {
                name[idx + 4..].trim()
            } else {
                name
            };
            if !name.is_empty() {
                ensure_participant(&mut participants, name);
            }
            continue;
        }

        // Skip note, loop, alt, opt, rect, end, activate, deactivate
        if line.starts_with("Note ")
            || line.starts_with("note ")
            || line.starts_with("loop ")
            || line.starts_with("alt ")
            || line.starts_with("else ")
            || line.starts_with("opt ")
            || line.starts_with("rect ")
            || line == "end"
            || line.starts_with("activate ")
            || line.starts_with("deactivate ")
        {
            continue;
        }

        // Try to parse as message: From ->> To: Label
        if let Some(msg) = parse_seq_message(line) {
            ensure_participant(&mut participants, &msg.from);
            ensure_participant(&mut participants, &msg.to);
            messages.push(msg);
        }
    }

    if participants.is_empty() && messages.is_empty() {
        return None;
    }

    Some(SequenceDiagram {
        participants,
        messages,
    })
}

/// Parse a sequence diagram message line like `Alice->>Bob: Hello`
fn parse_seq_message(line: &str) -> Option<SeqMessage> {
    // Try arrow patterns (longest first): -->> ->> --> ->
    let arrow_patterns: &[(&str, ArrowKind)] = &[
        ("-->>", ArrowKind::Dashed),
        ("->>", ArrowKind::Solid),
        ("-->", ArrowKind::DashedLine),
        ("->", ArrowKind::SolidLine),
    ];

    for (arrow_str, arrow_kind) in arrow_patterns {
        if let Some(arrow_pos) = line.find(arrow_str) {
            let from = line[..arrow_pos].trim();
            let rest = &line[arrow_pos + arrow_str.len()..];
            let (to, label) = if let Some(colon_pos) = rest.find(':') {
                (
                    rest[..colon_pos].trim(),
                    rest[colon_pos + 1..].trim().to_string(),
                )
            } else {
                (rest.trim(), String::new())
            };

            if !from.is_empty() && !to.is_empty() {
                return Some(SeqMessage {
                    from: from.to_string(),
                    to: to.to_string(),
                    label,
                    arrow: arrow_kind.clone(),
                });
            }
        }
    }
    None
}

// ────────────────────────────────────────────────────────────────────────────
// Renderer (egui Painter)
// ────────────────────────────────────────────────────────────────────────────

/// Colors used for mermaid diagram rendering, derived from the editor theme.
pub struct MermaidColors {
    pub node_fill: Color32,
    pub node_stroke: Color32,
    pub node_text: Color32,
    pub edge_color: Color32,
    pub label_color: Color32,
    pub background: Color32,
    pub participant_fill: Color32,
    pub arrow_color: Color32,
}

/// Layout info for a positioned flowchart node.
struct LayoutNode {
    id: String,
    label: String,
    shape: NodeShape,
    rect: Rect,
}

/// Render a parsed MermaidDiagram into an egui Painter, returning the height consumed.
///
/// `origin` is the top-left corner in screen coordinates.
/// `available_width` is the horizontal space available.
pub fn render_mermaid(
    painter: &egui::Painter,
    diagram: &MermaidDiagram,
    origin: Pos2,
    available_width: f32,
    colors: &MermaidColors,
) -> f32 {
    match diagram {
        MermaidDiagram::Flowchart(fc) => {
            render_flowchart(painter, fc, origin, available_width, colors)
        }
        MermaidDiagram::Sequence(sd) => {
            render_sequence(painter, sd, origin, available_width, colors)
        }
        MermaidDiagram::Unknown(src) => {
            render_unknown(painter, src, origin, available_width, colors)
        }
    }
}

/// Render an unparseable diagram as a styled code block with error notice.
fn render_unknown(
    painter: &egui::Painter,
    src: &str,
    origin: Pos2,
    available_width: f32,
    colors: &MermaidColors,
) -> f32 {
    let line_height = 16.0;
    let padding = 8.0;
    let font = egui::FontId::monospace(12.0);
    let header_font = egui::FontId::proportional(12.0);

    let lines: Vec<&str> = src.lines().collect();
    let header_height = line_height + 4.0;
    let block_height = header_height + lines.len() as f32 * line_height + padding * 2.0;

    // Background
    let bg_rect = Rect::from_min_size(origin, Vec2::new(available_width, block_height));
    painter.rect_filled(bg_rect, 4.0, colors.background);
    painter.rect_stroke(bg_rect, 4.0, Stroke::new(1.0, colors.node_stroke));

    // Header
    painter.text(
        Pos2::new(origin.x + padding, origin.y + padding),
        egui::Align2::LEFT_TOP,
        "Mermaid (unsupported diagram type)",
        header_font,
        colors.label_color,
    );

    // Code lines
    let code_y = origin.y + padding + header_height;
    for (i, line) in lines.iter().enumerate() {
        painter.text(
            Pos2::new(origin.x + padding, code_y + i as f32 * line_height),
            egui::Align2::LEFT_TOP,
            line,
            font.clone(),
            colors.node_text,
        );
    }

    block_height
}

/// Render a flowchart diagram.
fn render_flowchart(
    painter: &egui::Painter,
    fc: &Flowchart,
    origin: Pos2,
    available_width: f32,
    colors: &MermaidColors,
) -> f32 {
    if fc.nodes.is_empty() {
        return 0.0;
    }

    let node_h = 40.0f32;
    let node_padding_x = 24.0f32;
    let node_min_w = 80.0f32;
    let char_width = 8.0f32;
    let h_gap = 40.0f32;
    let v_gap = 50.0f32;
    let margin = 16.0f32;

    // Compute node widths based on label length
    let node_widths: Vec<f32> = fc
        .nodes
        .iter()
        .map(|n| {
            let text_w = n.label.len() as f32 * char_width + node_padding_x * 2.0;
            text_w.max(node_min_w)
        })
        .collect();

    // Simple layout: assign rows and columns using topological-ish ordering.
    // For LR/RL: nodes flow left-to-right in edge order.
    // For TD/BT: nodes flow top-to-bottom.
    let is_horizontal = matches!(
        fc.direction,
        FlowDirection::LeftRight | FlowDirection::RightLeft
    );

    // Build adjacency for topological layers
    let layers = compute_layers(&fc.nodes, &fc.edges);

    // Position nodes
    let mut layout_nodes: Vec<LayoutNode> = Vec::new();

    if is_horizontal {
        // Horizontal: layers go left-to-right, nodes in each layer stacked vertically
        let mut x = origin.x + margin;
        for layer in &layers {
            let layer_width = layer
                .iter()
                .map(|&idx| node_widths[idx])
                .fold(0.0f32, f32::max);
            let total_layer_height =
                layer.len() as f32 * node_h + (layer.len() as f32 - 1.0).max(0.0) * (v_gap * 0.5);
            let start_y = origin.y + margin;

            for (row, &idx) in layer.iter().enumerate() {
                let node = &fc.nodes[idx];
                let w = node_widths[idx];
                let cx = x + layer_width / 2.0;
                let cy = start_y
                    + row as f32 * (node_h + v_gap * 0.5)
                    + total_layer_height * 0.0
                    + node_h / 2.0;
                let r = Rect::from_center_size(Pos2::new(cx, cy), Vec2::new(w, node_h));
                layout_nodes.push(LayoutNode {
                    id: node.id.clone(),
                    label: node.label.clone(),
                    shape: node.shape.clone(),
                    rect: r,
                });
            }
            x += layer_width + h_gap;
        }
    } else {
        // Vertical: layers go top-to-bottom, nodes in each layer placed horizontally
        let mut y_pos = origin.y + margin;
        for layer in &layers {
            let total_layer_width: f32 = layer.iter().map(|&idx| node_widths[idx]).sum::<f32>()
                + (layer.len() as f32 - 1.0).max(0.0) * h_gap;
            let start_x = origin.x + (available_width - total_layer_width) / 2.0;
            let mut x = start_x.max(origin.x + margin);

            for &idx in layer {
                let node = &fc.nodes[idx];
                let w = node_widths[idx];
                let r = Rect::from_min_size(Pos2::new(x, y_pos), Vec2::new(w, node_h));
                layout_nodes.push(LayoutNode {
                    id: node.id.clone(),
                    label: node.label.clone(),
                    shape: node.shape.clone(),
                    rect: r,
                });
                x += w + h_gap;
            }
            y_pos += node_h + v_gap;
        }
    }

    // Compute bounding box
    let mut max_x = origin.x;
    let mut max_y = origin.y;
    for ln in &layout_nodes {
        max_x = max_x.max(ln.rect.right());
        max_y = max_y.max(ln.rect.bottom());
    }
    let diagram_height = max_y - origin.y + margin;
    let diagram_width = max_x - origin.x + margin;

    // Draw background rect
    let bg_rect = Rect::from_min_size(
        origin,
        Vec2::new(diagram_width.min(available_width), diagram_height),
    );
    painter.rect_filled(bg_rect, 4.0, colors.background);

    // Draw edges first (below nodes)
    for edge in &fc.edges {
        let from_layout = layout_nodes.iter().find(|n| n.id == edge.from);
        let to_layout = layout_nodes.iter().find(|n| n.id == edge.to);
        if let (Some(from), Some(to)) = (from_layout, to_layout) {
            draw_edge(painter, from, to, edge, is_horizontal, colors);
        }
    }

    // Draw nodes
    let font = egui::FontId::proportional(13.0);
    for ln in &layout_nodes {
        draw_node(painter, ln, &font, colors);
    }

    diagram_height
}

/// Compute simple layered assignment for graph layout.
fn compute_layers(nodes: &[FlowNode], edges: &[FlowEdge]) -> Vec<Vec<usize>> {
    let n = nodes.len();
    if n == 0 {
        return vec![];
    }

    // Map id -> index
    let id_to_idx: std::collections::HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();

    // Compute in-degree and build adjacency
    let mut in_degree = vec![0usize; n];
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    for edge in edges {
        if let (Some(&from), Some(&to)) = (
            id_to_idx.get(edge.from.as_str()),
            id_to_idx.get(edge.to.as_str()),
        ) {
            adj[from].push(to);
            in_degree[to] += 1;
        }
    }

    // BFS layering (Kahn's algorithm style)
    let mut layer_of = vec![0usize; n];
    let mut queue: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut visited = vec![false; n];

    // If there are no roots (cycle), just pick node 0
    if queue.is_empty() {
        queue.push(0);
    }

    let mut current_layer = 0;
    while !queue.is_empty() {
        let mut next_queue = Vec::new();
        for &idx in &queue {
            if visited[idx] {
                continue;
            }
            visited[idx] = true;
            layer_of[idx] = current_layer;
            for &child in &adj[idx] {
                if !visited[child] {
                    next_queue.push(child);
                }
            }
        }
        current_layer += 1;
        queue = next_queue;
    }

    // Handle any unvisited (disconnected) nodes
    for i in 0..n {
        if !visited[i] {
            layer_of[i] = current_layer;
            visited[i] = true;
        }
    }

    // Group by layer
    let max_layer = layer_of.iter().copied().max().unwrap_or(0);
    let mut layers: Vec<Vec<usize>> = vec![vec![]; max_layer + 1];
    for i in 0..n {
        layers[layer_of[i]].push(i);
    }

    // Remove empty layers
    layers.retain(|l| !l.is_empty());
    layers
}

/// Draw a flowchart node.
fn draw_node(
    painter: &egui::Painter,
    ln: &LayoutNode,
    font: &egui::FontId,
    colors: &MermaidColors,
) {
    let stroke = Stroke::new(2.0, colors.node_stroke);
    let center = ln.rect.center();

    match ln.shape {
        NodeShape::Diamond => {
            // Draw diamond as 4-point polygon
            let cx = center.x;
            let cy = center.y;
            let hw = ln.rect.width() / 2.0;
            let hh = ln.rect.height() / 2.0;
            let points = vec![
                Pos2::new(cx, cy - hh),
                Pos2::new(cx + hw, cy),
                Pos2::new(cx, cy + hh),
                Pos2::new(cx - hw, cy),
            ];
            painter.add(egui::Shape::convex_polygon(
                points,
                colors.node_fill,
                stroke,
            ));
        }
        NodeShape::Round | NodeShape::Stadium => {
            let rounding = ln.rect.height() / 2.0;
            painter.rect(ln.rect, rounding, colors.node_fill, stroke);
        }
        NodeShape::Hexagon => {
            let cx = center.x;
            let cy = center.y;
            let hw = ln.rect.width() / 2.0;
            let hh = ln.rect.height() / 2.0;
            let indent = 12.0f32;
            let points = vec![
                Pos2::new(cx - hw + indent, cy - hh),
                Pos2::new(cx + hw - indent, cy - hh),
                Pos2::new(cx + hw, cy),
                Pos2::new(cx + hw - indent, cy + hh),
                Pos2::new(cx - hw + indent, cy + hh),
                Pos2::new(cx - hw, cy),
            ];
            painter.add(egui::Shape::convex_polygon(
                points,
                colors.node_fill,
                stroke,
            ));
        }
        _ => {
            // Rectangle (with small rounding)
            painter.rect(ln.rect, 4.0, colors.node_fill, stroke);
        }
    }

    // Label text
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        &ln.label,
        font.clone(),
        colors.node_text,
    );
}

/// Draw an edge between two layout nodes.
fn draw_edge(
    painter: &egui::Painter,
    from: &LayoutNode,
    to: &LayoutNode,
    edge: &FlowEdge,
    is_horizontal: bool,
    colors: &MermaidColors,
) {
    let (start, end) = if is_horizontal {
        (
            Pos2::new(from.rect.right(), from.rect.center().y),
            Pos2::new(to.rect.left(), to.rect.center().y),
        )
    } else {
        (
            Pos2::new(from.rect.center().x, from.rect.bottom()),
            Pos2::new(to.rect.center().x, to.rect.top()),
        )
    };

    let (thickness, dash) = match edge.style {
        EdgeStyle::Solid => (1.5, false),
        EdgeStyle::Dotted => (1.5, true),
        EdgeStyle::Thick => (3.0, false),
        EdgeStyle::Plain => (1.5, false),
    };

    let stroke = Stroke::new(thickness, colors.edge_color);

    if dash {
        // Draw dashed line
        draw_dashed_line(painter, start, end, stroke, 6.0, 4.0);
    } else {
        painter.line_segment([start, end], stroke);
    }

    // Arrowhead (for non-Plain edges)
    if !matches!(edge.style, EdgeStyle::Plain) {
        draw_arrowhead(painter, start, end, colors.edge_color, thickness);
    }

    // Edge label
    if let Some(label) = &edge.label {
        let mid = Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
        let font = egui::FontId::proportional(11.0);

        // Small background for readability
        let label_w = label.len() as f32 * 6.5 + 8.0;
        let label_h = 16.0;
        let label_rect = Rect::from_center_size(mid, Vec2::new(label_w, label_h));
        painter.rect_filled(label_rect, 2.0, colors.background);

        painter.text(
            mid,
            egui::Align2::CENTER_CENTER,
            label,
            font,
            colors.label_color,
        );
    }
}

/// Draw a dashed line between two points.
fn draw_dashed_line(
    painter: &egui::Painter,
    from: Pos2,
    to: Pos2,
    stroke: Stroke,
    dash_len: f32,
    gap_len: f32,
) {
    let diff = to - from;
    let total_len = diff.length();
    if total_len < 0.1 {
        return;
    }
    let dir = diff / total_len;
    let mut pos = 0.0;
    while pos < total_len {
        let seg_start = from + dir * pos;
        let seg_end_len = (pos + dash_len).min(total_len);
        let seg_end = from + dir * seg_end_len;
        painter.line_segment([seg_start, seg_end], stroke);
        pos += dash_len + gap_len;
    }
}

/// Draw a small arrowhead at `to`, pointing from `from`.
fn draw_arrowhead(painter: &egui::Painter, from: Pos2, to: Pos2, color: Color32, _thickness: f32) {
    let diff = to - from;
    let len = diff.length();
    if len < 0.1 {
        return;
    }
    let dir = diff / len;
    let perp = Vec2::new(-dir.y, dir.x);

    let arrow_size = 8.0;
    let p1 = to;
    let p2 = to - dir * arrow_size + perp * (arrow_size * 0.4);
    let p3 = to - dir * arrow_size - perp * (arrow_size * 0.4);

    painter.add(egui::Shape::convex_polygon(
        vec![p1, p2, p3],
        color,
        Stroke::NONE,
    ));
}

/// Render a sequence diagram.
fn render_sequence(
    painter: &egui::Painter,
    sd: &SequenceDiagram,
    origin: Pos2,
    available_width: f32,
    colors: &MermaidColors,
) -> f32 {
    if sd.participants.is_empty() {
        return 0.0;
    }

    let participant_h = 36.0f32;
    let participant_min_w = 80.0f32;
    let char_width = 8.0f32;
    let h_gap = 30.0f32;
    let msg_row_h = 40.0f32;
    let margin = 16.0f32;
    let lifeline_top = origin.y + margin + participant_h + 8.0;

    // Compute participant widths
    let participant_widths: Vec<f32> = sd
        .participants
        .iter()
        .map(|p| (p.len() as f32 * char_width + 24.0).max(participant_min_w))
        .collect();

    let total_width: f32 = participant_widths.iter().sum::<f32>()
        + (sd.participants.len() as f32 - 1.0).max(0.0) * h_gap;
    let start_x = origin.x + (available_width - total_width).max(0.0) / 2.0;

    // Compute center x for each participant
    let mut centers_x: Vec<f32> = Vec::new();
    {
        let mut x = start_x;
        for (i, w) in participant_widths.iter().enumerate() {
            centers_x.push(x + w / 2.0);
            if i < participant_widths.len() - 1 {
                x += w + h_gap;
            }
        }
    }

    let diagram_height =
        margin * 2.0 + participant_h * 2.0 + 16.0 + sd.messages.len() as f32 * msg_row_h + 8.0;

    // Background
    let bg_rect = Rect::from_min_size(
        origin,
        Vec2::new(
            available_width.min(total_width + margin * 2.0),
            diagram_height,
        ),
    );
    painter.rect_filled(bg_rect, 4.0, colors.background);

    let font = egui::FontId::proportional(13.0);
    let label_font = egui::FontId::proportional(11.0);

    // Draw participant boxes (top)
    for (i, name) in sd.participants.iter().enumerate() {
        let cx = centers_x[i];
        let w = participant_widths[i];
        let r = Rect::from_center_size(
            Pos2::new(cx, origin.y + margin + participant_h / 2.0),
            Vec2::new(w, participant_h),
        );
        painter.rect(
            r,
            4.0,
            colors.participant_fill,
            Stroke::new(2.0, colors.node_stroke),
        );
        painter.text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            name,
            font.clone(),
            colors.node_text,
        );
    }

    // Draw lifelines
    let lifeline_bottom = lifeline_top + sd.messages.len() as f32 * msg_row_h;
    let lifeline_stroke = Stroke::new(1.0, colors.node_stroke.linear_multiply(0.4));
    for &cx in &centers_x {
        draw_dashed_line(
            painter,
            Pos2::new(cx, lifeline_top),
            Pos2::new(cx, lifeline_bottom),
            lifeline_stroke,
            5.0,
            3.0,
        );
    }

    // Draw participant boxes (bottom, mirrored)
    for (i, name) in sd.participants.iter().enumerate() {
        let cx = centers_x[i];
        let w = participant_widths[i];
        let r = Rect::from_center_size(
            Pos2::new(cx, lifeline_bottom + 8.0 + participant_h / 2.0),
            Vec2::new(w, participant_h),
        );
        painter.rect(
            r,
            4.0,
            colors.participant_fill,
            Stroke::new(2.0, colors.node_stroke),
        );
        painter.text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            name,
            font.clone(),
            colors.node_text,
        );
    }

    // Draw messages
    for (i, msg) in sd.messages.iter().enumerate() {
        let from_idx = sd.participants.iter().position(|p| p == &msg.from);
        let to_idx = sd.participants.iter().position(|p| p == &msg.to);
        if let (Some(fi), Some(ti)) = (from_idx, to_idx) {
            let y = lifeline_top + (i as f32 + 0.5) * msg_row_h;
            let from_x = centers_x[fi];
            let to_x = centers_x[ti];

            let (thickness, dashed) = match msg.arrow {
                ArrowKind::Solid | ArrowKind::SolidLine => (1.5, false),
                ArrowKind::Dashed | ArrowKind::DashedLine => (1.5, true),
            };

            let stroke = Stroke::new(thickness, colors.arrow_color);

            if from_x != to_x {
                if dashed {
                    draw_dashed_line(
                        painter,
                        Pos2::new(from_x, y),
                        Pos2::new(to_x, y),
                        stroke,
                        6.0,
                        4.0,
                    );
                } else {
                    painter.line_segment([Pos2::new(from_x, y), Pos2::new(to_x, y)], stroke);
                }

                // Arrowhead for >> arrows
                match msg.arrow {
                    ArrowKind::Solid | ArrowKind::Dashed => {
                        draw_arrowhead(
                            painter,
                            Pos2::new(from_x, y),
                            Pos2::new(to_x, y),
                            colors.arrow_color,
                            thickness,
                        );
                    }
                    ArrowKind::SolidLine | ArrowKind::DashedLine => {
                        // Open arrowhead (just lines)
                        let dir = if to_x > from_x { 1.0f32 } else { -1.0 };
                        let tip = Pos2::new(to_x, y);
                        let arrow_size = 7.0;
                        let p1 = Pos2::new(tip.x - dir * arrow_size, y - arrow_size * 0.4);
                        let p2 = Pos2::new(tip.x - dir * arrow_size, y + arrow_size * 0.4);
                        painter.line_segment([p1, tip], stroke);
                        painter.line_segment([p2, tip], stroke);
                    }
                }

                // Label above the arrow
                if !msg.label.is_empty() {
                    let mid_x = (from_x + to_x) / 2.0;
                    painter.text(
                        Pos2::new(mid_x, y - 12.0),
                        egui::Align2::CENTER_BOTTOM,
                        &msg.label,
                        label_font.clone(),
                        colors.label_color,
                    );
                }
            } else {
                // Self-message: draw a small loop
                let loop_w = 30.0;
                let loop_h = 15.0;
                painter.line_segment(
                    [Pos2::new(from_x, y), Pos2::new(from_x + loop_w, y)],
                    stroke,
                );
                painter.line_segment(
                    [
                        Pos2::new(from_x + loop_w, y),
                        Pos2::new(from_x + loop_w, y + loop_h),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        Pos2::new(from_x + loop_w, y + loop_h),
                        Pos2::new(from_x, y + loop_h),
                    ],
                    stroke,
                );
                draw_arrowhead(
                    painter,
                    Pos2::new(from_x + loop_w, y + loop_h),
                    Pos2::new(from_x, y + loop_h),
                    colors.arrow_color,
                    thickness,
                );
                if !msg.label.is_empty() {
                    painter.text(
                        Pos2::new(from_x + loop_w + 4.0, y + loop_h / 2.0),
                        egui::Align2::LEFT_CENTER,
                        &msg.label,
                        label_font.clone(),
                        colors.label_color,
                    );
                }
            }
        }
    }

    diagram_height
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Flowchart parsing ──────────────────────────────────────────────────

    #[test]
    fn test_parse_flowchart_basic() {
        let src = "graph TD\n    A[Start] --> B[End]";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Flowchart(fc) => {
                assert_eq!(fc.direction, FlowDirection::TopDown);
                assert_eq!(fc.nodes.len(), 2);
                assert_eq!(fc.nodes[0].id, "A");
                assert_eq!(fc.nodes[0].label, "Start");
                assert_eq!(fc.nodes[0].shape, NodeShape::Rect);
                assert_eq!(fc.nodes[1].id, "B");
                assert_eq!(fc.nodes[1].label, "End");
                assert_eq!(fc.edges.len(), 1);
                assert_eq!(fc.edges[0].from, "A");
                assert_eq!(fc.edges[0].to, "B");
            }
            _ => panic!("Expected Flowchart"),
        }
    }

    #[test]
    fn test_parse_flowchart_lr() {
        let src = "graph LR\n    A --> B --> C";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Flowchart(fc) => {
                assert_eq!(fc.direction, FlowDirection::LeftRight);
                assert_eq!(fc.nodes.len(), 3);
                assert_eq!(fc.edges.len(), 2);
                assert_eq!(fc.edges[0].from, "A");
                assert_eq!(fc.edges[0].to, "B");
                assert_eq!(fc.edges[1].from, "B");
                assert_eq!(fc.edges[1].to, "C");
            }
            _ => panic!("Expected Flowchart"),
        }
    }

    #[test]
    fn test_parse_flowchart_keyword() {
        let src = "flowchart LR\n    A[Start] --> B[End]";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Flowchart(fc) => {
                assert_eq!(fc.direction, FlowDirection::LeftRight);
                assert_eq!(fc.nodes.len(), 2);
            }
            _ => panic!("Expected Flowchart"),
        }
    }

    #[test]
    fn test_parse_node_shapes() {
        assert_eq!(
            parse_node_token("A[Rect]"),
            Some(FlowNode {
                id: "A".into(),
                label: "Rect".into(),
                shape: NodeShape::Rect,
            })
        );
        assert_eq!(
            parse_node_token("B(Round)"),
            Some(FlowNode {
                id: "B".into(),
                label: "Round".into(),
                shape: NodeShape::Round,
            })
        );
        assert_eq!(
            parse_node_token("C{Diamond}"),
            Some(FlowNode {
                id: "C".into(),
                label: "Diamond".into(),
                shape: NodeShape::Diamond,
            })
        );
        assert_eq!(
            parse_node_token("D([Stadium])"),
            Some(FlowNode {
                id: "D".into(),
                label: "Stadium".into(),
                shape: NodeShape::Stadium,
            })
        );
        assert_eq!(
            parse_node_token("E{{Hexagon}}"),
            Some(FlowNode {
                id: "E".into(),
                label: "Hexagon".into(),
                shape: NodeShape::Hexagon,
            })
        );
    }

    #[test]
    fn test_parse_edge_styles() {
        let src = "graph TD\n    A --> B\n    B -.-> C\n    C ==> D\n    D --- E";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Flowchart(fc) => {
                assert_eq!(fc.edges.len(), 4);
                assert_eq!(fc.edges[0].style, EdgeStyle::Solid);
                assert_eq!(fc.edges[1].style, EdgeStyle::Dotted);
                assert_eq!(fc.edges[2].style, EdgeStyle::Thick);
                assert_eq!(fc.edges[3].style, EdgeStyle::Plain);
            }
            _ => panic!("Expected Flowchart"),
        }
    }

    #[test]
    fn test_parse_edge_with_label() {
        let src = "graph TD\n    A -->|yes| B\n    A -->|no| C";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Flowchart(fc) => {
                assert_eq!(fc.edges.len(), 2);
                assert_eq!(fc.edges[0].label, Some("yes".to_string()));
                assert_eq!(fc.edges[1].label, Some("no".to_string()));
            }
            _ => panic!("Expected Flowchart"),
        }
    }

    #[test]
    fn test_parse_edge_with_text_label() {
        let src = "graph TD\n    A -- text --> B";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Flowchart(fc) => {
                assert_eq!(fc.edges.len(), 1);
                assert_eq!(fc.edges[0].label, Some("text".to_string()));
                assert_eq!(fc.edges[0].style, EdgeStyle::Solid);
            }
            _ => panic!("Expected Flowchart"),
        }
    }

    #[test]
    fn test_parse_flowchart_comments() {
        let src = "graph TD\n    %% This is a comment\n    A --> B";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Flowchart(fc) => {
                assert_eq!(fc.nodes.len(), 2);
                assert_eq!(fc.edges.len(), 1);
            }
            _ => panic!("Expected Flowchart"),
        }
    }

    #[test]
    fn test_parse_flowchart_semicolons() {
        let src = "graph TD\n    A[Start] --> B[End];";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Flowchart(fc) => {
                assert_eq!(fc.nodes.len(), 2);
                assert_eq!(fc.edges.len(), 1);
            }
            _ => panic!("Expected Flowchart"),
        }
    }

    #[test]
    fn test_parse_standalone_node() {
        let src = "graph TD\n    A[My Node]\n    A --> B";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Flowchart(fc) => {
                assert_eq!(fc.nodes.len(), 2);
                let a = fc.nodes.iter().find(|n| n.id == "A").unwrap();
                assert_eq!(a.label, "My Node");
                assert_eq!(a.shape, NodeShape::Rect);
            }
            _ => panic!("Expected Flowchart"),
        }
    }

    #[test]
    fn test_parse_direction_variants() {
        assert_eq!(parse_direction("graph TD"), FlowDirection::TopDown);
        assert_eq!(parse_direction("graph TB"), FlowDirection::TopDown);
        assert_eq!(parse_direction("graph LR"), FlowDirection::LeftRight);
        assert_eq!(parse_direction("graph RL"), FlowDirection::RightLeft);
        assert_eq!(parse_direction("graph BT"), FlowDirection::BottomTop);
    }

    // ── Sequence diagram parsing ───────────────────────────────────────────

    #[test]
    fn test_parse_sequence_basic() {
        let src = "sequenceDiagram\n    Alice->>Bob: Hello\n    Bob-->>Alice: Hi";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Sequence(sd) => {
                assert_eq!(sd.participants.len(), 2);
                assert_eq!(sd.participants[0], "Alice");
                assert_eq!(sd.participants[1], "Bob");
                assert_eq!(sd.messages.len(), 2);
                assert_eq!(sd.messages[0].from, "Alice");
                assert_eq!(sd.messages[0].to, "Bob");
                assert_eq!(sd.messages[0].label, "Hello");
                assert_eq!(sd.messages[0].arrow, ArrowKind::Solid);
                assert_eq!(sd.messages[1].arrow, ArrowKind::Dashed);
            }
            _ => panic!("Expected Sequence"),
        }
    }

    #[test]
    fn test_parse_sequence_explicit_participants() {
        let src =
            "sequenceDiagram\n    participant Alice\n    participant Bob\n    Alice->>Bob: Hello";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Sequence(sd) => {
                assert_eq!(sd.participants.len(), 2);
                assert_eq!(sd.participants[0], "Alice");
                assert_eq!(sd.participants[1], "Bob");
                assert_eq!(sd.messages.len(), 1);
            }
            _ => panic!("Expected Sequence"),
        }
    }

    #[test]
    fn test_parse_sequence_arrow_kinds() {
        let src = "sequenceDiagram\n    A->>B: solid\n    B-->>A: dashed\n    A->B: solidline\n    B-->A: dashedline";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Sequence(sd) => {
                assert_eq!(sd.messages.len(), 4);
                assert_eq!(sd.messages[0].arrow, ArrowKind::Solid);
                assert_eq!(sd.messages[1].arrow, ArrowKind::Dashed);
                assert_eq!(sd.messages[2].arrow, ArrowKind::SolidLine);
                assert_eq!(sd.messages[3].arrow, ArrowKind::DashedLine);
            }
            _ => panic!("Expected Sequence"),
        }
    }

    #[test]
    fn test_parse_sequence_with_notes_and_loops() {
        let src = "sequenceDiagram\n    participant A\n    participant B\n    Note over A: Thinking\n    loop Every minute\n        A->>B: ping\n    end";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Sequence(sd) => {
                // Notes and loop/end should be skipped, only the message parsed
                assert_eq!(sd.participants.len(), 2);
                assert_eq!(sd.messages.len(), 1);
                assert_eq!(sd.messages[0].label, "ping");
            }
            _ => panic!("Expected Sequence"),
        }
    }

    #[test]
    fn test_parse_sequence_actor() {
        let src = "sequenceDiagram\n    actor User\n    User->>System: login";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Sequence(sd) => {
                assert_eq!(sd.participants.len(), 2);
                assert!(sd.participants.contains(&"User".to_string()));
                assert!(sd.participants.contains(&"System".to_string()));
            }
            _ => panic!("Expected Sequence"),
        }
    }

    #[test]
    fn test_parse_sequence_participant_alias() {
        let src = "sequenceDiagram\n    participant A as Alice\n    participant B as Bob\n    Alice->>Bob: Hi";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Sequence(sd) => {
                assert_eq!(sd.participants[0], "Alice");
                assert_eq!(sd.participants[1], "Bob");
            }
            _ => panic!("Expected Sequence"),
        }
    }

    // ── Unknown / fallback ─────────────────────────────────────────────────

    #[test]
    fn test_parse_unknown_diagram() {
        let src = "pie\n    title Pets\n    \"Dogs\" : 40\n    \"Cats\" : 30";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Unknown(s) => {
                assert_eq!(s, src);
            }
            _ => panic!("Expected Unknown"),
        }
    }

    #[test]
    fn test_parse_empty() {
        let src = "";
        let diagram = parse_mermaid(src);
        assert!(matches!(diagram, MermaidDiagram::Unknown(_)));
    }

    // ── Layering ───────────────────────────────────────────────────────────

    #[test]
    fn test_compute_layers_linear() {
        let nodes = vec![
            FlowNode {
                id: "A".into(),
                label: "A".into(),
                shape: NodeShape::Default,
            },
            FlowNode {
                id: "B".into(),
                label: "B".into(),
                shape: NodeShape::Default,
            },
            FlowNode {
                id: "C".into(),
                label: "C".into(),
                shape: NodeShape::Default,
            },
        ];
        let edges = vec![
            FlowEdge {
                from: "A".into(),
                to: "B".into(),
                label: None,
                style: EdgeStyle::Solid,
            },
            FlowEdge {
                from: "B".into(),
                to: "C".into(),
                label: None,
                style: EdgeStyle::Solid,
            },
        ];
        let layers = compute_layers(&nodes, &edges);
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0], vec![0]); // A
        assert_eq!(layers[1], vec![1]); // B
        assert_eq!(layers[2], vec![2]); // C
    }

    #[test]
    fn test_compute_layers_branching() {
        let nodes = vec![
            FlowNode {
                id: "A".into(),
                label: "A".into(),
                shape: NodeShape::Default,
            },
            FlowNode {
                id: "B".into(),
                label: "B".into(),
                shape: NodeShape::Default,
            },
            FlowNode {
                id: "C".into(),
                label: "C".into(),
                shape: NodeShape::Default,
            },
        ];
        let edges = vec![
            FlowEdge {
                from: "A".into(),
                to: "B".into(),
                label: None,
                style: EdgeStyle::Solid,
            },
            FlowEdge {
                from: "A".into(),
                to: "C".into(),
                label: None,
                style: EdgeStyle::Solid,
            },
        ];
        let layers = compute_layers(&nodes, &edges);
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0], vec![0]); // A
        assert!(layers[1].contains(&1) && layers[1].contains(&2)); // B, C
    }

    #[test]
    fn test_compute_layers_disconnected() {
        let nodes = vec![
            FlowNode {
                id: "A".into(),
                label: "A".into(),
                shape: NodeShape::Default,
            },
            FlowNode {
                id: "B".into(),
                label: "B".into(),
                shape: NodeShape::Default,
            },
        ];
        let edges: Vec<FlowEdge> = vec![];
        let layers = compute_layers(&nodes, &edges);
        // Both have in-degree 0 so they should be in layer 0
        assert_eq!(layers.len(), 1);
        assert!(layers[0].contains(&0) && layers[0].contains(&1));
    }

    // ── Edge parsing helpers ───────────────────────────────────────────────

    #[test]
    fn test_extract_pipe_label() {
        assert_eq!(extract_pipe_label("|yes| B"), Some("yes".to_string()));
        assert_eq!(extract_pipe_label("B"), None);
        assert_eq!(extract_pipe_label("| | B"), None); // empty after trim
    }

    #[test]
    fn test_parse_seq_message() {
        let msg = parse_seq_message("Alice->>Bob: Hello World").unwrap();
        assert_eq!(msg.from, "Alice");
        assert_eq!(msg.to, "Bob");
        assert_eq!(msg.label, "Hello World");
        assert_eq!(msg.arrow, ArrowKind::Solid);
    }

    #[test]
    fn test_parse_seq_message_no_label() {
        let msg = parse_seq_message("A->>B").unwrap();
        assert_eq!(msg.from, "A");
        assert_eq!(msg.to, "B");
        assert_eq!(msg.label, "");
    }

    // ── Node token parsing edge cases ──────────────────────────────────────

    #[test]
    fn test_parse_node_token_plain_id() {
        assert_eq!(
            parse_node_token("myNode"),
            Some(FlowNode {
                id: "myNode".into(),
                label: "myNode".into(),
                shape: NodeShape::Default,
            })
        );
    }

    #[test]
    fn test_parse_node_token_empty() {
        assert_eq!(parse_node_token(""), None);
    }

    #[test]
    fn test_parse_node_cylinder() {
        assert_eq!(
            parse_node_token("DB[(Database)]"),
            Some(FlowNode {
                id: "DB".into(),
                label: "Database".into(),
                shape: NodeShape::Cylinder,
            })
        );
    }

    // ── Flowchart complex examples ─────────────────────────────────────────

    #[test]
    fn test_parse_flowchart_multiple_edges_on_separate_lines() {
        let src =
            "graph TD\n    A[Start] --> B{Decision}\n    B -->|yes| C[OK]\n    B -->|no| D[Fail]";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Flowchart(fc) => {
                assert_eq!(fc.nodes.len(), 4);
                assert_eq!(fc.edges.len(), 3);
                let b = fc.nodes.iter().find(|n| n.id == "B").unwrap();
                assert_eq!(b.shape, NodeShape::Diamond);
            }
            _ => panic!("Expected Flowchart"),
        }
    }

    #[test]
    fn test_parse_flowchart_skips_subgraph() {
        let src = "graph TD\n    subgraph cluster\n    A --> B\n    end";
        let diagram = parse_mermaid(src);
        match diagram {
            MermaidDiagram::Flowchart(fc) => {
                assert_eq!(fc.nodes.len(), 2);
                assert_eq!(fc.edges.len(), 1);
            }
            _ => panic!("Expected Flowchart"),
        }
    }
}
