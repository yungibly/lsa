//! A text-only, explicit browser. Entries are shared with inline output; screen
//! ownership, input, and restoration belong exclusively to this session.
use crate::{
    browser_terminal::{self, Event, Session},
    cli::Options,
    display,
    entry::{self, Entry, Kind},
};
use std::{
    collections::VecDeque,
    ffi::OsString,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::Duration,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const HISTORY_LIMIT: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Key {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Open,
    Parent,
    Detail,
    Escape,
    Refresh,
    Quit,
    Interrupt,
    Suspend,
}

#[derive(Default)]
enum InputState {
    #[default]
    Plain,
    Escape,
    Sequence {
        bytes: [u8; 32],
        len: usize,
    },
    Paste(usize),
}

#[derive(Default)]
struct Input {
    state: InputState,
}

impl Input {
    fn timeout(&self) -> Option<Duration> {
        matches!(self.state, InputState::Escape).then(|| Duration::from_millis(100))
    }

    fn escape(&mut self) -> Option<Key> {
        if matches!(self.state, InputState::Escape) {
            self.state = InputState::Plain;
            return Some(Key::Escape);
        }
        None
    }

    fn byte(&mut self, byte: u8) -> Option<Key> {
        match &mut self.state {
            InputState::Paste(matched) => {
                // Ignore bracketed paste entirely: pasted q/Enter are not actions.
                const END: &[u8] = b"\x1b[201~";
                *matched = if byte == END[*matched] {
                    *matched + 1
                } else {
                    usize::from(byte == END[0])
                };
                if *matched == END.len() {
                    self.state = InputState::Plain;
                }
                None
            }
            InputState::Escape => {
                self.state = if matches!(byte, b'[' | b'O') {
                    InputState::Sequence {
                        bytes: [0; 32],
                        len: 0,
                    }
                } else {
                    InputState::Plain
                };
                None // Unrecognized Alt combinations are not plain-key actions.
            }
            InputState::Sequence { bytes, len } => {
                if (0x40..=0x7e).contains(&byte) {
                    let key = match (&bytes[..(*len).min(bytes.len())], byte) {
                        (b"200", b'~') => {
                            self.state = InputState::Paste(0);
                            return None;
                        }
                        (b"", b'A') => Some(Key::Up),
                        (b"", b'B') => Some(Key::Down),
                        (b"", b'C') => Some(Key::Open),
                        (b"", b'D') => Some(Key::Parent),
                        (b"", b'H') | (b"1" | b"7", b'~') => Some(Key::Home),
                        (b"", b'F') | (b"4" | b"8", b'~') => Some(Key::End),
                        (b"5", b'~') => Some(Key::PageUp),
                        (b"6", b'~') => Some(Key::PageDown),
                        _ => None,
                    };
                    self.state = InputState::Plain;
                    key
                } else {
                    // Overlong sequences are discarded through their final byte.
                    if *len < bytes.len() {
                        bytes[*len] = byte;
                    }
                    *len = (*len + 1).min(bytes.len() + 1);
                    None
                }
            }
            InputState::Plain => match byte {
                0x1b => {
                    self.state = InputState::Escape;
                    None
                }
                b'k' => Some(Key::Up),
                b'j' => Some(Key::Down),
                2 => Some(Key::PageUp),
                6 => Some(Key::PageDown),
                b'g' => Some(Key::Home),
                b'G' => Some(Key::End),
                b'\r' | b'\n' | b'l' => Some(Key::Open),
                8 | 127 | b'h' => Some(Key::Parent),
                b' ' | b'\t' => Some(Key::Detail),
                b'r' => Some(Key::Refresh),
                b'q' | 4 => Some(Key::Quit),
                3 => Some(Key::Interrupt),
                26 => Some(Key::Suspend),
                _ => None,
            },
        }
    }
}

struct Position {
    name: Option<OsString>,
    selected: usize,
    top: usize,
}

struct Visit {
    path: PathBuf,
    position: Position,
}

struct Detail {
    text: String,
    top: usize,
}

struct Browser<'a> {
    opts: &'a Options,
    path: PathBuf,
    entries: Vec<Entry>,
    selected: usize,
    top: usize,
    history: VecDeque<Visit>,
    detail: Option<Detail>,
    message: String,
}

