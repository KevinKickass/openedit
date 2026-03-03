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
- [x] Memory-mapped file loading for large files (>100 MB, memmap2)
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
- [x] Drag & drop reorder (drag-and-drop tab reordering)
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
- [x] 24 languages: Rust, Go, Python, JavaScript, TypeScript, TSX, C, C++, Java, HTML, CSS, JSON, YAML, TOML, SQL, Bash, PHP, Ruby, Markdown, Lua, Swift, Haskell, Scala, Kotlin
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
- [x] Code folding (indentation-based)
- [x] Breadcrumb bar (symbol hierarchy based on cursor position)

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

### Macro System (done)
- [x] Start/stop recording (Ctrl+Q)
- [x] Playback (Ctrl+Shift+Q)
- [x] Run macro multiple times (dialog with count input)
- [x] Save/load macros to disk (JSON in config dir)
- [x] Named macro slots with load/delete UI
- [x] Edit macro (as script in editor tab, save parses back)

### Hex Editor (done)
- [x] Toggle hex view mode for current file (Ctrl+Shift+H)
- [x] Side-by-side hex + ASCII display (16 bytes/row)
- [x] Edit in hex mode (two-nibble entry, auto-advance, undoable)
- [x] Go to offset (Ctrl+G, hex/decimal input)

### Diff / Compare (done)
- [x] Compare two open files side-by-side (LCS-based)
- [x] Highlight additions, deletions, changes
- [x] Navigate between diffs (F7/Shift+F7, wrapping)
- [x] Merge direction per hunk (left→right / right→left buttons, undoable)

### Integrated Terminal (done)
- [x] Toggleable bottom panel (Ctrl+`)
- [x] Platform shell (bash/zsh on Unix via portable-pty)
- [x] Multiple terminal tabs (tab bar with +/x, auto-naming)
- [x] Send selection to terminal

### Git Integration (done)
- [x] Repository detection (git2 crate)
- [x] Branch name in status bar
- [x] Gutter marks for changed/added/removed lines
- [x] File status detection (modified, staged, untracked)
- [x] Blame display in editor (author + date)
- [x] Git operations (stage file, commit with dialog, toast notifications)

### Vim Mode (done — wired)
- [x] Mode enum (Normal, Insert, Visual, VisualLine, Command)
- [x] Toggle via command palette
- [x] Mode display in status bar
- [x] Key events routed to vim handler (two-phase event processing)
- [x] Normal mode motions (hjkl, w, b, e, 0, $, gg, G)
- [x] Operator-motion combos (dw, ci", yy, etc.)
- [x] Visual mode selection
- [x] Command mode (:w, :q, :wq, etc.)
- [x] Insert mode passthrough (bracket auto-close, clipboard, etc.)

### Snippet Engine (done)
- [x] Snippet struct with trigger/label/body
- [x] Built-in snippets for Rust, Python, TypeScript, Go (40+)
- [x] Placeholder expansion ($1, $2, ${N:default}, ${N|choice|list|})
- [x] SnippetEngine with try_expand() and tab navigation
- [x] Wired into text input handler (Tab triggers expansion)
- [x] Visual highlighting of active placeholders (teal bg + underline)
- [x] User-defined snippets (~/.config/openedit/snippets.json)

### LSP Support (done)
- [x] LSP client infrastructure (JSON-RPC)
- [x] Server startup for Rust, Python, TypeScript, Go, C/C++, Lua
- [x] didOpen/didChange notifications (debounced 300ms)
- [x] Hover tooltip rendering (Ctrl+hover)
- [x] Go-to-definition (F12, Ctrl+Click)
- [x] Diagnostics display (squiggles in editor)
- [x] Completion integration with autocomplete
- [x] Find references (Shift+F12, grouped results panel)
- [x] Rename symbol (F2, inline dialog, workspace-wide edits)

### Plugin Architecture
- [ ] Plugin API definition (Rust trait-based + optional WASM)
- [ ] Plugin manifest format (TOML)
- [ ] Plugin loading/unloading at runtime
- [ ] Plugin commands register into command palette
- [ ] Plugin access: buffer read/write, UI panels, status bar, menus
- [ ] Plugin distribution format

### Additional
- [x] 8 built-in themes (switchable via command palette)
- [x] Bracket colorization (rainbow brackets)
- [x] Toggle comment (Ctrl+/)
- [x] Theming engine (TOML theme files in ~/.config/openedit/themes/, base inheritance)
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
