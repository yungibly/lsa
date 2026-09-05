use crate::{
    cli::{Options, Sort},
    display::escape,
};
use std::{
    ffi::OsString,
    fs::{self, FileType, Metadata},
    io::{self, Write},
    os::unix::fs::{FileTypeExt, MetadataExt},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    File,
    Directory,
    Link,
    Pipe,
    Socket,
    Device,
    Unknown,
}
impl Kind {
    fn from_type(t: FileType) -> Self {
        if t.is_symlink() {
            Self::Link
        } else if t.is_dir() {
            Self::Directory
        } else if t.is_file() {
            Self::File
        } else if t.is_fifo() {
            Self::Pipe
        } else if t.is_socket() {
            Self::Socket
        } else if t.is_block_device() || t.is_char_device() {
            Self::Device
        } else {
            Self::Unknown
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::File => "[file]",
            Self::Directory => "[dir]",
            Self::Link => "[link]",
            Self::Pipe => "[pipe]",
            Self::Socket => "[socket]",
            Self::Device => "[device]",
            Self::Unknown => "[?]",
        }
    }
    fn marker(self) -> &'static str {
        match self {
            Self::Directory => "/",
            Self::Link => "@",
            Self::Pipe => "|",
            Self::Socket => "=",
            _ => "",
        }
    }
}

#[derive(Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub name: OsString,
    pub kind: Kind,
    // Keep the large platform stat structure off the ordinary name-only path.
    pub metadata: Option<Box<Metadata>>,
}
impl Entry {
    pub fn label(&self) -> String {
        format!("{}{}", escape(&self.name), self.kind.marker())
    }
    pub fn candidate(&self) -> bool {
        matches!(self.kind, Kind::File | Kind::Link)
            && self
                .path
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| {
                    ["jpg", "jpeg", "png", "gif", "webp", "bmp"]
                        .iter()
                        .any(|ext| x.eq_ignore_ascii_case(ext))
                })
    }
}

#[derive(Default)]
pub struct Listing {
    pub entries: Vec<Entry>,
    pub errors: Vec<String>,
    pub directory: bool,
    pub valid: bool,
}

pub fn list(path: &Path, opts: &Options) -> Listing {
    let mut result = Listing::default();
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) => {
            result
                .errors
                .push(format!("{}: {e}", escape(path.as_os_str())));
            return result;
        }
    };
    result.directory = meta.is_dir(); // Directory symlink operands remain links.
    if !result.directory {
        result.entries.push(Entry {
            path: path.into(),
            name: path.as_os_str().into(),
            kind: Kind::from_type(meta.file_type()),
            metadata: Some(Box::new(meta)),
        });
        result.valid = true;
        return result;
    }
    let iter = match fs::read_dir(path) {
        Ok(iter) => iter,
        Err(e) => {
            result
                .errors
                .push(format!("{}: {e}", escape(path.as_os_str())));
            return result;
        }
    };
    result.valid = true;
    for item in iter {
        let item = match item {
            Ok(item) => item,
            Err(e) => {
                result
                    .errors
                    .push(format!("{}: {e}", escape(path.as_os_str())));
                continue;
            }
        };
        let name = item.file_name();
        if !opts.all && name.as_encoded_bytes().starts_with(b".") {
            continue;
        }
        let path = item.path();
        let kind = match item.file_type() {
            Ok(t) => Kind::from_type(t),
            Err(e) => {
                result
                    .errors
                    .push(format!("{}: {e}", escape(path.as_os_str())));
                Kind::Unknown
            }
        };
        let metadata = if opts.long || opts.sort != Sort::Name {
            match fs::symlink_metadata(&path) {
                Ok(m) => Some(Box::new(m)),
                Err(e) => {
                    result
                        .errors
                        .push(format!("{}: {e}", escape(path.as_os_str())));
                    None
                }
            }
        } else {
            None
        };
        result.entries.push(Entry {
            path,
            name,
            kind,
            metadata,
        });
    }
    result.entries.sort_unstable_by(|a, b| {
        let primary = match opts.sort {
            Sort::Name => std::cmp::Ordering::Equal,
            Sort::Size => b
                .metadata
                .as_ref()
                .map(|m| m.len())
                .cmp(&a.metadata.as_ref().map(|m| m.len())),
            Sort::Time => b
                .metadata
                .as_ref()
                .map(|m| (m.mtime(), m.mtime_nsec()))
                .cmp(&a.metadata.as_ref().map(|m| (m.mtime(), m.mtime_nsec()))),
        };
        primary.then_with(|| a.name.as_encoded_bytes().cmp(b.name.as_encoded_bytes()))
    });
    if opts.reverse {
        result.entries.reverse();
    }
    result
}

