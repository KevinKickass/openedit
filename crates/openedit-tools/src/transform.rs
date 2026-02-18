use base64::Engine;
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};

use crate::ToolError;

/// Base64 encode the input string.
pub fn base64_encode(input: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(input.as_bytes())
}

/// Base64 decode the input string. Returns an error if the input is not valid Base64.
pub fn base64_decode(input: &str) -> Result<String, ToolError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(input.trim())
        .map_err(|e| ToolError::InvalidInput(format!("Invalid Base64: {}", e)))?;
    String::from_utf8(bytes)
        .map_err(|e| ToolError::InvalidInput(format!("Decoded Base64 is not valid UTF-8: {}", e)))
}

/// URL/percent-encode the input string.
pub fn url_encode(input: &str) -> String {
    utf8_percent_encode(input, NON_ALPHANUMERIC).to_string()
}

/// URL/percent-decode the input string. Returns an error if the input is not valid percent-encoded.
pub fn url_decode(input: &str) -> Result<String, ToolError> {
    percent_decode_str(input)
        .decode_utf8()
        .map(|s| s.into_owned())
        .map_err(|e| ToolError::InvalidInput(format!("Invalid URL encoding: {}", e)))
}

/// Pretty-print a JSON string with 2-space indentation.
pub fn json_pretty_print(input: &str) -> Result<String, ToolError> {
    let value: serde_json::Value = serde_json::from_str(input)
        .map_err(|e| ToolError::InvalidInput(format!("Invalid JSON: {}", e)))?;
    serde_json::to_string_pretty(&value)
        .map_err(|e| ToolError::TransformFailed(format!("JSON formatting failed: {}", e)))
}

/// Minify a JSON string (remove all unnecessary whitespace).
pub fn json_minify(input: &str) -> Result<String, ToolError> {
    let value: serde_json::Value = serde_json::from_str(input)
        .map_err(|e| ToolError::InvalidInput(format!("Invalid JSON: {}", e)))?;
    serde_json::to_string(&value)
        .map_err(|e| ToolError::TransformFailed(format!("JSON minification failed: {}", e)))
}

/// Pretty-print an XML string with 2-space indentation.
pub fn xml_pretty_print(input: &str) -> Result<String, ToolError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ToolError::InvalidInput("Empty input".to_string()));
    }
    if !trimmed.contains('<') {
        return Err(ToolError::InvalidInput("Input does not appear to be XML".to_string()));
    }

    let mut output = String::with_capacity(input.len() * 2);
    let mut depth: usize = 0;
    let mut chars = trimmed.chars().peekable();

    while chars.peek().is_some() {
        // Skip whitespace between tags
        skip_inter_tag_whitespace(&mut chars);

        if chars.peek().is_none() {
            break;
        }

        if chars.peek() == Some(&'<') {
            // Read the entire tag or comment/declaration
            let tag = read_tag(&mut chars)
                .map_err(|e| ToolError::InvalidInput(format!("Malformed XML: {}", e)))?;

            if tag.starts_with("<?") {
                // XML declaration / processing instruction
                write_indent(&mut output, depth);
                output.push_str(&tag);
                output.push('\n');
            } else if tag.starts_with("<!--") {
                // Comment
                write_indent(&mut output, depth);
                output.push_str(&tag);
                output.push('\n');
            } else if tag.starts_with("</") {
                // Closing tag
                depth = depth.saturating_sub(1);
                write_indent(&mut output, depth);
                output.push_str(&tag);
                output.push('\n');
            } else if tag.ends_with("/>") {
                // Self-closing tag
                write_indent(&mut output, depth);
                output.push_str(&tag);
                output.push('\n');
            } else if tag.starts_with("<!") {
                // DOCTYPE or other declaration
                write_indent(&mut output, depth);
                output.push_str(&tag);
                output.push('\n');
            } else {
                // Opening tag - check if the next content is text (not another tag)
                write_indent(&mut output, depth);
                output.push_str(&tag);

                // Peek ahead to see if this is an inline text element
                let text_content = read_text_content(&mut chars);
                if !text_content.is_empty() && chars.peek() == Some(&'<') {
                    // Check if the very next tag is the closing tag for this element
                    let saved: Vec<char> = chars.clone().collect();
                    let saved_str: String = saved.into_iter().collect();
                    let tag_name = extract_tag_name(&tag);
                    let expected_close = format!("</{}>", tag_name);
                    if saved_str.trim_start().starts_with(&expected_close) {
                        // Inline element: <tag>text</tag>
                        output.push_str(text_content.trim());
                        let close_tag = read_tag(&mut chars)
                            .map_err(|e| ToolError::InvalidInput(format!("Malformed XML: {}", e)))?;
                        output.push_str(&close_tag);
                        output.push('\n');
                        continue;
                    }
                }

                output.push('\n');
                depth += 1;

                // If there was non-empty text content, write it
                let trimmed_text = text_content.trim();
                if !trimmed_text.is_empty() {
                    write_indent(&mut output, depth);
                    output.push_str(trimmed_text);
                    output.push('\n');
                }
            }
        } else {
            // Text content between tags
            let text = read_text(&mut chars);
            let trimmed_text = text.trim();
            if !trimmed_text.is_empty() {
                write_indent(&mut output, depth);
                output.push_str(trimmed_text);
                output.push('\n');
            }
        }
    }

    // Remove trailing newline for clean output
    let result = output.trim_end_matches('\n').to_string();
    if result.is_empty() {
        return Err(ToolError::InvalidInput("No XML content found".to_string()));
    }
    Ok(result)
}

