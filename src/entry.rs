use crate::{
    cli::{Options, Sort},
    display::escape,
    filetype::{self, Classification},
};
use std::{
    ffi::{OsStr, OsString},
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

// Retain only listing/sorting fields, not the platform's full stat structure.
#[derive(Debug)]
pub struct Details {
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub len: u64,
    pub links: u64,
    pub modified: i64,
    pub modified_nsec: i64,
}
impl From<Metadata> for Details {
    fn from(meta: Metadata) -> Self {
        Self {
            mode: meta.mode(),
            uid: meta.uid(),
            gid: meta.gid(),
            len: meta.len(),
            links: meta.nlink(),
            modified: meta.mtime(),
            modified_nsec: meta.mtime_nsec(),
        }
    }
}

/// One listed name. Directory listings store only the name; its path is the
/// listing directory joined with it. File operands store their path as given.
#[derive(Debug)]
pub struct Entry {
    pub name: OsString,
    pub kind: Kind,
    // Filename classification, computed once for styling and preview choice.
    pub class: Classification,
    pub metadata: Option<Box<Details>>,
    // Styling retains only the executable bit, not a stat structure per entry.
    pub executable: bool,
}
impl Entry {
    pub fn new(name: impl Into<OsString>, kind: Kind) -> Self {
        let name = name.into();
        Self {
            class: filetype::classify(Path::new(&name)),
            name,
            kind,
            metadata: None,
            executable: false,
        }
    }
    pub fn candidate(&self) -> bool {
        matches!(self.kind, Kind::File | Kind::Link) && self.class.preview
    }
    pub fn artwork(&self) -> crate::artwork::Icon {
        use crate::{artwork::Icon, filetype::Category};
        match self.kind {
            Kind::Directory => Icon::Folder,
            Kind::Link => Icon::Link,
            Kind::Pipe | Kind::Socket | Kind::Device => Icon::Special,
            Kind::Unknown => Icon::Error,
            Kind::File => match self.class.category {
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
    /// The entry's filesystem path; `dir` is empty for file operands.
    pub fn path(&self, dir: &Path) -> PathBuf {
        join(dir, &self.name)
    }
}

pub fn join(dir: &Path, name: &OsStr) -> PathBuf {
    if dir.as_os_str().is_empty() {
        name.into()
    } else {
        dir.join(name)
    }
}

#[derive(Default)]
pub struct Listing {
    /// Directory containing the entries; empty for a file operand.
    pub dir: PathBuf,
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
    // Match everyday ls behavior: a directory-link operand lists its contents,
    // while -l/-d and links inside a directory retain the link itself.
    result.directory = meta.is_dir()
        || (meta.is_symlink()
            && !opts.long
            && !opts.directory
            && fs::metadata(path).is_ok_and(|target| target.is_dir()));
    if !result.directory || opts.directory {
        result.directory = false;
        let mut entry = Entry::new(path.as_os_str(), Kind::from_type(meta.file_type()));
        entry.executable = meta.is_file() && meta.mode() & 0o111 != 0;
        entry.metadata = Some(Box::new(meta.into()));
        result.entries.push(entry);
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
    result.dir = path.into();
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
        let kind = match item.file_type() {
            Ok(t) => Kind::from_type(t),
            Err(e) => {
                result
                    .errors
                    .push(format!("{}: {e}", escape(item.path().as_os_str())));
                Kind::Unknown
            }
        };
        result.entries.push(Entry::new(name, kind));
    }
    result
}

// Select the layout before reading per-entry metadata. Automatic grids and plain
// pipes do not pay for long details, and styling never causes a second stat.
pub fn prepare(listing: &mut Listing, opts: &Options, long: bool, need_mode: bool) {
    let need_metadata = long || opts.needs_metadata();
    // One reusable path buffer: no per-entry path allocation for stats.
    let mut path = listing.dir.clone();
    for entry in &mut listing.entries {
        if need_metadata || (need_mode && entry.kind == Kind::File) {
            path.push(&entry.name);
            match fs::symlink_metadata(&path) {
                Ok(meta) => {
                    entry.executable = entry.kind == Kind::File && meta.mode() & 0o111 != 0;
                    entry.metadata = need_metadata.then(|| Box::new(meta.into()));
                }
                Err(error) if need_metadata => listing
                    .errors
                    .push(format!("{}: {error}", escape(path.as_os_str()))),
                Err(_) => (),
            }
            path.pop();
        }
    }
    if opts.sort == Sort::None {
        if opts.reverse {
            listing.entries.reverse();
        }
        if opts.dirs_first {
            listing.entries.sort_by_key(|e| e.kind != Kind::Directory);
        }
        return;
    }
    listing.entries.sort_unstable_by(|a, b| {
        let primary = match opts.sort {
            Sort::Name | Sort::None => std::cmp::Ordering::Equal,
            Sort::Size => b
                .metadata
                .as_ref()
                .map(|m| m.len)
                .cmp(&a.metadata.as_ref().map(|m| m.len)),
            Sort::Time => b
                .metadata
                .as_ref()
                .map(|m| (m.modified, m.modified_nsec))
                .cmp(&a.metadata.as_ref().map(|m| (m.modified, m.modified_nsec))),
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
}

pub fn write_text(
    out: &mut impl Write,
    dir: &Path,
    entries: &[Entry],
    style: &crate::style::Style,
) -> io::Result<()> {
    for e in entries {
        style.write_name(out, dir, e)?;
        writeln!(out)?;
    }
    Ok(())
}
