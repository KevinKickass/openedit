# OpenEdit — Architecture Specification

## Design Goals

1. **Lightweight** — Small binary (<15 MB), low memory (<30 MB empty), instant startup (<200ms)
2. **Fast** — 60 FPS scrolling, <5ms input latency, handle 1 GB+ files
3. **Plugin-ready** — Every feature should be built as if it were a plugin (trait-based, decoupled)
4. **Notepad++ compatible** — Default keybindings, familiar behavior, similar feature set

## Technology

- **Language:** Rust (2021 edition, stable toolchain)
- **GUI:** egui via eframe (GPU-accelerated, immediate mode)
- **Text Buffer:** `ropey` crate (rope data structure)
- **Syntax:** `tree-sitter` with language grammars
- **File Watch:** `notify` crate
- **Dialogs:** `rfd` crate (native OS file dialogs)
- **Config:** `serde` + TOML
- **Encoding:** `encoding_rs` for conversion, `chardetng` for detection
- **Async I/O:** `tokio` or `smol` for non-blocking file operations
- **Regex:** `regex` crate

## Module Architecture

```
openedit/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── openedit-core/      # Text buffer, editing ops, search — NO UI dependency
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── buffer.rs       # Rope-based buffer (ropey wrapper)
│   │   │   ├── cursor.rs       # Cursor state, multi-cursor
│   │   │   ├── selection.rs    # Selection ranges
│   │   │   ├── edit.rs         # Edit operations (insert, delete, transform)
│   │   │   ├── undo.rs         # Undo/Redo (operation log, group by transaction)
│   │   │   ├── search.rs       # Find/replace engine (regex, multi-file)
│   │   │   ├── syntax.rs       # Tree-sitter wrapper
│   │   │   ├── folding.rs      # Fold ranges from tree-sitter
│   │   │   ├── encoding.rs     # Detect + convert encodings
│   │   │   ├── line_ending.rs  # CRLF/LF/CR detection + conversion
│   │   │   └── document.rs     # Document = buffer + cursor + syntax + undo + metadata
│   │   └── Cargo.toml
│   │
│   ├── openedit-tools/     # Built-in text transformations — NO UI dependency
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── base64.rs       # Base64 encode/decode
│   │   │   ├── url.rs          # URL encode/decode
│   │   │   ├── json.rs         # JSON pretty print / minify / validate
│   │   │   ├── xml.rs          # XML pretty print
│   │   │   ├── sort.rs         # Sort lines (various modes)
│   │   │   ├── deduplicate.rs  # Remove duplicate lines
│   │   │   ├── case.rs         # UPPER/lower/Title case conversion
│   │   │   ├── hash.rs         # MD5/SHA1/SHA256
│   │   │   ├── hex.rs          # Hex conversion + hex editor logic
│   │   │   ├── timestamp.rs    # Unix timestamp ↔ human readable
│   │   │   └── lines.rs        # Join, split, trim, reverse, etc.
│   │   └── Cargo.toml
│   │
│   ├── openedit-ui/        # All egui UI code
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── app.rs          # Main App struct implementing eframe::App
│   │   │   ├── tab_bar.rs      # Tab management
│   │   │   ├── editor_view.rs  # Text viewport rendering
│   │   │   ├── gutter.rs       # Line numbers, fold markers, bookmarks
│   │   │   ├── minimap.rs      # Document overview
│   │   │   ├── status_bar.rs   # Bottom status bar
│   │   │   ├── command_palette.rs
│   │   │   ├── search_panel.rs # Find/Replace UI
│   │   │   ├── sidebar.rs      # File explorer + function list
│   │   │   ├── terminal.rs     # Integrated terminal (Phase 3)
│   │   │   ├── diff_view.rs    # Side-by-side diff (Phase 3)
│   │   │   ├── theme.rs        # Theme loading + application
│   │   │   └── dialogs.rs      # Settings, about, etc.
│   │   └── Cargo.toml
│   │
│   └── openedit-plugin/    # Plugin API definitions
│       ├── src/
│       │   ├── lib.rs
│       │   ├── api.rs          # Plugin trait, capabilities
│       │   ├── manifest.rs     # Plugin TOML manifest parser
│       │   ├── registry.rs     # Command + event registry
│       │   └── loader.rs       # Dynamic plugin loading
│       └── Cargo.toml
│
└── src/
    └── main.rs             # Entry point: parse args, create window, run app
```

## Key Design Patterns

