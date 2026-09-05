use std::{ffi::OsStr, fmt::Write};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// UTF-8 remains readable; invalid bytes and terminal controls remain unambiguous.
pub fn escape(name: &OsStr) -> String {
    let mut out = String::new();
    for chunk in name.as_encoded_bytes().utf8_chunks() {
        for c in chunk.valid().chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if c.is_control()
                    || matches!(c, '\u{061c}' | '\u{200e}'..='\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2066}'..='\u{2069}') =>
                {
                    write!(out, "\\u{{{:x}}}", c as u32).unwrap();
                }
                c => out.push(c),
            }
        }
        for b in chunk.invalid() {
            write!(out, "\\x{b:02x}").unwrap();
        }
    }
    out
}

// Wrap rather than truncate, so every displayed name can be inspected inline.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = vec![String::new()];
    let mut used = 0;
    for g in text.graphemes(true) {
        let w = g.width();
        if used + w > width && used > 0 {
            lines.push(String::new());
            used = 0;
        }
        lines.last_mut().unwrap().push_str(g);
        used += w;
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStrExt;
    #[test]
    fn safe_lossless_display() {
        assert_eq!(
            escape(OsStr::from_bytes(b"a\x1b[31m\n\\\xff")),
            "a\\u{1b}[31m\\n\\\\\\xff"
        );
        assert_eq!(escape(OsStr::new("桃👩‍💻")), "桃👩‍💻");
        assert_eq!(escape(OsStr::new("a\u{202e}b")), "a\\u{202e}b");
    }
    #[test]
    fn wrap_preserves_graphemes_and_content() {
        let text = "桃e\u{301}👩‍💻.png";
        let lines = wrap(text, 4);
        assert_eq!(lines.concat(), text);
        assert!(lines.iter().all(|l| l.width() <= 4));
        assert!(lines.iter().any(|l| l.contains("👩‍💻")));
    }
}
