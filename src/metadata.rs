use crate::{
    artwork,
    cache::Cache,
    cli::Options,
    display::escape,
    entry::{Entry, Kind},
    kitty,
    preview::{self, Budget},
    style::Style,
    terminal::Terminal,
};
use std::{
    fs,
    io::{self, Write},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Mode,
    Links,
    User,
    Group,
    Uid,
    Gid,
    Size,
    Modified,
}

const DEFAULT_FIELDS: &[Field] = &[Field::Mode, Field::Size, Field::User, Field::Modified];
const NUMERIC_FIELDS: &[Field] = &[
    Field::Mode,
    Field::Links,
    Field::Uid,
    Field::Gid,
    Field::Size,
    Field::Modified,
];

struct FormatCache {
    users: std::collections::HashMap<u32, String>,
    groups: std::collections::HashMap<u32, String>,
    // Directory entries often share timestamps. Retain 64 exact-second values,
    // avoiding repeated localtime_r work in both width and output passes.
    times: [Option<(i64, Option<Timestamp>)>; 64],
}
impl Default for FormatCache {
    fn default() -> Self {
        Self {
            users: Default::default(),
            groups: Default::default(),
            times: std::array::from_fn(|_| None),
        }
    }
}
impl FormatCache {
    fn timestamp(&mut self, seconds: i64) -> Option<Timestamp> {
        let slot = &mut self.times[seconds.rem_euclid(64) as usize];
        if let Some((key, value)) = slot
            && *key == seconds
        {
            return *value;
        }
        let value = timestamp(seconds);
        *slot = Some((seconds, value));
        value
    }
    fn name(&mut self, id: u32, group: bool) -> String {
        let names = if group {
            &mut self.groups
        } else {
            &mut self.users
        };
        if let Some(name) = names.get(&id) {
            return name.clone();
        }
        if names.len() >= 64 {
            return id.to_string();
        }
        // One bounded buffer, including negative lookups cached per invocation.
        let mut buffer = vec![0u8; 16 * 1024];
        let name = unsafe {
            // Reentrant libc APIs write only into the supplied structs/buffer.
            // Their returned name pointer is consumed before the buffer is freed.
            if group {
                let mut record = std::mem::MaybeUninit::<libc::group>::uninit();
                let mut result = std::ptr::null_mut();
                if libc::getgrgid_r(
                    id,
                    record.as_mut_ptr(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    &mut result,
                ) == 0
                    && !result.is_null()
                {
                    Some(
                        std::ffi::CStr::from_ptr((*result).gr_name)
                            .to_bytes()
                            .to_vec(),
                    )
                } else {
                    None
                }
            } else {
                let mut record = std::mem::MaybeUninit::<libc::passwd>::uninit();
                let mut result = std::ptr::null_mut();
                if libc::getpwuid_r(
                    id,
                    record.as_mut_ptr(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    &mut result,
                ) == 0
                    && !result.is_null()
                {
                    Some(
                        std::ffi::CStr::from_ptr((*result).pw_name)
                            .to_bytes()
                            .to_vec(),
                    )
                } else {
                    None
                }
            }
        };
        use std::os::unix::ffi::OsStrExt;
        let name = name
            .map(|bytes| escape(std::ffi::OsStr::from_bytes(&bytes)))
            .unwrap_or_else(|| id.to_string());
        names.insert(id, name.clone());
        name
    }
}

impl Field {
    fn value(
        self,
        entry: &Entry,
        opts: &Options,
        owners: &mut FormatCache,
        time: Option<Timestamp>,
    ) -> String {
        let Some(m) = &entry.metadata else {
            return "?".into();
        };
        match self {
            Self::Mode => permissions(m.mode),
            Self::Links => m.links.to_string(),
            Self::User => owners.name(m.uid, false),
            Self::Group => owners.name(m.gid, true),
            Self::Uid => m.uid.to_string(),
            Self::Gid => m.gid.to_string(),
            Self::Size => size(m.len, opts.human),
            Self::Modified => format_time(time, opts.twelve_hour),
        }
    }
}

pub fn parse_fields(list: &str) -> Result<Vec<Field>, String> {
    let mut fields = Vec::new();
    for name in list.split(',') {
        let field = match name {
            "mode" => Field::Mode, "links" => Field::Links, "uid" => Field::Uid,
            "user" => Field::User, "group" => Field::Group,
            "gid" => Field::Gid, "size" => Field::Size, "modified" => Field::Modified,
            _ => return Err("fields must be a comma-separated list of mode, links, user, group, uid, gid, size, or modified; names are always shown".into()),
        };
        if fields.contains(&field) {
            return Err(format!("duplicate metadata field: {name}"));
        }
        fields.push(field);
    }
    Ok(fields)
}

pub struct Previews<'a, 'cache> {
    pub term: &'a Terminal,
    pub budget: &'a mut Budget,
    pub cache: &'a mut Cache<'cache>,
}

pub fn write(
    out: &mut impl Write,
    entries: &[Entry],
    opts: &Options,
    style: &Style,
    mut previews: Option<Previews<'_, '_>>,
) -> io::Result<()> {
    use unicode_width::UnicodeWidthStr;
    let fields = if !opts.fields.is_empty() {
        &opts.fields
    } else if opts.numeric {
        NUMERIC_FIELDS
    } else {
        DEFAULT_FIELDS
    };
    let mut owners = FormatCache::default();
    let mut widths = [0; 8];
    let heading = |field: Field| match field {
        Field::Mode => "Permissions",
        Field::Links => "Links",
        Field::User => "Owner",
        Field::Group => "Group",
        Field::Uid => "UID",
        Field::Gid => "GID",
        Field::Size => "Size",
        Field::Modified => "Modified",
    };
    // Convert each timestamp at most once across width and output passes. Keep
    // compact civil components (24 bytes including Option), not formatted strings.
    // The bounded exact-second cache still avoids conversions for repeated times.
    let times: Vec<_> = if fields.contains(&Field::Modified) {
        entries
            .iter()
            .map(|entry| {
                entry
                    .metadata
                    .as_ref()
                    .and_then(|m| owners.timestamp(m.modified))
            })
            .collect()
    } else {
        Vec::new()
    };
    for (row, entry) in entries.iter().enumerate() {
        for (i, field) in fields.iter().enumerate() {
            let width = if *field == Field::Modified && entry.metadata.is_some() {
                time_width(times[row], opts.twelve_hour)
            } else {
                field.value(entry, opts, &mut owners, None).width()
            };
            widths[i] = widths[i].max(width);
        }
    }
    if opts.header {
        for (i, field) in fields.iter().enumerate() {
            widths[i] = widths[i].max(heading(*field).width());
            let value = format!("{:<width$}", heading(*field), width = widths[i]);
            style.paint(out, "1;4", &value)?;
            write!(out, " ")?;
        }
        style.paint(out, "1;4", "Name")?;
        writeln!(out)?;
    }
    let metadata_width = widths[..fields.len()].iter().sum::<usize>() + fields.len();
    // One-row miniatures add no height. Each occupies the name's icon gutter,
    // and narrow terminals retain the ordinary complete text listing.
    let mini = previews
        .as_ref()
        .filter(|p| {
            p.term.kitty
                && p.term.rows >= 3
                && p.term.cols >= metadata_width + 4 + 12
                && p.budget.attempts_left > 0
                && entries.iter().any(Entry::candidate)
        })
        .map(|p| {
            let w = f64::from(p.term.cell_width) * 3.0;
            let h = f64::from(p.term.cell_height);
            let scale = (96.0 / w).min(64.0 / h).min(1.0);
            (
                (w * scale).round().max(1.0) as u32,
                (h * scale).round().max(1.0) as u32,
            )
        });
    for (row, entry) in entries.iter().enumerate() {
        let image = if let Some((w, h)) = mini
            && entry.candidate()
        {
            let p = previews.as_mut().unwrap();
            let bytes = kitty::byte_len(w, h, 3, 1);
            if p.budget.begin(bytes) {
                Some(
                    preview::load(&entry.path, w, h, p.cache).unwrap_or_else(|_| {
                        artwork::render(artwork::Icon::Error, w, h, style.color)
                    }),
                )
            } else {
                None
            }
        } else {
            None
        };
        if image.is_some() {
            // Scroll before placement, then write this entry into the reserved
            // line. Do not print spaces over the image cells after placing it.
            out.write_all(b"\r\n\x1b[1A\r")?;
        }
        for (i, field) in fields.iter().enumerate() {
            let value = field.value(entry, opts, &mut owners, times.get(row).copied().flatten());
            let padding = widths[i] - value.width();
            let right = matches!(field, Field::Links | Field::Uid | Field::Gid | Field::Size);
            if right {
                write!(out, "{:padding$}", "")?;
            }
            if *field == Field::Mode && style.color {
                for c in value.chars() {
                    style.paint(
                        out,
                        match c {
                            'r' => "33",
                            'w' => "31",
                            'x' | 's' | 't' => "32",
                            '-' => "2",
                            _ => "36",
                        },
                        c.encode_utf8(&mut [0; 4]),
                    )?;
                }
            } else {
                style.paint(
                    out,
                    match field {
                        Field::Size => "32",
                        Field::Modified => "2",
                        Field::User | Field::Group | Field::Uid | Field::Gid => "33",
                        _ => "2",
                    },
                    &value,
                )?;
            }
            if !right {
                write!(out, "{:padding$}", "")?;
            }
            write!(out, " ")?;
        }
        if mini.is_some() {
            if let Some(image) = image {
                kitty::write(out, &image, 3, 1)?;
                previews.as_mut().unwrap().budget.placed(kitty::byte_len(
                    image.width(),
                    image.height(),
                    3,
                    1,
                ));
                write!(out, "\x1b[{}G", metadata_width + 5)?;
            } else if style.icons != crate::style::Icons::None {
                style.write_label(out, entry, style.icon(entry))?;
                write!(
                    out,
                    "{:padding$}",
                    "",
                    padding = 4 - style.icon(entry).width()
                )?;
            } else {
                write!(out, "    ")?;
            }
            style.write_label(out, entry, &style.name(entry))?;
        } else {
            style.write_name(out, entry)?;
        }
        if entry.kind == Kind::Link {
            let target = fs::read_link(&entry.path)
                .map(|p| escape(p.as_os_str()))
                .unwrap_or_else(|_| "?".into());
            style.paint(out, "2", &format!(" -> {target}"))?;
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

#[derive(Clone, Copy, Debug)]
struct Timestamp {
    year: i64,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
}

fn timestamp(seconds: i64) -> Option<Timestamp> {
    // Infer the C time type; musl is transitioning the named alias to time64.
    let time = seconds as _;
    let mut tm = std::mem::MaybeUninit::<libc::tm>::uninit();
    // SAFETY: both pointers are valid, and tm is read only after success.
    if unsafe { libc::localtime_r(&time, tm.as_mut_ptr()) }.is_null() {
        return None;
    }
    let tm = unsafe { tm.assume_init() };
    Some(Timestamp {
        year: i64::from(tm.tm_year) + 1900,
        month: (tm.tm_mon + 1) as u8,
        day: tm.tm_mday as u8,
        hour: tm.tm_hour as u8,
        minute: tm.tm_min as u8,
    })
}

fn time_width(time: Option<Timestamp>, twelve_hour: bool) -> usize {
    let year = time.map_or(4, |t| {
        let digits = t.year.unsigned_abs().checked_ilog10().unwrap_or(0) as usize + 1;
        (digits + usize::from(t.year < 0)).max(4)
    });
    year + 12 + if twelve_hour { 3 } else { 0 }
}

fn format_time(time: Option<Timestamp>, twelve_hour: bool) -> String {
    let Some(t) = time else {
        return if twelve_hour {
            "????-??-?? ??:?? ??"
        } else {
            "????-??-?? ??:??"
        }
        .into();
    };
    let hour = if twelve_hour {
        (t.hour + 11) % 12 + 1
    } else {
        t.hour
    };
    let suffix = if !twelve_hour {
        ""
    } else if t.hour < 12 {
        " AM"
    } else {
        " PM"
    };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}{suffix}",
        t.year, t.month, t.day, hour, t.minute
    )
}

// mode_t constants are u16 on macOS and u32 on Linux; retain portable casts.
#[allow(clippy::unnecessary_cast)]
fn permissions(mode: u32) -> String {
    let mut out = String::with_capacity(10);
    out.push_str(match mode & libc::S_IFMT as u32 {
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
    fn clock_formats_midnight_noon_and_unusual_year_widths() {
        for (hour, expected) in [
            (0, "12:07 AM"),
            (1, "01:07 AM"),
            (11, "11:07 AM"),
            (12, "12:07 PM"),
            (13, "01:07 PM"),
            (23, "11:07 PM"),
        ] {
            for year in [-10000, -1, 0, 2026, 10000, i64::from(i32::MAX) + 1900] {
                let time = Some(Timestamp {
                    year,
                    month: 9,
                    day: 10,
                    hour,
                    minute: 7,
                });
                assert!(format_time(time, true).ends_with(expected));
                for twelve_hour in [false, true] {
                    assert_eq!(
                        format_time(time, twelve_hour).len(),
                        time_width(time, twelve_hour)
                    );
                }
            }
        }
        for twelve_hour in [false, true] {
            assert_eq!(
                format_time(None, twelve_hour).len(),
                time_width(None, twelve_hour)
            );
        }
    }
    #[test]
    fn missing_metadata_keeps_the_entry_and_fields() {
        let entry = Entry {
            path: "lost".into(),
            name: "lost".into(),
            kind: Kind::File,
            metadata: None,
            executable: false,
        };
        let opts = Options {
            long: true,
            fields: vec![Field::Size, Field::Mode],
            ..Options::default()
        };
        let mut out = Vec::new();
        write(&mut out, &[entry], &opts, &Style::default(), None).unwrap();
        assert_eq!(out, b"? ? lost\n");
    }
    #[test]
    #[allow(clippy::unnecessary_cast)] // mode_t differs between macOS and Linux.
    fn unix_modes_and_sizes() {
        assert_eq!(permissions(libc::S_IFDIR as u32 | 0o1777), "drwxrwxrwt");
        assert_eq!(permissions(libc::S_IFREG as u32 | 0o4644), "-rwSr--r--");
        assert_eq!(size(1536, true), "1.5K");
        assert_eq!(size(1536, false), "1536");
    }
}
