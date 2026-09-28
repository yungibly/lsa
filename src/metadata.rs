use crate::{
    artwork::Icon,
    cli::{Options, Time},
    display::escape,
    entry::{Entry, Kind},
    kitty, pool,
    preview::{self, Previews},
    style::Style,
};
use std::{
    fmt::Write as _,
    fs,
    io::{self, Write},
    path::Path,
    sync::Mutex,
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
    Allocated,
    Inode,
    Modified,
}
const FIELD_COUNT: usize = 10;

/// Columns for this listing: `--fields`, or a preset shaped by -n, -g and -o,
/// then -s and -i prepend allocated size and inode (ls order).
fn selected(opts: &Options) -> Vec<Field> {
    let mut fields = if !opts.fields.is_empty() {
        opts.fields.clone()
    } else if opts.numeric {
        let mut fields = vec![Field::Mode, Field::Links];
        if !opts.no_owner {
            fields.push(Field::Uid);
        }
        if !opts.no_group {
            fields.push(Field::Gid);
        }
        fields.extend([Field::Size, Field::Modified]);
        fields
    } else {
        // Readable details omit the group; -g shows it in place of the owner.
        let mut fields = vec![Field::Mode, Field::Size];
        match (opts.no_owner, opts.no_group) {
            (false, _) => fields.push(Field::User),
            (true, false) => fields.push(Field::Group),
            (true, true) => {}
        }
        fields.push(Field::Modified);
        fields
    };
    for (wanted, field) in [(opts.blocks, Field::Allocated), (opts.inode, Field::Inode)] {
        if wanted && !fields.contains(&field) {
            fields.insert(0, field);
        }
    }
    fields
}

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
    fn name(&mut self, id: u32, group: bool) -> Option<&str> {
        use std::collections::hash_map::Entry;
        let names = if group {
            &mut self.groups
        } else {
            &mut self.users
        };
        let at_capacity = names.len() >= 64;
        let vacant = match names.entry(id) {
            Entry::Occupied(name) => return Some(name.into_mut().as_str()),
            Entry::Vacant(_) if at_capacity => return None,
            Entry::Vacant(name) => name,
        };
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
        Some(vacant.insert(name).as_str())
    }
}

impl Field {
    fn value(
        self,
        out: &mut String,
        entry: &Entry,
        opts: &Options,
        owners: &mut FormatCache,
        time: Option<Timestamp>,
    ) {
        out.clear();
        let Some(m) = &entry.metadata else {
            out.push('?');
            return;
        };
        match self {
            Self::Mode => permissions(out, m.mode),
            Self::Links => write!(out, "{}", m.links).unwrap(),
            Self::User | Self::Group => {
                let id = if self == Self::Group { m.gid } else { m.uid };
                if let Some(name) = owners.name(id, self == Self::Group) {
                    out.push_str(name);
                } else {
                    write!(out, "{id}").unwrap();
                }
            }
            Self::Uid => write!(out, "{}", m.uid).unwrap(),
            Self::Gid => write!(out, "{}", m.gid).unwrap(),
            // A directory's byte count describes its index, not its contents.
            Self::Size if entry.kind == Kind::Directory => out.push('-'),
            Self::Size => size(out, m.len, opts.human),
            Self::Allocated => size(out, m.blocks.saturating_mul(512), opts.human),
            Self::Inode => write!(out, "{}", m.inode).unwrap(),
            Self::Modified => format_time(out, time, opts.twelve_hour),
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
            "allocated" => Field::Allocated, "inode" => Field::Inode,
            _ => return Err("fields must be a comma-separated list of mode, links, user, group, uid, gid, size, allocated, inode, or modified; names are always shown".into()),
        };
        if fields.contains(&field) {
            return Err(format!("duplicate metadata field: {name}"));
        }
        fields.push(field);
    }
    Ok(fields)
}

