use chardetng::EncodingDetector;
use encoding_rs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncodingError {
    #[error("Failed to decode file with encoding {0}")]
    DecodeFailed(String),
    #[error("Unknown encoding: {0}")]
    UnknownEncoding(String),
}

/// Supported encodings for file I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Encoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    Ascii,
    Latin1,
    Windows1252,
    ShiftJis,
    EucJp,
    Gb18030,
    EucKr,
    Koi8R,
}

impl Encoding {
    pub fn display_name(&self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Utf8Bom => "UTF-8 with BOM",
            Encoding::Utf16Le => "UTF-16 LE",
            Encoding::Utf16Be => "UTF-16 BE",
            Encoding::Ascii => "ASCII",
            Encoding::Latin1 => "ISO-8859-1",
            Encoding::Windows1252 => "Windows-1252",
            Encoding::ShiftJis => "Shift-JIS",
            Encoding::EucJp => "EUC-JP",
            Encoding::Gb18030 => "GB18030",
            Encoding::EucKr => "EUC-KR",
            Encoding::Koi8R => "KOI8-R",
        }
    }

    /// All supported encodings for UI display.
    pub fn all() -> &'static [Encoding] {
        &[
            Encoding::Utf8,
            Encoding::Utf8Bom,
            Encoding::Utf16Le,
            Encoding::Utf16Be,
            Encoding::Ascii,
            Encoding::Latin1,
            Encoding::Windows1252,
            Encoding::ShiftJis,
            Encoding::EucJp,
            Encoding::Gb18030,
            Encoding::EucKr,
            Encoding::Koi8R,
        ]
    }

    /// Get the encoding_rs encoder for this encoding.
    fn to_encoding_rs(&self) -> &'static encoding_rs::Encoding {
        match self {
            Encoding::Utf8 | Encoding::Utf8Bom => encoding_rs::UTF_8,
            Encoding::Utf16Le => encoding_rs::UTF_16LE,
            Encoding::Utf16Be => encoding_rs::UTF_16BE,
            Encoding::Ascii => encoding_rs::WINDOWS_1252, // ASCII is a subset
            Encoding::Latin1 | Encoding::Windows1252 => encoding_rs::WINDOWS_1252,
            Encoding::ShiftJis => encoding_rs::SHIFT_JIS,
            Encoding::EucJp => encoding_rs::EUC_JP,
            Encoding::Gb18030 => encoding_rs::GB18030,
            Encoding::EucKr => encoding_rs::EUC_KR,
            Encoding::Koi8R => encoding_rs::KOI8_R,
        }
    }

    /// Detect the encoding of raw bytes.
    pub fn detect(bytes: &[u8]) -> Self {
        // Check BOM first
        if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            return Encoding::Utf8Bom;
        }
        if bytes.starts_with(&[0xFF, 0xFE]) {
            return Encoding::Utf16Le;
        }
        if bytes.starts_with(&[0xFE, 0xFF]) {
            return Encoding::Utf16Be;
        }

        // Use chardetng for non-BOM detection
        let mut detector = EncodingDetector::new();
        detector.feed(bytes, true);
        let enc = detector.guess(None, true);

        if enc == encoding_rs::UTF_8 {
            Encoding::Utf8
        } else if enc == encoding_rs::SHIFT_JIS {
            Encoding::ShiftJis
        } else if enc == encoding_rs::EUC_JP {
            Encoding::EucJp
        } else if enc == encoding_rs::GB18030 {
            Encoding::Gb18030
        } else if enc == encoding_rs::EUC_KR {
            Encoding::EucKr
        } else if enc == encoding_rs::KOI8_R {
            Encoding::Koi8R
        } else if enc == encoding_rs::WINDOWS_1252 {
            Encoding::Windows1252
        } else {
            Encoding::Utf8 // fallback
        }
    }

    /// Decode raw bytes to a String using this encoding.
    pub fn decode(&self, bytes: &[u8]) -> Result<String, EncodingError> {
        let data = match self {
            Encoding::Utf8Bom => {
                if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                    &bytes[3..]
                } else {
                    bytes
                }
            }
            _ => bytes,
        };

        let enc = self.to_encoding_rs();
        let (result, _, had_errors) = enc.decode(data);
        if had_errors {
            Err(EncodingError::DecodeFailed(self.display_name().to_string()))
        } else {
            Ok(result.into_owned())
        }
    }

    /// Encode a String to raw bytes using this encoding.
    pub fn encode(&self, text: &str) -> Vec<u8> {
        let enc = self.to_encoding_rs();
        let mut bytes = Vec::new();

        // Add BOM if needed
        if *self == Encoding::Utf8Bom {
            bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
        }

        let (result, _, _) = enc.encode(text);
        bytes.extend_from_slice(&result);
        bytes
    }
}

impl Default for Encoding {
    fn default() -> Self {
        Encoding::Utf8
    }
}

impl std::fmt::Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_utf8() {
        let bytes = "hello world".as_bytes();
        assert_eq!(Encoding::detect(bytes), Encoding::Utf8);
    }

    #[test]
    fn test_detect_utf8_bom() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("hello".as_bytes());
        assert_eq!(Encoding::detect(&bytes), Encoding::Utf8Bom);
    }

    #[test]
    fn test_detect_utf16le_bom() {
        let bytes = vec![0xFF, 0xFE, b'h', 0, b'i', 0];
        assert_eq!(Encoding::detect(&bytes), Encoding::Utf16Le);
    }

    #[test]
    fn test_roundtrip_utf8() {
        let text = "Hello, 世界! 🌍";
        let enc = Encoding::Utf8;
        let bytes = enc.encode(text);
        let decoded = enc.decode(&bytes).unwrap();
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_roundtrip_utf8_bom() {
        let text = "hello";
        let enc = Encoding::Utf8Bom;
        let bytes = enc.encode(text);
        assert!(bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
        let decoded = enc.decode(&bytes).unwrap();
        assert_eq!(decoded, text);
    }
}