fn read_directory(path: &Path, opts: &Options) -> io::Result<entry::Listing> {
    let listing = entry::list(path, opts);
    if !listing.valid || !listing.directory {
        return Err(io::Error::other(
            listing.errors.first().cloned().unwrap_or_else(|| {
                format!(
                    "{}: --browse requires a directory",
                    display::escape(path.as_os_str())
                )
            }),
        ));
    }
    Ok(listing)
}

impl<'a> Browser<'a> {
    fn new(opts: &'a Options) -> io::Result<Self> {
        // Resolve only explicitly requested navigation. Symlink entries retain
        // their identity in listings; history remembers the route back through links.
        let path = fs::canonicalize(&opts.paths[0]).map_err(|e| {
            io::Error::other(format!(
                "{}: {e}",
                display::escape(opts.paths[0].as_os_str())
            ))
        })?;
        let listing = read_directory(&path, opts)?;
        let mut browser = Self {
            opts,
            path,
            entries: Vec::new(),
            selected: 0,
            top: 0,
            history: VecDeque::new(),
            detail: None,
            message: String::new(),
        };
        browser.install(listing, None);
        Ok(browser)
    }

    fn position(&self) -> Position {
        Position {
            name: self.entries.get(self.selected).map(|e| e.name.clone()),
            selected: self.selected,
            top: self.top,
        }
    }

    fn install(&mut self, listing: entry::Listing, position: Option<&Position>) {
        self.message = if let Some(error) = listing.errors.first() {
            format!("{} listing error(s): {error}", listing.errors.len())
        } else {
            String::new()
        };
        self.entries = listing.entries;
        self.selected = position
            .map_or(0, |p| {
                self.entries
                    .iter()
                    .position(|e| Some(&e.name) == p.name.as_ref())
                    .unwrap_or(p.selected)
            })
            .min(self.entries.len().saturating_sub(1));
        self.top = position.map_or(0, |p| p.top);
        self.detail = None;
    }

    fn keep_visible(&mut self, height: usize) {
        self.top = self.top.min(self.entries.len().saturating_sub(height));
        if self.selected < self.top {
            self.top = self.selected;
        } else if self.selected >= self.top + height {
            self.top = self.selected + 1 - height;
        }
    }

    fn open(&mut self) -> io::Result<()> {
        let Some(entry) = self.entries.get(self.selected) else {
            return Ok(());
        };
        if entry.kind != Kind::Directory
            && (entry.kind != Kind::Link || !fs::metadata(&entry.path)?.is_dir())
        {
            self.show_detail();
            return Ok(());
        }
        let path = fs::canonicalize(&entry.path)?;
        let listing = read_directory(&path, self.opts)?;
        if self.history.len() == HISTORY_LIMIT {
            self.history.pop_front();
        }
        self.history.push_back(Visit {
            path: self.path.clone(),
            position: self.position(),
        });
        self.path = path;
        self.install(listing, None);
        Ok(())
    }

    fn parent(&mut self) -> io::Result<()> {
        if let Some(visit) = self.history.back() {
            let listing = read_directory(&visit.path, self.opts)?;
            let visit = self.history.pop_back().unwrap();
            self.path = visit.path;
            self.install(listing, Some(&visit.position));
        } else if let Some(parent) = self.path.parent() {
            let listing = read_directory(parent, self.opts)?;
            let position = Position {
                name: self.path.file_name().map(|name| name.to_os_string()),
                selected: 0,
                top: 0,
            };
            self.path = parent.into();
            self.install(listing, Some(&position));
        }
        Ok(())
    }

    fn show_detail(&mut self) {
        let text = if let Some(entry) = self.entries.get(self.selected) {
            format!(
                "Name: {}\nKind: {}\nPath: {}",
                display::escape(&entry.name),
                entry.kind.label(),
                display::escape(entry.path.as_os_str())
            )
        } else {
            format!(
                "Path: {}\n(empty directory)",
                display::escape(self.path.as_os_str())
            )
        };
        self.detail = Some(Detail { text, top: 0 });
    }

