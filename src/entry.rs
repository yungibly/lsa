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
}

#[derive(Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub name: OsString,
    pub kind: Kind,
    // Styling retains only the executable bit, not a stat structure per entry.
    pub metadata: Option<Box<Metadata>>,
    pub executable: bool,
}
impl Entry {
    pub fn candidate(&self) -> bool {
        matches!(self.kind, Kind::File | Kind::Link)
            && crate::filetype::classify(&self.path).preview
    }
    pub fn artwork(&self) -> crate::artwork::Icon {
        use crate::{
            artwork::Icon,
            filetype::{self, Category},
        };
        match self.kind {
            Kind::Directory => Icon::Folder,
            Kind::Link => Icon::Link,
            Kind::Pipe | Kind::Socket | Kind::Device => Icon::Special,
            Kind::Unknown => Icon::Error,
            Kind::File => match filetype::classify(&self.path).category {
                Category::Image => Icon::Image,
                Category::Video => Icon::Video,
                Category::Audio => Icon::Audio,
                Category::Archive => Icon::Archive,
                Category::Code => Icon::Code,
                Category::Config => Icon::Config,
                _ => Icon::File,
            },
        }
    }
}

#[derive(Default)]
pub struct Listing {
    pub entries: Vec<Entry>,
    pub errors: Vec<String>,
    pub directory: bool,
    pub valid: bool,
}

pub fn list(path: &Path, opts: &Options, need_mode: bool) -> Listing {
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
    // Match everyday ls behavior: a directory-link operand lists its contents,
    // while -l/-d and links inside a directory retain the link itself.
    result.directory = meta.is_dir()
        || (meta.is_symlink()
            && !opts.long
            && !opts.directory
            && fs::metadata(path).is_ok_and(|target| target.is_dir()));
    if !result.directory || opts.directory {
        result.directory = false;
        result.entries.push(Entry {
            path: path.into(),
            name: path.as_os_str().into(),
            kind: Kind::from_type(meta.file_type()),
            executable: meta.is_file() && meta.mode() & 0o111 != 0,
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
        let mut executable = false;
        let metadata = if opts.needs_metadata() || (need_mode && kind == Kind::File) {
            match fs::symlink_metadata(&path) {
                Ok(m) => {
                    executable = kind == Kind::File && m.mode() & 0o111 != 0;
                    opts.needs_metadata().then(|| Box::new(m))
                }
                Err(e) => {
                    if opts.needs_metadata() {
                        result
                            .errors
                            .push(format!("{}: {e}", escape(path.as_os_str())));
                    }
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
            executable,
        });
    }
    if opts.sort == Sort::None {
        if opts.reverse {
            result.entries.reverse();
        }
        if opts.dirs_first {
            result.entries.sort_by_key(|e| e.kind != Kind::Directory);
        }
        return result;
    }
    result.entries.sort_unstable_by(|a, b| {
        let primary = match opts.sort {
            Sort::Name | Sort::None => std::cmp::Ordering::Equal,
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
        let within_group = primary
            .then_with(|| crate::sort::name(a.name.as_encoded_bytes(), b.name.as_encoded_bytes()));
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

pub fn write_text(
    out: &mut impl Write,
    entries: &[Entry],
    style: &crate::style::Style,
) -> io::Result<()> {
    for e in entries {
        style.write_name(out, e)?;
        writeln!(out)?;
    }
    Ok(())
}
