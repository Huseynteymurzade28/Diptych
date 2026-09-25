// ─── Text Helpers ───

/// Shortens `s` to at most `max_chars` characters, ending with `…` when cut.
///
/// Counts Unicode scalar values, not bytes, so multi-byte names
/// (e.g. `çok_güzel_dosya.txt`) are never split mid-character.
pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let kept: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{}…", kept)
}

#[cfg(test)]
mod tests {
    use super::truncate_chars;

    #[test]
    fn short_names_are_untouched() {
        assert_eq!(truncate_chars("notes.txt", 18), "notes.txt");
        assert_eq!(truncate_chars("", 18), "");
    }

    #[test]
    fn long_ascii_is_cut_with_ellipsis() {
        let out = truncate_chars("abcdefghijklmnopqrstuvwxyz", 10);
        assert_eq!(out, "abcdefghi…");
        assert_eq!(out.chars().count(), 10);
    }

    #[test]
    fn multibyte_names_do_not_panic() {
        // Byte 17 falls inside 'ğ' here — the old byte-slicing code panicked.
        let out = truncate_chars("çok_güzel_dosya_adı_uzun.txt", 18);
        assert_eq!(out, "çok_güzel_dosya_a…");
        assert_eq!(out.chars().count(), 18);

        assert_eq!(truncate_chars("📁📁📁📁📁", 3), "📁📁…");
    }

    #[test]
    fn exact_length_is_untouched() {
        assert_eq!(truncate_chars("şşşşş", 5), "şşşşş");
    }
}
