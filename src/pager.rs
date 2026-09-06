//! An explicit pager. Entries are shared with inline output; screen
//! ownership, input, and restoration belong exclusively to this session.
use crate::{
    cli::Options,
    display,
    entry::{self, Entry},
    pager_graphics::{Geometry, IMAGE_ROWS},
    pager_previews::{Previews, View},
    pager_terminal::{self, Event, Session},
};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::Duration,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Key {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Left,
    Right,
    Back,
    Detail,
    Escape,
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
                        (b"", b'C') => Some(Key::Right),
                        (b"", b'D') => Some(Key::Left),
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
                b'h' => Some(Key::Left),
                b'l' => Some(Key::Right),
                b'b' | 2 => Some(Key::PageUp),
                b' ' | 6 => Some(Key::PageDown),
                b'g' => Some(Key::Home),
                b'G' => Some(Key::End),
                b'\r' | b'\n' | b'\t' => Some(Key::Detail),
                8 | 127 => Some(Key::Back),
                b'q' | 4 => Some(Key::Quit),
                3 => Some(Key::Interrupt),
                26 => Some(Key::Suspend),
                _ => None,
            },
        }
    }
}

struct Detail {
    text: String,
    top: usize,
}

struct Pager {
    path: PathBuf,
    entries: Vec<Entry>,
    selected: usize,
    top: usize,
    detail: Option<Detail>,
    message: String,
    has_candidates: bool,
}

fn read_directory(path: &Path, opts: &Options) -> io::Result<entry::Listing> {
    let listing = entry::list(path, opts);
    if !listing.valid || !listing.directory {
        return Err(io::Error::other(
            listing.errors.first().cloned().unwrap_or_else(|| {
                format!(
                    "{}: --page requires a directory",
                    display::escape(path.as_os_str())
                )
            }),
        ));
    }
    Ok(listing)
}

impl Pager {
    fn new(opts: &Options) -> io::Result<Self> {
        // Resolve the explicit operand once. The listing's membership/order is
        // fixed for this invocation; directory entries are never navigation.
        let path = fs::canonicalize(&opts.paths[0]).map_err(|e| {
            io::Error::other(format!(
                "{}: {e}",
                display::escape(opts.paths[0].as_os_str())
            ))
        })?;
        let listing = read_directory(&path, opts)?;
        let has_candidates = listing.entries.iter().any(Entry::candidate);
        let message = listing.errors.first().map_or_else(String::new, |error| {
            format!("{} listing error(s): {error}", listing.errors.len())
        });
        Ok(Self {
            path,
            entries: listing.entries,
            selected: 0,
            top: 0,
            detail: None,
            message,
            has_candidates,
        })
    }

    fn keep_visible(&mut self, height: usize) {
        self.top = self.top.min(self.entries.len().saturating_sub(height));
        if self.selected < self.top {
            self.top = self.selected;
        } else if self.selected >= self.top + height {
            self.top = self.selected + 1 - height;
        }
    }

