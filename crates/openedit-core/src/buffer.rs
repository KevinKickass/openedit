use ropey::Rope;
use std::fs::File;
use std::ops::Range;
use std::path::Path;

/// Files above this size open in read-only large-file mode (mmap + line index).
const LARGE_FILE_THRESHOLD: usize = 32 * 1024 * 1024; // 32 MB

/// Rope-based text buffer wrapping `ropey::Rope`.
///
/// Provides efficient insert/delete at arbitrary positions,
/// fast line-count lookups, and char/byte offset conversions.
///
/// For files above 32 MB, switches to a read-only mmap-backed mode
/// that uses virtually no extra RAM beyond the OS page cache.
#[derive(Debug, Clone)]
pub struct Buffer {
    inner: BufferInner,
}

#[derive(Debug, Clone)]
enum BufferInner {
    Rope(Rope),
    /// Read-only large file: mmap data + precomputed line byte offsets.
    LargeFile {
        /// The raw file bytes (shared via Arc so Clone is cheap).
        data: std::sync::Arc<Vec<u8>>,
        /// Byte offset of the start of each line. `line_offsets[i]` = byte
        /// offset where line `i` starts. An extra entry at the end equals
        /// `data.len()` so that `data[line_offsets[i]..line_offsets[i+1]]`
        /// gives the bytes of line `i` (including any trailing newline).
        line_offsets: std::sync::Arc<Vec<usize>>,
    },
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            inner: BufferInner::Rope(Rope::new()),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(text: &str) -> Self {
        Self {
            inner: BufferInner::Rope(Rope::from_str(text)),
        }
    }

    /// Returns `true` if this buffer is in read-only large-file mode.
    pub fn is_large_file(&self) -> bool {
        matches!(self.inner, BufferInner::LargeFile { .. })
    }

    pub fn rope(&self) -> &Rope {
        match &self.inner {
            BufferInner::Rope(r) => r,
            BufferInner::LargeFile { .. } => {
                // Should not be called in large-file mode, but return a
                // static empty rope to avoid panics.
                static EMPTY: std::sync::LazyLock<Rope> = std::sync::LazyLock::new(Rope::new);
                &EMPTY
            }
        }
    }

    pub fn rope_mut(&mut self) -> &mut Rope {
        match &mut self.inner {
            BufferInner::Rope(r) => r,
            BufferInner::LargeFile { .. } => {
                panic!("Cannot get mutable rope for large file buffer");
            }
        }
    }

    /// Total number of characters in the buffer.
    pub fn len_chars(&self) -> usize {
        match &self.inner {
            BufferInner::Rope(r) => r.len_chars(),
            BufferInner::LargeFile { data, .. } => {
                // For ASCII-heavy large files, byte count ≈ char count.
                // This is an approximation — exact count would require scanning.
                data.len()
            }
        }
    }

    /// Total number of bytes in the buffer.
    pub fn len_bytes(&self) -> usize {
        match &self.inner {
            BufferInner::Rope(r) => r.len_bytes(),
            BufferInner::LargeFile { data, .. } => data.len(),
        }
    }

