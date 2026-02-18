use digest::Digest;

/// Compute the MD5 hash of the input string, returned as a lowercase hex string.
pub fn md5_hash(input: &str) -> String {
    let mut hasher = md5::Md5::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Compute the SHA-1 hash of the input string, returned as a lowercase hex string.
pub fn sha1_hash(input: &str) -> String {
    let mut hasher = sha1::Sha1::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Compute the SHA-256 hash of the input string, returned as a lowercase hex string.
pub fn sha256_hash(input: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5_hash() {
        // Well-known: MD5("Hello, World!") = 65a8e27d8879283831b664bd8b7f0ad4
        assert_eq!(md5_hash("Hello, World!"), "65a8e27d8879283831b664bd8b7f0ad4");
    }

    #[test]
    fn test_md5_hash_empty() {
        // MD5("") = d41d8cd98f00b204e9800998ecf8427e
        assert_eq!(md5_hash(""), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_sha1_hash() {
        // SHA-1("Hello, World!") = 0a0a9f2a6772942557ab5355d76af442f8f65e01
        assert_eq!(sha1_hash("Hello, World!"), "0a0a9f2a6772942557ab5355d76af442f8f65e01");
    }

    #[test]
    fn test_sha1_hash_empty() {
        // SHA-1("") = da39a3ee5e6b4b0d3255bfef95601890afd80709
        assert_eq!(sha1_hash(""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }

    #[test]
    fn test_sha256_hash() {
        // SHA-256("Hello, World!") = dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f
        assert_eq!(
            sha256_hash("Hello, World!"),
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        );
    }

    #[test]
    fn test_sha256_hash_empty() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        assert_eq!(
            sha256_hash(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
