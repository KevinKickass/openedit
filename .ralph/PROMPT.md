# OpenEdit — Cross-Platform Text & Code Editor

## Vision

OpenEdit is a modern, fast, cross-platform text and code editor — think Notepad++ but built for 2026 and running everywhere. Lightweight, snappy, no Electron bloat. Opens instantly, handles large files without breaking a sweat, and has the features developers and power users actually need.

**Target:** Replace Notepad++ on Windows, provide a native-feeling equivalent on macOS and Linux that doesn't exist today. Not an IDE — a damn good editor.

## Tech Stack

| Component | Technology | Reason |
|-----------|-----------|--------|
| Language | **Rust** | Performance, memory safety, cross-platform |
| GUI Framework | **egui** (via eframe) | Immediate mode, native on all platforms, GPU-accelerated, no bloat |
| Text Engine | Custom rope-based buffer | Handle multi-GB files efficiently |
| Syntax Highlighting | **tree-sitter** | Incremental, fast, accurate, huge language support |
| File Watching | **notify** crate | Cross-platform file change detection |
| Config | **TOML** | Simple, human-readable |
| Build | **Cargo** + CI/CD | Cross-compile for Windows/macOS/Linux |

### Why egui over Tauri/GTK/Qt?
- No web tech overhead (unlike Tauri/Electron)
- Single binary, no runtime dependencies
- GPU-accelerated rendering = smooth scrolling even in huge files
- Immediate mode = simple state management
- Native look achievable with theming

## Core Features (MVP)

### Editor Basics
- [ ] Multi-tab interface with drag & drop reordering
- [ ] Line numbers with configurable display
- [ ] Syntax highlighting via tree-sitter (50+ languages out of the box)
- [ ] Word wrap (toggle)
- [ ] Show/hide whitespace characters (spaces, tabs, newlines)
- [ ] Line endings display and conversion (CRLF/LF/CR)
- [ ] Encoding detection and conversion (UTF-8, UTF-16, ISO-8859-1, etc.)
- [ ] Auto-indent and smart indent
- [ ] Bracket matching and auto-close
- [ ] Current line highlight
- [ ] Column/block selection (Alt+Drag)
- [ ] Multi-cursor editing (Ctrl+D for next occurrence)
- [ ] Minimap (code overview sidebar)
- [ ] Zoom in/out (Ctrl+Scroll)

### File Handling
- [ ] Open files instantly (lazy loading for large files)
- [ ] Handle files > 1 GB without lag (rope data structure + viewport rendering)
- [ ] File change detection (external modification warning)
- [ ] Auto-save / crash recovery
- [ ] Session restore (reopen tabs from last session)
- [ ] Recent files list
- [ ] Drag & drop files to open
- [ ] Multiple windows support

### Search & Replace
- [ ] Find in file (Ctrl+F) with regex support
- [ ] Replace in file (Ctrl+H) with regex + capture groups
- [ ] Find in files / folder (Ctrl+Shift+F) with result panel
- [ ] Incremental search (highlight as you type)
- [ ] Search history
- [ ] Bookmark lines (toggle, navigate next/prev)

### Navigation
- [ ] Go to line (Ctrl+G)
- [ ] Go to file (Ctrl+P) — fuzzy file finder
- [ ] Go to symbol (Ctrl+Shift+O) — via tree-sitter
- [ ] Breadcrumb navigation (file path + symbol path)
- [ ] Split view (horizontal/vertical)
- [ ] Side-by-side diff view

### Quality of Life
- [ ] Command palette (Ctrl+Shift+P)
- [ ] Customizable keyboard shortcuts
- [ ] Customizable themes (dark/light + custom)
- [ ] Plugin/extension system (later, but architecture must support it)
- [ ] Integrated terminal panel (optional, toggleable)
- [ ] File explorer sidebar (tree view)
- [ ] Print support
- [ ] **Markdown live preview** (split view, renders as you type, GFM support)

## Notepad++ Feature Parity Checklist

These are the features that make Notepad++ indispensable for many users:

- [ ] **Macro recording & playback** — Record keystrokes, replay, save macros
- [ ] **Column editor** — Insert text/numbers in column mode across lines
- [ ] **Document map** — Minimap with highlighted viewport
- [ ] **Multi-document interface** — Tabs + ability to move tabs between windows
- [ ] **Compare plugin equivalent** — Built-in diff/compare two files
- [ ] **Hex editor mode** — View/edit binary files
- [ ] **Folding** — Code folding based on syntax (tree-sitter powered)
- [ ] **Auto-completion** — Basic word completion from current document
- [ ] **Function list** — Panel showing functions/classes in current file
- [ ] **Sort lines** — Ascending, descending, case-sensitive, numeric
- [ ] **Remove duplicate lines**
- [ ] **Base64/URL encode/decode** — Built-in text transformations
- [ ] **JSON/XML pretty print** — Format/validate structured data
- [ ] **Line operations** — Join, split, move up/down, duplicate

## Architecture

