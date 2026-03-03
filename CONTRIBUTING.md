# Contributing to OpenEdit

Thanks for your interest in contributing! OpenEdit is built with Rust and aims to be fast, lightweight, and simple.

## Getting Started

### Prerequisites

- Rust 1.75+ (stable toolchain)
- Git

### Build

```bash
git clone https://github.com/KevinKickass/openedit.git
cd openedit
cargo build
```

### Run

```bash
cargo run -- myfile.txt
```

### Test

```bash
cargo test --workspace
```

## Project Structure

```
openedit/
├── src/main.rs              # Entry point
├── crates/
│   ├── openedit-core/       # Text buffer, file I/O, encoding, search
│   ├── openedit-tools/      # Built-in tools (base64, hashing, formatting)
│   └── openedit-ui/         # GUI (egui/eframe), tabs, panels, themes
├── Cargo.toml               # Workspace root
└── CONTRIBUTING.md           # You are here
```

- **openedit-core** — No GUI dependencies. Pure logic: rope buffer, file operations, encoding detection, search/replace, syntax highlighting, LSP client.
- **openedit-tools** — Standalone text transformations. Each tool is a pure function: input text → output text.
- **openedit-ui** — Everything visual. Depends on core and tools.

## How to Contribute

### Reporting Bugs

Open an issue with:
- What you did
- What you expected
- What happened instead
- OS, version, and how you installed OpenEdit

### Suggesting Features

Open an issue. Keep it focused — one feature per issue.

### Submitting Code

1. Fork the repo
2. Create a branch (`fix/cursor-click`, `feat/bracket-matching`, etc.)
3. Make your changes
4. Run `cargo test --workspace` and `cargo clippy --workspace`
5. Open a PR against `main`

### Code Style

- Run `cargo fmt` before committing
- No warnings from `cargo clippy`
- No `unwrap()` in library code — use `anyhow::Result` or `thiserror`
- Keep functions short and focused
- Write tests for new functionality

### Commit Messages

```
fix: correct cursor position on click in wrapped lines
feat: add XML pretty print tool
refactor: extract encoding detection into standalone module
```

Use lowercase, imperative mood, no period at the end.

### Adding a Tool

Tools live in `crates/openedit-tools/`. Each tool is a pure function:

```rust
pub fn my_tool(input: &str) -> Result<String> {
    // transform input
    Ok(output)
}
```

Add it to the module, write a test, register it in the UI — done.

### Adding Syntax Highlighting

Tree-sitter grammars go in `openedit-core`. Add the grammar dependency to `Cargo.toml`, register the language, and add a test file.

## Performance Matters

OpenEdit targets:
- Cold start under 200ms
- Keystroke latency under 5ms
- Memory under 30 MB idle

If your change adds latency or memory usage, document why it's worth it.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
