use std::{ffi::OsString, path::PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Protocol {
    #[default]
    Auto,
    Kitty,
    None,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Sort {
    #[default]
    Name,
    Size,
    Time,
}

#[derive(Debug, Default)]
pub struct Options {
    pub paths: Vec<PathBuf>,
    pub all: bool,
    pub long: bool,
    pub human: bool,
    pub reverse: bool,
    pub sort: Sort,
    pub dirs_first: bool,
    pub fields: Vec<crate::metadata::Field>,
    pub grid: bool,
    pub page: bool,
    pub one: bool,
    pub no_images: bool,
    pub protocol: Protocol,
    pub preview_limit: usize,
    pub diagnose: bool,
    pub cache_dir: Option<PathBuf>,
    pub no_cache: bool,
    pub clear_cache: bool,
    pub cache_stats: bool,
    pub help: bool,
    pub version: bool,
}

pub const HELP: &str = "lsa — directory listings with inline Kitty thumbnails

Usage: lsa [OPTIONS] [PATH ...]

  -a, -A                 Include hidden entries (neither adds . or ..)
  -l, --long             Long text listing: mode, links, uid, gid, size, local time
  -h                     Human-readable sizes with -l
  --fields=LIST          Choose long columns: mode,links,uid,gid,size,modified
                         Comma-separated; names always follow the selected fields
  -1                     One name per line
  -t / -S                Sort newest / largest first; ties by raw filename bytes
  -r                     Reverse the selected order
  --dirs-first           Directories first; -r reverses within each group
  --grid                 Mixed-entry thumbnail grid on a graphics terminal
  --page                 Page one directory listing; requires stdin/stdout TTY
  --no-images            Compact text only; never open image contents
  --protocol=auto|kitty|none
                         Auto recognizes direct Ghostty/Kitty sessions
  --preview-limit=N      Preview attempts per listing / pager viewport (0..256; default 64)
  --cache-dir=PATH       Opt-in thumbnail cache under PATH (also accepts a space)
  --no-cache             Disable cache reads/writes regardless of option order
  --clear-cache          Clear this cache and exit; requires --cache-dir, no paths
  --cache-stats          Report hits, misses, writes, and cache I/O errors to stderr
  --diagnose             Explain layout for each path without decoding images
  --help / --version     Show help / version
  --                     End options, including for paths beginning with -

TTY output defaults to row-wise text columns. Image-heavy listings that fit
one screen and the preview budget automatically use a grid. Non-TTY output
is always one entry per line. -l, -1, --no-images, and protocol=none
override --grid. Unknown terminals and multiplexers default to text.
Preview errors retain entries and do not fail the listing. Exit: 0 success,
1 listing/output/cache-clear errors, 2 invalid options. Closed pipes exit successfully.
Caching is off by default. Text, diagnostics, and exhausted preview budgets never
open the cache. Cache I/O failures fall back to decoding. Storage uses 64 replaceable
slots plus one staging file (under 20 MiB of file contents); collisions evict a slot.

Pager: arrows or h/j/k/l move spatially; Space/PgDn pages forward, b/PgUp back;
Home/End or g/G jump; Enter shows the full name/path (Esc/Backspace closes);
q or Ctrl-D quits; Ctrl-C exits with 130; Ctrl-Z suspends with terminal restored.
The listing is read once; directories and links are entries, with no navigation.
Sorting and hidden flags apply. Kitty/Ghostty sessions preview the mixed viewport;
--no-images/--protocol=none keep text. One decoder, up to 32 visible images and two
prefetched thumbnails. Attempt and 8 MiB image-command caps renew on viewport change;
selection-only redraws do not renew them. Inline caps remain per invocation.
Pager images are individually deleted on scrolling/resize/exit. Cache is opt-in.
--page cannot combine with -l, --fields, -1, --grid, --diagnose, --clear-cache,
or multiple paths. Redirected --page fails without reading stdin.
";

impl Options {
    pub fn cache_path(&self) -> Option<&std::path::Path> {
        self.cache_dir.as_deref().filter(|_| !self.no_cache)
    }
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Options, String> {
    let mut opts = Options {
        preview_limit: 64,
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
            "--page" => opts.page = true,
            "--browse" => return Err("--browse was removed; use --page to page one listing".into()),
            "--long" => opts.long = true,
            "--dirs-first" => opts.dirs_first = true,
            "--no-images" => opts.no_images = true,
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
            _ if s.starts_with("--protocol=") => {
                opts.protocol = match &s[11..] {
                    "auto" => Protocol::Auto,
                    "kitty" => Protocol::Kitty,
                    "none" => Protocol::None,
                    _ => return Err("protocol must be auto, kitty, or none".into()),
                };
            }
            _ if s.starts_with("--preview-limit=") => {
                opts.preview_limit = s[16..]
                    .parse()
                    .ok()
                    .filter(|n| *n <= 256)
                    .ok_or("preview limit must be an integer from 0 to 256")?;
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
                        '1' => opts.one = true,
                        'r' => opts.reverse = true,
                        't' => opts.sort = Sort::Time,
                        'S' => opts.sort = Sort::Size,
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
    if opts.page
        && (opts.paths.len() != 1
            || opts.long
            || opts.one
            || opts.grid
            || opts.diagnose
            || opts.clear_cache)
    {
        return Err("--page requires one directory and cannot combine with -l, --fields, -1, --grid, --diagnose, or --clear-cache".into());
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
    fn flags_and_operands() {
        let o = args(&["-lahSr", "dir", "--", "-image.png"]).unwrap();
        assert!(o.all && o.long && o.human && o.reverse);
        assert_eq!(o.sort, Sort::Size);
        assert_eq!(o.paths, [PathBuf::from("dir"), PathBuf::from("-image.png")]);
    }
    #[test]
    fn rejects_unknown_and_unbounded_options() {
        for a in [
            "--unknown",
            "-z",
            "--protocol=sixel",
            "--preview-limit=-1",
            "--preview-limit=257",
        ] {
            assert!(args(&[a]).is_err(), "{a}");
        }
        assert_eq!(args(&["--preview-limit=0"]).unwrap().preview_limit, 0);
    }

    #[test]
    fn pager_options() {
        assert!(args(&["--browse"]).unwrap_err().contains("use --page"));
        let o = args(&["--page", "-aSr", "--dirs-first"]).unwrap();
        assert!(o.page && o.all && o.reverse && o.dirs_first);
        assert_eq!(o.sort, Sort::Size);
        assert_eq!(o.paths, [PathBuf::from(".")]);
        for flags in [
            vec!["--page", "a", "b"],
            vec!["--page", "-l"],
            vec!["--page", "--fields=size"],
            vec!["--page", "-1"],
            vec!["--page", "--grid"],
            vec!["--page", "--diagnose"],
            vec!["--page", "--clear-cache", "--cache-dir=cache"],
        ] {
            assert!(args(&flags).is_err(), "{flags:?}");
        }
    }
    #[test]
    fn metadata_options() {
        let o = args(&["--fields=size,modified", "--dirs-first", "-hr"]).unwrap();
        assert!(o.long && o.dirs_first && o.human && o.reverse);
        assert_eq!(
            o.fields,
            [
                crate::metadata::Field::Size,
                crate::metadata::Field::Modified
            ]
        );
        assert!(args(&["--long"]).unwrap().long);
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
