use crate::{
    artwork::Icon,
    columns,
    display::wrap,
    entry::Entry,
    kitty, pool,
    preview::{self, Previews},
    style::Style,
    terminal::Terminal,
};
use std::{
    io::{self, Write},
    path::Path,
    sync::Mutex,
};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    pub columns: usize,
    pub tile: usize,
    pub image_cols: usize,
    pub image_rows: usize,
    pub width: u32,
    pub height: u32,
    pub bytes: usize,
}

impl Geometry {
    pub fn new(term: &Terminal, rows: usize) -> Option<Self> {
        if term.cols < 12 || term.rows < 6 {
            return None;
        }
        // Reserve the rightmost cell; bound row scratch space on very wide TTYs.
        let available = (term.cols - 1).min(240);
        let image_rows = rows.clamp(1, 12).min(term.rows - 3);
        let preferred_cols = (14 * image_rows).div_ceil(3);
        let columns = (available / (preferred_cols + 10)).clamp(1, 16);
        let tile = available / columns;
        let image_cols = (tile - 2).min(preferred_cols);
        let width = image_cols as f64 * f64::from(term.cell_width);
        let height = image_rows as f64 * f64::from(term.cell_height);
        let scale = (320.0 / width).min(240.0 / height).min(1.0);
        let width = (width * scale).round().max(1.0) as u32;
        let height = (height * scale).round().max(1.0) as u32;
        Some(Self {
            columns,
            tile,
            image_cols,
            image_rows,
            width,
            height,
            bytes: kitty::byte_len(width, height, image_cols, image_rows),
        })
    }
}

pub fn write(
    out: &mut impl Write,
    dir: &Path,
    entries: &[Entry],
    geometry: Geometry,
    style: &Style,
    previews: Previews<'_, '_>,
) -> io::Result<()> {
    let Previews {
        term,
        budget,
        cache,
        art,
    } = previews;
    let Geometry {
        columns,
        tile,
        image_cols,
        image_rows,
        width,
        height,
        bytes,
    } = geometry;
    // Settle every budget decision first, exactly as drawing in order would:
    // a row draws only if all of its tiles fit, and candidates spend attempts
    // left to right. Workers can then decode ahead without changing output.
    let mut drawn = 0;
    let mut attempts = Vec::new();
    let mut jobs = Vec::new();
    for row in entries.chunks(columns) {
        if budget.attempts_left == 0
            || bytes * row.len() > budget.bytes_left
            || row.len() > budget.placements_left
        {
            break;
        }
        for entry in row {
            let attempt = entry.candidate() && budget.begin(bytes);
            if attempt {
                jobs.push(preview::Job {
                    path: entry.path(dir),
                    width,
                    height,
                });
            }
            attempts.push(attempt);
            budget.placed(bytes);
        }
        drawn += row.len();
    }
    let cache = Mutex::new(cache);
    preview::decoding(&jobs, || {
        pool::ordered(
            &jobs,
            pool::workers(),
            |job| preview::decode(job, &cache),
            |results| -> io::Result<()> {
                for (row_index, row) in entries[..drawn].chunks(columns).enumerate() {
                    // Reserve the image area AND a label line before placing images.
                    // This scrolls first, so a placement never extends below the
                    // visible screen.
                    for _ in 0..=image_rows {
                        out.write_all(b"\r\n")?;
                    }
                    out.flush()?;
                    for (column, entry) in row.iter().enumerate() {
                        // Stay below the row while waiting for a decode, so
                        // interrupting a slow input leaves the cursor clear of images
                        // already emitted.
                        let decoded;
                        let image = if attempts[row_index * columns + column] {
                            match results.next() {
                                Some(Ok(payload)) => {
                                    decoded = payload;
                                    &decoded
                                }
                                _ => art.get(Icon::Error, width, height, style.color),
                            }
                        } else {
                            art.get(entry.artwork(), width, height, style.color)
                        };
                        let left = column * tile + (tile - image_cols) / 2 + 1;
                        write!(out, "\x1b[{}A\x1b[{}G", image_rows + 1, left)?;
                        image.write(out, image_cols, image_rows)?;
                        write!(out, "\r\x1b[{}B", image_rows + 1)?;
                        out.flush()?;
                    }
                    out.write_all(b"\x1b[1A")?;
                    let labels: Vec<_> =
                        row.iter().map(|e| wrap(&style.name(e), tile - 2)).collect();
                    let lines = labels.iter().map(Vec::len).max().unwrap_or(0);
                    for line in 0..lines {
                        for (entry, label) in row.iter().zip(&labels) {
                            let text = label.get(line).map(String::as_str).unwrap_or("");
                            let left = (tile - text.width()) / 2;
                            write!(out, "{:left$}", "")?;
                            style.write_label(out, dir, entry, text)?;
                            write!(out, "{:padding$}", "", padding = tile - text.width() - left)?;
                        }
                        out.write_all(b"\r\n")?;
                    }
                    out.write_all(b"\r\n")?;
                    out.flush()?;
                }
                // Once a budget is spent, remaining names continue the gallery's
                // across-the-row order as compact text.
                let remaining = &entries[drawn..];
                if remaining.is_empty() {
                    return Ok(());
                }
                columns::Plan::across(remaining, term.cols, style).write(out, dir, remaining, style)
            },
        )
    })
}
