use crate::{entry::Entry, style::Style};
use std::io::{self, Write};
use unicode_width::UnicodeWidthStr;

// Maxima for each row-wise column. Search is bounded even on huge reported TTYs.
// Store widths, not another full copy of every escaped filename.
fn plan(widths: &[usize], terminal_cols: usize) -> Vec<usize> {
    let available = terminal_cols.saturating_sub(1);
    let min = widths.iter().copied().min().unwrap_or(0);
    let max_columns = ((available + 2) / (min + 2)).min(widths.len()).min(64);
    for columns in (2..=max_columns).rev() {
        let mut cells = vec![0; columns];
        let mut used = 2 * (columns - 1);
        for (i, &width) in widths.iter().enumerate() {
            let cell = &mut cells[i % columns];
            if width > *cell {
                used += width - *cell;
                *cell = width;
            }
            if used > available {
                break;
            }
        }
        if used <= available {
            return cells;
        }
    }
    vec![widths.iter().copied().max().unwrap_or(0)]
}

pub struct Plan {
    widths: Vec<usize>,
    cells: Vec<usize>,
}

impl Plan {
    pub fn new(entries: &[Entry], terminal_cols: usize, style: &Style) -> Self {
        let widths: Vec<_> = entries.iter().map(|e| style.label(e).width()).collect();
        let cells = plan(&widths, terminal_cols);
        Self { widths, cells }
    }

    pub fn write(&self, out: &mut impl Write, entries: &[Entry], style: &Style) -> io::Result<()> {
        for (row, row_widths) in entries
            .chunks(self.cells.len())
            .zip(self.widths.chunks(self.cells.len()))
        {
            for (i, entry) in row.iter().enumerate() {
                style.write_name(out, entry)?;
                if i + 1 < row.len() {
                    write!(
                        out,
                        "{:padding$}",
                        "",
                        padding = self.cells[i] - row_widths[i] + 2
                    )?;
                }
            }
            writeln!(out)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::Kind;

    fn output(names: &[&str], cols: usize) -> String {
        let entries: Vec<_> = names
            .iter()
            .map(|name| Entry {
                path: name.into(),
                name: (*name).into(),
                kind: Kind::File,
                metadata: None,
                executable: false,
            })
            .collect();
        let mut out = Vec::new();
        Plan::new(&entries, cols, &Style::default())
            .write(&mut out, &entries, &Style::default())
            .unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn aligned_row_order_and_ragged_last_row() {
        assert_eq!(
            output(&["a", "bb", "ccc", "d", "ee"], 10),
            "a    bb\nccc  d\nee\n"
        );
        assert_eq!(
            output(&["a", "bb", "ccc", "d", "ee"], 11),
            "a  bb  ccc\nd  ee\n"
        );
    }

    #[test]
    fn preserves_unicode_controls_and_long_names() {
        let out = output(&["桃", "e\u{301}", "👩‍💻", "x\n"], 9);
        assert_eq!(out, "桃  e\u{301}\n👩‍💻  x\\n\n");
        assert!(out.lines().all(|line| line.width() < 9));
        assert_eq!(
            output(&["very long filename", "a"], 8),
            "very long filename\na\n"
        );
        assert_eq!(output(&[], 1), "");
        assert_eq!(output(&["a", "b"], 1), "a\nb\n");
    }

    #[test]
    fn leaves_right_edge_free() {
        assert_eq!(output(&["aa", "bb"], 6), "aa\nbb\n");
        assert_eq!(output(&["aa", "bb"], 7), "aa  bb\n");
        assert!(plan(&[1; 1000], usize::from(u16::MAX)).len() <= 64);
    }
}
