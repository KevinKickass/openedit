# OpenEdit — Feature Specifications

## Encoding System (HIGH PRIORITY)

### Detection
- On file open: auto-detect encoding using `chardetng`
- BOM detection: UTF-8 BOM, UTF-16 LE/BE BOM, UTF-32
- Fallback: configurable default (UTF-8)
- Display detected encoding in status bar

### Supported Encodings
- UTF-8 (with/without BOM)
- UTF-16 LE / UTF-16 BE
- ISO-8859-1 (Latin-1)
- ISO-8859-15 (Latin-9)
- Windows-1252
- ASCII
- Shift-JIS
- EUC-JP
- GB2312 / GBK / GB18030
- EUC-KR
- KOI8-R (Cyrillic)

### Conversion
- Click encoding in status bar → dropdown with all encodings
- "Reinterpret as..." (re-read file bytes as different encoding)
- "Convert to..." (transcode content to new encoding, keep text)
- On save: encode in currently selected encoding
- Warn if content has characters unsupported by target encoding

---

## Base64 Tool (HIGH PRIORITY)

### Encode
- Select text → Menu/Command: "Base64 Encode"
- Selection is replaced with base64-encoded version
- Also available: Encode entire document

### Decode
- Select base64 text → Menu/Command: "Base64 Decode"
- Validates input, shows error if invalid base64
- Selection is replaced with decoded text

### Variants
- Standard Base64 (RFC 4648)
- URL-safe Base64 (RFC 4648 §5)
- MIME Base64 (line-wrapped at 76 chars)

### UI
- Accessible via:
  - Menu: Tools → Base64 → Encode / Decode
  - Command palette: "Base64 Encode" / "Base64 Decode"
  - Keyboard shortcut (configurable, no default — Notepad++ doesn't have one)
  - Right-click context menu on selection

---

## Text Transformation Tools

All tools operate on selection (or entire document if no selection):

| Tool | Menu Path | Description |
|------|-----------|-------------|
| Base64 Encode | Tools → Encoding → Base64 Encode | RFC 4648 standard |
| Base64 Decode | Tools → Encoding → Base64 Decode | With validation |
| URL Encode | Tools → Encoding → URL Encode | Percent-encoding |
| URL Decode | Tools → Encoding → URL Decode | |
| HTML Entity Encode | Tools → Encoding → HTML Encode | &amp; &lt; etc. |
| HTML Entity Decode | Tools → Encoding → HTML Decode | |
| JSON Pretty Print | Tools → Format → JSON Pretty Print | 2-space indent |
| JSON Minify | Tools → Format → JSON Minify | Remove whitespace |
| JSON Validate | Tools → Format → JSON Validate | Check + show errors |
| XML Pretty Print | Tools → Format → XML Pretty Print | |
| UPPERCASE | Edit → Convert Case → UPPERCASE | |
| lowercase | Edit → Convert Case → lowercase | |
| Title Case | Edit → Convert Case → Title Case | |
| camelCase | Edit → Convert Case → camelCase | |
| snake_case | Edit → Convert Case → snake_case | |
| Sort Lines Asc | Edit → Line Operations → Sort Ascending | |
| Sort Lines Desc | Edit → Line Operations → Sort Descending | |
| Sort Lines Numeric | Edit → Line Operations → Sort Numeric | |
| Remove Duplicates | Edit → Line Operations → Remove Duplicate Lines | |
| Remove Empty Lines | Edit → Line Operations → Remove Empty Lines | |
| Join Lines | Edit → Line Operations → Join Lines | |
| Reverse Lines | Edit → Line Operations → Reverse Line Order | |
| Trim Trailing | Edit → Line Operations → Trim Trailing Whitespace | |
| MD5 Hash | Tools → Hash → MD5 | Of selection |
| SHA1 Hash | Tools → Hash → SHA-1 | Of selection |
| SHA256 Hash | Tools → Hash → SHA-256 | Of selection |
| Unix Timestamp → Date | Tools → Convert → Timestamp to Date | |
| Date → Unix Timestamp | Tools → Convert → Date to Timestamp | |
| Hex → Decimal | Tools → Convert → Hex to Decimal | |
| Decimal → Hex | Tools → Convert → Decimal to Hex | |

---

## Plugin System Architecture

### Goals
- Built-in tools (base64, sort, etc.) are implemented as internal plugins
- Same API surface for internal and external plugins
- Plugins can: register commands, add UI panels, transform text, react to events
- No plugin should be able to crash the editor

### Plugin Manifest (plugin.toml)
```toml
[plugin]
name = "my-plugin"
version = "1.0.0"
description = "Does cool stuff"
author = "Someone"
license = "MIT"
min_editor_version = "0.1.0"

[[commands]]
id = "my_plugin.do_thing"
name = "Do The Thing"
category = "Tools"
keybinding = "ctrl+alt+t"  # optional

[permissions]
file_read = true
file_write = false
network = false
shell = false
```

### Plugin Types (Future)
1. **Rust native** — Compiled into editor or loaded as dynamic library
2. **WASM** — Sandboxed, safe, cross-platform (preferred for community plugins)
3. **Script** — Lua or similar for simple text transformations

### Internal Plugin Interface
For Phase 1, built-in tools use the same trait:
```rust
pub trait Tool: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn category(&self) -> &str;
    fn transform(&self, input: &str, options: &ToolOptions) -> Result<String>;
}
```
This ensures when the plugin system is built in Phase 3, migrating tools to plugins is trivial.

---

## Notepad++ Compatibility Notes

### Keybindings
- Default keybinding set matches Notepad++
- Optional keybinding presets: "Notepad++", "VS Code", "Sublime Text", "Vim" (Phase 4)
- Custom keybindings via config.toml

### Session Files
- Notepad++ stores sessions as XML in `session.xml`
- OpenEdit should be able to **import** Notepad++ session files (open the same tabs)
- OpenEdit's own session format: simpler TOML/JSON

### Theme Import
- Notepad++ themes are XML (`<NotepadPlus><LexerStyles>...`)
- OpenEdit should support importing Notepad++ theme XML files
- Map Notepad++ style IDs to tree-sitter highlight groups

### Language Detection
- Match Notepad++ file extension → language mapping
- Support `<Language>` definitions from Notepad++ `langs.xml`