    fn key(&mut self, key: Key, width: usize, height: usize) -> io::Result<()> {
        if let Some(detail) = &mut self.detail {
            match key {
                Key::Escape | Key::Detail | Key::Open | Key::Parent => self.detail = None,
                _ => {
                    detail.top = moved(
                        detail.top,
                        detail_lines(&detail.text, width)
                            .len()
                            .saturating_sub(height),
                        key,
                        height,
                    )
                }
            }
            return Ok(());
        }
        match key {
            Key::Open => self.open()?,
            Key::Parent => self.parent()?,
            Key::Refresh => {
                let listing = read_directory(&self.path, self.opts)?;
                self.install(listing, Some(&self.position()));
            }
            Key::Detail => self.show_detail(),
            _ => {
                self.selected = moved(
                    self.selected,
                    self.entries.len().saturating_sub(1),
                    key,
                    height,
                )
            }
        }
        self.keep_visible(height);
        Ok(())
    }

    fn render(&mut self, out: &mut impl Write, cols: usize, rows: usize) -> io::Result<()> {
        out.write_all(b"\x1b[0m\x1b[2J\x1b[H")?;
        let width = cols.saturating_sub(1);
        if cols < 12 || rows < 5 {
            line(out, 1, "Resize to at least 12x5; q quits", width, false)?;
            return out.flush();
        }
        let height = rows - 3;
        self.keep_visible(height);
        let header = format!("lsa  {}", display::escape(self.path.as_os_str()));
        line(out, 1, &header, width, false)?;
        if let Some(detail) = &mut self.detail {
            let lines = detail_lines(&detail.text, width);
            detail.top = detail.top.min(lines.len().saturating_sub(height));
            for (i, text) in lines.iter().skip(detail.top).take(height).enumerate() {
                line(out, i + 2, text, width, false)?;
            }
            let status = format!(
                "Full name/path | lines {}-{} of {}",
                detail.top + 1,
                (detail.top + height).min(lines.len()),
                lines.len()
            );
            line(out, rows - 1, &status, width, false)?;
            line(
                out,
                rows,
                "Up/Down PgUp/PgDn scroll | Esc closes | q quit",
                width,
                false,
            )?;
        } else {
            if self.entries.is_empty() {
                line(out, 2, "(empty directory)", width, false)?;
            }
            for (i, entry) in self.entries.iter().enumerate().skip(self.top).take(height) {
                let selected = i == self.selected;
                let text = format!("{}{}", if selected { "> " } else { "  " }, entry.label());
                line(out, i - self.top + 2, &text, width, selected)?;
            }
            let status = if self.message.is_empty() {
                format!(
                    "{} / {} | Space: full name/path | r refresh",
                    if self.entries.is_empty() {
                        0
                    } else {
                        self.selected + 1
                    },
                    self.entries.len()
                )
            } else {
                self.message.clone()
            };
            line(out, rows - 1, &status, width, false)?;
            line(
                out,
                rows,
                "Up/Down j/k | PgUp/PgDn g/G | Enter open | h parent | q quit",
                width,
                false,
            )?;
        }
        out.flush()
    }
}

fn moved(current: usize, last: usize, key: Key, page: usize) -> usize {
    match key {
        Key::Up => current.saturating_sub(1),
        Key::Down => current.saturating_add(1).min(last),
        Key::PageUp => current.saturating_sub(page),
        Key::PageDown => current.saturating_add(page).min(last),
        Key::Home => 0,
        Key::End => last,
        _ => current,
    }
}

// Clip only the overview; Space offers lossless, scrollable inspection. Keep
// graphemes intact, show a continuation marker, and never touch the last column.
fn line(
    out: &mut impl Write,
    row: usize,
    text: &str,
    width: usize,
    selected: bool,
) -> io::Result<()> {
    write!(out, "\x1b[{row};1H")?;
    if selected {
        out.write_all(b"\x1b[7m")?;
    }
    let clipped = text.width() > width;
    let available = width.saturating_sub(usize::from(clipped));
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        if used + grapheme.width() > available {
            break;
        }
        out.write_all(grapheme.as_bytes())?;
        used += grapheme.width();
    }
    if clipped && width > 0 {
        out.write_all(b">")?;
    }
    out.write_all(b"\x1b[0m")
}

fn detail_lines(text: &str, width: usize) -> Vec<String> {
    let mut result = Vec::new();
    for line in text.lines() {
        let mut safe = String::new();
        for grapheme in line.graphemes(true) {
            if grapheme.width() > width {
                use std::fmt::Write;
                for c in grapheme.chars() {
                    write!(safe, "\\u{{{:x}}}", c as u32).unwrap();
                }
            } else {
                safe.push_str(grapheme);
            }
        }
        result.extend(display::wrap(&safe, width));
    }
    result
}

