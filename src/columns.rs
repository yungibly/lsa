use crate::{entry::Entry, style::Style};
use std::{
    io::{self, Write},
    path::Path,
};

// Column widths for the widest arrangement that fits, leaving the rightmost
// cell free. The search is bounded even on huge reported TTYs, and plans keep
// widths rather than another copy of every escaped filename.
//
// `ls -C` fills columns top to bottom (entry i sits in column i / rows); `-x`
// fills rows left to right (column i % columns).
fn plan(widths: &[usize], terminal_cols: usize, across: bool) -> (Vec<usize>, usize) {
    let available = terminal_cols.saturating_sub(1);
    let min = widths.iter().copied().min().unwrap_or(0);
    let max_columns = ((available + 2) / (min + 2)).min(widths.len()).min(64);
    for columns in (2..=max_columns).rev() {
        let rows = widths.len().div_ceil(columns);
        // Down columns, fewer may be needed; that count is tried on its own.
        if !across && widths.len().div_ceil(rows) != columns {
            continue;
        }
        let mut cells = vec![0; columns];
        let mut used = 2 * (columns - 1);
        for (i, &width) in widths.iter().enumerate() {
            let cell = &mut cells[if across { i % columns } else { i / rows }];
            if width > *cell {
                used += width - *cell;
                *cell = width;
            }
            if used > available {
                break;
            }
        }
        if used <= available {
            return (cells, rows);
        }
    }
    (
        vec![widths.iter().copied().max().unwrap_or(0)],
        widths.len(),
    )
}

pub struct Plan {
    widths: Vec<usize>,
    cells: Vec<usize>,
    rows: usize,
    across: bool,
}

impl Plan {
    /// Compact columns filled down, like `ls -C`.
    pub fn new(entries: &[Entry], terminal_cols: usize, style: &Style) -> Self {
        Self::with(entries, terminal_cols, style, false)
    }

    /// Compact columns filled across rows, like `ls -x`; also the order a
    /// gallery continues in once its image budget runs out.
    pub fn across(entries: &[Entry], terminal_cols: usize, style: &Style) -> Self {
        Self::with(entries, terminal_cols, style, true)
    }

    fn with(entries: &[Entry], terminal_cols: usize, style: &Style, across: bool) -> Self {
        let widths: Vec<_> = entries.iter().map(|e| style.label_width(e)).collect();
        let (cells, rows) = plan(&widths, terminal_cols, across);
        Self {
            widths,
            cells,
            rows,
            across,
        }
    }

    pub fn write(
        &self,
        out: &mut impl Write,
        dir: &Path,
        entries: &[Entry],
        style: &Style,
    ) -> io::Result<()> {
        let columns = self.cells.len();
        for row in 0..self.rows {
            for column in 0..columns {
                let (index, next) = if self.across {
                    (row * columns + column, row * columns + column + 1)
                } else {
                    (column * self.rows + row, (column + 1) * self.rows + row)
                };
                let Some(entry) = entries.get(index) else {
                    break;
                };
                style.write_name(out, dir, entry)?;
                if column + 1 < columns && next < entries.len() {
                    let padding = self.cells[column] - self.widths[index] + 2;
                    write!(out, "{:padding$}", "")?;
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
    use unicode_width::UnicodeWidthStr;

    fn output(names: &[&str], cols: usize, across: bool) -> String {
        let entries: Vec<_> = names
            .iter()
            .map(|name| Entry::new(name, Kind::File))
            .collect();
        let style = Style::default();
        let plan = if across {
            Plan::across(&entries, cols, &style)
        } else {
            Plan::new(&entries, cols, &style)
        };
        let mut out = Vec::new();
        plan.write(&mut out, Path::new(""), &entries, &style)
            .unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn columns_fill_down_and_rows_fill_across() {
        let names = ["a", "bb", "ccc", "d", "ee"];
        assert_eq!(output(&names, 10, true), "a    bb\nccc  d\nee\n");
        assert_eq!(output(&names, 11, true), "a  bb  ccc\nd  ee\n");
        assert_eq!(output(&names, 10, false), "a    d\nbb   ee\nccc\n");
        assert_eq!(output(&names, 11, false), "a    d\nbb   ee\nccc\n");
        assert_eq!(output(&names, 13, false), "a   ccc  ee\nbb  d\n");
        // Sorted names read down each column, with a ragged last column.
        let fruit = [
            "apple", "banana", "cherry", "date", "elder", "fig", "grape", "honeydew", "kiwi",
            "lemon",
        ];
        assert_eq!(
            output(&fruit, 40, false),
            "apple   cherry  elder  grape     kiwi\nbanana  date    fig    honeydew  lemon\n"
        );
        assert_eq!(
            output(&fruit, 30, false),
            "apple   elder     kiwi\nbanana  fig       lemon\ncherry  grape\ndate    honeydew\n"
        );
    }

    #[test]
    fn preserves_unicode_controls_and_long_names() {
        for across in [false, true] {
            let out = output(&["桃", "e\u{301}", "👩‍💻", "x\n"], 9, across);
            assert!(out.lines().all(|line| line.width() < 9));
            for name in ["桃", "e\u{301}", "👩‍💻", "x\\n"] {
                assert_eq!(out.matches(name).count(), 1, "{out}");
            }
            assert_eq!(
                output(&["very long filename", "a"], 8, across),
                "very long filename\na\n"
            );
            assert_eq!(output(&[], 1, across), "");
            assert_eq!(output(&["a", "b"], 1, across), "a\nb\n");
        }
        assert_eq!(
            output(&["桃", "e\u{301}", "👩‍💻", "x\n"], 9, true),
            "桃  e\u{301}\n👩‍💻  x\\n\n"
        );
        assert_eq!(
            output(&["桃", "e\u{301}", "👩‍💻", "x\n"], 9, false),
            "桃  👩‍💻\ne\u{301}   x\\n\n"
        );
    }

    #[test]
    fn leaves_right_edge_free_and_bounds_search() {
        for across in [false, true] {
            assert_eq!(output(&["aa", "bb"], 6, across), "aa\nbb\n");
            assert_eq!(output(&["aa", "bb"], 7, across), "aa  bb\n");
            assert!(plan(&[1; 1000], usize::from(u16::MAX), across).0.len() <= 64);
        }
        // Every entry appears exactly once, in order, for many shapes.
        for n in 0..40 {
            let names: Vec<String> = (0..n).map(|i| "x".repeat(i % 7 + 1)).collect();
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            for cols in [1, 9, 20, 43, 80] {
                for across in [false, true] {
                    let out = output(&names, cols, across);
                    assert_eq!(out.split_whitespace().count(), n, "{n} {cols} {across}");
                    assert!(out.lines().all(|line| line.width() < cols.max(8)));
                }
            }
        }
    }
}
