use crate::{
    display::wrap,
    entry::Entry,
    kitty,
    preview::{self, Budget},
    terminal::Terminal,
};
use std::io::{self, Write};
use unicode_width::UnicodeWidthStr;

const ROWS: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    pub columns: usize,
    pub tile: usize,
    pub width: u32,
    pub height: u32,
    pub bytes: usize,
}

impl Geometry {
    pub fn new(term: &Terminal) -> Option<Self> {
        if term.cols < 12 || term.rows < 8 {
            return None;
        }
        // Reserve the rightmost cell; bound row scratch space on very wide TTYs.
        let available = (term.cols - 1).min(240);
        let columns = (available / 24).clamp(1, 10);
        let tile = available / columns;
        let width = (tile - 2) as f64 * f64::from(term.cell_width);
        let height = ROWS as f64 * f64::from(term.cell_height);
        let scale = (320.0 / width).min(240.0 / height).min(1.0);
        let width = (width * scale).round().max(1.0) as u32;
        let height = (height * scale).round().max(1.0) as u32;
        Some(Self {
            columns,
            tile,
            width,
            height,
            bytes: kitty::byte_len(width, height, tile - 2, ROWS),
        })
    }

    pub fn minimum_rows(&self, entries: usize) -> usize {
        entries.div_ceil(self.columns).saturating_mul(ROWS + 2)
    }

    pub fn output_rows(&self, entries: &[Entry]) -> usize {
        entries
            .chunks(self.columns)
            .map(|row| {
                let lines = row
                    .iter()
                    .map(|e| wrap(&e.label(), self.tile - 2).len())
                    .max()
                    .unwrap_or(0);
                ROWS + lines + 1
            })
            .sum()
    }
}

pub fn write(
    out: &mut impl Write,
    entries: &[Entry],
    geometry: Geometry,
    budget: &mut Budget,
) -> io::Result<()> {
    let Geometry {
        columns,
        tile,
        width,
        height,
        bytes,
    } = geometry;
    let image_cols = tile - 2;
    for row in entries.chunks(columns) {
        // Reserve the image area AND a label line before placing images. This
        // scrolls first, so a placement never extends below the visible screen.
        for _ in 0..=ROWS {
            out.write_all(b"\r\n")?;
        }
        out.flush()?;
        for (column, entry) in row.iter().enumerate() {
            // Stay below the row during decoding, so interrupting a slow input
            // leaves the cursor clear of images already emitted.
            let mut image = None;
            let placeholder = if !entry.candidate() {
                Some(entry.kind.label())
            } else if !budget.begin(bytes) {
                Some("[limit]")
            } else {
                match preview::load(&entry.path, width, height) {
                    Ok(thumb) => {
                        image = Some(thumb);
                        None
                    }
                    Err(_) => {
                        budget.failed += 1;
                        Some("[no preview]")
                    }
                }
            };
            write!(out, "\x1b[{}A\x1b[{}G", ROWS + 1, column * tile + 1)?;
            if let Some(image) = image {
                kitty::write(out, &image, image_cols, ROWS)?;
                budget.bytes_left -= bytes;
                budget.shown += 1;
            }
            if let Some(label) = placeholder {
                write!(out, "{}", wrap(label, image_cols)[0])?;
            }
            write!(out, "\r\x1b[{}B", ROWS + 1)?;
            out.flush()?;
        }
        out.write_all(b"\x1b[1A")?;
        let labels: Vec<_> = row.iter().map(|e| wrap(&e.label(), image_cols)).collect();
        let lines = labels.iter().map(Vec::len).max().unwrap_or(0);
        for line in 0..lines {
            for label in &labels {
                let text = label.get(line).map(String::as_str).unwrap_or("");
                write!(out, "{text}{}", " ".repeat(tile - text.width()))?;
            }
            out.write_all(b"\r\n")?;
        }
        out.write_all(b"\r\n")?;
        out.flush()?;
    }
    Ok(())
}
