/// Common utilities

/// Truncates a string to `max_bytes` bytes while safely respecting UTF-8 boundaries.
/// Never cuts in the middle of a multi-byte character.
#[inline]
pub fn trunc(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Truncates a string to `max_bytes` bytes and appends suffix if truncated.
/// Returns an owned String.
pub fn trunc_owned(s: &str, max_bytes: usize, suffix: &str) -> String {
    if s.len() <= max_bytes {
        s.to_string()
    } else {
        format!("{}{}", trunc(s, max_bytes), suffix)
    }
}

/// For terminal preview: replaces newlines with ↵ and truncates to max_bytes.
pub fn preview(s: &str, max_bytes: usize) -> String {
    let replaced = s.replace('\n', "↵");
    trunc_owned(&replaced, max_bytes, "...")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trunc_short_string() {
        assert_eq!(trunc("hello", 10), "hello");
    }

    #[test]
    fn trunc_exact_length() {
        assert_eq!(trunc("hello", 5), "hello");
    }

    #[test]
    fn trunc_long_ascii() {
        assert_eq!(trunc("hello world", 5), "hello");
    }

    #[test]
    fn trunc_empty() {
        assert_eq!(trunc("", 10), "");
    }

    #[test]
    fn trunc_multibyte_korean() {
        // "안녕" = 6 bytes (3 bytes each), truncating at 5 should give "안" (3 bytes)
        let s = "안녕하세요";
        let t = trunc(s, 5);
        assert!(s.is_char_boundary(t.len()), "must end on char boundary");
        assert!(t.len() <= 5);
    }

    #[test]
    fn trunc_multibyte_exact() {
        // "안" = 3 bytes, trunc at 3 should give "안"
        assert_eq!(trunc("안녕", 3), "안");
    }

    #[test]
    fn trunc_owned_no_suffix_when_fits() {
        assert_eq!(trunc_owned("hi", 10, "..."), "hi");
    }

    #[test]
    fn trunc_owned_adds_suffix_when_truncated() {
        let result = trunc_owned("hello world", 5, "...");
        assert_eq!(result, "hello...");
    }

    #[test]
    fn preview_replaces_newlines() {
        let result = preview("line1\nline2", 100);
        assert!(result.contains("↵"), "newline should be replaced");
        assert!(!result.contains('\n'));
    }

    #[test]
    fn preview_truncates_long() {
        let s = "a".repeat(50);
        let result = preview(&s, 10);
        assert!(result.len() <= 13); // 10 + "..." = 13
    }
}
