use crate::{
    cli::{Dereference, Options, Sort, Time},
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
    pub inode: u64,
    /// Device number for character/block devices.
    pub rdev: u64,
    /// BSD file flags; zero on platforms without st_flags.
    pub flags: u32,
    // 512-byte units, as st_blocks reports on Linux and macOS.
    pub blocks: u64,
    /// The selected timestamp (`-c` / `-u`), shown and used by `-t`.
    pub modified: i64,
    pub modified_nsec: i64,
}
impl Details {
    pub fn new(meta: &Metadata, time: Time) -> Self {
        let (modified, modified_nsec) = match time {
            Time::Modified => (meta.mtime(), meta.mtime_nsec()),
            Time::Changed => (meta.ctime(), meta.ctime_nsec()),
            Time::Accessed => (meta.atime(), meta.atime_nsec()),
        };
        Self {
            mode: meta.mode(),
            uid: meta.uid(),
            gid: meta.gid(),
            len: meta.len(),
            links: meta.nlink(),
            inode: meta.ino(),
            rdev: meta.rdev(),
            flags: {
                #[cfg(target_os = "macos")]
                {
                    std::os::macos::fs::MetadataExt::st_flags(meta)
                }
                #[cfg(not(target_os = "macos"))]
                {
                    0
                }
            },
            blocks: meta.blocks(),
            modified,
            modified_nsec,
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
    // A symlink whose target cannot be resolved; checked when styling, long
    // details or grids need it, never for plain names.
    pub broken: bool,
}
impl Entry {
    pub fn new(name: impl Into<OsString>, kind: Kind) -> Self {
        let name = name.into();
        Self {
            class: filetype::classify(Path::new(&name)),
            ..Self::unclassified(name, kind)
        }
    }
    /// For output that never styles or previews (plain names in a pipe),
    /// skipping filename classification.
    fn unclassified(name: OsString, kind: Kind) -> Self {
        Self {
            name,
            kind,
            class: Classification {
                category: filetype::Category::File,
                preview: false,
            },
            metadata: None,
            executable: false,
            broken: false,
        }
    }
    /// A dangling link is not a failed image: it keeps link artwork and does
    /// not spend a preview attempt.
    pub fn candidate(&self) -> bool {
        matches!(self.kind, Kind::File | Kind::Link) && self.class.preview && !self.broken
    }
    pub fn artwork(&self) -> crate::artwork::Icon {
        use crate::{artwork::Icon, filetype::Category};
        match self.kind {
            Kind::Directory => Icon::Folder,
            Kind::Link if self.broken => Icon::BrokenLink,
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

/// Classify one command-line operand without opening any directory contents.
/// This allows file operands to be sorted and printed before directories while
/// retaining only one directory listing at a time.
pub fn operand(path: &Path, opts: &Options, classify: bool) -> Listing {
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
    let mut entry = if classify {
        Entry::new(path.as_os_str(), Kind::from_type(meta.file_type()))
    } else {
        Entry::unclassified(path.as_os_str().into(), Kind::from_type(meta.file_type()))
    };
    let follow = match opts.dereference {
        Dereference::All | Dereference::CommandLine => true,
        Dereference::Never => false,
        Dereference::Default => !opts.long && !opts.directory && !opts.classify,
    };
    let target = if meta.is_symlink() && (follow || classify) {
        match fs::metadata(path) {
            Ok(target) => Some(target),
            Err(error) => {
                entry.broken = true;
                if follow && opts.dereference != Dereference::Default {
                    result
                        .errors
                        .push(format!("{}: {error}", escape(path.as_os_str())));
                }
                None
            }
        }
    } else {
        None
    };
    // Default behavior follows directory-link operands only. -H/-L also use
    // target metadata for file-link operands, retaining the operand's name.
    let meta = target
        .as_ref()
        .filter(|m| follow && (opts.dereference != Dereference::Default || m.is_dir()))
        .unwrap_or(&meta);
    entry.kind = Kind::from_type(meta.file_type());
    entry.executable = meta.is_file() && meta.mode() & 0o111 != 0;
    entry.metadata = Some(Box::new(Details::new(meta, opts.time)));
    result.directory = meta.is_dir() && !opts.directory;
    result.entries.push(entry);
    result.valid = true;
    result
}

/// Read one directory. `-L` resolves links before layout selection so a linked
/// directory is both represented and traversed as a directory. Other policies
/// retain link identity inside listings. Plain names need no per-file stats.
pub fn read_directory(path: &Path, opts: &Options, classify: bool) -> Listing {
    let mut result = Listing {
        dir: path.into(),
        directory: true,
        ..Listing::default()
    };
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
    if opts.dot_entries {
        for name in [".", ".."] {
            result.entries.push(if classify {
                Entry::new(name, Kind::Directory)
            } else {
                Entry::unclassified(name.into(), Kind::Directory)
            });
        }
    }
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
        let mut entry = if classify {
            Entry::new(name, kind)
        } else {
            Entry::unclassified(name, kind)
        };
        if opts.dereference == Dereference::All && kind == Kind::Link {
            match fs::metadata(item.path()) {
                Ok(meta) => {
                    entry.kind = Kind::from_type(meta.file_type());
                    entry.executable = meta.is_file() && meta.mode() & 0o111 != 0;
                    // Preserve this stat for long output or time/size ordering.
                    entry.metadata = Some(Box::new(Details::new(&meta, opts.time)));
                }
                Err(error) => {
                    entry.broken = true;
                    result
                        .errors
                        .push(format!("{}: {error}", escape(item.path().as_os_str())));
                }
            }
        }
        result.entries.push(entry);
    }
    result
}

// Select the layout before reading per-entry metadata. Automatic grids and plain
// pipes do not pay for long details, and styling never causes a second stat.
// `resolve_links` adds one target check per symlink so dangling links stand out.
pub fn prepare(
    listing: &mut Listing,
    opts: &Options,
    long: bool,
    need_mode: bool,
    resolve_links: bool,
) {
    let need_metadata = long || opts.needs_metadata();
    // One reusable path buffer: no per-entry path allocation for stats.
    let mut path = listing.dir.clone();
    for entry in &mut listing.entries {
        let check_link =
            resolve_links && entry.kind == Kind::Link && !entry.broken && entry.metadata.is_none();
        let read_metadata =
            entry.metadata.is_none() && (need_metadata || (need_mode && entry.kind == Kind::File));
        if !check_link && !read_metadata {
            continue;
        }
        // push/pop is not reversible for "." or multi-component operands.
        // Restore the base while retaining the buffer allocation instead.
        path.clone_from(&listing.dir);
        path.push(&entry.name);
        if check_link {
            entry.broken = fs::metadata(&path).is_err();
        }
        if read_metadata {
            // Resolved -L links already have target metadata. All remaining
            // entries intentionally use lstat, including broken links.
            match fs::symlink_metadata(&path) {
                Ok(meta) => {
                    entry.executable = entry.kind == Kind::File && meta.mode() & 0o111 != 0;
                    entry.metadata =
                        need_metadata.then(|| Box::new(Details::new(&meta, opts.time)));
                }
                Err(error) if need_metadata => listing
                    .errors
                    .push(format!("{}: {error}", escape(path.as_os_str()))),
                Err(_) => (),
            }
        }
    }
    sort(&mut listing.entries, opts);
}

/// The same ordering applies to directory entries and explicit operands.
pub fn sort(entries: &mut [Entry], opts: &Options) {
    if opts.sort == Sort::None {
        if opts.reverse {
            entries.reverse();
        }
        if opts.dirs_first {
            entries.sort_by_key(|e| e.kind != Kind::Directory);
        }
        return;
    }
    entries.sort_unstable_by(|a, b| {
        let primary = match opts.sort {
            Sort::Name | Sort::None => std::cmp::Ordering::Equal,
            // Directories have no displayed size; they follow sized entries.
            Sort::Size => {
                let size = |e: &Entry| {
                    e.metadata
                        .as_ref()
                        .filter(|_| e.kind != Kind::Directory)
                        .map(|m| m.len)
                };
                size(b).cmp(&size(a))
            }
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
        style.write_entry(out, dir, e)?;
        writeln!(out)?;
    }
    Ok(())
}
