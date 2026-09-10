use crate::{
    cli::{Options, When},
    display::escaped,
    entry::{Entry, Kind},
    filetype::{self, Category},
    terminal::Terminal,
};
use std::{
    borrow::Cow,
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
            Kind::File
                if entry.executable
                    && filetype::classify(&entry.path).category == Category::File =>
            {
                if nerd {
                    "\u{f489}"
                } else {
                    "*"
                }
            }
            Kind::File => match (filetype::classify(&entry.path).category, nerd) {
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

    pub fn label<'a>(&self, entry: &'a Entry) -> Cow<'a, str> {
        let name = self.name(entry);
        if self.icons == Icons::None {
            name
        } else {
            format!("{} {name}", self.icon(entry)).into()
        }
    }

    /// Names below a thumbnail retain classification and links, without a
    /// redundant font glyph. Long-view thumbnails use the same text path.
    pub fn name<'a>(&self, entry: &'a Entry) -> Cow<'a, str> {
        let mut name = escaped(&entry.name);
        if self.classify {
            let suffix = match entry.kind {
                Kind::Directory => "/",
                Kind::Link => "@",
                Kind::Pipe => "|",
                Kind::Socket => "=",
                Kind::File if entry.executable => "*",
                _ => "",
            };
            if !suffix.is_empty() {
                name.to_mut().push_str(suffix);
            }
        }
        name
    }

    fn code<'a>(&'a self, entry: &Entry) -> &'a str {
        let (key, default) = match entry.kind {
            Kind::Directory => ("di", "1;34"),
            Kind::Link => ("ln", "36"),
            Kind::Pipe => ("pi", "33"),
            Kind::Socket => ("so", "35"),
            Kind::Device => ("bd", "33"),
            Kind::Unknown => ("fi", "31"),
            Kind::File
                if entry.executable
                    && filetype::classify(&entry.path).category == Category::File =>
            {
                ("ex", "32")
            }
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
                    match filetype::classify(&entry.path).category {
                        Category::Image | Category::Video | Category::Audio => "35",
                        Category::Archive => "31",
                        Category::Code => "36",
                        Category::Config => "33",
                        _ => "37",
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
                        .contains(style.label(&entry).as_ref())
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