/// Minify an XML string (remove unnecessary whitespace).
pub fn xml_minify(input: &str) -> Result<String, ToolError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ToolError::InvalidInput("Empty input".to_string()));
    }
    if !trimmed.contains('<') {
        return Err(ToolError::InvalidInput("Input does not appear to be XML".to_string()));
    }

    let mut output = String::with_capacity(input.len());
    let mut chars = trimmed.chars().peekable();

    while chars.peek().is_some() {
        if chars.peek() == Some(&'<') {
            let tag = read_tag(&mut chars)
                .map_err(|e| ToolError::InvalidInput(format!("Malformed XML: {}", e)))?;
            output.push_str(&tag);
        } else {
            // Read text content and collapse whitespace
            let text = read_text(&mut chars);
            let trimmed_text = text.trim();
            if !trimmed_text.is_empty() {
                output.push_str(trimmed_text);
            }
        }
    }

    Ok(output)
}

/// Write indentation (2 spaces per level).
fn write_indent(output: &mut String, depth: usize) {
    for _ in 0..depth {
        output.push_str("  ");
    }
}

/// Skip whitespace characters that appear between tags.
fn skip_inter_tag_whitespace(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() && c != '<' {
            chars.next();
        } else {
            break;
        }
    }
}

/// Read a complete tag from '<' to '>', handling comments and CDATA.
fn read_tag(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Result<String, String> {
    let mut tag = String::new();
    // Consume the '<'
    if let Some(c) = chars.next() {
        tag.push(c);
    } else {
        return Err("Unexpected end of input".to_string());
    }

    // Check for comment
    if chars.peek() == Some(&'!') {
        tag.push(chars.next().unwrap());
        // Check for <!-- comment -->
        if chars.peek() == Some(&'-') {
            tag.push(chars.next().unwrap());
            if chars.peek() == Some(&'-') {
                tag.push(chars.next().unwrap());
                // Read until -->
                loop {
                    match chars.next() {
                        Some(c) => {
                            tag.push(c);
                            if tag.ends_with("-->") {
                                return Ok(tag);
                            }
                        }
                        None => return Err("Unterminated comment".to_string()),
                    }
                }
            }
        }
        // Other <! declarations (DOCTYPE, CDATA, etc.)
        let mut bracket_depth = 1;
        for c in chars.by_ref() {
            tag.push(c);
            if c == '<' {
                bracket_depth += 1;
            } else if c == '>' {
                bracket_depth -= 1;
                if bracket_depth == 0 {
                    return Ok(tag);
                }
            }
        }
        return Err("Unterminated declaration".to_string());
    }

    // Check for processing instruction <?...?>
    if chars.peek() == Some(&'?') {
        tag.push(chars.next().unwrap());
        loop {
            match chars.next() {
                Some(c) => {
                    tag.push(c);
                    if tag.ends_with("?>") {
                        return Ok(tag);
                    }
                }
                None => return Err("Unterminated processing instruction".to_string()),
            }
        }
    }

    // Regular tag: read until '>'
    let mut in_quote = false;
    let mut quote_char = '"';
    for c in chars.by_ref() {
        tag.push(c);
        if in_quote {
            if c == quote_char {
                in_quote = false;
            }
        } else if c == '"' || c == '\'' {
            in_quote = true;
            quote_char = c;
        } else if c == '>' {
            return Ok(tag);
        }
    }

    Err("Unterminated tag".to_string())
}

/// Read text content until the next '<' or end of input.
fn read_text(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut text = String::new();
    while let Some(&c) = chars.peek() {
        if c == '<' {
            break;
        }
        text.push(c);
        chars.next();
    }
    text
}

/// Read text content between a tag and the next tag, without consuming the next '<'.
fn read_text_content(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    read_text(chars)
}

/// Extract the tag name from an opening tag string like "<tagname attr=\"val\">".
fn extract_tag_name(tag: &str) -> String {
    let inner = tag.trim_start_matches('<').trim_end_matches('>').trim_end_matches('/');
    // Tag name is the first word
    inner
        .split(|c: char| c.is_whitespace() || c == '/' || c == '>')
        .next()
        .unwrap_or("")
        .to_string()
}

/// Convert a decimal number to hexadecimal.
pub fn dec_to_hex(input: &str) -> Result<String, ToolError> {
    let trimmed = input.trim();
    let n: i64 = trimmed
        .parse()
        .map_err(|e| ToolError::InvalidInput(format!("Not a valid integer: {}", e)))?;
    Ok(format!("0x{:X}", n))
}

/// Convert a hexadecimal number to decimal.
pub fn hex_to_dec(input: &str) -> Result<String, ToolError> {
    let trimmed = input.trim().trim_start_matches("0x").trim_start_matches("0X");
    let n = i64::from_str_radix(trimmed, 16)
        .map_err(|e| ToolError::InvalidInput(format!("Not a valid hex number: {}", e)))?;
    Ok(n.to_string())
}

/// Convert a Unix timestamp to a human-readable UTC date string.
pub fn timestamp_to_date(input: &str) -> Result<String, ToolError> {
    let trimmed = input.trim();
    let ts: i64 = trimmed
        .parse()
        .map_err(|e| ToolError::InvalidInput(format!("Not a valid timestamp: {}", e)))?;
    let secs = ts;
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Simple date calculation from Unix epoch (1970-01-01)
    let (year, month, day) = days_to_ymd(days);
    Ok(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        year, month, day, hours, minutes, seconds
    ))
}

