# OpenEdit — Fix Plan / Priority Roadmap

## Core Principles
- **Lightweight first** — Must feel instant. Sub-200ms startup, minimal memory.
- **Fast** — No lag ever. 60 FPS scrolling, <5ms keystroke latency, handle GB files.
- **Plugin-ready** — Architecture must support plugins from day 1, even if plugin API comes later.
- **Notepad++ compatible** — Where possible, match keybindings, behavior, and file format support.

---

## High Priority (Phase 1 — MVP)

### Project Foundation
- [ ] Cargo workspace setup with egui/eframe
- [ ] Basic window with dark theme
- [ ] CI/CD pipeline (GitHub Actions: build Windows/macOS/Linux)

### Text Buffer
- [ ] Rope-based text buffer (use `ropey` crate)
- [ ] Efficient insert/delete at any position
- [ ] Line index for fast line-number lookups
- [ ] Memory-mapped file loading for large files (>100 MB)
- [ ] Lazy viewport rendering (only render visible lines)

### Editor Viewport
- [ ] GPU-accelerated text rendering via egui
- [ ] Line numbers gutter
- [ ] Cursor rendering and movement (arrow keys, Home/End, Ctrl+arrows)
- [ ] Text selection (Shift+arrows, Shift+Click, double-click word, triple-click line)
- [ ] Current line highlight
- [ ] Smooth scrolling

### File I/O
- [ ] Open file (native dialog via `rfd` crate)
- [ ] Save / Save As
- [ ] **Encoding detection** (chardetng or similar) — auto-detect on open
- [ ] **Encoding conversion** — switch encoding from status bar (UTF-8, UTF-16 LE/BE, ISO-8859-1, Windows-1252, etc.)
- [ ] Line ending detection and display (CRLF/LF/CR)
- [ ] Line ending conversion (status bar toggle)

### Multi-Tab
- [ ] Tab bar with close buttons
- [ ] Ctrl+Tab / Ctrl+Shift+Tab to switch
- [ ] Modified indicator (dot/asterisk on unsaved tabs)
- [ ] Drag & drop reorder
- [ ] Middle-click to close
- [ ] Context menu (Close, Close Others, Close All, Copy Path)

### Basic Editing
- [ ] Undo/Redo (operation-based, not snapshot-based)
- [ ] Cut/Copy/Paste (system clipboard)
- [ ] Select All
- [ ] Duplicate line (Ctrl+D — Notepad++ compat)
- [ ] Move line up/down (Alt+Up/Down)
- [ ] Delete line (Ctrl+Shift+K)
- [ ] Auto-indent on Enter
- [ ] Tab/Shift+Tab indent/unindent selection

### Search
- [ ] Find (Ctrl+F) — incremental highlight
- [ ] Replace (Ctrl+H)
- [ ] Regex support
- [ ] Case sensitive / whole word toggles
- [ ] Search wrap-around
- [ ] Match count display

### Syntax Highlighting
- [ ] tree-sitter integration
- [ ] Language auto-detection from file extension
- [ ] Top 20 languages: Rust, Go, Python, JavaScript, TypeScript, C, C++, Java, HTML, CSS, JSON, YAML, TOML, XML, SQL, Bash, PHP, Ruby, Markdown, Lua
- [ ] Language selector in status bar

### Status Bar
- [ ] Cursor position (Ln X, Col Y)
- [ ] Selection count (chars/lines selected)
- [ ] File encoding (clickable → change)
- [ ] Line ending (clickable → change)
- [ ] Language (clickable → change)
- [ ] Tab size / Spaces vs Tabs (clickable → change)

---

## Medium Priority (Phase 2 — Power Features)

### Built-in Tools (Notepad++ Parity)
- [ ] **Base64 encode/decode** — Selection → Transform → Base64 Encode/Decode
- [ ] **URL encode/decode**
- [ ] **JSON pretty print / minify**
- [ ] **XML pretty print**
- [ ] **Sort lines** (ascending, descending, case-insensitive, numeric)
- [ ] **Remove duplicate lines**
- [ ] **Remove empty lines**
- [ ] **Join lines**
- [ ] **UPPERCASE / lowercase / Title Case**
- [ ] **Trim trailing whitespace**
- [ ] **Line operations** — split, reverse, shuffle
- [ ] **Hex to decimal / decimal to hex conversion**
- [ ] **Hash generation** (MD5, SHA1, SHA256 of selection)
- [ ] **Timestamp conversion** (unix ↔ human readable)