pub fn write_text(out: &mut impl Write, entries: &[Entry], opts: &Options) -> io::Result<()> {
    for e in entries {
        if opts.long {
            if let Some(m) = &e.metadata {
                write!(
                    out,
                    "{} {:>3} {:>5} {:>5} {:>10} {} ",
                    permissions(m.mode()),
                    m.nlink(),
                    m.uid(),
                    m.gid(),
                    size(m.len(), opts.human),
                    timestamp(m.mtime())
                )?;
            } else {
                write!(
                    out,
                    "??????????   ?     ?     ?          ? ????-??-?? ??:?? "
                )?;
            }
        }
        write!(out, "{}", e.label())?;
        if opts.long && e.kind == Kind::Link {
            match fs::read_link(&e.path) {
                Ok(target) => write!(out, " -> {}", escape(target.as_os_str()))?,
                Err(_) => write!(out, " -> ?")?,
            }
        }
        writeln!(out)?;
    }
    Ok(())
}

fn size(n: u64, human: bool) -> String {
    if !human || n < 1024 {
        return n.to_string();
    }
    let mut v = n as f64;
    let mut unit = "";
    for u in ["K", "M", "G", "T", "P", "E"] {
        v /= 1024.0;
        unit = u;
        if v < 1024.0 {
            break;
        }
    }
    format!("{v:.1}{unit}")
}

fn timestamp(seconds: i64) -> String {
    let time = seconds as libc::time_t;
    let mut tm = std::mem::MaybeUninit::<libc::tm>::uninit();
    // SAFETY: both pointers are valid, and tm is read only after success.
    if unsafe { libc::localtime_r(&time, tm.as_mut_ptr()) }.is_null() {
        return "????-??-?? ??:??".into();
    }
    let tm = unsafe { tm.assume_init() };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min
    )
}

fn permissions(mode: u32) -> String {
    let mut out = String::from(match mode & libc::S_IFMT as u32 {
        x if x == libc::S_IFDIR as u32 => "d",
        x if x == libc::S_IFLNK as u32 => "l",
        x if x == libc::S_IFIFO as u32 => "p",
        x if x == libc::S_IFSOCK as u32 => "s",
        x if x == libc::S_IFCHR as u32 => "c",
        x if x == libc::S_IFBLK as u32 => "b",
        _ => "-",
    });
    for shift in [6, 3, 0] {
        out.push(if mode & (4 << shift) != 0 { 'r' } else { '-' });
        out.push(if mode & (2 << shift) != 0 { 'w' } else { '-' });
        let exec = mode & (1 << shift) != 0;
        let special =
            mode & (if shift == 6 {
                0o4000
            } else if shift == 3 {
                0o2000
            } else {
                0o1000
            }) != 0;
        out.push(match (exec, special, shift) {
            (true, true, 0) => 't',
            (false, true, 0) => 'T',
            (true, true, _) => 's',
            (false, true, _) => 'S',
            (true, false, _) => 'x',
            _ => '-',
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unix_modes_and_sizes() {
        assert_eq!(permissions(libc::S_IFDIR as u32 | 0o1777), "drwxrwxrwt");
        assert_eq!(permissions(libc::S_IFREG as u32 | 0o4644), "-rwSr--r--");
        assert_eq!(size(1536, true), "1.5K");
        assert_eq!(size(1536, false), "1536");
    }
}
