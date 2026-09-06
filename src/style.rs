use crate::{
    cli::{Options, When},
    display::escape,
    entry::{Entry, Kind},
    terminal::Terminal,
};
use std::{
    collections::HashMap,
    env,
    io::{self, Write},
    path::PathBuf,
};

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
    pub hyperlink_root: Option<PathBuf>,
    types: HashMap<String, String>,
    suffixes: Vec<(String, String)>,
}

impl Style {
    pub fn detect(opts: &Options, term: &Terminal) -> Self {
        let interactive = term.tty && env::var_os("TERM").is_some_and(|v| v != "dumb");
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
            hyperlink_root: opts.hyperlink.then(env::current_dir).and_then(Result::ok),
            ..Self::default()
        };
        if style.color
            && let Ok(colors) = env::var("LS_COLORS")
        {
            style.read_colors(&colors);
        }
        style
    }

    fn read_colors(&mut self, colors: &str) {
        // Only bounded SGR values are accepted; environment text cannot inject
        // arbitrary terminal commands. Support common type and literal *suffix rules.
        for rule in colors.split(':').take(128) {
            let Some((key, value)) = rule.split_once('=') else {
                continue;
            };
            if key.len() > 64
                || value.len() > 64
                || !value.bytes().all(|b| b.is_ascii_digit() || b == b';')
            {
                continue;
            }
            if let Some(suffix) = key.strip_prefix('*') {
                self.suffixes.push((suffix.into(), value.into()));
            } else if ["di", "ln", "pi", "so", "bd", "cd", "ex", "fi"].contains(&key) {
                self.types.insert(key.into(), value.into());
            }
        }
    }

    pub fn needs_mode(&self) -> bool {
        self.color || self.icons != Icons::None || self.classify
    }

    fn category(entry: &Entry) -> &'static str {
        let extension = entry
            .path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let is = |extensions: &[&str]| extensions.iter().any(|s| extension.eq_ignore_ascii_case(s));
        if is(&[
            "jpg", "jpeg", "png", "gif", "webp", "bmp", "svg", "avif", "heic", "tif", "tiff", "ico",
        ]) {
            "image"
        } else if is(&["mp4", "mov", "mkv", "webm", "avi", "m4v"]) {
            "video"
        } else if is(&["mp3", "wav", "flac", "ogg", "m4a", "aac", "aiff"]) {
            "audio"
        } else if is(&[
            "zip", "gz", "xz", "bz2", "zst", "tar", "7z", "rar", "dmg", "iso",
        ]) {
            "archive"
        } else if is(&[
            "rs", "py", "js", "ts", "tsx", "jsx", "go", "c", "h", "cpp", "hpp", "swift", "rb",
            "java", "sh", "bash", "zsh", "html", "css", "sql",
        ]) {
            "code"
        } else if is(&["json", "toml", "yaml", "yml", "xml", "ini", "conf", "lock"])
            || entry.name == ".gitignore"
            || entry.name == "Makefile"
            || entry.name == "Dockerfile"
        {
            "config"
        } else if is(&[
            "md", "txt", "pdf", "doc", "docx", "rtf", "csv", "xlsx", "log",
        ]) || entry.name == "LICENSE"
            || entry.name == "README"
        {
            "document"
        } else {
            "file"
        }
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
            Kind::File if entry.executable => {
                if nerd {
                    "\u{f489}"
                } else {
                    "*"
                }
            }
            Kind::File => match (Self::category(entry), nerd) {
                ("image", true) => "\u{f1c5}",
                ("image", false) => "▧",
                ("video", true) => "\u{f1c8}",
                ("video", false) => "▷",
                ("audio", true) => "\u{f001}",
                ("audio", false) => "♪",
                ("archive", true) => "\u{f1c6}",
                ("archive", false) => "▣",
                ("code", true) => "\u{f121}",
                ("code", false) => "λ",
                ("config", true) => "\u{e615}",
                ("config", false) => "≡",
                ("document", true) => "\u{f15c}",
                ("document", false) => "≡",
                (_, true) => "\u{f15b}",
                (_, false) => "·",
            },
        }
    }

    pub fn label(&self, entry: &Entry) -> String {
        if self.icons == Icons::None && !self.classify {
            return escape(&entry.name);
        }
        let mut label = String::new();
        if self.icons != Icons::None {
            label.push_str(self.icon(entry));
            label.push(' ');
        }
        label.push_str(&escape(&entry.name));
        if self.classify {
            label.push_str(match entry.kind {
                Kind::Directory => "/",
                Kind::Link => "@",
                Kind::Pipe => "|",
                Kind::Socket => "=",
                Kind::File if entry.executable => "*",
                _ => "",
            });
        }
        label
    }

    fn code<'a>(&'a self, entry: &Entry) -> &'a str {
        let (key, default) = match entry.kind {
            Kind::Directory => ("di", "1;34"),
            Kind::Link => ("ln", "36"),
            Kind::Pipe => ("pi", "33"),
            Kind::Socket => ("so", "35"),
            Kind::Device => ("bd", "33"),
            Kind::Unknown => ("fi", "31"),
            Kind::File if entry.executable => ("ex", "32"),
            Kind::File => {
                if let Some(name) = entry.name.to_str() {
                    for (suffix, color) in self.suffixes.iter().rev() {
                        if name.ends_with(suffix) {
                            return color;
                        }
                    }
                }
                (
                    "fi",
                    match Self::category(entry) {
                        "image" | "video" | "audio" => "35",
                        "archive" => "31",
                        "code" => "36",
                        "config" => "33",
                        _ => "",
                    },
                )
            }
        };
        self.types.get(key).map(String::as_str).unwrap_or(default)
    }

    pub fn write_name(&self, out: &mut impl Write, entry: &Entry) -> io::Result<()> {
        self.write_label(out, entry, &self.label(entry))
    }

    // Width calculation and wrapping always operate on plain text, before SGR
    // or OSC framing. Wrapping calls this separately for each label fragment.
    pub fn write_label(&self, out: &mut impl Write, entry: &Entry, text: &str) -> io::Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        if let Some(root) = &self.hyperlink_root {
            write!(out, "\x1b]8;;{}\x1b\\", file_url(&root.join(&entry.path)))?;
        }
        if self.color {
            self.paint(out, self.code(entry), text)?;
        } else {
            out.write_all(text.as_bytes())?;
        }
        if self.hyperlink_root.is_some() {
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

fn file_url(path: &std::path::Path) -> String {
    use std::fmt::Write;
    let mut url = String::from("file://");
    for &b in path.as_os_str().as_encoded_bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~".contains(&b) {
            url.push(char::from(b));
        } else {
            write!(url, "%{b:02X}").unwrap();
        }
    }
    url
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_width::UnicodeWidthStr;
    fn entry(name: &str, kind: Kind) -> Entry {
        Entry {
            path: name.into(),
            name: name.into(),
            kind,
            metadata: None,
            executable: false,
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
                assert_eq!(style.label(&entry).width(), name.width() + 2);
                let mut out = Vec::new();
                style.write_name(&mut out, &entry).unwrap();
                assert!(
                    String::from_utf8(out)
                        .unwrap()
                        .contains(&style.label(&entry))
                );
            }
        }
    }
    #[test]
    fn colors_cannot_inject_terminal_commands_and_last_suffix_wins() {
        let mut style = Style {
            color: true,
            ..Style::default()
        };
        style.read_colors("di=31:*.rs=35:*.rs=38;5;123:ln=\x1b]bad:fi=0");
        assert_eq!(style.code(&entry("src", Kind::Directory)), "31");
        assert_eq!(style.code(&entry("a.rs", Kind::File)), "38;5;123");
        assert_eq!(style.code(&entry("a", Kind::Link)), "36");
        assert_eq!(style.code(&entry("a", Kind::File)), "0");
    }
    #[test]
    fn hyperlink_percent_encodes_raw_bytes_without_resolving_targets() {
        use std::os::unix::ffi::OsStrExt;
        assert_eq!(
            file_url(std::path::Path::new(std::ffi::OsStr::from_bytes(
                b"/a b/\xff\x1b#?"
            ))),
            "file:///a%20b/%FF%1B%23%3F"
        );
    }
}
