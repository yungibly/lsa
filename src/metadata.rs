use crate::{
    cli::Options,
    display::escape,
    entry::{Entry, Kind},
};
use std::{
    fs,
    io::{self, Write},
    os::unix::fs::MetadataExt,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Mode,
    Links,
    Uid,
    Gid,
    Size,
    Modified,
}

const DEFAULT_FIELDS: &[Field] = &[
    Field::Mode,
    Field::Links,
    Field::Uid,
    Field::Gid,
    Field::Size,
    Field::Modified,
];

impl Field {
    fn value(self, entry: &Entry, human: bool) -> String {
        let Some(m) = &entry.metadata else {
            return "?".into();
        };
        match self {
            Self::Mode => permissions(m.mode()),
            Self::Links => m.nlink().to_string(),
            Self::Uid => m.uid().to_string(),
            Self::Gid => m.gid().to_string(),
            Self::Size => size(m.len(), human),
            Self::Modified => timestamp(m.mtime()),
        }
    }
}

pub fn parse_fields(list: &str) -> Result<Vec<Field>, String> {
    let mut fields = Vec::new();
    for name in list.split(',') {
        let field = match name {
            "mode" => Field::Mode, "links" => Field::Links, "uid" => Field::Uid,
            "gid" => Field::Gid, "size" => Field::Size, "modified" => Field::Modified,
            _ => return Err("fields must be a comma-separated list of mode, links, uid, gid, size, or modified; names are always shown".into()),
        };
        if fields.contains(&field) {
            return Err(format!("duplicate metadata field: {name}"));
        }
        fields.push(field);
    }
    Ok(fields)
}

pub fn write(out: &mut impl Write, entries: &[Entry], opts: &Options) -> io::Result<()> {
    let fields = if opts.fields.is_empty() {
        DEFAULT_FIELDS
    } else {
        &opts.fields
    };
    // Field values are ASCII. Two passes retain only six widths, not a second
    // directory-sized collection of formatted metadata or filenames.
    let mut widths = [0; 6];
    for entry in entries {
        for (i, field) in fields.iter().enumerate() {
            widths[i] = widths[i].max(field.value(entry, opts.human).len());
        }
    }
    for entry in entries {
        for (i, field) in fields.iter().enumerate() {
            let value = field.value(entry, opts.human);
            let width = widths[i];
            if matches!(field, Field::Mode | Field::Modified) {
                write!(out, "{value:<width$} ")?;
            } else {
                write!(out, "{value:>width$} ")?;
            }
        }
        write!(out, "{}", entry.label())?;
        if entry.kind == Kind::Link {
            match fs::read_link(&entry.path) {
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
    fn missing_metadata_keeps_the_entry_and_fields() {
        let entry = Entry {
            path: "lost".into(),
            name: "lost".into(),
            kind: Kind::File,
            metadata: None,
        };
        let opts = Options {
            long: true,
            fields: vec![Field::Size, Field::Mode],
            ..Options::default()
        };
        let mut out = Vec::new();
        write(&mut out, &[entry], &opts).unwrap();
        assert_eq!(out, b"? ? lost\n");
    }
    #[test]
    fn unix_modes_and_sizes() {
        assert_eq!(permissions(libc::S_IFDIR as u32 | 0o1777), "drwxrwxrwt");
        assert_eq!(permissions(libc::S_IFREG as u32 | 0o4644), "-rwSr--r--");
        assert_eq!(size(1536, true), "1.5K");
        assert_eq!(size(1536, false), "1536");
    }
}
