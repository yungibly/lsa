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
        let within_group =
            primary.then_with(|| a.name.as_encoded_bytes().cmp(b.name.as_encoded_bytes()));
        let within_group = if opts.reverse {
            within_group.reverse()
        } else {
            within_group
        };
        let group = if opts.dirs_first {
            (b.kind == Kind::Directory).cmp(&(a.kind == Kind::Directory))
        } else {
            std::cmp::Ordering::Equal
        };
        group.then(within_group)
    });
    result
}

pub fn write_text(out: &mut impl Write, entries: &[Entry], opts: &Options) -> io::Result<()> {
    if opts.long {
        return crate::metadata::write(out, entries, opts);
    }
    for e in entries {
        writeln!(out, "{}", e.label())?;
    }
    Ok(())
}