    fn fit_view(&mut self, geometry: Option<Geometry>, rows: usize) {
        if let Some(g) = geometry {
            let selected_row = self.selected / g.columns;
            let mut top_row = (self.top / g.columns).min(
                self.entries
                    .len()
                    .div_ceil(g.columns)
                    .saturating_sub(g.lines),
            );
            if selected_row < top_row {
                top_row = selected_row;
            }
            if selected_row >= top_row + g.lines {
                top_row = selected_row + 1 - g.lines;
            }
            self.top = top_row * g.columns;
        } else {
            self.keep_visible(rows.saturating_sub(3).max(1));
        }
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

    fn key(&mut self, key: Key, width: usize, height: usize, columns: usize) {
        if let Some(detail) = &mut self.detail {
            match key {
                Key::Escape | Key::Detail | Key::Back => self.detail = None,
                _ => {
                    detail.top = moved(
                        detail.top,
                        detail_lines(&detail.text, width)
                            .len()
                            .saturating_sub(height),
                        key,
                        height,
                        1,
                    )
                }
            }
            return;
        }
        if key == Key::Detail {
            self.show_detail();
        } else {
            // Paging advances the viewport itself, keeping the selection's
            // relative position when possible. Arrow scrolling moves only as
            // far as needed to keep the selected entry visible.
            match key {
                Key::PageUp => self.top = self.top.saturating_sub(height),
                Key::PageDown => self.top = self.top.saturating_add(height),
                _ => {}
            }
            self.selected = moved(
                self.selected,
                self.entries.len().saturating_sub(1),
                key,
                height,
                columns,
            );
        }
    }

    fn render(
        &mut self,
        out: &mut impl Write,
        cols: usize,
        rows: usize,
        geometry: Option<Geometry>,
        previews: &Previews,
        available: usize,
    ) -> io::Result<()> {
        if geometry.is_some() {
            // ED(2) clears graphics too. Erase only text, leaving owned placements
            // intact until the viewport manager moves/deletes them explicitly.
            out.write_all(b"\x1b[0m\x1b[H")?;
            for row in 1..=rows {
                write!(out, "\x1b[{row};1H\x1b[2K")?;
            }
        } else {
            out.write_all(b"\x1b[0m\x1b[2J\x1b[H")?;
        }
        let width = cols.saturating_sub(1);
        if cols < 12 || rows < 5 {
            line(out, 1, "Resize to at least 12x5; q quits", width, false)?;
            return out.flush();
        }
        let height = rows - 3;
        self.fit_view(geometry, rows);
        let header = display::escape(self.path.as_os_str());
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
            for (i, entry) in self
                .entries
                .iter()
                .enumerate()
                .skip(self.top)
                .take(geometry.map_or(height, Geometry::capacity))
            {
                let selected = i == self.selected;
                let text = format!("{}{}", if selected { "> " } else { "  " }, entry.label());
                if let Some(g) = geometry {
                    let (row, col) = g.position(i - self.top);
                    let placeholder = if entry.candidate() {
                        previews.placeholder(i, available)
                    } else {
                        entry.kind.label()
                    };
                    let offset = (g.tile - 2).saturating_sub(placeholder.width()) / 2;
                    cell(
                        out,
                        row + 2,
                        col + offset,
                        placeholder,
                        g.tile - 2 - offset,
                        false,
                    )?;
                    cell(out, row + IMAGE_ROWS, col, &text, g.tile - 2, selected)?;
                } else {
                    line(out, i - self.top + 2, &text, width, selected)?;
                }
            }
            let status = if self.message.is_empty() {
                format!(
                    "{} / {}  {}",
                    if self.entries.is_empty() {
                        0
                    } else {
                        self.selected + 1
                    },
                    self.entries.len(),
                    self.entries
                        .get(self.selected)
                        .map_or_else(String::new, Entry::label)
                )
            } else {
                self.message.clone()
            };
            line(out, rows - 1, &status, width, false)?;
            line(
                out,
                rows,
                "q quit | Space/b page | Arrows move | Enter name",
                width,
                false,
            )?;
        }
        out.flush()
    }
}

fn moved(current: usize, last: usize, key: Key, page: usize, columns: usize) -> usize {
    match key {
        Key::Left if !current.is_multiple_of(columns) => current - 1,
        Key::Right if current % columns + 1 < columns => (current + 1).min(last),
        Key::Up if current >= columns => current - columns,
        Key::Down => current.saturating_add(columns).min(last),
        Key::PageUp => current.saturating_sub(page),
        Key::PageDown => current.saturating_add(page).min(last),
        Key::Home => 0,
        Key::End => last,
        _ => current,
    }
}

// Clip only the overview; Enter offers lossless, scrollable inspection. Keep
// graphemes intact, show a continuation marker, and never touch the last column.
fn line(
    out: &mut impl Write,
    row: usize,
    text: &str,
    width: usize,
    selected: bool,
) -> io::Result<()> {
    cell(out, row, 1, text, width, selected)
}

