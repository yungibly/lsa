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

/// Which timestamp long listings show and `-t` sorts by.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Time {
    #[default]
    Modified,
    Changed,
    Accessed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Sort {
    #[default]
    Name,
    Size,
    Time,
    None,
}

/// Which symbolic links supply target information and directory contents.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Dereference {
    #[default]
    Default,
    CommandLine,
    All,
    Never,
}

#[derive(Debug, Default)]
pub struct Options {
    pub paths: Vec<PathBuf>,
    pub all: bool,
    pub dot_entries: bool,
    pub recursive: bool,
    pub dereference: Dereference,
    pub long: bool,
    pub columns: bool,
    pub commas: bool,
    pub width: Option<usize>,
    /// `-x`: fill compact columns across rows instead of down columns.
    pub across: bool,
    pub twelve_hour: bool,
    pub human: bool,
    pub kibibytes: bool,
    pub quote_controls: bool,
    pub xattrs: bool,
    pub acl: bool,
    pub flags: bool,
    pub context: bool,
    pub numeric: bool,
    /// `-g` / `-o`: long listing without the owner / group column.
    pub no_owner: bool,
    pub no_group: bool,
    /// `-i` / `-s`: prepend inode / allocated-size columns.
    pub inode: bool,
    pub blocks: bool,
    pub time: Time,
    pub header: bool,
    pub reverse: bool,
    pub sort: Sort,
    pub dirs_first: bool,
    pub directory: bool,
    pub classify: bool,
    pub slash: bool,
    pub fields: Vec<crate::metadata::Field>,
    pub grid: bool,
    pub one: bool,
    pub no_images: bool,
    pub color: When,
    pub icons: When,
    pub hyperlink: When,
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
  lsa                         Automatic details or image grid
  lsa -la --header             Details with column headings
  lsa --grid --thumbnail-size=5 photos
                        Larger thumbnails (5 terminal rows high)
  lsa --preview-limit=1024 photos
                        Preview more images in a large directory

Everyday options:
  -a, --all             Include hidden entries, including . and ..
  -A, --almost-all      Include hidden entries, except . and ..
  -l, --long            Show details; tiny previews on graphics terminals
  -C, --columns         Compact columns sorted down; no automatic image grid
  -x                    Compact columns sorted across rows
  -m                    Comma-separated names, wrapped to the available width
  -w, --width=N         Set output width in columns (1..65535)
  --12-hour             Show modification times with AM/PM (default: 24-hour)
  -h, --human-readable  Human-readable sizes (the default); --bytes uses bytes
  -k                    Show allocated space in 1,024-byte units; file sizes in bytes
  -n                    Long listing with numeric uid/gid and link count
  -o / -g               Long listing without group / without owner
  -i / -s               Prepend inode numbers / allocated size in text listings
  -c / -u               Show and sort (-t) by status change / access time
  -1, --oneline         One entry per line; no images
  -d, --directory       List directory operands themselves
  -R, --recursive       List subdirectories recursively; detect directory loops
  -H / -L / -P          Follow operand links / all links / no links
                        Default: follow directory operands unless -l, -d or -F
  -F, --classify        Append / @ * | = type indicators
  -p                    Append / to directories only
  -t / -S               Sort newest / largest first
  -r, --reverse         Reverse the order within directory groups
  -U                    Keep filesystem order (skip sorting)
  -f                    Include hidden entries and . / .., skip sorting
  -v                    Natural name order (already the default)
  --sort=name|size|time|none
                        Natural, case-insensitive name order is the default
  --dirs-first          Group directories before other entries
  --header              Long output with column headings (implies -l)
  --fields=LIST         Choose long columns (implies -l): mode,links,user,
                        group,uid,gid,size,allocated,inode,modified,flags
  -@, --xattrs          List extended attribute names and sizes (implies -l)
  -e, --acl             Show access control lists (implies -l)
  -O, --flags           Show filesystem flags where supported (implies -l)
  -Z, --context         Show security context where supported (implies -l)
  --time=mtime|ctime|atime
                        Select modification, status-change or access time

Appearance:
  -q / -b               Replace control characters with ? / escape them (default)
  --color=auto|always|never
                        Auto on terminals; honors NO_COLOR and LS_COLORS
  --icons=auto|always|never
                        Auto: Nerd icons in Ghostty, portable symbols elsewhere
                        Always: Nerd icons (requires font support)
  --no-icons            Disable icons
  --hyperlink=auto|always|never
                        Auto: clickable names on local terminals (OSC 8)
  --hyperlink           Force links, including in pipes and SSH sessions
  --no-hyperlink        Disable links

Images:
  --grid                Thumbnail grid; built-in artwork for other entries
  --thumbnail-size=N    Grid height in terminal rows (1..12; default 3)
                        Width/spacing follow size; shrinks to fit the terminal
                        Long output keeps its one-row miniatures
  --no-images           Text only; never open image contents
  --protocol=auto|kitty|none
                        Auto recognizes direct Ghostty/Kitty sessions
  --preview-limit=N     Preview attempts across all paths (0..4096; default 256)
  --cache-dir=PATH      Opt-in, bounded thumbnail cache (also accepts a space)
  --no-cache            Disable caching regardless of option order
  --clear-cache         Clear the selected cache and exit; requires --cache-dir
  --cache-stats         Report cache counters to stderr
  --diagnose            Explain each layout without decoding or accessing cache
  --help / --version    Show help / version
  --                    End options before paths beginning with -

Output always stays in terminal scrollback and returns to the shell. Image-heavy
listings (at least half image candidates), small mixed listings, and individual
images preview automatically. Up to four decoders work ahead in listing order.
Output is capped at 128 MiB of image commands / 4096 placements, including
artwork; remaining names print as compact text. Terminal text defaults to long
details; -l uses one-row thumbnails; -1 and --no-images keep text. No pager or
input handling. The last text format (-l, -C, -x, -1 or -m) wins; options that
imply long output also select that format. Explicit text formats override --grid,
as do image-disabling flags. A notice explains this. Pipes default to plain
names, one per line; explicit columns and comma-separated output still apply;
--grid never sends images to pipes. Unknown terminals, multiplexers and SSH
sessions use text. On macOS, HEIC/HEIF, AVIF, TIFF, JPEG XL, PSD and camera RAW
previews use the system image decoders. Preview failures are quiet and never
hide names. Filenames are complete, with terminal controls escaped.

Exit: 0 success (including closed pipes), 1 listing/output/cache-clear error,
2 invalid options. See README.md for details and resource limits.
";

impl Options {
    /// The most recent explicit text format wins. Decorations and metadata
    /// selections survive so a later -l can use them again.
    fn format(&mut self, format: char) {
        self.long = format == 'l';
        self.columns = matches!(format, 'C' | 'x');
        self.across = format == 'x';
        self.one = format == '1';
        self.commas = format == 'm';
    }
    fn hidden(&mut self, dots: bool) {
        self.all = true;
        self.dot_entries = dots;
    }
    fn readable_sizes(&mut self) {
        self.human = true;
        self.kibibytes = false;
    }
    fn byte_sizes(&mut self) {
        self.human = false;
        self.kibibytes = false;
    }
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
        } else if self.columns {
            Some("-C / -x / --columns selects compact text columns")
        } else if self.commas {
            Some("-m selects comma-separated text")
        } else if self.inode || self.blocks {
            Some("-i / -s selects text with metadata prefixes")
        } else {
            None
        };
        if let Some(reason) = reason {
            let ignored = self.grid_request()?;
            return Some(format!(
                "{ignored} ignored: {reason}; omit the long/column/line options to use a grid"
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
    /// Explicit grid options, named for notices when they cannot apply.
    pub fn grid_request(&self) -> Option<&'static str> {
        match (self.grid, self.thumbnail_size.is_some()) {
            (true, true) => Some("--grid and --thumbnail-size"),
            (true, false) => Some("--grid"),
            (false, true) => Some("--thumbnail-size"),
            (false, false) => None,
        }
    }
    pub fn cache_path(&self) -> Option<&std::path::Path> {
        self.cache_dir.as_deref().filter(|_| !self.no_cache)
    }
    pub fn needs_metadata(&self) -> bool {
        self.long || self.inode || self.blocks || matches!(self.sort, Sort::Time | Sort::Size)
    }
}

fn parse_width(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|n| (1..=65535).contains(n))
        .ok_or_else(|| "width must be an integer from 1 to 65535".into())
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
            "--long" => opts.format('l'),
            "--columns" => opts.format('C'),
            "--recursive" => opts.recursive = true,
            "--dereference-command-line" => opts.dereference = Dereference::CommandLine,
            "--dereference" => opts.dereference = Dereference::All,
            "--no-dereference" => opts.dereference = Dereference::Never,
            "--12-hour" => opts.twelve_hour = true,
            "--all" => opts.hidden(true),
            "--almost-all" => opts.hidden(false),
            "--oneline" => opts.format('1'),
            "--human-readable" => opts.readable_sizes(),
            "--numeric-uid-gid" => {
                opts.numeric = true;
                opts.format('l');
            }
            "--inode" => opts.inode = true,
            "--size" => opts.blocks = true,
            "--no-group" => opts.no_group = true,
            "--hide-control-chars" => opts.quote_controls = true,
            "--escape" => opts.quote_controls = false,
            "--xattrs" | "--acl" | "--flags" | "--context" => {
                match s {
                    "--xattrs" => opts.xattrs = true,
                    "--acl" => opts.acl = true,
                    "--flags" => opts.flags = true,
                    "--context" => opts.context = true,
                    _ => unreachable!(),
                }
                opts.format('l');
            }
            "--directory" | "--list-dirs" => opts.directory = true,
            "--classify" => {
                opts.classify = true;
                opts.slash = false;
            }
            "--reverse" => opts.reverse = true,
            "--dirs-first" | "--group-directories-first" => opts.dirs_first = true,
            "--bytes" => opts.byte_sizes(),
            "--header" => {
                opts.header = true;
                opts.format('l');
            }
            "--no-images" => opts.no_images = true,
            "--color" | "--colour" => opts.color = When::Always,
            "--icons" => opts.icons = When::Always,
            "--no-icons" => opts.icons = When::Never,
            "--hyperlink" => opts.hyperlink = When::Always,
            "--no-hyperlink" => opts.hyperlink = When::Never,
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
            _ if s == "--width" || s.starts_with("--width=") => {
                let value = match s.split_once('=') {
                    Some((_, value)) => value.to_owned(),
                    None => args
                        .next()
                        .and_then(|v| v.into_string().ok())
                        .ok_or("--width requires an integer")?,
                };
                opts.width = Some(parse_width(&value)?);
            }
            _ if s.starts_with("--fields=") => {
                opts.fields = crate::metadata::parse_fields(&s[9..])?;
                opts.format('l');
            }
            _ if s.starts_with("--color=") || s.starts_with("--colour=") => {
                opts.color = When::parse(s.split_once('=').unwrap().1, "color")?;
            }
            _ if s.starts_with("--icons=") => opts.icons = When::parse(&s[8..], "icons")?,
            _ if s.starts_with("--hyperlink=") => {
                opts.hyperlink = When::parse(&s[12..], "hyperlink")?;
            }
            _ if s.starts_with("--time=") => {
                opts.time = match &s[7..] {
                    "mtime" | "modified" | "modification" => Time::Modified,
                    "ctime" | "changed" | "status" => Time::Changed,
                    "atime" | "accessed" | "access" | "use" => Time::Accessed,
                    _ => return Err("time must be mtime, ctime, or atime".into()),
                };
            }
            _ if s.starts_with("--sort=") => {
                opts.sort = match &s[7..] {
                    "name" | "version" => Sort::Name,
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
                let mut flags = s[1..].chars();
                while let Some(c) = flags.next() {
                    match c {
                        'a' => opts.hidden(true),
                        'A' => opts.hidden(false),
                        'l' | 'C' | 'x' | '1' | 'm' => opts.format(c),
                        'h' => opts.readable_sizes(),
                        'k' => {
                            opts.human = false;
                            opts.kibibytes = true;
                        }
                        'w' => {
                            let value = if flags.as_str().is_empty() {
                                args.next()
                                    .and_then(|v| v.into_string().ok())
                                    .ok_or("-w requires an integer")?
                            } else {
                                flags.by_ref().collect()
                            };
                            opts.width = Some(parse_width(&value)?);
                        }
                        'q' => opts.quote_controls = true,
                        'b' => opts.quote_controls = false,
                        'H' => opts.dereference = Dereference::CommandLine,
                        'L' => opts.dereference = Dereference::All,
                        'P' => opts.dereference = Dereference::Never,
                        '@' | 'e' | 'O' | 'Z' => {
                            match c {
                                '@' => opts.xattrs = true,
                                'e' => opts.acl = true,
                                'O' => opts.flags = true,
                                'Z' => opts.context = true,
                                _ => unreachable!(),
                            }
                            opts.format('l');
                        }
                        'n' => {
                            opts.format('l');
                            opts.numeric = true;
                        }
                        'o' => {
                            opts.format('l');
                            opts.no_group = true;
                        }
                        'g' => {
                            opts.format('l');
                            opts.no_owner = true;
                        }
                        'i' => opts.inode = true,
                        's' => opts.blocks = true,
                        'c' => opts.time = Time::Changed,
                        'u' => opts.time = Time::Accessed,
                        'd' => opts.directory = true,
                        'F' => {
                            opts.classify = true;
                            opts.slash = false;
                        }
                        'p' => {
                            opts.slash = true;
                            opts.classify = false;
                        }
                        'G' => opts.color = When::Auto,
                        'r' => opts.reverse = true,
                        't' => opts.sort = Sort::Time,
                        'S' => opts.sort = Sort::Size,
                        'U' => opts.sort = Sort::None,
                        'v' => opts.sort = Sort::Name,
                        'R' => opts.recursive = true,
                        'f' => {
                            opts.hidden(true);
                            opts.sort = Sort::None;
                            opts.reverse = false;
                        }
                        _ if s.len() > 2 => {
                            return Err(format!(
                                "unknown option -{} in {}",
                                crate::display::escape(std::ffi::OsStr::new(
                                    c.encode_utf8(&mut [0; 4])
                                )),
                                crate::display::escape(&arg)
                            ));
                        }
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
        assert!(!args(&[]).unwrap().long);
        assert!(args(&["--12-hour"]).unwrap().twelve_hour);
        assert!(args(&["--bytes", "-h"]).unwrap().human);
        assert!(!args(&["-h", "--bytes"]).unwrap().human);
    }
    #[test]
    fn ls_compatibility_flags() {
        let o = args(&["-x"]).unwrap();
        assert!(o.columns && o.across);
        for (flags, across) in [
            (&["-xC"][..], false),
            (&["-Cx"], true),
            (&["-x", "--columns"], false),
        ] {
            assert_eq!(args(flags).unwrap().across, across, "{flags:?}");
        }
        let o = args(&["-pv"]).unwrap();
        assert!(o.slash && !o.classify && o.sort == Sort::Name);
        assert_eq!(args(&["-tv"]).unwrap().sort, Sort::Name);
        let o = args(&["-is"]).unwrap();
        assert!(!o.long && o.inode && o.blocks && o.needs_metadata());
        let o = args(&["-o"]).unwrap();
        assert!(o.long && o.no_group && !o.no_owner);
        let o = args(&["-g"]).unwrap();
        assert!(o.long && o.no_owner && !o.no_group);
        assert_eq!(args(&[]).unwrap().time, Time::Modified);
        assert_eq!(args(&["-c"]).unwrap().time, Time::Changed);
        assert_eq!(args(&["-cu"]).unwrap().time, Time::Accessed);
        assert!(!args(&["-c"]).unwrap().long);
        for flags in [&["-R"][..], &["-lR"], &["--recursive"]] {
            assert!(args(flags).unwrap().recursive);
        }
        assert_eq!(args(&["-lz"]).unwrap_err(), "unknown option -z in -lz");
        assert_eq!(args(&["-z"]).unwrap_err(), "unknown option: -z");
        assert_eq!(
            args(&["-l\u{1b}"]).unwrap_err(),
            "unknown option -\\u{1b} in -l\\u{1b}"
        );
    }
    #[test]
    fn final_text_format_wins_and_prefixes_do_not_change_it() {
        for (flags, expected) in [
            (vec!["-l1"], '1'),
            (vec!["-1l"], 'l'),
            (vec!["-lC"], 'C'),
            (vec!["-Cx"], 'x'),
            (vec!["-xm"], 'm'),
            (vec!["-m", "--long"], 'l'),
            (vec!["--header", "-1"], '1'),
            (vec!["--fields=size", "--columns"], 'C'),
            (vec!["-C", "-n"], 'l'),
            (vec!["-1", "-@"], 'l'),
            (vec!["-O", "-m"], 'm'),
            (vec!["-Cis"], 'C'),
            (vec!["-is1"], '1'),
        ] {
            let o = args(&flags).unwrap();
            assert_eq!(o.long, expected == 'l', "{flags:?}");
            assert_eq!(o.columns, matches!(expected, 'C' | 'x'), "{flags:?}");
            assert_eq!(o.across, expected == 'x', "{flags:?}");
            assert_eq!(o.one, expected == '1', "{flags:?}");
            assert_eq!(o.commas, expected == 'm', "{flags:?}");
        }
    }
    #[test]
    fn link_hidden_and_size_policies_follow_argument_order() {
        for (flags, expected) in [
            (vec![], Dereference::Default),
            (vec!["-H"], Dereference::CommandLine),
            (vec!["-HL"], Dereference::All),
            (vec!["-LP"], Dereference::Never),
            (
                vec!["--no-dereference", "--dereference-command-line"],
                Dereference::CommandLine,
            ),
            (vec!["-P", "--dereference"], Dereference::All),
        ] {
            assert_eq!(args(&flags).unwrap().dereference, expected);
        }
        assert!(args(&["-a"]).unwrap().dot_entries);
        assert!(!args(&["-aA"]).unwrap().dot_entries);
        assert!(args(&["-Aa"]).unwrap().dot_entries);
        let o = args(&["-ltrf"]).unwrap();
        assert!(o.all && o.dot_entries && o.long && !o.reverse);
        assert_eq!(o.sort, Sort::None);
        assert_eq!(args(&["-ft"]).unwrap().sort, Sort::Time);
        assert!(args(&["-hk"]).unwrap().kibibytes);
        assert!(!args(&["-hk"]).unwrap().human);
        assert!(!args(&["-k", "--bytes"]).unwrap().kibibytes);
        assert!(args(&["-kh"]).unwrap().human);
        assert!(!args(&["-kh"]).unwrap().kibibytes);
        assert!(args(&["-bq"]).unwrap().quote_controls);
        assert!(!args(&["-qb"]).unwrap().quote_controls);
        assert!(args(&["-Fp"]).unwrap().slash);
        assert!(!args(&["-Fp"]).unwrap().classify);
    }
    #[test]
    fn common_long_aliases_and_extended_metadata() {
        let o = args(&[
            "--human-readable",
            "--numeric-uid-gid",
            "--inode",
            "--size",
            "--no-group",
        ])
        .unwrap();
        assert!(o.long && o.human && o.numeric && o.inode && o.blocks && o.no_group);
        for flag in [
            "-@",
            "-e",
            "-O",
            "-Z",
            "--xattrs",
            "--acl",
            "--flags",
            "--context",
        ] {
            assert!(args(&[flag]).unwrap().long);
        }
        let o = args(&["-@eOZ"]).unwrap();
        assert!(o.xattrs && o.acl && o.flags && o.context);
        assert_eq!(args(&["--time=atime"]).unwrap().time, Time::Accessed);
        assert_eq!(args(&["--time=ctime"]).unwrap().time, Time::Changed);
        assert_eq!(args(&["-c", "--time=mtime"]).unwrap().time, Time::Modified);
    }
    #[test]
    fn output_width_accepts_long_and_short_values() {
        for flags in [
            vec!["--width=40"],
            vec!["--width", "40"],
            vec!["-w", "40"],
            vec!["-w40"],
            vec!["-Cw40"],
        ] {
            assert_eq!(args(&flags).unwrap().width, Some(40));
        }
        for flags in [
            vec!["--width"],
            vec!["-w"],
            vec!["--width=0"],
            vec!["-w65536"],
            vec!["-wxyz"],
        ] {
            assert!(args(&flags).is_err());
        }
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
            "--time=wat",
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
            "-C",
            "-x",
            "--columns",
            "--fields=size",
            "-l",
            "-n",
            "-1",
            "-m",
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
