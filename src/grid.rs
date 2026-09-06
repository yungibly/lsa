use crate::{
    artwork,
    cache::Cache,
    columns,
    display::wrap,
    entry::Entry,
    kitty,
    preview::{self, Budget},
    style::Style,
    terminal::Terminal,
};
use std::io::{self, Write};
use unicode_width::UnicodeWidthStr;

const ROWS: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    pub columns: usize,
    pub tile: usize,
    pub image_cols: usize,
    pub width: u32,
    pub height: u32,
    pub bytes: usize,
}

impl Geometry {
    pub fn new(term: &Terminal) -> Option<Self> {
        if term.cols < 12 || term.rows < 6 {
            return None;
        }
        // Reserve the rightmost cell; bound row scratch space on very wide TTYs.
        let available = (term.cols - 1).min(240);
        let columns = (available / 24).clamp(1, 10);
        let tile = available / columns;
        let image_cols = (tile - 2).min(14);
        let width = image_cols as f64 * f64::from(term.cell_width);
        let height = ROWS as f64 * f64::from(term.cell_height);
        let scale = (320.0 / width).min(240.0 / height).min(1.0);
        let width = (width * scale).round().max(1.0) as u32;
        let height = (height * scale).round().max(1.0) as u32;
        Some(Self {
            columns,
            tile,
            image_cols,
            width,
            height,
            bytes: kitty::byte_len(width, height, image_cols, ROWS),
        })
    }
}

pub fn write(
    out: &mut impl Write,
    entries: &[Entry],
    geometry: Geometry,
    term: &Terminal,
    style: &Style,
    budget: &mut Budget,
    cache: &mut Cache<'_>,
) -> io::Result<()> {
    let Geometry {
        columns,
        tile,
        image_cols,
        width,
        height,
        bytes,
    } = geometry;
    for (row_index, row) in entries.chunks(columns).enumerate() {
        if budget.attempts_left == 0
            || bytes * row.len() > budget.bytes_left
            || row.len() > budget.placements_left
        {
            let remaining = &entries[row_index * columns..];
            return columns::Plan::new(remaining, term.cols, style).write(out, remaining, style);
        }
        // Reserve the image area AND a label line before placing images. This
        // scrolls first, so a placement never extends below the visible screen.
        for _ in 0..=ROWS {
            out.write_all(b"\r\n")?;
        }
        out.flush()?;
        for (column, entry) in row.iter().enumerate() {
            // Stay below the row during decoding, so interrupting a slow input
            // leaves the cursor clear of images already emitted.
            let image = if entry.candidate() && budget.begin(bytes) {
                preview::load(&entry.path, width, height, cache).unwrap_or_else(|_| {
                    artwork::render(artwork::Icon::Error, width, height, style.color)
                })
            } else {
                artwork::render(entry.artwork(), width, height, style.color)
            };
            let left = column * tile + (tile - image_cols) / 2 + 1;
            write!(out, "\x1b[{}A\x1b[{}G", ROWS + 1, left)?;
            kitty::write(out, &image, image_cols, ROWS)?;
            budget.placed(bytes);
            write!(out, "\r\x1b[{}B", ROWS + 1)?;
            out.flush()?;
        }
        out.write_all(b"\x1b[1A")?;
        let labels: Vec<_> = row.iter().map(|e| wrap(&style.name(e), tile - 2)).collect();
        let lines = labels.iter().map(Vec::len).max().unwrap_or(0);
        for line in 0..lines {
            for (entry, label) in row.iter().zip(&labels) {
                let text = label.get(line).map(String::as_str).unwrap_or("");
                let left = (tile - text.width()) / 2;
                write!(out, "{:left$}", "")?;
                style.write_label(out, entry, text)?;
                write!(out, "{:padding$}", "", padding = tile - text.width() - left)?;
            }
            out.write_all(b"\r\n")?;
        }
        out.write_all(b"\r\n")?;
        out.flush()?;
    }
    Ok(())
}