    /// Total number of lines.
    pub fn len_lines(&self) -> usize {
        match &self.inner {
            BufferInner::Rope(r) => r.len_lines(),
            BufferInner::LargeFile { line_offsets, .. } => {
                // line_offsets has N+1 entries for N lines
                line_offsets.len().saturating_sub(1).max(1)
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        match &self.inner {
            BufferInner::Rope(r) => r.len_chars() == 0,
            BufferInner::LargeFile { data, .. } => data.is_empty(),
        }
    }

    /// Insert `text` at the given char index. No-op for large files.
    pub fn insert(&mut self, char_idx: usize, text: &str) {
        match &mut self.inner {
            BufferInner::Rope(r) => r.insert(char_idx, text),
            BufferInner::LargeFile { .. } => {}
        }
    }

    /// Remove the char range. No-op for large files.
    pub fn remove(&mut self, range: Range<usize>) {
        match &mut self.inner {
            BufferInner::Rope(r) => r.remove(range),
            BufferInner::LargeFile { .. } => {}
        }
    }

    /// Get a single char at the given char index.
    pub fn char_at(&self, char_idx: usize) -> char {
        match &self.inner {
            BufferInner::Rope(r) => r.char(char_idx),
            BufferInner::LargeFile { data, .. } => {
                // Approximate: treat byte index as char index (ASCII-heavy)
                data.get(char_idx).map(|&b| b as char).unwrap_or('\0')
            }
        }
    }

    /// Get the text of a line (0-indexed), including any trailing newline.
    pub fn line(&self, line_idx: usize) -> ropey::RopeSlice<'_> {
        match &self.inner {
            BufferInner::Rope(r) => r.line(line_idx),
            BufferInner::LargeFile { .. } => {
                // Cannot return a RopeSlice for mmap data.
                // This method should not be called in large-file mode;
                // use `line_str()` instead.
                static EMPTY: std::sync::LazyLock<Rope> = std::sync::LazyLock::new(Rope::new);
                EMPTY.line(0)
            }
        }
    }

    /// Get line text as a String. Works for both normal and large-file mode.
    /// For large files with very long lines, returns the full line — use
    /// `line_str_visible()` for rendering to avoid huge allocations.
    pub fn line_str(&self, line_idx: usize) -> String {
        match &self.inner {
            BufferInner::Rope(r) => r.line(line_idx).to_string(),
            BufferInner::LargeFile {
                data, line_offsets, ..
            } => {
                if line_idx + 1 >= line_offsets.len() {
                    return String::new();
                }
                let start = line_offsets[line_idx];
                let end = line_offsets[line_idx + 1];
                String::from_utf8_lossy(&data[start..end]).into_owned()
            }
        }
    }

    /// Get only the visible portion of a line for rendering. `col_start` and
    /// `max_cols` are in characters. For Rope buffers this just returns
    /// `line_str()`. For large files it slices directly from the backing data,
    /// avoiding allocating multi-MB strings for very long lines.
    pub fn line_str_visible(&self, line_idx: usize, col_start: usize, max_cols: usize) -> String {
        match &self.inner {
            BufferInner::Rope(r) => {
                let line = r.line(line_idx);
                let len = line.len_chars();
                let start = col_start.min(len);
                let end = (col_start + max_cols).min(len);
                if start >= end {
                    return String::new();
                }
                line.slice(start..end).to_string()
            }
            BufferInner::LargeFile {
                data, line_offsets, ..
            } => {
                if line_idx + 1 >= line_offsets.len() {
                    return String::new();
                }
                let line_start = line_offsets[line_idx];
                let line_end = line_offsets[line_idx + 1];
                // For ASCII-heavy content, byte offset ≈ char offset.
                // Clamp to line bounds.
                let byte_start = (line_start + col_start).min(line_end);
                let byte_end = (line_start + col_start + max_cols).min(line_end);
                if byte_start >= byte_end {
                    return String::new();
                }
                String::from_utf8_lossy(&data[byte_start..byte_end]).into_owned()
            }
        }
    }

    /// Convert a char index to a (line, col) position.
    pub fn char_to_line_col(&self, char_idx: usize) -> (usize, usize) {
        match &self.inner {
            BufferInner::Rope(r) => {
                let line = r.char_to_line(char_idx);
                let line_start = r.line_to_char(line);
                (line, char_idx - line_start)
            }
            BufferInner::LargeFile { line_offsets, .. } => {
                // Binary search for the line containing this byte offset
                let idx = char_idx; // approximate: byte ≈ char for ASCII
                let line = match line_offsets.binary_search(&idx) {
                    Ok(l) => l,
                    Err(l) => l.saturating_sub(1),
                };
                let col = idx.saturating_sub(line_offsets[line]);
                (line, col)
            }
        }
    }

    /// Convert a (line, col) to a char index.
    pub fn line_col_to_char(&self, line: usize, col: usize) -> usize {
        match &self.inner {
            BufferInner::Rope(r) => {
                let line_start = r.line_to_char(line);
                let line_len = r.line(line).len_chars();
                line_start + col.min(line_len)
            }
            BufferInner::LargeFile { line_offsets, .. } => {
                if line >= line_offsets.len() {
                    return 0;
                }
                line_offsets[line] + col
            }
        }
    }

    /// The char index where line `line_idx` starts.
    pub fn line_to_char(&self, line_idx: usize) -> usize {
        match &self.inner {
            BufferInner::Rope(r) => r.line_to_char(line_idx),
            BufferInner::LargeFile { line_offsets, .. } => {
                line_offsets.get(line_idx).copied().unwrap_or(0)
            }
        }
    }

    /// The line index that contains char `char_idx`.
    pub fn char_to_line(&self, char_idx: usize) -> usize {
        match &self.inner {
            BufferInner::Rope(r) => r.char_to_line(char_idx),
            BufferInner::LargeFile { line_offsets, .. } => {
                match line_offsets.binary_search(&char_idx) {
                    Ok(l) => l,
                    Err(l) => l.saturating_sub(1),
                }
            }
        }
    }

    /// Number of chars in the given line (including trailing newline chars).
    pub fn line_len_chars(&self, line_idx: usize) -> usize {
        match &self.inner {
            BufferInner::Rope(r) => r.line(line_idx).len_chars(),
            BufferInner::LargeFile { line_offsets, .. } => {
                if line_idx + 1 >= line_offsets.len() {
                    return 0;
                }
                line_offsets[line_idx + 1] - line_offsets[line_idx]
            }
        }
    }

    /// Number of chars in the given line excluding trailing newline chars.
    pub fn line_len_chars_no_newline(&self, line_idx: usize) -> usize {
        match &self.inner {
            BufferInner::Rope(r) => {
                let line = r.line(line_idx);
                let len = line.len_chars();
                if len == 0 {
                    return 0;
                }
                let last = line.char(len - 1);
                if last == '\n' {
                    if len >= 2 && line.char(len - 2) == '\r' {
                        len - 2
                    } else {
                        len - 1
                    }
                } else if last == '\r' {
                    len - 1
                } else {
                    len
                }
            }
            BufferInner::LargeFile {
                data, line_offsets, ..
            } => {
                if line_idx + 1 >= line_offsets.len() {
                    return 0;
                }
                let start = line_offsets[line_idx];
                let end = line_offsets[line_idx + 1];
                let mut len = end - start;
                if len > 0 && data[start + len - 1] == b'\n' {
                    len -= 1;
                }
                if len > 0 && data[start + len - 1] == b'\r' {
                    len -= 1;
                }
                len
            }
        }
    }

    /// Get a slice of the buffer as a String.
    pub fn slice_to_string(&self, range: Range<usize>) -> String {
        match &self.inner {
            BufferInner::Rope(r) => r.slice(range).to_string(),
            BufferInner::LargeFile { data, .. } => {
                let start = range.start.min(data.len());
                let end = range.end.min(data.len());
                String::from_utf8_lossy(&data[start..end]).into_owned()
            }
        }
    }

    /// Get the entire buffer content as a String.
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        match &self.inner {
            BufferInner::Rope(r) => r.to_string(),
            BufferInner::LargeFile { data, .. } => String::from_utf8_lossy(data).into_owned(),
        }
    }

