use memmap2::Mmap;
use ropey::Rope;
use std::fs::File;
use std::ops::Range;
use std::path::Path;

const LARGE_FILE_THRESHOLD: usize = 100 * 1024 * 1024; // 100 MB

/// Rope-based text buffer wrapping `ropey::Rope`.
///
/// Provides efficient insert/delete at arbitrary positions,
/// fast line-count lookups, and char/byte offset conversions.
#[derive(Debug, Clone)]
pub struct Buffer {
    rope: Rope,
}

impl Buffer {
    pub fn new() -> Self {
        Self { rope: Rope::new() }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
        }
    }

    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    pub fn rope_mut(&mut self) -> &mut Rope {
        &mut self.rope
    }

    /// Total number of characters in the buffer.
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Total number of bytes in the buffer.
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    /// Total number of lines (including trailing newline producing an empty final line).
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    /// Insert `text` at the given char index.
    pub fn insert(&mut self, char_idx: usize, text: &str) {
        self.rope.insert(char_idx, text);
    }

    /// Remove the char range `[start..end)`.
    pub fn remove(&mut self, range: Range<usize>) {
        self.rope.remove(range);
    }

    /// Get a single char at the given char index.
    pub fn char_at(&self, char_idx: usize) -> char {
        self.rope.char(char_idx)
    }

    /// Get the text of a line (0-indexed), including any trailing newline.
    pub fn line(&self, line_idx: usize) -> ropey::RopeSlice<'_> {
        self.rope.line(line_idx)
    }

    /// Convert a char index to a (line, col) position.
    pub fn char_to_line_col(&self, char_idx: usize) -> (usize, usize) {
        let line = self.rope.char_to_line(char_idx);
        let line_start = self.rope.line_to_char(line);
        (line, char_idx - line_start)
    }

    /// Convert a (line, col) to a char index.
    pub fn line_col_to_char(&self, line: usize, col: usize) -> usize {
        let line_start = self.rope.line_to_char(line);
        let line_len = self.line_len_chars(line);
        line_start + col.min(line_len)
    }

    /// The char index where line `line_idx` starts.
    pub fn line_to_char(&self, line_idx: usize) -> usize {
        self.rope.line_to_char(line_idx)
    }

    /// The line index that contains char `char_idx`.
    pub fn char_to_line(&self, char_idx: usize) -> usize {
        self.rope.char_to_line(char_idx)
    }

    /// Number of chars in the given line (including trailing newline chars).
    pub fn line_len_chars(&self, line_idx: usize) -> usize {
        self.rope.line(line_idx).len_chars()
    }

    /// Number of chars in the given line excluding trailing newline chars.
    pub fn line_len_chars_no_newline(&self, line_idx: usize) -> usize {
        let line = self.rope.line(line_idx);
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

    /// Get a slice of the buffer as a String.
    pub fn slice_to_string(&self, range: Range<usize>) -> String {
        self.rope.slice(range).to_string()
    }

    /// Get the entire buffer content as a String.
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }

    /// Replace the range `[start..end)` with `text`.
    pub fn replace(&mut self, range: Range<usize>, text: &str) {
        self.rope.remove(range.clone());
        self.rope.insert(range.start, text);
    }

    /// Load a file from the given path, using memory mapping for large files.
    /// Falls back to regular loading for small files or if memory mapping fails.
    pub fn load_file(path: &Path) -> std::io::Result<Self> {
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len() as usize;

        if file_size >= LARGE_FILE_THRESHOLD {
            log::info!(
                "Loading large file ({} MB) with memory mapping: {}",
                file_size / (1024 * 1024),
                path.display()
            );
            return Self::load_file_mmap(path);
        }

        // Small file: use regular loading
        let text = std::fs::read_to_string(path)?;
        Ok(Self::from_str(&text))
    }

    /// Load a file using memory mapping (for large files).
    fn load_file_mmap(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Convert bytes to string, handling encoding
        let text = String::from_utf8_lossy(&mmap).to_string();
        Ok(Self::from_str(&text))
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
}