pub fn run(opts: &Options, out: &mut impl Write) -> io::Result<u8> {
    browser_terminal::validate()?;
    let mut browser = Browser::new(opts)?;
    let mut session = Session::enter(out)?;
    let mut input = Input::default();
    let mut buffer = [0; 256];
    let mut size = browser_terminal::size();
    let mut dirty = true;
    let code = 'events: loop {
        if dirty {
            browser.render(session.out, size.0, size.1)?;
            dirty = false;
        }
        let mut keys = Vec::new();
        match session.wait(&mut buffer, input.timeout())? {
            Event::Exit(code) => break code,
            Event::Suspend => {
                session.suspend()?;
                size = browser_terminal::size();
                dirty = true;
                continue;
            }
            Event::Redraw => {
                size = browser_terminal::size();
                dirty = true;
                continue;
            }
            Event::Timeout => keys.extend(input.escape()),
            Event::Input(n) => {
                for byte in &buffer[..n] {
                    keys.extend(input.byte(*byte));
                }
            }
        }
        for key in keys {
            match key {
                Key::Quit => break 'events 0,
                Key::Interrupt => break 'events 130,
                Key::Suspend => {
                    session.suspend()?;
                    size = browser_terminal::size();
                }
                _ => {
                    if let Err(error) = browser.key(
                        key,
                        size.0.saturating_sub(1).max(1),
                        size.1.saturating_sub(3).max(1),
                    ) {
                        browser.message = format!("Cannot navigate/refresh: {error}");
                    }
                }
            }
            dirty = true;
        }
    };
    session.restore()?;
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragmented_keys_unknown_sequences_and_paste() {
        let mut input = Input::default();
        let sequence = b"\x1b[A\x1bOB\x1b[6~\x1b[1;5A\x1b[200~q\rjj\x1b[201~g";
        let keys: Vec<_> = sequence.iter().filter_map(|b| input.byte(*b)).collect();
        assert_eq!(keys, [Key::Up, Key::Down, Key::PageDown, Key::Home]);
        assert!(input.byte(27).is_none());
        assert!(input.timeout().is_some());
        assert_eq!(input.escape(), Some(Key::Escape));
        assert!(input.timeout().is_none());
        for byte in b"\x1b[".iter().chain([b'0'; 1000].iter()).chain(b"~j") {
            let key = input.byte(*byte);
            assert_eq!(key, (*byte == b'j').then_some(Key::Down));
        }
    }

    #[test]
    fn clipped_overview_and_complete_grapheme_detail() {
        let text = "桃e\u{301}👩‍💻long name";
        let mut out = Vec::new();
        line(&mut out, 2, text, 6, true).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "\x1b[2;1H\x1b[7m桃e\u{301}👩‍💻>\x1b[0m"
        );
        let lines = detail_lines(text, 6);
        assert_eq!(lines.concat(), text);
        assert!(lines.iter().all(|line| line.width() <= 6));
    }

    #[test]
    fn restored_selection_uses_raw_name_and_keeps_viewport() {
        let opts = Options::default();
        let make = |names: &[&str]| entry::Listing {
            entries: names
                .iter()
                .map(|name| Entry {
                    path: PathBuf::from(name),
                    name: OsString::from(name),
                    kind: Kind::File,
                    metadata: None,
                })
                .collect(),
            valid: true,
            directory: true,
            errors: Vec::new(),
        };
        let mut browser = Browser {
            opts: &opts,
            path: PathBuf::from("."),
            entries: Vec::new(),
            selected: 0,
            top: 0,
            history: VecDeque::new(),
            detail: None,
            message: String::new(),
        };
        browser.install(make(&["a", "b", "c", "d", "e"]), None);
        browser.key(Key::End, 80, 2).unwrap();
        assert_eq!((browser.selected, browser.top), (4, 3));
        let position = browser.position();
        browser.install(make(&["0", "a", "b", "c", "d", "e"]), Some(&position));
        browser.keep_visible(3);
        assert_eq!((browser.selected, browser.top), (5, 3));
        browser.install(make(&["a"]), Some(&position));
        browser.keep_visible(3);
        assert_eq!((browser.selected, browser.top), (0, 0));
        browser.install(make(&[]), Some(&position));
        browser.key(Key::PageDown, 80, 1).unwrap();
        assert_eq!((browser.selected, browser.top), (0, 0));
    }
}
