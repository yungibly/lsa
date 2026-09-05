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
    pub grid: bool,
    pub one: bool,
    pub no_images: bool,
    pub protocol: Protocol,
    pub preview_limit: usize,
    pub diagnose: bool,
    pub help: bool,
    pub version: bool,
}

pub const HELP: &str = "lsa — directory listings with inline Kitty thumbnails

Usage: lsa [OPTIONS] [PATH ...]

  -a, -A                 Include hidden entries (neither adds . or ..)
  -l                     Long text listing: mode, links, uid, gid, size, local time
  -h                     Human-readable sizes with -l
  -1                     One name per line
  -t / -S                Sort newest / largest first; ties by raw filename bytes
  -r                     Reverse the selected order
  --grid                 Mixed-entry thumbnail grid on a graphics terminal
  --no-images            Compact text only; never open image contents
  --protocol=auto|kitty|none
                         Auto recognizes direct Ghostty/Kitty sessions
  --preview-limit=N      Attempt at most N previews per invocation (0..256; default 64)
  --diagnose             Explain layout for each path without decoding images
  --help / --version     Show help / version
  --                     End options, including for paths beginning with -

TTY output defaults to row-wise text columns. Image-heavy listings that fit
one screen and the preview budget automatically use a grid. Non-TTY output
is always one entry per line. -l, -1, --no-images, and protocol=none
override --grid. Unknown terminals and multiplexers default to text.
Preview errors retain entries and do not fail the listing. Exit: 0 success,
1 listing/output errors, 2 invalid options. Closed pipes exit successfully.
";

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Options, String> {
    let mut opts = Options {
        preview_limit: 64,
        ..Options::default()
    };
    let mut operands = false;
    for arg in args {
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
            "--no-images" => opts.no_images = true,
            "--diagnose" => opts.diagnose = true,
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
    if opts.paths.is_empty() {
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
    fn flags_and_operands() {
        let o = args(&["-lahSr", "dir", "--", "-image.png"]).unwrap();
        assert!(o.all && o.long && o.human && o.reverse);
        assert_eq!(o.sort, Sort::Size);
        assert_eq!(o.paths, [PathBuf::from("dir"), PathBuf::from("-image.png")]);
    }
    #[test]
    fn rejects_unknown_and_unbounded_options() {
        for a in [
            "--browse",
            "-z",
            "--protocol=sixel",
            "--preview-limit=-1",
            "--preview-limit=257",
        ] {
            assert!(args(&[a]).is_err(), "{a}");
        }
        assert_eq!(args(&["--preview-limit=0"]).unwrap().preview_limit, 0);
    }
}
