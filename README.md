# OpenEdit

[![Build](https://github.com/KevinKickass/openedit/actions/workflows/ci.yml/badge.svg)](https://github.com/KevinKickass/openedit/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)]()

A fast, lightweight, cross-platform text and code editor. Built with Rust.

Opens instantly. Handles large files. Runs everywhere. No bloat.

## Features

- **Lightweight** — Single binary, <15 MB, <30 MB RAM, starts in under 200ms
- **Syntax Highlighting** — 50+ languages via tree-sitter
- **Large File Support** — Opens 1 GB+ files without breaking a sweat
- **Multi-Tab** — Drag & drop, session restore, modified indicators
- **Encoding** — Auto-detection, conversion between UTF-8, UTF-16, ISO-8859, Windows-1252, and more
- **Search & Replace** — Regex support, find in files, incremental highlighting
- **Multi-Cursor** — Ctrl+Click, select next occurrence, column/block selection
- **Markdown Preview** — Live split-view rendering with GitHub Flavored Markdown
- **Built-in Tools** — Base64, URL encode/decode, JSON/XML formatting, line sorting, hashing
- **Hex Editor** — View and edit binary files
- **Code Folding** — Collapse/expand code blocks
- **Minimap** — Document overview sidebar
- **Macro System** — Record, playback, save keystroke macros
- **Diff View** — Compare two files side-by-side
- **Command Palette** — Fuzzy search for any action
- **Plugin Architecture** — Extend with Rust or WASM plugins
- **Notepad++ Keybindings** — Feels familiar from day one

## Installation

### Pre-built Binaries

Download the latest release for your platform:

| Platform | Download |
|----------|----------|
| Windows (x64) | [openedit-windows-x64.zip]() |
| Windows (ARM64) | [openedit-windows-arm64.zip]() |
| macOS (Intel) | [openedit-macos-x64.dmg]() |
| macOS (Apple Silicon) | [openedit-macos-arm64.dmg]() |
| Linux (x64, AppImage) | [openedit-linux-x64.AppImage]() |
| Linux (x64, .deb) | [openedit-linux-x64.deb]() |
| Linux (x64, .rpm) | [openedit-linux-x64.rpm]() |

### Build from Source

Requires Rust 1.75+ (stable toolchain).

```bash
git clone https://github.com/KevinKickass/openedit.git
cd openedit
cargo build --release
```

Binary is at `target/release/openedit`.

## Usage

```bash
# Open a file
openedit myfile.txt

# Open multiple files in tabs
openedit file1.rs file2.go file3.py

# Open at specific line
openedit myfile.rs:42

# Pipe from stdin
cat log.txt | openedit -
```

## Keyboard Shortcuts

Default keybindings follow Notepad++ conventions.

| Action | Shortcut |
|--------|----------|
| New File | Ctrl+N |
| Open | Ctrl+O |
| Save | Ctrl+S |
| Save As | Ctrl+Shift+S |
| Close Tab | Ctrl+W |
| Find | Ctrl+F |
| Replace | Ctrl+H |
| Find in Files | Ctrl+Shift+F |
| Go to Line | Ctrl+G |
| Command Palette | Ctrl+Shift+P |
| Duplicate Line | Ctrl+D |
| Delete Line | Ctrl+Shift+K |
| Move Line Up/Down | Alt+Up/Down |
| Toggle Comment | Ctrl+/ |
| Toggle Bookmark | Ctrl+F2 |
| Next/Prev Bookmark | F2 / Shift+F2 |
| Toggle Markdown Preview | Ctrl+Shift+M |
| Zoom In/Out | Ctrl+= / Ctrl+- |
| Column Selection | Alt+Shift+Drag |

All shortcuts are fully customizable in settings.

## Built-in Tools

Access via the Tools menu or Command Palette:

**Encoding** — Base64 Encode/Decode, URL Encode/Decode, HTML Entity Encode/Decode

**Formatting** — JSON Pretty Print/Minify, XML Pretty Print

**Line Operations** — Sort (asc/desc/numeric), Remove Duplicates, Remove Empty Lines, Join Lines, Reverse, Trim Whitespace

**Case Conversion** — UPPERCASE, lowercase, Title Case, camelCase, snake_case

**Hashing** — MD5, SHA-1, SHA-256

**Conversion** — Hex/Decimal, Unix Timestamp/Date

## Configuration

Config lives at:
- Linux/macOS: `~/.config/openedit/config.toml`
- Windows: `%APPDATA%\openedit\config.toml`

```toml
[editor]
font_family = "JetBrains Mono"
font_size = 14
tab_size = 4
use_spaces = true
word_wrap = false
show_whitespace = false
auto_indent = true
highlight_current_line = true

[ui]
theme = "dark"
show_line_numbers = true
show_minimap = true
show_sidebar = false

[files]
default_encoding = "utf-8"
restore_session = true
```

## Supported Languages

Syntax highlighting for 50+ languages including:

Bash, C, C++, C#, CSS, Dart, Dockerfile, Elixir, Go, GraphQL, Haskell, HTML, Java, JavaScript, JSON, Kotlin, Lua, Markdown, Objective-C, OCaml, Perl, PHP, Python, R, Ruby, Rust, Scala, SQL, Swift, TOML, TypeScript, YAML, XML, Zig, and more.

## Performance

| Metric | Target |
|--------|--------|
| Cold start | < 200ms |
| Open 100 MB file | < 1s |
| Open 1 GB file | < 3s |
| Keystroke latency | < 5ms |
| Scrolling | 60 FPS |
| Memory (idle) | < 30 MB |
| Binary size | < 15 MB |

## Tech Stack

- **Rust** — Performance and memory safety
- **egui/eframe** — GPU-accelerated immediate mode GUI
- **ropey** — Rope-based text buffer for large files
- **tree-sitter** — Incremental syntax highlighting
- **pulldown-cmark** — Markdown parsing and preview

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT
