use crate::{
    cli::{Options, When},
    display::escaped,
    entry::{Entry, Kind},
    filetype::Category,
    terminal::Terminal,
};
use std::{
    borrow::Cow,
    collections::HashMap,
    env,
    ffi::OsStr,
    io::{self, Write},
    path::{Component, Path},
};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Icons {
    #[default]
    None,
    Portable,
    Nerd,
}

#[derive(Default)]
pub struct Style {
    pub color: bool,
    pub icons: Icons,
    pub classify: bool,
    /// `-p`: mark only directories with `/`.
    pub slash: bool,
    hyperlinks: Option<Hyperlinks>,
    colors: Colors,
}

/// Validated LS_COLORS rules. Type keys map directly; `*suffix` rules are
/// indexed so lookups stay constant-time with the hundreds of rules that
/// generated palettes (dircolors, vivid) contain. The last matching rule wins.
#[derive(Default)]
struct Colors {
    types: HashMap<&'static str, String>,
    // Suffixes beginning with `.`, exact and ASCII-lowercased: (rule, code).
    exact: HashMap<String, (usize, String)>,
    folded: HashMap<String, (usize, String)>,
    // Other literal suffixes such as `*~`, matched by comparison.
    other: Vec<(usize, String, String)>,
}

const TYPE_KEYS: [&str; 10] = ["di", "ln", "or", "mi", "pi", "so", "bd", "cd", "ex", "fi"];
// Bounds on untrusted environment text: rule count and key/value length.
const COLOR_RULES: usize = 4096;
const COLOR_FIELD: usize = 64;

impl Colors {
    fn read(&mut self, colors: &str) {
        // Only bounded SGR values are accepted; environment text cannot inject
        // arbitrary terminal commands.
        for (index, rule) in colors.split(':').take(COLOR_RULES).enumerate() {
            let Some((key, value)) = rule.split_once('=') else {
                continue;
            };
            if key.len() > COLOR_FIELD
                || value.len() > COLOR_FIELD
                || !value.bytes().all(|b| b.is_ascii_digit() || b == b';')
            {
                continue;
            }
            if let Some(suffix) = key.strip_prefix('*') {
                if suffix.starts_with('.') {
                    let rule = (index, value.to_owned());
                    self.folded
                        .insert(suffix.to_ascii_lowercase(), rule.clone());
                    self.exact.insert(suffix.to_owned(), rule);
                } else if !suffix.is_empty() {
                    self.other.push((index, suffix.into(), value.into()));
                }
            } else if let Some(key) = TYPE_KEYS.iter().find(|k| **k == key) {
                self.types.insert(key, value.into());
            }
        }
    }

    fn get(&self, key: &str, default: &'static str) -> &str {
        self.types.get(key).map_or(default, String::as_str)
    }

    /// The last matching suffix rule. Exact case wins over an ASCII
    /// case-insensitive match (`*.jpg` also colors `PHOTO.JPG`).
    fn suffix(&self, name: &str) -> Option<&str> {
        fn later<'a>(best: &mut Option<(usize, &'a str)>, index: usize, code: &'a str) {
            if best.is_none_or(|(current, _)| index > current) {
                *best = Some((index, code));
            }
        }
        if self.exact.is_empty() && self.other.is_empty() {
            return None;
        }
        let dots = || name.match_indices('.').map(|(at, _)| &name[at..]);
        let mut best = None;
        for (index, code) in dots().filter_map(|suffix| self.exact.get(suffix)) {
            later(&mut best, *index, code);
        }
        for (index, suffix, code) in &self.other {
            if name.ends_with(suffix.as_str()) {
                later(&mut best, *index, code);
            }
        }
        if best.is_none() {
            // Case-insensitive fallback; rule keys never exceed COLOR_FIELD.
            let mut buffer = [0; COLOR_FIELD];
            for suffix in dots().filter(|suffix| suffix.len() <= COLOR_FIELD) {
                let folded = &mut buffer[..suffix.len()];
                folded.copy_from_slice(suffix.as_bytes());
                folded.make_ascii_lowercase();
                // ASCII lowercasing keeps valid UTF-8 valid.
                if let Some((index, code)) = std::str::from_utf8(folded)
                    .ok()
                    .and_then(|folded| self.folded.get(folded))
                {
                    later(&mut best, *index, code);
                }
            }
        }
        best.map(|(_, code)| code)
    }
}

impl Style {
    pub fn detect(opts: &Options, term: &Terminal) -> Self {
        let interactive =
            term.tty && env::var_os("TERM").is_some_and(|v| !v.is_empty() && v != "dumb");
        let no_color = env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
        let mut style = Self {
            color: opts.color.enabled(interactive && !no_color),
            icons: if !opts.icons.enabled(interactive) {
                Icons::None
            } else if opts.icons == When::Always
                || term.name == "ghostty"
                || env::var_os("TERM").is_some_and(|v| v == "xterm-ghostty")
            {
                Icons::Nerd
            } else {
                Icons::Portable
            },
            classify: opts.classify,
            slash: opts.slash,
            // A remote file need not exist on the machine displaying the
            // terminal. Force remains available, with a hostname in every URI.
            hyperlinks: opts
                .hyperlink
                .enabled(interactive && !term.ssh)
                .then(Hyperlinks::detect)
                .flatten(),
            ..Self::default()
        };
        if style.color
            && let Ok(colors) = env::var("LS_COLORS")
        {
            style.colors.read(&colors);
        }
        style
    }

    pub fn needs_mode(&self) -> bool {
        self.color || self.icons != Icons::None || self.classify
    }

    pub fn icon(&self, entry: &Entry) -> &'static str {
        let nerd = self.icons == Icons::Nerd;
        match entry.kind {
            Kind::Directory => {
                if nerd {
                    "\u{f07b}"
                } else {
                    "▸"
                }
            }
            Kind::Link if entry.broken => {
                if nerd {
                    "\u{f127}"
                } else {
                    "↛"
                }
            }
            Kind::Link => {
                if nerd {
                    "\u{f0c1}"
                } else {
                    "↗"
                }
            }
            Kind::Pipe => {
                if nerd {
                    "\u{f731}"
                } else {
                    "│"
                }
            }
            Kind::Socket => {
                if nerd {
                    "\u{f1e6}"
                } else {
                    "○"
                }
            }
            Kind::Device => {
                if nerd {
                    "\u{f0a0}"
                } else {
                    "▣"
                }
            }
            Kind::Unknown => "?",
            Kind::File if entry.executable && entry.class.category == Category::File => {
                if nerd {
                    "\u{f489}"
                } else {
                    "*"
                }
            }
            Kind::File => match (entry.class.category, nerd) {
                (Category::Image, true) => "\u{f1c5}",
                (Category::Image, false) => "▧",
                (Category::Video, true) => "\u{f1c8}",
                (Category::Video, false) => "▷",
                (Category::Audio, true) => "\u{f001}",
                (Category::Audio, false) => "♪",
                (Category::Archive, true) => "\u{f1c6}",
                (Category::Archive, false) => "▣",
                (Category::Code, true) => "\u{f121}",
                (Category::Code, false) => "λ",
                (Category::Config, true) => "\u{e615}",
                (Category::Config, false) => "≡",
                (Category::Document, true) => "\u{f15c}",
                (Category::Document, false) => "≡",
                (_, true) => "\u{f15b}",
                (_, false) => "·",
            },
        }
    }

    // Measure the same components that write_name streams, without building an
    // icon-prefixed String just to discard it after planning the columns.
    // The separating space and ASCII classifier reset Unicode width state, so
    // combining/emoji sequences cannot cross these component boundaries.
    pub fn label_width(&self, entry: &Entry) -> usize {
        escaped(&entry.name).width()
            + self.suffix(entry).len()
            + if self.icons == Icons::None {
                0
            } else {
                self.icon(entry).width() + 1
            }
    }

    /// Names below a thumbnail retain classification and links, without a
    /// redundant font glyph. Long-view thumbnails use the same text path.
    pub fn name<'a>(&self, entry: &'a Entry) -> Cow<'a, str> {
        let mut name = escaped(&entry.name);
        let suffix = self.suffix(entry);
        if !suffix.is_empty() {
            name.to_mut().push_str(suffix);
        }
        name
    }

    fn suffix(&self, entry: &Entry) -> &'static str {
        if !self.classify {
            return if self.slash && entry.kind == Kind::Directory {
                "/"
            } else {
                ""
            };
        }
        match entry.kind {
            Kind::Directory => "/",
            Kind::Link => "@",
            Kind::Pipe => "|",
            Kind::Socket => "=",
            Kind::File if entry.executable => "*",
            _ => "",
        }
    }

    fn code<'a>(&'a self, entry: &Entry) -> &'a str {
        let (key, default) = match entry.kind {
            Kind::Directory => ("di", "1;34"),
            Kind::Link if entry.broken => ("or", "31"),
            Kind::Link => ("ln", "36"),
            Kind::Pipe => ("pi", "33"),
            Kind::Socket => ("so", "35"),
            Kind::Device => ("bd", "33"),
            Kind::Unknown => ("fi", "31"),
            Kind::File if entry.executable && entry.class.category == Category::File => {
                ("ex", "32")
            }
            Kind::File => {
                if let Some(code) = entry
                    .name
                    .to_str()
                    .and_then(|name| self.colors.suffix(name))
                {
                    return code;
                }
                (
                    "fi",
                    // Ordinary files keep the terminal's default foreground,
                    // which stays readable on light and dark themes alike.
                    match entry.class.category {
                        Category::Image | Category::Video | Category::Audio => "35",
                        Category::Archive => "31",
                        Category::Code => "36",
                        Category::Config => "33",
                        _ => "",
                    },
                )
            }
        };
        self.colors.get(key, default)
    }

    /// Color for a long listing's ` -> target`: dim, or `mi` when dangling.
    pub fn target_code(&self, entry: &Entry) -> &str {
        if entry.broken {
            self.colors.get("mi", "31")
        } else {
            "2"
        }
    }

    /// Write a complete label. `dir` is the listing directory (empty for file
    /// operands); it locates the entry for clickable links.
    pub fn write_name(&self, out: &mut impl Write, dir: &Path, entry: &Entry) -> io::Result<()> {
        let name = escaped(&entry.name);
        let (icon, space) = if self.icons == Icons::None {
            ("", "")
        } else {
            (self.icon(entry), " ")
        };
        self.write_parts(out, dir, entry, &[icon, space, &name, self.suffix(entry)])
    }

    // Width calculation and wrapping always operate on plain text, before SGR
    // or OSC framing. Wrapping calls this separately for each label fragment.
    pub fn write_label(
        &self,
        out: &mut impl Write,
        dir: &Path,
        entry: &Entry,
        text: &str,
    ) -> io::Result<()> {
        self.write_parts(out, dir, entry, &[text])
    }

    // Keep one SGR/OSC frame around the complete label while writing borrowed
    // components. Safe names with icons or classification need no label buffer.
    fn write_parts(
        &self,
        out: &mut impl Write,
        dir: &Path,
        entry: &Entry,
        parts: &[&str],
    ) -> io::Result<()> {
        if parts.iter().all(|part| part.is_empty()) {
            return Ok(());
        }
        if let Some(links) = &self.hyperlinks {
            links.open(out, dir, &entry.name)?;
        }
        let code = if self.color { self.code(entry) } else { "" };
        if !code.is_empty() {
            write!(out, "\x1b[{code}m")?;
        }
        for part in parts {
            out.write_all(part.as_bytes())?;
        }
        if !code.is_empty() {
            out.write_all(b"\x1b[0m")?;
        }
        if self.hyperlinks.is_some() {
            out.write_all(b"\x1b]8;;\x1b\\")?;
        }
        Ok(())
    }

    pub fn paint(&self, out: &mut impl Write, code: &str, text: &str) -> io::Result<()> {
        if self.color && !code.is_empty() {
            write!(out, "\x1b[{code}m{text}\x1b[0m")
        } else {
            out.write_all(text.as_bytes())
        }
    }
}

struct Hyperlinks {
    prefix: Vec<u8>,
    root: Vec<u8>,
}

impl Hyperlinks {
    fn detect() -> Option<Self> {
        let root = env::current_dir().ok()?;
        let mut hostname = [0u8; 256];
        // SAFETY: gethostname writes at most the supplied buffer length. Reject
        // failures and missing terminators rather than emit a partial authority.
        if unsafe { libc::gethostname(hostname.as_mut_ptr().cast(), hostname.len()) } != 0 {
            return None;
        }
        let end = hostname.iter().position(|&b| b == 0)?;
        (end > 0).then(|| Self::new(&root, &hostname[..end]))
    }

    fn new(root: &Path, hostname: &[u8]) -> Self {
        let mut prefix = b"\x1b]8;;file://".to_vec();
        write_uri_bytes(&mut prefix, hostname, false).unwrap();
        let mut encoded_root = Vec::new();
        write_uri_bytes(&mut encoded_root, root.as_os_str().as_encoded_bytes(), true).unwrap();
        if !encoded_root.ends_with(b"/") {
            encoded_root.push(b'/');
        }
        Self {
            prefix,
            root: encoded_root,
        }
    }

    // `dir` is empty for a file operand, whose name is then its whole path.
    fn open(&self, out: &mut impl Write, dir: &Path, name: &OsStr) -> io::Result<()> {
        let name = Path::new(name);
        let absolute = if dir.as_os_str().is_empty() {
            name.is_absolute()
        } else {
            dir.is_absolute()
        };
        out.write_all(&self.prefix)?;
        out.write_all(if absolute { b"/" } else { &self.root })?;
        // Preserve symlink and .. semantics; no canonicalization or per-entry
        // path/URL allocation. `.` components never change the target, so they
        // are dropped (listing `.` would otherwise link `/cwd/./name`).
        let mut separator = false;
        for component in dir.components().chain(name.components()) {
            let bytes = match component {
                Component::Normal(part) => part.as_encoded_bytes(),
                Component::ParentDir => b"..",
                Component::RootDir | Component::CurDir | Component::Prefix(_) => continue,
            };
            if separator {
                out.write_all(b"/")?;
            }
            write_uri_bytes(out, bytes, false)?;
            separator = true;
        }
        out.write_all(b"\x1b\\")
    }
}