```
┌──────────────────────────────────────────────┐
│                  OpenEdit                     │
├──────────────────────────────────────────────┤
│  UI Layer (egui)                             │
│  ├── TabBar                                  │
│  ├── Editor Viewport (GPU-rendered)          │
│  ├── Sidebar (File Explorer, Function List)  │
│  ├── Bottom Panel (Terminal, Search Results)  │
│  ├── Command Palette                         │
│  └── Status Bar                              │
├──────────────────────────────────────────────┤
│  Editor Core                                 │
│  ├── Buffer (Rope-based text storage)        │
│  ├── Cursor & Selection Engine               │
│  ├── Undo/Redo (operation-based)             │
│  ├── Syntax Engine (tree-sitter)             │
│  └── Search Engine (regex, multi-file)       │
├──────────────────────────────────────────────┤
│  Platform Layer                              │
│  ├── File I/O (async, memory-mapped)         │
│  ├── File Watcher (notify)                   │
│  ├── Clipboard                               │
│  ├── Config (TOML)                           │
│  └── Native Dialogs (rfd crate)              │
└──────────────────────────────────────────────┘
```

### Module Structure
```
src/
├── main.rs                 # Entry point, window creation
├── app.rs                  # Main application state & egui frame
├── ui/
│   ├── mod.rs
│   ├── tab_bar.rs          # Tab management UI
│   ├── editor_view.rs      # Main text editing viewport
│   ├── sidebar.rs          # File explorer, function list
│   ├── status_bar.rs       # Cursor pos, encoding, line ending, language
│   ├── command_palette.rs  # Fuzzy command search
│   ├── search_panel.rs     # Find/Replace UI
│   ├── minimap.rs          # Code overview
│   └── terminal.rs         # Integrated terminal
├── core/
│   ├── mod.rs
│   ├── buffer.rs           # Rope-based text buffer
│   ├── cursor.rs           # Cursor, selection, multi-cursor
│   ├── edit.rs             # Edit operations (insert, delete, etc.)
│   ├── undo.rs             # Undo/Redo stack
│   ├── search.rs           # Search & replace engine
│   ├── syntax.rs           # Tree-sitter integration
│   ├── folding.rs          # Code folding
│   └── encoding.rs         # Encoding detection & conversion
├── io/
│   ├── mod.rs
│   ├── file.rs             # File open/save (async, mmap for large files)
│   ├── watcher.rs          # File change detection
│   ├── session.rs          # Session save/restore
│   └── config.rs           # Settings management
├── tools/
│   ├── mod.rs
│   ├── sort.rs             # Line sorting
│   ├── transform.rs        # Base64, URL encode, JSON format, etc.
│   ├── diff.rs             # File comparison
│   ├── hex.rs              # Hex editor mode
│   └── macro.rs            # Macro recording & playback
└── platform/
    ├── mod.rs
    ├── clipboard.rs        # Platform clipboard access
    ├── dialogs.rs          # Native open/save dialogs
    └── fonts.rs            # Font loading & management
```

## Performance Targets

| Metric | Target |
|--------|--------|
| Cold start | < 200ms |
| Open 100 MB file | < 1s |
| Open 1 GB file | < 3s (viewport only, lazy load) |
| Keystroke latency | < 5ms |
| Scrolling | 60 FPS constant |
| Memory (empty) | < 30 MB |
| Memory (10 tabs, medium files) | < 150 MB |
| Binary size | < 15 MB |

## Build Targets

- **Windows:** x86_64, aarch64 (native .exe, optional .msi installer)
- **macOS:** x86_64 (Intel), aarch64 (Apple Silicon) — .app bundle + .dmg
- **Linux:** x86_64, aarch64 — AppImage + .deb + .rpm + Flatpak

## Development Phases

### Phase 1: Core Editor (MVP)
1. Project setup (Cargo workspace, egui window, CI)
2. Rope-based text buffer with basic editing
3. Viewport rendering (line numbers, cursor, selection)
4. File open/save with encoding detection
5. Multi-tab interface
6. Basic syntax highlighting (tree-sitter, 10 languages)
7. Find/Replace with regex
8. Undo/Redo
9. Keyboard shortcuts (Notepad++ defaults)
10. Settings (TOML config, themes)

### Phase 2: Power Features
1. Multi-cursor editing
2. Code folding
3. Minimap
4. File explorer sidebar
5. Split view
6. Find in files
7. Command palette
8. Go to line/file/symbol
9. Auto-completion (word-based)
10. Bracket matching & auto-close

### Phase 3: Notepad++ Parity
1. Macro recording & playback
2. Column editor
3. Hex editor mode
4. Diff/Compare view
5. Line operations (sort, deduplicate, transform)
6. JSON/XML formatting
7. Base64/URL encoding tools
8. Print support
9. Session management
10. Function list panel

### Phase 4: Polish & Ecosystem
1. Plugin/extension architecture
2. Integrated terminal
3. Theme marketplace/sharing
4. Language server protocol (LSP) basic support
5. Installer/packaging for all platforms
6. Auto-update mechanism
7. Localization (i18n)

## Name & Branding

**OpenEdit** — Open source, opens everything, open to extension.

Clean, minimal branding. No mascot needed. Icon: Simple geometric representation of a text cursor or document.

## License

MIT or Apache 2.0 (keep it permissive and open).