fn cell(
    out: &mut impl Write,
    row: usize,
    col: usize,
    text: &str,
    width: usize,
    selected: bool,
) -> io::Result<()> {
    write!(out, "\x1b[{row};{col}H")?;
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
        used += 1;
    }
    if selected {
        write!(out, "{:width$}", "", width = width - used)?;
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

fn terminal_geometry(opts: &Options) -> ((usize, usize), Option<Geometry>) {
    let term = crate::terminal::Terminal::detect(opts);
    (
        (term.cols.min(512), term.rows.min(256)),
        Geometry::new(&term),
    )
}

pub fn run(opts: &Options, out: &mut impl Write) -> io::Result<u8> {
    pager_terminal::validate()?;
    let mut pager = Pager::new(opts)?;
    let mut session = Session::enter(out)?;
    let mut previews = Previews::new(opts.preview_limit, opts.cache_path().map(Path::to_path_buf));
    let mut input = Input::default();
    let mut buffer = [0; 256];
    let (mut size, mut base_geometry) = terminal_geometry(opts);
    let mut dirty = true;
    let code = 'events: loop {
        let geometry = base_geometry.filter(|_| pager.detail.is_none() && pager.has_candidates);
        pager.fit_view(geometry, size.1);
        if dirty {
            let view = geometry.map(|geometry| View {
                geometry,
                range: pager.top..(pager.top + geometry.capacity()).min(pager.entries.len()),
            });
            previews.sync(
                view,
                pager.entries.len(),
                &mut session.graphics,
                session.out,
            )?;
            pager.render(
                session.out,
                size.0,
                size.1,
                geometry,
                &previews,
                session.graphics.remaining(),
            )?;
            dirty = previews.paint(&mut session.graphics, session.out)?;
            session.out.write_all(b"\x1b[0m")?;
            session.out.flush()?;
            dirty |=
                previews.schedule(&pager.entries, pager.selected, session.graphics.remaining());
        }
        if dirty {
            continue;
        }
        let mut keys = Vec::new();
        match session.wait(&mut buffer, input.timeout(), previews.fd())? {
            Event::Exit(code) => break code,
            Event::Suspend => {
                previews.invalidate(&mut session.graphics, session.out)?;
                session.suspend()?;
                (size, base_geometry) = terminal_geometry(opts);
                input = Input::default();
                dirty = true;
                continue;
            }
            Event::Redraw => {
                (size, base_geometry) = terminal_geometry(opts);
                dirty = true;
                continue;
            }
            Event::Preview => {
                previews.complete();
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
                    previews.invalidate(&mut session.graphics, session.out)?;
                    session.suspend()?;
                    (size, base_geometry) = terminal_geometry(opts);
                    input = Input::default();
                }
                _ => {
                    // Recompute for each key: Enter can change the mode halfway
                    // through one input batch, and details always scroll by lines.
                    let grid =
                        base_geometry.filter(|_| pager.detail.is_none() && pager.has_candidates);
                    let page = grid.map_or(size.1.saturating_sub(3).max(1), Geometry::capacity);
                    pager.key(
                        key,
                        size.0.saturating_sub(1).max(1),
                        page,
                        grid.map_or(1, |g| g.columns),
                    );
                }
            }
            dirty = true;
        }
    };
    let stats = previews.stats;
    drop(previews); // Stop scheduling before restoring the terminal; never join a decode.
    session.restore()?;
    if opts.cache_stats {
        let _ = writeln!(
            io::stderr().lock(),
            "lsa: cache: {} hits, {} misses, {} writes, {} errors",
            stats.hits,
            stats.misses,
            stats.writes,
            stats.errors
        );
    }
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
    fn spatial_movement_stays_in_rows_and_handles_last_partial_row() {
        // Three columns, last row contains only index 9.
        assert_eq!(moved(0, 9, Key::Left, 9, 3), 0);
        assert_eq!(moved(1, 9, Key::Left, 9, 3), 0);
        assert_eq!(moved(1, 9, Key::Right, 9, 3), 2);
        assert_eq!(moved(2, 9, Key::Right, 9, 3), 2);
        assert_eq!(moved(2, 9, Key::Down, 9, 3), 5);
        assert_eq!(moved(5, 9, Key::Up, 9, 3), 2);
        assert_eq!(moved(8, 9, Key::Down, 9, 3), 9);
        assert_eq!(moved(9, 9, Key::Right, 9, 3), 9);
        assert_eq!(moved(4, 9, Key::Left, 6, 1), 4);
        assert_eq!(moved(4, 9, Key::Right, 6, 1), 4);
        assert_eq!(moved(4, 9, Key::PageDown, 6, 1), 9);
        assert_eq!(moved(0, 0, Key::Down, 1, 3), 0);
    }
}