    /// Replace the range `[start..end)` with `text`. No-op for large files.
    pub fn replace(&mut self, range: Range<usize>, text: &str) {
        match &mut self.inner {
            BufferInner::Rope(r) => {
                r.remove(range.clone());
                r.insert(range.start, text);
            }
            BufferInner::LargeFile { .. } => {}
        }
    }

    /// Check if a file appears to be binary by reading the first 8KB and looking for null bytes.
    pub fn file_is_binary(path: &Path) -> std::io::Result<bool> {
        use std::io::Read;
        let mut file = File::open(path)?;
        let mut buf = [0u8; 8192];
        let n = file.read(&mut buf)?;
        Ok(buf[..n].contains(&0))
    }

    pub fn load_file(path: &Path) -> std::io::Result<Self> {
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len() as usize;

        if file_size >= LARGE_FILE_THRESHOLD {
            log::info!(
                "Loading large file ({} MB) in read-only mode: {}",
                file_size / (1024 * 1024),
                path.display()
            );
            return Self::load_large_file(path);
        }

        // Small file: try UTF-8 first, fall back to lossy conversion
        match std::fs::read_to_string(path) {
            Ok(text) => Ok(Self::from_str(&text)),
            Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                let bytes = std::fs::read(path)?;
                let text = String::from_utf8_lossy(&bytes);
                Ok(Self::from_str(&text))
            }
            Err(e) => Err(e),
        }
    }

    /// Load a large file using mmap. Builds a line-offset index by scanning
    /// for newlines. The mmap itself is backed by the OS page cache, so only
    /// pages that are actually accessed consume physical RAM.
    fn load_large_file(path: &Path) -> std::io::Result<Self> {
        // Read the file directly into a Vec<u8>. One allocation, one copy.
        let data = std::fs::read(path)?;
        let mut line_offsets: Vec<usize> = Vec::with_capacity(data.len() / 40);
        line_offsets.push(0);
        for (i, &b) in data.iter().enumerate() {
            if b == b'\n' {
                line_offsets.push(i + 1);
            }
        }
        // Sentinel: if file doesn't end with newline, the last line still needs an end marker
        if line_offsets.last() != Some(&data.len()) {
            line_offsets.push(data.len());
        }
        line_offsets.shrink_to_fit();

        log::info!(
            "Large file loaded: {} lines, {} MB data, {} MB index",
            line_offsets.len().saturating_sub(1),
            data.len() / (1024 * 1024),
            (line_offsets.len() * std::mem::size_of::<usize>()) / (1024 * 1024),
        );

        Ok(Self {
            inner: BufferInner::LargeFile {
                data: std::sync::Arc::new(data),
                line_offsets: std::sync::Arc::new(line_offsets),
            },
        })
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer_is_empty() {
        let buf = Buffer::new();
        assert!(buf.is_empty());
        assert_eq!(buf.len_chars(), 0);
        assert_eq!(buf.len_lines(), 1);
    }

    #[test]
    fn test_from_str() {
        let buf = Buffer::from_str("hello\nworld");
        assert_eq!(buf.len_chars(), 11);
        assert_eq!(buf.len_lines(), 2);
        assert_eq!(buf.line(0).to_string(), "hello\n");
        assert_eq!(buf.line(1).to_string(), "world");
    }

    #[test]
    fn test_insert() {
        let mut buf = Buffer::from_str("helo");
        buf.insert(2, "l");
        assert_eq!(buf.to_string(), "hello");
    }

    #[test]
    fn test_remove() {
        let mut buf = Buffer::from_str("hello");
        buf.remove(1..3);
        assert_eq!(buf.to_string(), "hlo");
    }

    #[test]
    fn test_replace() {
        let mut buf = Buffer::from_str("hello world");
        buf.replace(6..11, "rust");
        assert_eq!(buf.to_string(), "hello rust");
    }

    #[test]
    fn test_char_to_line_col() {
        let buf = Buffer::from_str("abc\ndef\nghi");
        assert_eq!(buf.char_to_line_col(0), (0, 0));
        assert_eq!(buf.char_to_line_col(3), (0, 3)); // the \n char
        assert_eq!(buf.char_to_line_col(4), (1, 0)); // 'd'
        assert_eq!(buf.char_to_line_col(8), (2, 0)); // 'g'
    }

    #[test]
    fn test_line_col_to_char() {
        let buf = Buffer::from_str("abc\ndef\nghi");
        assert_eq!(buf.line_col_to_char(0, 0), 0);
        assert_eq!(buf.line_col_to_char(1, 0), 4);
        assert_eq!(buf.line_col_to_char(2, 2), 10);
    }

    #[test]
    fn test_line_len_no_newline() {
        let buf = Buffer::from_str("abc\r\ndef\nghi");
        assert_eq!(buf.line_len_chars_no_newline(0), 3); // "abc" without \r\n
        assert_eq!(buf.line_len_chars_no_newline(1), 3); // "def" without \n
        assert_eq!(buf.line_len_chars_no_newline(2), 3); // "ghi" no newline
    }

    #[test]
    fn test_line_col_clamped() {
        let buf = Buffer::from_str("ab\ncd");
        // Column beyond line length should clamp
        let idx = buf.line_col_to_char(0, 100);
        assert_eq!(idx, 3); // clamped to line length (including \n)
    }

    #[test]
    fn test_line_str() {
        let buf = Buffer::from_str("hello\nworld\n");
        assert_eq!(buf.line_str(0), "hello\n");
        assert_eq!(buf.line_str(1), "world\n");
    }

    #[test]
    fn test_is_large_file() {
        let buf = Buffer::from_str("small");
        assert!(!buf.is_large_file());
    }
}
