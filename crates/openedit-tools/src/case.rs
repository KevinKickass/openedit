/// Convert text to UPPERCASE.
pub fn to_uppercase(text: &str) -> String {
    text.to_uppercase()
}

/// Convert text to lowercase.
pub fn to_lowercase(text: &str) -> String {
    text.to_lowercase()
}

/// Convert text to Title Case.
pub fn to_title_case(text: &str) -> String {
    text.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    let lower: String = chars.collect::<String>().to_lowercase();
                    format!("{}{}", upper, lower)
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Split text into words, handling snake_case, camelCase, PascalCase, kebab-case,
/// and whitespace-separated words.
fn split_into_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch == '_' || ch == '-' || ch.is_whitespace() {
            if !current.is_empty() {
                words.push(current.clone());
                current.clear();
            }
        } else if ch.is_uppercase() && !current.is_empty() {
            // camelCase boundary: push accumulated word and start new one
            words.push(current.clone());
            current.clear();
            current.push(ch);
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// Convert text to camelCase: "hello_world" -> "helloWorld"
pub fn to_camel_case(text: &str) -> String {
    let words = split_into_words(text);
    let mut result = String::new();
    for (i, word) in words.iter().enumerate() {
        if i == 0 {
            result.push_str(&word.to_lowercase());
        } else {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                result.extend(first.to_uppercase());
                result.push_str(&chars.collect::<String>().to_lowercase());
            }
        }
    }
    result
}

/// Convert text to snake_case: "helloWorld" -> "hello_world"
pub fn to_snake_case(text: &str) -> String {
    let words = split_into_words(text);
    words.iter()
        .map(|w| w.to_lowercase())
        .collect::<Vec<_>>()
        .join("_")
}

/// Convert text to PascalCase: "hello_world" -> "HelloWorld"
pub fn to_pascal_case(text: &str) -> String {
    let words = split_into_words(text);
    let mut result = String::new();
    for word in &words {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            result.extend(first.to_uppercase());
            result.push_str(&chars.collect::<String>().to_lowercase());
        }
    }
    result
}

/// Convert text to kebab-case: "hello_world" -> "hello-world"
pub fn to_kebab_case(text: &str) -> String {
    let words = split_into_words(text);
    words.iter()
        .map(|w| w.to_lowercase())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uppercase() {
        assert_eq!(to_uppercase("hello World"), "HELLO WORLD");
    }

    #[test]
    fn test_lowercase() {
        assert_eq!(to_lowercase("HELLO World"), "hello world");
    }

    #[test]
    fn test_title_case() {
        assert_eq!(to_title_case("hello world"), "Hello World");
    }

    #[test]
    fn test_camel_case_from_snake() {
        assert_eq!(to_camel_case("hello_world"), "helloWorld");
    }

    #[test]
    fn test_camel_case_from_kebab() {
        assert_eq!(to_camel_case("hello-world"), "helloWorld");
    }

    #[test]
    fn test_camel_case_from_pascal() {
        assert_eq!(to_camel_case("HelloWorld"), "helloWorld");
    }

    #[test]
    fn test_snake_case_from_camel() {
        assert_eq!(to_snake_case("helloWorld"), "hello_world");
    }

    #[test]
    fn test_snake_case_from_pascal() {
        assert_eq!(to_snake_case("HelloWorld"), "hello_world");
    }

    #[test]
    fn test_snake_case_from_kebab() {
        assert_eq!(to_snake_case("hello-world"), "hello_world");
    }

    #[test]
    fn test_pascal_case_from_snake() {
        assert_eq!(to_pascal_case("hello_world"), "HelloWorld");
    }

    #[test]
    fn test_pascal_case_from_camel() {
        assert_eq!(to_pascal_case("helloWorld"), "HelloWorld");
    }

    #[test]
    fn test_kebab_case_from_snake() {
        assert_eq!(to_kebab_case("hello_world"), "hello-world");
    }

    #[test]
    fn test_kebab_case_from_camel() {
        assert_eq!(to_kebab_case("helloWorld"), "hello-world");
    }
}