pub fn write(
    out: &mut impl Write,
    dir: &Path,
    entries: &[Entry],
    opts: &Options,
    style: &Style,
    previews: Option<Previews<'_, '_>>,
) -> io::Result<()> {
    use unicode_width::UnicodeWidthStr;
    let fields = &selected(opts);
    let mut owners = FormatCache::default();
    // Reuse field storage across alignment and output; retain no formatted
    // metadata strings per entry, including numeric account-cache fallbacks.
    let mut value = String::with_capacity(32);
    let mut widths = [0; FIELD_COUNT];
    let heading = |field: Field| match field {
        Field::Mode => "Permissions",
        Field::Links => "Links",
        Field::User => "Owner",
        Field::Group => "Group",
        Field::Uid => "UID",
        Field::Gid => "GID",
        Field::Size => "Size",
        Field::Allocated => "Allocated",
        Field::Inode => "Inode",
        Field::Modified => match opts.time {
            Time::Modified => "Modified",
            Time::Changed => "Changed",
            Time::Accessed => "Accessed",
        },
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
                field.value(&mut value, entry, opts, &mut owners, None);
                value.width()
            };
            widths[i] = widths[i].max(width);
        }
    }
    if opts.header {
        for (i, field) in fields.iter().enumerate() {
            widths[i] = widths[i].max(heading(*field).width());
            value.clear();
            write!(value, "{:<width$}", heading(*field), width = widths[i]).unwrap();
            style.paint(out, "1;4", &value)?;
            write!(out, " ")?;
        }
        style.paint(out, "1;4", "Name")?;
        writeln!(out)?;
    }
    let metadata_width = widths[..fields.len()].iter().sum::<usize>() + fields.len();
    // One-row miniatures add no height. Each occupies the name's icon gutter,
    // and narrow terminals retain the ordinary complete text listing.
    let mut mini = None;
    let mut jobs = Vec::new();
    let mut attempts = vec![false; entries.len()];
    let mut none = crate::cache::Cache::new(None);
    let (cache, mut art) = match previews {
        Some(Previews {
            term,
            budget,
            cache,
            art,
        }) => {
            if term.kitty
                && term.rows >= 3
                && term.cols >= metadata_width + 4 + 12
                && budget.attempts_left > 0
                && entries.iter().any(Entry::candidate)
            {
                let w = f64::from(term.cell_width) * 3.0;
                let h = f64::from(term.cell_height);
                let scale = (96.0 / w).min(64.0 / h).min(1.0);
                let (w, h) = (
                    (w * scale).round().max(1.0) as u32,
                    (h * scale).round().max(1.0) as u32,
                );
                // Settle attempts in listing order first, exactly as drawing
                // would, so workers can decode ahead without changing output.
                let bytes = kitty::byte_len(w, h, 3, 1);
                for (attempt, entry) in attempts.iter_mut().zip(entries) {
                    if entry.candidate() && budget.begin(bytes) {
                        budget.placed(bytes);
                        *attempt = true;
                        jobs.push(preview::Job {
                            path: entry.path(dir),
                            width: w,
                            height: h,
                        });
                    }
                }
                mini = Some((w, h));
            }
            (cache, Some(art))
        }
        None => (&mut none, None),
    };
    let cache = Mutex::new(cache);
    pool::ordered(
        &jobs,
        pool::workers(),
        |job| preview::decode(job, &cache),
        |results| -> io::Result<()> {
            for (row, entry) in entries.iter().enumerate() {
                // Some(Err) marks a failed preview, drawn with shared error art.
                let image = if attempts[row] {
                    // Show finished lines while a slow source decodes.
                    if !results.is_ready() {
                        out.flush()?;
                    }
                    Some(results.next().unwrap_or(Err(())))
                } else {
                    None
                };
                if image.is_some() {
                    // Scroll before placement, then write this entry into the
                    // reserved line. Do not print spaces over the image cells.
                    out.write_all(b"\r\n\x1b[1A\r")?;
                }
                for (i, field) in fields.iter().enumerate() {
                    field.value(
                        &mut value,
                        entry,
                        opts,
                        &mut owners,
                        times.get(row).copied().flatten(),
                    );
                    let padding = widths[i] - value.width();
                    let right = matches!(
                        field,
                        Field::Links
                            | Field::Uid
                            | Field::Gid
                            | Field::Size
                            | Field::Allocated
                            | Field::Inode
                    );
                    if right {
                        write!(out, "{:padding$}", "")?;
                    }
                    if *field == Field::Mode && style.color {
                        // One SGR per run of equally colored characters, then a
                        // single reset: each switch resets, so dim never carries.
                        let mut current = "";
                        for c in value.chars() {
                            let code = match c {
                                'r' => "33",
                                'w' => "31",
                                'x' | 's' | 't' => "32",
                                '-' => "2",
                                _ => "36",
                            };
                            if code != current {
                                write!(out, "\x1b[0;{code}m")?;
                                current = code;
                            }
                            out.write_all(c.encode_utf8(&mut [0; 4]).as_bytes())?;
                        }
                        out.write_all(b"\x1b[0m")?;
                    } else {
                        style.paint(
                            out,
                            match field {
                                Field::Size | Field::Allocated => "32",
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
                if let Some((w, h)) = mini {
                    if let Some(image) = image {
                        match &image {
                            Ok(payload) => payload.write(out, 3, 1)?,
                            Err(()) => art
                                .as_deref_mut()
                                .expect("previews supply artwork")
                                .get(Icon::Error, w, h, style.color)
                                .write(out, 3, 1)?,
                        }
                        write!(out, "\x1b[{}G", metadata_width + 5)?;
                    } else if style.icons != crate::style::Icons::None {
                        style.write_label(out, dir, entry, style.icon(entry))?;
                        write!(
                            out,
                            "{:padding$}",
                            "",
                            padding = 4 - style.icon(entry).width()
                        )?;
                    } else {
                        write!(out, "    ")?;
                    }
                    style.write_label(out, dir, entry, &style.name(entry))?;
                } else {
                    style.write_name(out, dir, entry)?;
                }
                if entry.kind == Kind::Link {
                    let target = fs::read_link(entry.path(dir))
                        .map(|p| escape(p.as_os_str()))
                        .unwrap_or_else(|_| "?".into());
                    style.paint(out, style.target_code(entry), &format!(" -> {target}"))?;
                }
                writeln!(out)?;
            }
            Ok(())
        },
    )
}

fn size(out: &mut String, n: u64, human: bool) {
    if !human || n < 1024 {
        write!(out, "{n}").unwrap();
        return;
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
    write!(out, "{v:.1}{unit}").unwrap();
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

fn format_time(out: &mut String, time: Option<Timestamp>, twelve_hour: bool) {
    let Some(t) = time else {
        out.push_str(if twelve_hour {
            "????-??-?? ??:?? ??"
        } else {
            "????-??-?? ??:??"
        });
        return;
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
    write!(
        out,
        "{:04}-{:02}-{:02} {:02}:{:02}{suffix}",
        t.year, t.month, t.day, hour, t.minute
    )
    .unwrap();
}

// mode_t constants are u16 on macOS and u32 on Linux; retain portable casts.
#[allow(clippy::unnecessary_cast)]
fn permissions(out: &mut String, mode: u32) {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clock_formats_midnight_noon_and_unusual_year_widths() {
        let mut value = String::new();
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
                value.clear();
                format_time(&mut value, time, true);
                assert!(value.ends_with(expected));
                for twelve_hour in [false, true] {
                    value.clear();
                    format_time(&mut value, time, twelve_hour);
                    assert_eq!(value.len(), time_width(time, twelve_hour));
                }
            }
        }
        for twelve_hour in [false, true] {
            value.clear();
            format_time(&mut value, None, twelve_hour);
            assert_eq!(value.len(), time_width(None, twelve_hour));
        }
    }
    #[test]
    fn missing_metadata_keeps_the_entry_and_fields() {
        let entry = Entry::new("lost", Kind::File);
        let opts = Options {
            long: true,
            fields: vec![Field::Size, Field::Mode],
            ..Options::default()
        };
        let mut out = Vec::new();
        write(
            &mut out,
            Path::new(""),
            &[entry],
            &opts,
            &Style::default(),
            None,
        )
        .unwrap();
        assert_eq!(out, b"? ? lost\n");
    }
    #[test]
    fn owner_group_inode_and_allocation_presets() {
        use Field::*;
        let fields =
            |flags: &[&str]| selected(&crate::cli::parse(flags.iter().map(Into::into)).unwrap());
        assert_eq!(fields(&["-l"]), [Mode, Size, User, Modified]);
        assert_eq!(fields(&["-o"]), [Mode, Size, User, Modified]);
        assert_eq!(fields(&["-g"]), [Mode, Size, Group, Modified]);
        assert_eq!(fields(&["-og"]), [Mode, Size, Modified]);
        assert_eq!(fields(&["-n"]), [Mode, Links, Uid, Gid, Size, Modified]);
        assert_eq!(fields(&["-ng"]), [Mode, Links, Gid, Size, Modified]);
        assert_eq!(fields(&["-no"]), [Mode, Links, Uid, Size, Modified]);
        assert_eq!(
            fields(&["-is"]),
            [Inode, Allocated, Mode, Size, User, Modified]
        );
        assert_eq!(fields(&["-i", "--fields=size,inode"]), [Size, Inode]);
        assert_eq!(fields(&["-s", "--fields=mode"]), [Allocated, Mode]);
    }
    #[test]
    fn directories_show_no_byte_size_and_mode_colors_switch_per_run() {
        let details = |mode: u32| {
            Some(Box::new(crate::entry::Details {
                mode,
                uid: 0,
                gid: 0,
                len: 736,
                links: 1,
                inode: 0,
                blocks: 0,
                modified: 0,
                modified_nsec: 0,
            }))
        };
        let mut folder = Entry::new("folder", Kind::Directory);
        folder.metadata = details(0o040755);
        let mut file = Entry::new("file", Kind::File);
        file.metadata = details(0o100644);
        let opts = Options {
            long: true,
            human: true,
            fields: vec![Field::Size, Field::Mode],
            ..Options::default()
        };
        let mut out = Vec::new();
        write(
            &mut out,
            Path::new(""),
            &[folder, file],
            &opts,
            &Style::default(),
            None,
        )
        .unwrap();
        assert_eq!(out, b"  - drwxr-xr-x folder\n736 -rw-r--r-- file\n");
        let mut style = Style::default();
        style.color = true;
        let mut file = Entry::new("file", Kind::File);
        file.metadata = details(0o100644);
        let opts = Options {
            fields: vec![Field::Mode],
            ..opts
        };
        let mut out = Vec::new();
        write(&mut out, Path::new(""), &[file], &opts, &style, None).unwrap();
        let expected = "\x1b[0;2m-\x1b[0;33mr\x1b[0;31mw\x1b[0;2m-\x1b[0;33mr\x1b[0;2m--\x1b[0;33mr\x1b[0;2m--\x1b[0m file\n";
        assert_eq!(String::from_utf8(out).unwrap(), expected);
    }
    #[test]
    #[allow(clippy::unnecessary_cast)] // mode_t differs between macOS and Linux.
    fn unix_modes_and_sizes() {
        let mut value = String::new();
        for (mode, expected) in [
            (libc::S_IFDIR as u32 | 0o1777, "drwxrwxrwt"),
            (libc::S_IFREG as u32 | 0o4644, "-rwSr--r--"),
        ] {
            value.clear();
            permissions(&mut value, mode);
            assert_eq!(value, expected);
        }
        for (n, human, expected) in [
            (0, true, "0"),
            (1023, true, "1023"),
            (1024, true, "1.0K"),
            (1536, true, "1.5K"),
            (1536, false, "1536"),
            (1024 * 1024 - 1, true, "1024.0K"),
            (1024 * 1024, true, "1.0M"),
            (u64::MAX, true, "16.0E"),
            (u64::MAX, false, "18446744073709551615"),
        ] {
            value.clear();
            size(&mut value, n, human);
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn account_cache_bound_keeps_names_and_numeric_fallbacks() {
        let mut owners = FormatCache::default();
        for id in 0..64 {
            owners.users.insert(id, "owner".into());
            owners.groups.insert(id, "group".into());
        }
        for group in [false, true] {
            assert_eq!(
                owners.name(63, group),
                Some(if group { "group" } else { "owner" })
            );
            assert_eq!(owners.name(u32::MAX, group), None);
        }
        let mut entry = Entry::new("entry", Kind::File);
        entry.metadata = Some(Box::new(crate::entry::Details {
            mode: 0,
            uid: u32::MAX,
            gid: 63,
            len: 0,
            links: u64::MAX,
            inode: u64::MAX,
            blocks: u64::MAX,
            modified: 0,
            modified_nsec: 0,
        }));
        let opts = Options::default();
        let mut value = String::new();
        for (field, expected) in [
            (Field::Links, "18446744073709551615"),
            (Field::User, "4294967295"),
            (Field::Group, "group"),
            (Field::Uid, "4294967295"),
            (Field::Gid, "63"),
            (Field::Size, "0"),
            (Field::Inode, "18446744073709551615"),
            (Field::Allocated, "18446744073709551615"),
        ] {
            field.value(&mut value, &entry, &opts, &mut owners, None);
            assert_eq!(value, expected);
        }
        entry.metadata = None;
        Field::User.value(&mut value, &entry, &opts, &mut owners, None);
        assert_eq!(value, "?");
        assert_eq!(owners.users.len(), 64);
        assert_eq!(owners.groups.len(), 64);
    }
}