### Command Registry
Every action (cut, paste, sort lines, base64 encode, etc.) is a `Command`:
```rust
pub trait Command: Send + Sync {
    fn id(&self) -> &str;           // "editor.sort_lines_asc"
    fn name(&self) -> &str;         // "Sort Lines (Ascending)"
    fn category(&self) -> &str;     // "Edit" / "Tools" / "View"
    fn keybinding(&self) -> Option<KeyCombo>;
    fn execute(&self, ctx: &mut EditorContext) -> Result<()>;
    fn is_available(&self, ctx: &EditorContext) -> bool { true }
}
```
Built-in features and plugins both register commands the same way. Command palette searches this registry.

### Event System
Decoupled communication between components:
```rust
pub enum EditorEvent {
    FileOpened { doc_id: DocId, path: PathBuf },
    FileSaved { doc_id: DocId, path: PathBuf },
    BufferChanged { doc_id: DocId, range: Range },
    CursorMoved { doc_id: DocId, positions: Vec<Position> },
    TabActivated { doc_id: DocId },
    TabClosed { doc_id: DocId },
    ThemeChanged { theme_id: String },
    // ...extensible
}
```
Plugins can subscribe to events and emit custom events.

### Document Model
Each open file is a `Document`:
```rust
pub struct Document {
    pub id: DocId,
    pub buffer: RopeBuffer,         // The text content
    pub cursors: MultiCursorState,  // All cursor positions
    pub undo_stack: UndoManager,    // Operation history
    pub syntax: Option<SyntaxState>,// Tree-sitter state
    pub folds: FoldState,           // Collapsed regions
    pub bookmarks: Vec<usize>,      // Bookmarked lines
    pub path: Option<PathBuf>,      // None = untitled
    pub encoding: Encoding,         // Current encoding
    pub line_ending: LineEnding,    // CRLF/LF/CR
    pub language: Option<Language>, // Detected/selected language
    pub modified: bool,             // Has unsaved changes
    pub read_only: bool,
}
```

## Notepad++ Keybinding Defaults

| Action | Keybinding |
|--------|-----------|
| New | Ctrl+N |
| Open | Ctrl+O |
| Save | Ctrl+S |
| Save As | Ctrl+Shift+S |
| Close Tab | Ctrl+W |
| Undo | Ctrl+Z |
| Redo | Ctrl+Y |
| Find | Ctrl+F |
| Replace | Ctrl+H |
| Find in Files | Ctrl+Shift+F |
| Go to Line | Ctrl+G |
| Duplicate Line | Ctrl+D |
| Delete Line | Ctrl+Shift+K |
| Move Line Up | Alt+Up |
| Move Line Down | Alt+Down |
| Toggle Comment | Ctrl+/ |
| Indent | Tab |
| Unindent | Shift+Tab |
| Toggle Bookmark | Ctrl+F2 |
| Next Bookmark | F2 |
| Prev Bookmark | Shift+F2 |
| Start/Stop Recording | Ctrl+Shift+R |
| Playback Macro | Ctrl+Shift+P |
| Column Editor | Alt+C |
| Zoom In | Ctrl+= |
| Zoom Out | Ctrl+- |
| Word Wrap Toggle | View menu |
| Show Whitespace | View menu |

## Configuration

Config file: `~/.config/openedit/config.toml` (Linux/macOS) / `%APPDATA%/openedit/config.toml` (Windows)

```toml
[editor]
font_family = "JetBrains Mono"
font_size = 14
tab_size = 4
use_spaces = true
word_wrap = false
show_whitespace = false
show_minimap = true
auto_indent = true
auto_close_brackets = true
highlight_current_line = true
trim_trailing_on_save = false

[ui]
theme = "dark"
show_line_numbers = true
show_status_bar = true
show_sidebar = false
sidebar_width = 250

[files]
default_encoding = "utf-8"
default_line_ending = "auto"  # auto = OS default
auto_save_interval = 0        # 0 = disabled, seconds
restore_session = true
max_recent_files = 25

[keybindings]
# Override defaults here
# "ctrl+d" = "editor.select_next_occurrence"  # VS Code style
# Default is Notepad++ style (duplicate line)
```

## Performance Strategy

### Large File Handling
1. Files < 10 MB: Load entirely into rope buffer
2. Files 10 MB - 100 MB: Load into rope, but render only viewport
3. Files > 100 MB: Memory-map, load chunks on demand, limited editing
4. Files > 1 GB: Read-only by default, viewport-only rendering

### Rendering
- Only render visible lines (viewport culling)
- Cache syntax highlighting per line (invalidate on edit)
- Debounce syntax re-parse (16ms after last edit)
- Background thread for tree-sitter parsing
- GPU text rendering via egui's painter

### Search
- Small files (<1 MB): Direct regex search
- Large files: Streaming search with progress indicator
- Find in files: Parallel search using `ignore` crate (respects .gitignore)
