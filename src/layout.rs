use crate::{
    cli::{Options, Protocol},
    entry::Entry,
    grid::Geometry,
    preview::OUTPUT_LIMIT,
    terminal::Terminal,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Long,
    Lines,
    Columns,
    Grid(Geometry),
}
impl Layout {
    pub fn name(self) -> &'static str {
        match self {
            Self::Long => "long",
            Self::Lines => "lines",
            Self::Columns => "columns",
            Self::Grid(_) => "grid",
        }
    }
}

pub struct Choice {
    pub layout: Layout,
    pub reason: &'static str,
}

pub fn choose(entries: &[Entry], term: &Terminal, opts: &Options) -> Choice {
    let text = |reason| Choice {
        layout: Layout::Columns,
        reason,
    };
    if opts.long {
        return Choice {
            layout: Layout::Long,
            reason: "long listing requested",
        };
    }
    if !term.tty || opts.one {
        return Choice {
            layout: Layout::Lines,
            reason: if opts.one {
                "one entry per line requested"
            } else {
                "stdout is not a terminal"
            },
        };
    }
    if opts.no_images || opts.protocol == Protocol::None {
        return text("images disabled");
    }
    if !term.kitty {
        return text(term.reason);
    }
    let Some(geometry) = Geometry::new(term) else {
        return text("terminal too small for grid");
    };
    if opts.grid {
        return Choice {
            layout: Layout::Grid(geometry),
            reason: "grid requested",
        };
    }
    // Reject obviously tall listings without classifying contents or formatting
    // names. Candidate checks below use extensions/types only, never open files.
    let height = term.rows.saturating_sub(2);
    if geometry.minimum_rows(entries.len()) > height {
        return text("grid exceeds one screen");
    }
    let candidates = entries.iter().filter(|e| e.candidate()).count();
    if candidates < 4 {
        return text("fewer than four preview candidates");
    }
    if candidates < entries.len().div_ceil(2) {
        return text("fewer than half the entries are preview candidates");
    }
    if candidates > opts.preview_limit || candidates.saturating_mul(geometry.bytes) > OUTPUT_LIMIT {
        return text("grid exceeds preview budget");
    }
    if geometry.output_rows(entries) > height {
        return text("wrapped grid labels exceed one screen");
    }
    Choice {
        layout: Layout::Grid(geometry),
        reason: "image-heavy listing fits one screen and preview budget",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cli, entry::Kind};

    fn terminal(cols: usize, rows: usize) -> Terminal {
        Terminal {
            tty: true,
            kitty: true,
            reason: "test",
            name: "ghostty".into(),
            version: "1.3.1".into(),
            cols,
            rows,
            cell_width: 8,
            cell_height: 17,
            estimated_cell: false,
        }
    }
    fn entries(images: usize, files: usize) -> Vec<Entry> {
        (0..images + files)
            .map(|i| {
                let name = format!("{i:03}.{}", if i < images { "png" } else { "txt" });
                Entry {
                    path: (&name).into(),
                    name: name.into(),
                    kind: Kind::File,
                    metadata: None,
                }
            })
            .collect()
    }
    fn opts(args: &[&str]) -> Options {
        cli::parse(args.iter().map(Into::into)).unwrap()
    }
    fn is_grid(entries: &[Entry], term: &Terminal, args: &[&str]) -> bool {
        matches!(choose(entries, term, &opts(args)).layout, Layout::Grid(_))
    }

    #[test]
    fn image_count_and_fraction_boundaries() {
        let term = terminal(122, 40);
        assert!(is_grid(&entries(4, 0), &term, &[]));
        assert!(is_grid(&entries(4, 4), &term, &[]));
        for (images, files) in [(0, 0), (0, 10), (3, 0), (4, 5), (4, 1000)] {
            assert!(!is_grid(&entries(images, files), &term, &[]));
        }
        let mut mixed = entries(4, 0);
        mixed[0].kind = Kind::Directory;
        assert!(!is_grid(&mixed, &term, &[]));
        mixed[0].kind = Kind::Link;
        assert!(is_grid(&mixed, &term, &[]));
    }

    #[test]
    fn screen_height_includes_wrapped_labels_and_prompt_room() {
        let mut files = entries(4, 0);
        assert!(is_grid(&files, &terminal(80, 16), &[]));
        assert!(!is_grid(&files, &terminal(80, 15), &[]));
        files[0].name = "this filename wraps over two lines.png".into();
        assert!(!is_grid(&files, &terminal(80, 16), &[]));
        assert!(is_grid(&files, &terminal(80, 17), &[]));
        assert!(!is_grid(&files, &terminal(11, 40), &["--grid"]));
        assert!(!is_grid(&files, &terminal(122, 7), &["--grid"]));
    }

    #[test]
    fn auto_respects_budgets_but_explicit_grid_remains_available() {
        let files = entries(4, 0);
        let term = terminal(122, 40);
        assert!(!is_grid(&files, &term, &["--preview-limit=3"]));
        assert!(!is_grid(&files, &term, &["--preview-limit=0"]));
        assert!(is_grid(&files, &term, &["--grid", "--preview-limit=0"]));
        let mut big = terminal(122, 1000);
        big.cell_width = 16;
        big.cell_height = 48;
        assert!(!is_grid(&entries(64, 0), &big, &[]));
        assert!(is_grid(&entries(64, 0), &big, &["--grid"]));
    }

    #[test]
    fn text_overrides_and_redirection_win_in_any_order() {
        let files = entries(4, 0);
        let mut term = terminal(122, 40);
        for (flag, layout) in [
            ("-1", Layout::Lines),
            ("-l", Layout::Long),
            ("--no-images", Layout::Columns),
            ("--protocol=none", Layout::Columns),
        ] {
            for args in [[flag, "--grid"], ["--grid", flag]] {
                assert_eq!(choose(&files, &term, &opts(&args)).layout, layout);
            }
        }
        term.tty = false;
        assert_eq!(
            choose(&files, &term, &opts(&["--grid", "--protocol=kitty"])).layout,
            Layout::Lines
        );
        term.tty = true;
        term.kitty = false;
        assert_eq!(
            choose(&files, &term, &opts(&["--grid"])).layout,
            Layout::Columns
        );
    }
}