fn write_uri_bytes(out: &mut impl Write, bytes: &[u8], path: bool) -> io::Result<()> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut start = 0;
    for (index, &b) in bytes.iter().enumerate() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) || (path && b == b'/') {
            continue;
        }
        out.write_all(&bytes[start..index])?;
        out.write_all(&[b'%', HEX[usize::from(b >> 4)], HEX[usize::from(b & 15)]])?;
        start = index + 1;
    }
    out.write_all(&bytes[start..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Independent allocating reference for the streamed URI writer.
    fn file_url(path: &Path) -> String {
        use std::fmt::Write;
        let mut url = String::from("file://test-host");
        for &b in path.as_os_str().as_encoded_bytes() {
            if b.is_ascii_alphanumeric() || b"/-._~".contains(&b) {
                url.push(char::from(b));
            } else {
                write!(url, "%{b:02X}").unwrap();
            }
        }
        url
    }
    fn entry(name: &str, kind: Kind) -> Entry {
        Entry::new(name, kind)
    }

    fn label(style: &Style, entry: &Entry) -> String {
        let name = style.name(entry);
        if style.icons == Icons::None {
            name.into_owned()
        } else {
            format!("{} {name}", style.icon(entry))
        }
    }
    #[test]
    fn width_is_independent_of_colors_and_all_icons_are_one_cell() {
        for icons in [Icons::Portable, Icons::Nerd] {
            let style = Style {
                icons,
                color: true,
                ..Style::default()
            };
            for name in ["x", "x.png", "x.mp3", "x.zip", "x.rs", "x.json", "x.md"] {
                let entry = entry(name, Kind::File);
                assert_eq!(style.icon(&entry).width(), 1);
                assert_eq!(style.label_width(&entry), name.width() + 2);
                let mut out = Vec::new();
                style.write_name(&mut out, Path::new(""), &entry).unwrap();
                assert!(
                    String::from_utf8(out)
                        .unwrap()
                        .contains(&label(&style, &entry))
                );
            }
        }
    }
    #[test]
    fn streamed_names_keep_complete_widths_and_single_color_link_frames() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
        let names = [
            OsStr::new(""),
            OsStr::new("ordinary.rs"),
            OsStr::new("桃e\u{301}👩‍💻"),
            OsStr::new("\u{fe0f}\u{20e3}1\u{fe0f}\u{20e3}"),
            OsStr::new("لاא\u{200d}ל가\u{200d}"),
            OsStr::new("line\nslash\\"),
            OsStr::from_bytes(b"raw\xff\x1b"),
        ];
        for icons in [Icons::None, Icons::Portable, Icons::Nerd] {
            for classify in [false, true] {
                for color in [false, true] {
                    for hyperlink in [false, true] {
                        let mut style = Style {
                            icons,
                            classify,
                            color,
                            hyperlinks: hyperlink
                                .then(|| Hyperlinks::new(Path::new("/lsa fixtures"), b"test-host")),
                            ..Style::default()
                        };
                        // Empty codes omit SGR entirely, while the suffix rule
                        // still styles the complete icon/name/classifier label.
                        style.colors.read("di=:*.rs=38;5;123");
                        for kind in [
                            Kind::File,
                            Kind::Directory,
                            Kind::Link,
                            Kind::Pipe,
                            Kind::Socket,
                            Kind::Device,
                            Kind::Unknown,
                        ] {
                            for name in names {
                                let mut entry = Entry::new(name, kind);
                                entry.executable = true;
                                let text = label(&style, &entry);
                                assert_eq!(style.label_width(&entry), text.width());
                                let mut expected = Vec::new();
                                if !text.is_empty() {
                                    if hyperlink {
                                        write!(
                                            expected,
                                            "\x1b]8;;{}\x1b\\",
                                            file_url(
                                                &PathBuf::from("/lsa fixtures").join(&entry.name)
                                            )
                                        )
                                        .unwrap();
                                    }
                                    style
                                        .paint(&mut expected, style.code(&entry), &text)
                                        .unwrap();
                                    if hyperlink {
                                        expected.extend_from_slice(b"\x1b]8;;\x1b\\");
                                    }
                                }
                                let mut actual = Vec::new();
                                style
                                    .write_name(&mut actual, Path::new(""), &entry)
                                    .unwrap();
                                assert_eq!(actual, expected);
                            }
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn colors_cannot_inject_terminal_commands_and_last_suffix_wins() {
        let mut style = Style {
            color: true,
            ..Style::default()
        };
        style
            .colors
            .read("di=31:*.rs=35:*.rs=38;5;123:ln=\x1b]bad:fi=0");
        assert_eq!(style.code(&entry("src", Kind::Directory)), "31");
        assert_eq!(style.code(&entry("a.rs", Kind::File)), "38;5;123");
        assert_eq!(style.code(&entry("a", Kind::Link)), "36");
        assert_eq!(style.code(&entry("a", Kind::File)), "0");
    }
    #[test]
    fn ordinary_files_keep_the_default_foreground() {
        let style = Style {
            color: true,
            ..Style::default()
        };
        for name in ["notes.txt", "README", "unknown.weird", "no-extension"] {
            assert_eq!(style.code(&entry(name, Kind::File)), "", "{name}");
        }
        // Recognized categories and kinds keep their palette colors.
        assert_eq!(style.code(&entry("a.png", Kind::File)), "35");
        assert_eq!(style.code(&entry("a.zip", Kind::File)), "31");
        assert_eq!(style.code(&entry("dir", Kind::Directory)), "1;34");
        let mut out = Vec::new();
        style
            .write_name(&mut out, Path::new(""), &entry("notes.txt", Kind::File))
            .unwrap();
        assert_eq!(out, b"notes.txt");
    }
    #[test]
    fn dangling_links_stand_out_and_honor_orphan_colors() {
        let mut broken = entry("gone", Kind::Link);
        broken.broken = true;
        let working = entry("here", Kind::Link);
        let mut style = Style {
            color: true,
            icons: Icons::Nerd,
            ..Style::default()
        };
        assert_eq!((style.code(&broken), style.code(&working)), ("31", "36"));
        assert_eq!(
            (style.target_code(&broken), style.target_code(&working)),
            ("31", "2")
        );
        assert_ne!(style.icon(&broken), style.icon(&working));
        style.icons = Icons::Portable;
        assert_ne!(style.icon(&broken), style.icon(&working));
        assert_eq!(style.icon(&broken).width(), 1);
        style.colors.read("or=1;31:mi=4:ln=35");
        assert_eq!(style.code(&broken), "1;31");
        assert_eq!(style.target_code(&broken), "4");
        assert_eq!(style.code(&working), "35");
    }
    #[test]
    fn large_palettes_are_indexed_case_insensitively_and_bounded() {
        let mut style = Style {
            color: true,
            ..Style::default()
        };
        // Generated palettes hold hundreds of rules; the former 128-rule cut
        // silently dropped these later ones.
        let mut rules: Vec<String> = (0..600)
            .map(|i| format!("*.ext{i}=38;5;{}", i % 256))
            .collect();
        rules.extend([
            "*.jpg=35".into(),
            "*.JPG=36".into(),
            "*.gz=31".into(),
            "*.tar.gz=32".into(),
            "*~=2".into(),
            format!("*.{}=33", "x".repeat(64)),
            "*=7".into(),
        ]);
        style.colors.read(&rules.join(":"));
        assert_eq!(style.code(&entry("a.ext599", Kind::File)), "38;5;87");
        assert_eq!(style.code(&entry("a.jpg", Kind::File)), "35");
        assert_eq!(style.code(&entry("a.JPG", Kind::File)), "36");
        // No exact match: ASCII case-insensitive, last matching rule wins.
        assert_eq!(style.code(&entry("a.Jpg", Kind::File)), "36");
        assert_eq!(style.code(&entry("A.EXT7", Kind::File)), "38;5;7");
        assert_eq!(style.code(&entry("a.tar.gz", Kind::File)), "32");
        assert_eq!(style.code(&entry("a.gz", Kind::File)), "31");
        assert_eq!(style.code(&entry("draft.txt~", Kind::File)), "2");
        // Keys longer than 64 bytes and empty suffixes are ignored.
        assert_eq!(
            style.code(&entry(&format!("a.{}", "x".repeat(64)), Kind::File)),
            ""
        );
        assert_eq!(style.code(&entry("notes.txt", Kind::File)), "");
        // Non-UTF-8 names never match suffix rules.
        use std::os::unix::ffi::OsStrExt;
        let raw = Entry::new(OsStr::from_bytes(b"\xff.jpg"), Kind::File);
        assert_eq!(style.code(&raw), "35");
        // The rule count is bounded; later rules past the cap are ignored.
        let mut capped = Style {
            color: true,
            ..Style::default()
        };
        let mut many: Vec<String> = (0..COLOR_RULES).map(|i| format!("*.n{i}=1")).collect();
        many.push("*.late=32".into());
        capped.colors.read(&many.join(":"));
        assert_eq!(capped.code(&entry("a.n4095", Kind::File)), "1");
        assert_eq!(capped.code(&entry("a.late", Kind::File)), "");
    }
    #[test]
    fn hyperlink_percent_encodes_raw_bytes_without_resolving_targets() {
        use std::os::unix::ffi::OsStrExt;
        let links = Hyperlinks::new(Path::new("/a b"), b"host/name\x1b#?");
        for (path, expected) in [
            (&b"\xff\x1b#?"[..], "/a%20b/%FF%1B%23%3F"),
            (&b"/absolute/../link"[..], "/absolute/../link"),
            (&b"link/../file"[..], "/a%20b/link/../file"),
        ] {
            let mut out = Vec::new();
            links
                .open(&mut out, Path::new(""), std::ffi::OsStr::from_bytes(path))
                .unwrap();
            assert_eq!(
                out,
                format!("\x1b]8;;file://host%2Fname%1B%23%3F{expected}\x1b\\").as_bytes()
            );
        }
    }

    #[test]
    fn hyperlinks_drop_current_directory_components_only() {
        let links = Hyperlinks::new(Path::new("/cwd"), b"host");
        for (dir, name, expected) in [
            (".", "file", "/cwd/file"),
            ("./a/./b", "file", "/cwd/a/b/file"),
            ("", "./x/../y", "/cwd/x/../y"),
            ("", ".", "/cwd/"),
            ("/", "etc", "/etc"),
            ("", "/", "/"),
            ("sub/", "file", "/cwd/sub/file"),
            ("a//b", "c", "/cwd/a/b/c"),
        ] {
            let mut out = Vec::new();
            links
                .open(&mut out, Path::new(dir), OsStr::new(name))
                .unwrap();
            assert_eq!(
                out,
                format!("\x1b]8;;file://host{expected}\x1b\\").as_bytes(),
                "{dir:?} {name:?}"
            );
        }
    }

    #[test]
    fn hyperlink_streaming_matches_reference_for_every_byte() {
        use std::os::unix::ffi::OsStrExt;
        let links = Hyperlinks::new(Path::new("/"), b"test-host");
        let bytes: Vec<u8> = (1..=255).collect();
        let path = Path::new(std::ffi::OsStr::from_bytes(&bytes));
        let mut out = Vec::new();
        links
            .open(&mut out, Path::new(""), path.as_os_str())
            .unwrap();
        assert_eq!(
            out,
            format!("\x1b]8;;{}\x1b\\", file_url(&Path::new("/").join(path))).as_bytes()
        );
    }
}