/// Convert days since epoch to (year, month, day).
fn days_to_ymd(mut days: i64) -> (i64, u32, u32) {
    let mut year = 1970i64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let month_days = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u32;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days as u32 + 1)
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Encode special HTML characters as HTML entities.
///
/// Converts `&`, `<`, `>`, `"`, and `'` to their corresponding HTML entities.
pub fn html_encode(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(ch),
        }
    }
    output
}

/// Decode HTML entities back to their corresponding characters.
///
/// Supports named entities (`&amp;`, `&lt;`, `&gt;`, `&quot;`, `&#39;`)
/// and numeric entities (decimal `&#123;` and hexadecimal `&#x7B;`).
pub fn html_decode(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '&' {
            // Collect entity text up to ';' or a reasonable limit
            let mut entity = String::new();
            let mut found_semicolon = false;
            // HTML entities are at most ~10 chars long (e.g. &#xFFFFFF;)
            for _ in 0..10 {
                match chars.peek() {
                    Some(&';') => {
                        chars.next();
                        found_semicolon = true;
                        break;
                    }
                    Some(&c) => {
                        entity.push(c);
                        chars.next();
                    }
                    None => break,
                }
            }

            if found_semicolon {
                match entity.as_str() {
                    "amp" => output.push('&'),
                    "lt" => output.push('<'),
                    "gt" => output.push('>'),
                    "quot" => output.push('"'),
                    "#39" | "apos" => output.push('\''),
                    s if s.starts_with("#x") || s.starts_with("#X") => {
                        // Hexadecimal numeric entity
                        if let Ok(code) = u32::from_str_radix(&s[2..], 16) {
                            if let Some(c) = char::from_u32(code) {
                                output.push(c);
                            } else {
                                // Invalid code point, keep original
                                output.push('&');
                                output.push_str(&entity);
                                output.push(';');
                            }
                        } else {
                            output.push('&');
                            output.push_str(&entity);
                            output.push(';');
                        }
                    }
                    s if s.starts_with('#') => {
                        // Decimal numeric entity
                        if let Ok(code) = s[1..].parse::<u32>() {
                            if let Some(c) = char::from_u32(code) {
                                output.push(c);
                            } else {
                                output.push('&');
                                output.push_str(&entity);
                                output.push(';');
                            }
                        } else {
                            output.push('&');
                            output.push_str(&entity);
                            output.push(';');
                        }
                    }
                    _ => {
                        // Unknown entity, keep as-is
                        output.push('&');
                        output.push_str(&entity);
                        output.push(';');
                    }
                }
            } else {
                // No semicolon found, output the '&' and collected chars literally
                output.push('&');
                output.push_str(&entity);
            }
        } else {
            output.push(ch);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode("Hello, World!"), "SGVsbG8sIFdvcmxkIQ==");
    }

    #[test]
    fn test_base64_encode_empty() {
        assert_eq!(base64_encode(""), "");
    }

    #[test]
    fn test_base64_decode() {
        assert_eq!(
            base64_decode("SGVsbG8sIFdvcmxkIQ==").unwrap(),
            "Hello, World!"
        );
    }

    #[test]
    fn test_base64_decode_invalid() {
        assert!(base64_decode("not-valid-base64!!!").is_err());
    }

    #[test]
    fn test_url_encode() {
        assert_eq!(url_encode("hello world"), "hello%20world");
    }

    #[test]
    fn test_url_encode_special_chars() {
        assert_eq!(url_encode("a=1&b=2"), "a%3D1%26b%3D2");
    }

    #[test]
    fn test_url_decode() {
        assert_eq!(url_decode("hello%20world").unwrap(), "hello world");
    }

    #[test]
    fn test_url_decode_special_chars() {
        assert_eq!(url_decode("a%3D1%26b%3D2").unwrap(), "a=1&b=2");
    }

    #[test]
    fn test_json_pretty_print() {
        let input = r#"{"name":"test","value":42}"#;
        let expected = "{\n  \"name\": \"test\",\n  \"value\": 42\n}";
        assert_eq!(json_pretty_print(input).unwrap(), expected);
    }

    #[test]
    fn test_json_pretty_print_invalid() {
        assert!(json_pretty_print("not json").is_err());
    }

    #[test]
    fn test_json_minify() {
        let input = "{\n  \"name\": \"test\",\n  \"value\": 42\n}";
        let expected = r#"{"name":"test","value":42}"#;
        assert_eq!(json_minify(input).unwrap(), expected);
    }

    #[test]
    fn test_json_minify_invalid() {
        assert!(json_minify("{invalid}").is_err());
    }

    #[test]
    fn test_html_encode_basic() {
        assert_eq!(
            html_encode("<div class=\"test\">&</div>"),
            "&lt;div class=&quot;test&quot;&gt;&amp;&lt;/div&gt;"
        );
    }

    #[test]
    fn test_html_encode_single_quote() {
        assert_eq!(html_encode("it's"), "it&#39;s");
    }

    #[test]
    fn test_html_encode_no_special() {
        assert_eq!(html_encode("hello world"), "hello world");
    }

    #[test]
    fn test_html_encode_empty() {
        assert_eq!(html_encode(""), "");
    }

    #[test]
    fn test_html_decode_named_entities() {
        assert_eq!(
            html_decode("&lt;div class=&quot;test&quot;&gt;&amp;&lt;/div&gt;"),
            "<div class=\"test\">&</div>"
        );
    }

    #[test]
    fn test_html_decode_single_quote() {
        assert_eq!(html_decode("it&#39;s"), "it's");
    }

    #[test]
    fn test_html_decode_decimal_numeric() {
        // &#123; = '{', &#125; = '}'
        assert_eq!(html_decode("&#123;hello&#125;"), "{hello}");
    }

    #[test]
    fn test_html_decode_hex_numeric() {
        // &#x7B; = '{', &#x7D; = '}'
        assert_eq!(html_decode("&#x7B;hello&#x7D;"), "{hello}");
    }

    #[test]
    fn test_html_decode_no_entities() {
        assert_eq!(html_decode("hello world"), "hello world");
    }

    #[test]
    fn test_html_decode_empty() {
        assert_eq!(html_decode(""), "");
    }

    #[test]
    fn test_html_roundtrip() {
        let original = "<p class=\"greeting\">Hello & 'World'!</p>";
        assert_eq!(html_decode(&html_encode(original)), original);
    }

    #[test]
    fn test_html_decode_unknown_entity() {
        // Unknown named entities are preserved as-is
        assert_eq!(html_decode("&nbsp;"), "&nbsp;");
    }

    #[test]
    fn test_html_decode_ampersand_without_semicolon() {
        // Bare ampersand without valid entity should be preserved
        assert_eq!(html_decode("a & b"), "a & b");
    }

    // --- XML Pretty Print tests ---

    #[test]
    fn test_xml_pretty_print_basic() {
        let input = "<root><child>text</child></root>";
        let expected = "<root>\n  <child>text</child>\n</root>";
        assert_eq!(xml_pretty_print(input).unwrap(), expected);
    }

    #[test]
    fn test_xml_pretty_print_nested() {
        let input = "<root><parent><child>val</child></parent></root>";
        let expected = "<root>\n  <parent>\n    <child>val</child>\n  </parent>\n</root>";
        assert_eq!(xml_pretty_print(input).unwrap(), expected);
    }

    #[test]
    fn test_xml_pretty_print_self_closing() {
        let input = "<root><br/><hr /></root>";
        let expected = "<root>\n  <br/>\n  <hr />\n</root>";
        assert_eq!(xml_pretty_print(input).unwrap(), expected);
    }

    #[test]
    fn test_xml_pretty_print_declaration() {
        let input = "<?xml version=\"1.0\"?><root><item/></root>";
        let expected = "<?xml version=\"1.0\"?>\n<root>\n  <item/>\n</root>";
        assert_eq!(xml_pretty_print(input).unwrap(), expected);
    }

    #[test]
    fn test_xml_pretty_print_comment() {
        let input = "<root><!-- a comment --><child/></root>";
        let expected = "<root>\n  <!-- a comment -->\n  <child/>\n</root>";
        assert_eq!(xml_pretty_print(input).unwrap(), expected);
    }

    #[test]
    fn test_xml_pretty_print_attributes() {
        let input = "<root><item id=\"1\" name=\"test\">value</item></root>";
        let expected = "<root>\n  <item id=\"1\" name=\"test\">value</item>\n</root>";
        assert_eq!(xml_pretty_print(input).unwrap(), expected);
    }

    #[test]
    fn test_xml_pretty_print_empty_input() {
        assert!(xml_pretty_print("").is_err());
    }

    #[test]
    fn test_xml_pretty_print_not_xml() {
        assert!(xml_pretty_print("just plain text").is_err());
    }

    // --- XML Minify tests ---

    #[test]
    fn test_xml_minify_basic() {
        let input = "<root>\n  <child>text</child>\n</root>";
        let expected = "<root><child>text</child></root>";
        assert_eq!(xml_minify(input).unwrap(), expected);
    }

    #[test]
    fn test_xml_minify_nested() {
        let input = "<root>\n  <parent>\n    <child>val</child>\n  </parent>\n</root>";
        let expected = "<root><parent><child>val</child></parent></root>";
        assert_eq!(xml_minify(input).unwrap(), expected);
    }

    #[test]
    fn test_xml_minify_self_closing() {
        let input = "<root>\n  <br/>\n  <hr />\n</root>";
        let expected = "<root><br/><hr /></root>";
        assert_eq!(xml_minify(input).unwrap(), expected);
    }

    #[test]
    fn test_xml_minify_declaration() {
        let input = "<?xml version=\"1.0\"?>\n<root>\n  <item/>\n</root>";
        let expected = "<?xml version=\"1.0\"?><root><item/></root>";
        assert_eq!(xml_minify(input).unwrap(), expected);
    }

    #[test]
    fn test_xml_minify_comment() {
        let input = "<root>\n  <!-- a comment -->\n  <child/>\n</root>";
        let expected = "<root><!-- a comment --><child/></root>";
        assert_eq!(xml_minify(input).unwrap(), expected);
    }

    #[test]
    fn test_xml_minify_empty_input() {
        assert!(xml_minify("").is_err());
    }

    #[test]
    fn test_xml_minify_not_xml() {
        assert!(xml_minify("just plain text").is_err());
    }

    #[test]
    fn test_xml_roundtrip() {
        let input = "<root><child attr=\"val\">text</child><empty/></root>";
        let pretty = xml_pretty_print(input).unwrap();
        let minified = xml_minify(&pretty).unwrap();
        assert_eq!(minified, input);
    }

    // --- Hex/Decimal conversion tests ---

    #[test]
    fn test_dec_to_hex() {
        assert_eq!(dec_to_hex("255").unwrap(), "0xFF");
        assert_eq!(dec_to_hex("0").unwrap(), "0x0");
        assert_eq!(dec_to_hex("16").unwrap(), "0x10");
    }

    #[test]
    fn test_dec_to_hex_invalid() {
        assert!(dec_to_hex("abc").is_err());
    }

    #[test]
    fn test_hex_to_dec() {
        assert_eq!(hex_to_dec("0xFF").unwrap(), "255");
        assert_eq!(hex_to_dec("10").unwrap(), "16");
        assert_eq!(hex_to_dec("0x0").unwrap(), "0");
    }

    #[test]
    fn test_hex_to_dec_invalid() {
        assert!(hex_to_dec("xyz").is_err());
    }

    // --- Timestamp conversion tests ---

    #[test]
    fn test_timestamp_to_date() {
        assert_eq!(timestamp_to_date("0").unwrap(), "1970-01-01 00:00:00 UTC");
        assert_eq!(timestamp_to_date("1700000000").unwrap(), "2023-11-14 22:13:20 UTC");
    }

    #[test]
    fn test_timestamp_to_date_invalid() {
        assert!(timestamp_to_date("not_a_number").is_err());
    }
}
