use std::{ffi::OsString, path::PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Protocol {
    #[default]
    Auto,
    Kitty,
    None,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum When {
    #[default]
    Auto,
    Always,
    Never,
}
impl When {
    fn parse(value: &str, option: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            _ => Err(format!("{option} must be auto, always, or never")),
        }
    }
    pub fn enabled(self, automatic: bool) -> bool {
        match self {
            Self::Auto => automatic,
            Self::Always => true,
            Self::Never => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Sort {
    #[default]
    Name,
    Size,
    Time,
    None,
}

#[derive(Debug, Default)]
pub struct Options {
    pub paths: Vec<PathBuf>,
    pub all: bool,
    pub long: bool,
    pub human: bool,
    pub numeric: bool,
    pub header: bool,
    pub reverse: bool,
    pub sort: Sort,
    pub dirs_first: bool,
    pub directory: bool,
    pub classify: bool,
    pub fields: Vec<crate::metadata::Field>,
    pub grid: bool,
    pub one: bool,
    pub no_images: bool,
    pub color: When,
    pub icons: When,
    pub hyperlink: bool,
    pub protocol: Protocol,
    pub preview_limit: usize,
    pub thumbnail_size: Option<usize>,
    pub diagnose: bool,
    pub cache_dir: Option<PathBuf>,
    pub no_cache: bool,
    pub clear_cache: bool,
    pub cache_stats: bool,
    pub help: bool,
    pub version: bool,
}

pub const HELP: &str = "lsa — familiar listings, useful details, inline image previews

Usage: lsa [OPTIONS] [PATH ...]

Examples:
  lsa                         Automatic columns or image grid
  lsa -la --header             Details with column headings
  lsa --grid --thumbnail-size=5 photos
                              Larger thumbnails (5 terminal rows high)
  lsa --preview-limit=1024 photos
                              Preview more images in a large directory

Everyday options:
  -a, -A, --all          Include hidden entries (without . and ..)
  -l, --long            Readable details; tiny image previews on graphics terminals
  -h                    Human-readable sizes (the default); --bytes uses bytes
  -n                    Long listing with numeric uid/gid and link count
  -1, --oneline         One entry per line; no images
  -d, --directory       List directory operands themselves
  -F, --classify        Append / @ * | = type indicators
  -t / -S               Sort newest / largest first
  -r, --reverse         Reverse the order within directory groups
  -U                    Keep filesystem order (skip sorting)
  --sort=name|size|time|none
                         Natural, case-insensitive name order is the default
  --dirs-first          Group directories before other entries
  --header              Long output with column headings (implies -l)
  --fields=LIST         Choose long columns (implies -l):
                         mode,links,user,group,uid,gid,size,modified

Appearance:
  --color=auto|always|never
                         Auto on terminals; honors NO_COLOR and LS_COLORS
  --icons=auto|always|never
                         Auto: Nerd icons in Ghostty, portable symbols elsewhere
                         Always: Nerd icons (requires font support)
  --no-icons            Disable icons
  --hyperlink           Make names clickable using OSC 8 links

Images:
  --grid                Compact thumbnails and folder/file artwork for every tile
  --thumbnail-size=N    Grid height in terminal rows (1..12; default 3)
                         Width/spacing follow size; shrinks to fit the terminal
                         Long output keeps its one-row miniatures
  --no-images           Text only; never open image contents
  --protocol=auto|kitty|none
                         Auto recognizes direct Ghostty/Kitty sessions
  --preview-limit=N     At most N attempts across all paths (0..4096; default 256)
  --cache-dir=PATH      Opt-in, bounded thumbnail cache (also accepts a space)
  --no-cache            Disable caching regardless of option order
  --clear-cache         Clear the selected cache and exit; requires --cache-dir
  --cache-stats         Report cache counters to stderr
  --diagnose            Explain each layout without decoding or accessing cache
  --help / --version    Show help / version
  --                    End options before paths beginning with -

Output always stays in terminal scrollback and returns to the shell. Image-heavy
listings (at least half image candidates), small mixed listings, and individual
images preview automatically. Work is sequential and capped at 128 MiB of image
commands / 4096 placements, including artwork; remaining names print as compact text.
No pager or input handling. -l uses one-row thumbnails; -1 and --no-images keep text.
Long options (-l, -n, --header, --fields) select details over --grid; -1 and image
disabling flags also override --grid, regardless of order. A notice explains this.
Pipes default to plain names, one per line; --grid never sends images to pipes.
Unknown terminals and multiplexers use text. Preview failures are quiet and never
hide names. Filenames are complete, with terminal controls escaped.

Exit: 0 success (including closed pipes), 1 listing/output/cache-clear error,
2 invalid options. See README.md for details and resource limits.
";

impl Options {
    /// Explain explicit requests that cannot affect the selected layout once,
    /// without turning familiar text overrides into option errors.
    pub fn layout_notice(&self) -> Option<String> {
        let reason = if self.long {
            Some(if self.header {
                "--header selects long output (it implies -l)"
            } else if !self.fields.is_empty() {
                "--fields selects long output (it implies -l)"
            } else {
                "-l / -n / --long selects long output"
            })
        } else if self.one {
            Some("-1 / --oneline selects text lines")
        } else {
            None
        };
        if let Some(reason) = reason {
            let ignored = match (self.grid, self.thumbnail_size.is_some()) {
                (true, true) => "--grid and --thumbnail-size",
                (true, false) => "--grid",
                (false, true) => "--thumbnail-size",
                (false, false) => return None,
            };
            return Some(format!(
                "{ignored} ignored: {reason}; omit the long/line options to use a grid"
            ));
        }
        if self.grid {
            let reason = if self.no_images {
                "--no-images disables images"
            } else if self.protocol == Protocol::None {
                "--protocol=none disables images"
            } else if self.preview_limit == 0 {
                "--preview-limit=0 disables previews"
            } else {
                return None;
            };
            return Some(format!("--grid ignored: {reason}"));
        }
        None
    }
    pub fn cache_path(&self) -> Option<&std::path::Path> {
        self.cache_dir.as_deref().filter(|_| !self.no_cache)
    }
    pub fn needs_metadata(&self) -> bool {
        self.long || matches!(self.sort, Sort::Time | Sort::Size)
    }
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Options, String> {
    let mut opts = Options {
        preview_limit: 256,
        human: true,
        ..Options::default()
    };
    let mut operands = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if operands || arg == "-" || !arg.as_encoded_bytes().starts_with(b"-") {
            opts.paths.push(arg.into());
            continue;
        }
        let s = arg
            .to_str()
            .ok_or("options must be valid UTF-8; use -- before paths")?;
        match s {
            "--" => operands = true,
            "--help" => opts.help = true,
            "--version" => opts.version = true,
            "--grid" => opts.grid = true,
            "--long" => opts.long = true,
            "--all" | "--almost-all" => opts.all = true,
            "--oneline" => opts.one = true,
            "--directory" | "--list-dirs" => opts.directory = true,
            "--classify" => opts.classify = true,
            "--reverse" => opts.reverse = true,
            "--dirs-first" | "--group-directories-first" => opts.dirs_first = true,
            "--bytes" => opts.human = false,
            "--header" => {
                opts.header = true;
                opts.long = true;
            }
            "--no-images" => opts.no_images = true,
            "--color" | "--colour" => opts.color = When::Always,
            "--icons" => opts.icons = When::Always,
            "--no-icons" => opts.icons = When::Never,
            "--hyperlink" => opts.hyperlink = true,
            "--diagnose" => opts.diagnose = true,
            "--no-cache" => opts.no_cache = true,
            "--clear-cache" => opts.clear_cache = true,
            "--cache-stats" => opts.cache_stats = true,
            "--cache-dir" => {
                let path = args
                    .next()
                    .filter(|p| !p.is_empty())
                    .ok_or("--cache-dir requires a nonempty path")?;
                opts.cache_dir = Some(path.into());
            }
            _ if s.starts_with("--cache-dir=") => {
                if s[12..].is_empty() {
                    return Err("--cache-dir requires a nonempty path".into());
                }
                opts.cache_dir = Some(s[12..].into());
            }
            _ if s.starts_with("--fields=") => {
                opts.fields = crate::metadata::parse_fields(&s[9..])?;
                opts.long = true;
            }
            _ if s.starts_with("--color=") || s.starts_with("--colour=") => {
                opts.color = When::parse(s.split_once('=').unwrap().1, "color")?;
            }
            _ if s.starts_with("--icons=") => opts.icons = When::parse(&s[8..], "icons")?,
            _ if s.starts_with("--sort=") => {
                opts.sort = match &s[7..] {
                    "name" => Sort::Name,
                    "size" => Sort::Size,
                    "time" | "modified" => Sort::Time,
                    "none" => Sort::None,
                    _ => return Err("sort must be name, size, time, or none".into()),
                };
            }
            _ if s.starts_with("--protocol=") => {
                opts.protocol = match &s[11..] {
                    "auto" => Protocol::Auto,
                    "kitty" => Protocol::Kitty,
                    "none" => Protocol::None,
                    _ => return Err("protocol must be auto, kitty, or none".into()),
                };
            }
            _ if s == "--preview-limit"
                || s.starts_with("--preview-limit=")
                || s == "--thumbnail-size"
                || s.starts_with("--thumbnail-size=") =>
            {
                let option = s.split('=').next().unwrap();
                let value = match s.split_once('=') {
                    Some((_, value)) => value.to_owned(),
                    None => args
                        .next()
                        .and_then(|v| v.into_string().ok())
                        .ok_or_else(|| format!("{option} requires an integer"))?,
                };
                let (min, max) = if option == "--preview-limit" {
                    (0, 4096)
                } else {
                    (1, 12)
                };
                let number = value
                    .parse::<usize>()
                    .ok()
                    .filter(|n| (min..=max).contains(n))
                    .ok_or_else(|| format!("{option} must be an integer from {min} to {max}"))?;
                if option == "--preview-limit" {
                    opts.preview_limit = number;
                } else {
                    opts.thumbnail_size = Some(number);
                }
            }
            _ if s.starts_with("--") => {
                return Err(format!("unknown option: {}", crate::display::escape(&arg)));
            }
            _ => {
                for c in s[1..].chars() {
                    match c {
                        'a' | 'A' => opts.all = true,
                        'l' => opts.long = true,
                        'h' => opts.human = true,
                        'n' => {
                            opts.long = true;
                            opts.numeric = true;
                        }
                        '1' => opts.one = true,
                        'd' => opts.directory = true,
                        'F' => opts.classify = true,
                        'G' => opts.color = When::Auto,
                        'r' => opts.reverse = true,
                        't' => opts.sort = Sort::Time,
                        'S' => opts.sort = Sort::Size,
                        'U' => opts.sort = Sort::None,
                        _ => {
                            return Err(format!(
                                "unknown option: {}",
                                crate::display::escape(&arg)
                            ));
                        }
                    }
                }
            }
        }
    }
    if opts.clear_cache
        && (opts.cache_dir.is_none() || opts.no_cache || opts.diagnose || !opts.paths.is_empty())
    {
        return Err("--clear-cache requires --cache-dir and cannot combine with paths, --no-cache, or --diagnose".into());
    }
    if opts.paths.is_empty() && !opts.clear_cache {
        opts.paths.push(".".into());
    }
    Ok(opts)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(s: &[&str]) -> Result<Options, String> {
        parse(s.iter().map(OsString::from))
    }
    #[test]
    fn familiar_flags_and_operands() {
        let o = args(&["-lahSrdF", "dir", "--", "-image.png"]).unwrap();
        assert!(o.all && o.long && o.human && o.reverse && o.directory && o.classify);
        assert_eq!(o.sort, Sort::Size);
        assert_eq!(o.paths, [PathBuf::from("dir"), PathBuf::from("-image.png")]);
        assert!(args(&[]).unwrap().human);
        assert!(args(&["--bytes", "-h"]).unwrap().human);
        assert!(!args(&["-h", "--bytes"]).unwrap().human);
    }
    #[test]
    fn rejects_unknown_and_unbounded_options() {
        for a in [
            "--page",
            "--inline",
            "--unknown",
            "-z",
            "--protocol=sixel",
            "--preview-limit=-1",
            "--preview-limit=4097",
            "--thumbnail-size=0",
            "--thumbnail-size=13",
            "--thumbnail-size=1.5",
            "--color=wat",
            "--icons=wat",
            "--sort=wat",
        ] {
            assert!(args(&[a]).is_err(), "{a}");
        }
        assert_eq!(args(&["--preview-limit=0"]).unwrap().preview_limit, 0);
    }
    #[test]
    fn explicit_appearance_flags_follow_argument_order() {
        assert_eq!(
            args(&["--color=never", "--color=always"]).unwrap().color,
            When::Always
        );
        assert_eq!(args(&["--icons", "--no-icons"]).unwrap().icons, When::Never);
        assert_eq!(
            args(&["--no-icons", "--icons=auto"]).unwrap().icons,
            When::Auto
        );
    }
    #[test]
    fn metadata_options() {
        let o = args(&["--fields=size,modified", "--dirs-first", "-hr"]).unwrap();
        assert!(o.long && o.dirs_first && o.human && o.reverse);
        for a in [
            "--fields=",
            "--fields=name",
            "--fields=size,size",
            "--fields=size,",
            "--fields=unknown",
        ] {
            assert!(args(&[a]).is_err(), "{a}");
        }
    }
    #[test]
    fn preview_controls_accept_spaces_and_equal_signs() {
        assert_eq!(args(&[]).unwrap().preview_limit, 256);
        for flags in [
            vec!["--preview-limit=4096", "--thumbnail-size=12"],
            vec!["--preview-limit", "4096", "--thumbnail-size", "12"],
        ] {
            let opts = args(&flags).unwrap();
            assert_eq!(opts.preview_limit, 4096);
            assert_eq!(opts.thumbnail_size, Some(12));
        }
        for flags in [
            vec!["--preview-limit"],
            vec!["--thumbnail-size"],
            vec!["--thumbnail-size", "huge"],
        ] {
            assert!(args(&flags).is_err());
        }
    }
    #[test]
    fn explains_layout_overrides_in_either_order() {
        for flag in [
            "--header",
            "--fields=size",
            "-l",
            "-n",
            "-1",
            "--no-images",
            "--protocol=none",
            "--preview-limit=0",
        ] {
            for flags in [[flag, "--grid"], ["--grid", flag]] {
                assert!(
                    args(&flags)
                        .unwrap()
                        .layout_notice()
                        .unwrap()
                        .contains("--grid ignored:")
                );
            }
        }
        assert!(
            args(&["--header", "--thumbnail-size=5"])
                .unwrap()
                .layout_notice()
                .unwrap()
                .contains("--thumbnail-size ignored:")
        );
        assert!(args(&["--header"]).unwrap().layout_notice().is_none());
        assert!(
            args(&["--grid", "--thumbnail-size=5"])
                .unwrap()
                .layout_notice()
                .is_none()
        );
    }
    #[test]
    fn cache_controls_are_explicit_and_disable_wins() {
        assert!(args(&[]).unwrap().cache_path().is_none());
        for flags in [
            vec!["--cache-dir=local", "--no-cache"],
            vec!["--no-cache", "--cache-dir", "local"],
        ] {
            assert!(args(&flags).unwrap().cache_path().is_none());
        }
        let o = args(&["--clear-cache", "--cache-dir=local"]).unwrap();
        assert!(o.clear_cache && o.paths.is_empty());
        for flags in [
            vec!["--cache-dir"],
            vec!["--cache-dir="],
            vec!["--clear-cache"],
            vec!["--clear-cache", "--cache-dir=x", "--no-cache"],
            vec!["--clear-cache", "--cache-dir=x", "--diagnose"],
            vec!["--clear-cache", "--cache-dir=x", "some-path"],
        ] {
            assert!(args(&flags).is_err(), "{flags:?}");
        }
    }
}