### Multi-Cursor & Column Mode
- [ ] Ctrl+Click to add cursor
- [ ] Ctrl+D to select next occurrence
- [ ] Ctrl+Shift+L to select all occurrences
- [ ] Alt+Shift+Drag for column/block selection
- [ ] Column editor (insert text/numbers at column across lines)

### Navigation
- [ ] Go to line (Ctrl+G)
- [ ] Command palette (Ctrl+Shift+P) — fuzzy search all commands
- [ ] Bookmarks (Ctrl+F2 toggle, F2 next, Shift+F2 prev — Notepad++ compat)
- [ ] Bracket matching (highlight + Ctrl+B to jump)
- [ ] Code folding (tree-sitter based)
- [ ] Breadcrumb bar

### Markdown Preview (KILLER FEATURE)
- [ ] Split view: editor left, rendered preview right
- [ ] Live preview — updates as you type (debounced ~100ms)
- [ ] Render via egui: headings, bold, italic, code blocks, lists, tables, links, images
- [ ] Syntax highlighting in code blocks (tree-sitter)
- [ ] Scroll sync between editor and preview
- [ ] Toggle preview: Ctrl+Shift+M
- [ ] Export to HTML
- [ ] Support GitHub Flavored Markdown (GFM): tables, task lists, strikethrough
- [ ] Image preview (inline, from relative paths or URLs)
- [ ] Mermaid diagram rendering (stretch goal)
- [ ] Use `pulldown-cmark` crate for parsing

### Find in Files
- [ ] Ctrl+Shift+F — search across folder/project
- [ ] Results panel with file:line previews
- [ ] Replace in files
- [ ] Include/exclude file patterns

### Panels
- [ ] File explorer sidebar (tree view)
- [ ] Function list panel (tree-sitter symbols)
- [ ] Minimap / document map
- [ ] Split view (horizontal + vertical)

### File Features
- [ ] File change detection (external modification → reload prompt)
- [ ] Auto-save / crash recovery (swap files)
- [ ] Session restore (reopen last tabs on startup)
- [ ] Recent files list (Ctrl+Shift+O)
- [ ] Drag & drop files onto window to open

---

## Low Priority (Phase 3 — Advanced)

### Macro System
- [ ] Start/stop recording (Ctrl+Shift+R — Notepad++ compat)
- [ ] Playback (Ctrl+Shift+P)
- [ ] Run macro multiple times
- [ ] Save/load macros
- [ ] Edit macro (as script/commands)

### Hex Editor
- [ ] Toggle hex view mode for current file
- [ ] Side-by-side hex + ASCII display
- [ ] Edit in hex mode
- [ ] Go to offset

### Diff / Compare
- [ ] Compare two open files side-by-side
- [ ] Highlight additions, deletions, changes
- [ ] Navigate between diffs
- [ ] Merge direction (left→right, right→left)

### Plugin Architecture
- [ ] Plugin API definition (Rust trait-based + optional WASM)
- [ ] Plugin manifest format (TOML)
- [ ] Plugin loading/unloading at runtime
- [ ] Plugin commands register into command palette
- [ ] Plugin access: buffer read/write, UI panels, status bar, menus
- [ ] Plugin distribution format

### Integrated Terminal
- [ ] Toggleable bottom panel
- [ ] Platform shell (cmd/powershell on Windows, bash/zsh on Unix)
- [ ] Multiple terminal tabs
- [ ] Send selection to terminal

### Additional
- [ ] LSP basic support (hover, go-to-definition, diagnostics)
- [ ] Theming engine (JSON/TOML theme files)
- [ ] Theme import from VS Code / Notepad++ themes
- [ ] Auto-update mechanism
- [ ] Localization (i18n)
- [ ] Print support
- [ ] Installer packaging (MSI, DMG, AppImage, Flatpak, deb, rpm)

---

## Completed
- [x] Project initialization
- [x] Project planning and specification

## Notes
- **Performance is non-negotiable.** If a feature makes the editor slow, it doesn't ship.
- **Notepad++ keybindings as default.** Users switching from Notepad++ should feel at home.
- **Plugin architecture must be considered in every design decision.** Even if plugins come in Phase 3, don't paint yourself into a corner. Use trait-based abstractions, command registry pattern, event system.
- **egui has limitations** — no native text input handling on some platforms. May need platform-specific input handling for IME support (CJK input methods). Research early.
- **Rope buffer is critical** — get this right in Phase 1. Everything builds on it.
