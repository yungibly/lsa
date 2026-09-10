use crate::{
    cli::{Options, Protocol},
    entry::Entry,
    grid::Geometry,
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
        layout: if opts.columns {
            Layout::Columns
        } else {
            Layout::Long
        },
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
    if opts.columns {
        return text("compact columns requested");
    }
    if opts.no_images || opts.protocol == Protocol::None {
        return text("images disabled");
    }
    if !term.kitty {
        return text(term.reason);
    }
    if opts.preview_limit == 0 {
        return text("preview budget disabled");
    }
    let Some(geometry) = Geometry::new(term, opts.thumbnail_size.unwrap_or(3)) else {
        return text("terminal too small for grid");
    };
    let candidates = entries.iter().filter(|e| e.candidate()).count();
    if opts.grid {
        return Choice {
            layout: Layout::Grid(geometry),
            reason: "grid requested",
        };
    }
    if candidates == 0 {
        return text("no preview candidates");
    }
    if candidates >= entries.len().div_ceil(2) || entries.len() <= geometry.columns {
        Choice {
            layout: Layout::Grid(geometry),
            reason: "image-heavy or small mixed listing; bounded inline previews",
        }
    } else {
        text("sparse images in a mixed listing")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cli, entry::Kind};
    fn term() -> Terminal {
        Terminal {
            tty: true,
            kitty: true,
            reason: "test",
            name: "ghostty".into(),
            version: "".into(),
            cols: 122,
            rows: 40,
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
                    executable: false,
                }
            })
            .collect()
    }
    fn choice(images: usize, files: usize, args: &[&str]) -> Choice {
        choose(
            &entries(images, files),
            &term(),
            &cli::parse(args.iter().map(Into::into)).unwrap(),
        )
    }
    #[test]
    fn previews_single_images_small_mixed_lists_and_tall_galleries() {
        for (images, files) in [(1, 0), (1, 4), (3, 0), (4, 4), (1000, 0)] {
            assert!(matches!(choice(images, files, &[]).layout, Layout::Grid(_)));
        }
        for (images, files) in [(0, 0), (0, 1000), (1, 5), (4, 5), (4, 1000)] {
            assert_eq!(choice(images, files, &[]).layout, Layout::Long);
        }
        assert!(matches!(
            choice(1, 1000, &["--grid"]).layout,
            Layout::Grid(_)
        ));
    }
    #[test]
    fn explicit_text_and_zero_budget_never_decode() {
        for (flag, layout) in [
            ("-1", Layout::Lines),
            ("-C", Layout::Columns),
            ("-l", Layout::Long),
            ("--no-images", Layout::Long),
            ("--protocol=none", Layout::Long),
            ("--preview-limit=0", Layout::Long),
        ] {
            for args in [[flag, "--grid"], ["--grid", flag]] {
                assert_eq!(choice(4, 0, &args).layout, layout);
            }
        }
        let mut term = term();
        let opts = cli::parse(["--grid".into(), "--protocol=kitty".into()]).unwrap();
        term.tty = false;
        assert_eq!(choose(&entries(4, 0), &term, &opts).layout, Layout::Lines);
        term.tty = true;
        term.kitty = false;
        assert_eq!(choose(&entries(4, 0), &term, &opts).layout, Layout::Long);
    }
}
