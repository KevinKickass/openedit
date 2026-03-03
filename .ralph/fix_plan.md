# OpenEdit — Fix Plan / Priority Roadmap

## Core Principles
- **Lightweight first** — Must feel instant. Sub-200ms startup, minimal memory.
- **Fast** — No lag ever. 60 FPS scrolling, <5ms keystroke latency, handle GB files.
- **Plugin-ready** — Architecture must support plugins from day 1, even if plugin API comes later.
- **Notepad++ compatible** — Where possible, match keybindings, behavior, and file format support.

---

## High Priority (Phase 1 — MVP)

### Project Foundation
- [x] Cargo workspace setup with egui/eframe
- [x] Basic window with dark theme
- [x] CI/CD pipeline (GitHub Actions: build Windows/macOS/Linux)

### Text Buffer
- [x] Rope-based text buffer (use `ropey` crate)
- [x] Efficient insert/delete at any position
- [x] Line index for fast line-number lookups
- [x] Memory-mapped file loading for large files (>100 MB)
- [x] Lazy viewport rendering (only render visible lines)

### Editor Viewport
- [x] GPU-accelerated text rendering via egui
- [x] Line numbers gutter
- [x] Cursor rendering and movement (arrow keys, Home/End, Ctrl+arrows)
- [x] Text selection (Shift+arrows, Shift+Click, double-click word, triple-click line)
- [x] Current line highlight
- [x] Smooth scrolling

### File I/O
- [x] Open file (native dialog via `rfd` crate)
- [x] Save / Save As
- [x] **Encoding detection** (chardetng or similar) — auto-detect on open
- [x] **Encoding conversion** — switch encoding from status bar (UTF-8, UTF-16 LE/BE, ISO-8859-1, Windows-1252, etc.)
- [x] Line ending detection and display (CRLF/LF/CR)
- [x] Line ending conversion (status bar toggle)

### Multi-Tab
- [x] Tab bar with close buttons
- [x] Ctrl+Tab / Ctrl+Shift+Tab to switch
- [x] Modified indicator (dot/asterisk on unsaved tabs)
- [x] Drag & drop reorder
- [x] Middle-click to close
- [x] Context menu (Close, Close Others, Close All, Copy Path)

### Basic Editing
- [x] Undo/Redo (operation-based, not snapshot-based)
- [x] Cut/Copy/Paste (system clipboard)
- [x] Select All
- [x] Duplicate line (Ctrl+D — Notepad++ compat)
- [x] Move line up/down (Alt+Up/Down)
- [x] Delete line (Ctrl+Shift+K)
- [x] Auto-indent on Enter
- [x] Tab/Shift+Tab indent/unindent selection

### Search
- [x] Find (Ctrl+F) — incremental highlight
- [x] Replace (Ctrl+H)
- [x] Regex support
- [x] Case sensitive / whole word toggles
- [x] Search wrap-around
- [x] Match count display

### Syntax Highlighting
- [x] tree-sitter integration
- [x] Language auto-detection from file extension
- [x] Top 20 languages: Rust, Go, Python, JavaScript, TypeScript, C, C++, Java, HTML, CSS, JSON, YAML, TOML, XML, SQL, Bash, PHP, Ruby, Markdown, Lua
- [x] Language selector in status bar

### Status Bar
- [x] Cursor position (Ln X, Col Y)
- [x] Selection count (chars/lines selected)
- [x] File encoding (clickable → change)
- [x] Line ending (clickable → change)
- [x] Language (clickable → change)
- [x] Tab size / Spaces vs Tabs (clickable → change)

---

## Medium Priority (Phase 2 — Power Features)

### Built-in Tools (Notepad++ Parity)
- [x] **Base64 encode/decode** — Selection → Transform → Base64 Encode/Decode
- [x] **URL encode/decode**
- [x] **JSON pretty print / minify**
- [x] **XML pretty print**
- [x] **Sort lines** (ascending, descending, case-insensitive, numeric)
- [x] **Remove duplicate lines**
- [x] **Remove empty lines**
- [x] **Join lines**
- [x] **UPPERCASE / lowercase / Title Case**
- [x] **Trim trailing whitespace**
- [x] **Line operations** — split, reverse, shuffle
- [x] **Hex to decimal / decimal to hex conversion**
- [x] **Hash generation** (MD5, SHA1, SHA256 of selection)
- [x] **Timestamp conversion** (unix ↔ human readable)

### Multi-Cursor & Column Mode
- [x] Ctrl+Click to add cursor
- [x] Ctrl+D to select next occurrence
- [x] Ctrl+Shift+L to select all occurrences
- [x] Alt+Shift+Drag for column/block selection
- [x] Column editor (insert text/numbers at column across lines)

### Navigation
- [x] Go to line (Ctrl+G)
- [x] Command palette (Ctrl+Shift+P) — fuzzy search all commands
- [x] Bookmarks (Ctrl+F2 toggle, F2 next, Shift+F2 prev — Notepad++ compat)
- [x] Bracket matching (highlight + Ctrl+B to jump)
- [x] Code folding (tree-sitter based)
- [ ] Breadcrumb bar

### Markdown Preview (KILLER FEATURE)
- [x] Split view: editor left, rendered preview right
- [x] Live preview — updates as you type (debounced ~100ms)
- [x] Render via egui: headings, bold, italic, code blocks, lists, tables, links, images
- [x] Syntax highlighting in code blocks (tree-sitter)
- [x] Scroll sync between editor and preview
- [x] Toggle preview: Ctrl+Shift+M
- [x] Export to HTML
- [x] Support GitHub Flavored Markdown (GFM): tables, task lists, strikethrough
- [x] Image preview (inline, from relative paths or URLs)
- [ ] Mermaid diagram rendering (stretch goal)
- [x] Use `pulldown-cmark` crate for parsing

### Find in Files
- [x] Ctrl+Shift+F — search across folder/project
- [x] Results panel with file:line previews
- [x] Replace in files
- [x] Include/exclude file patterns

### Panels
- [x] File explorer sidebar (tree view)
- [x] Function list panel (tree-sitter symbols)
- [x] Minimap / document map
- [x] Split view (horizontal + vertical)

### File Features
- [x] File change detection (external modification → reload prompt)
- [x] Auto-save / crash recovery (swap files)
- [x] Session restore (reopen last tabs on startup)
- [x] Recent files list (Ctrl+Shift+O)
- [x] Drag & drop files onto window to open

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
