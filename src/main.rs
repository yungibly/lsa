mod artwork;
mod cache;
mod cli;
mod columns;
mod display;
mod entry;
mod filetype;
mod grid;
mod kitty;
mod layout;
mod metadata;
mod preview;
mod sort;
mod style;
mod svg;
mod terminal;

use std::{
    io::{self, BufWriter, Write},
    path::Path,
};

fn main() -> std::process::ExitCode {
    let opts = match cli::parse(std::env::args_os().skip(1)) {
        Ok(opts) => opts,
        Err(e) => {
            let _ = writeln!(io::stderr().lock(), "lsa: {e}\nTry lsa --help");
            return 2.into();
        }
    };
    let mut out = BufWriter::with_capacity(16 * 1024, io::stdout().lock());
    let result = run(&opts, &mut out).and_then(|code| {
        out.flush()?;
        Ok(code)
    });
    match result {
        Ok(code) => code.into(),
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => 0.into(),
        Err(e) => {
            let _ = writeln!(io::stderr().lock(), "lsa: output: {e}");
            1.into()
        }
    }
}

fn run<W: Write>(opts: &cli::Options, out: &mut W) -> io::Result<u8> {
    if opts.help {
        write!(out, "{}", cli::HELP)?;
        return Ok(0);
    }
    if opts.version {
        writeln!(out, "lsa {}", env!("CARGO_PKG_VERSION"))?;
        return Ok(0);
    }
    if opts.clear_cache {
        return match cache::clear(opts.cache_dir.as_deref().unwrap()) {
            Ok(()) => {
                writeln!(out, "Thumbnail cache cleared.")?;
                Ok(0)
            }
            Err(error) => {
                let _ = writeln!(io::stderr().lock(), "lsa: cache clear: {error}");
                Ok(1)
            }
        };
    }
    let term = terminal::Terminal::detect(opts);
    let style = style::Style::detect(opts, &term);
    if let Some(notice) = opts.layout_notice() {
        let _ = writeln!(io::stderr().lock(), "lsa: {notice}");
    }
    if opts.diagnose {
        writeln!(
            out,
            "stdout_tty={}\nterminal={}\nversion={}\ngeometry={}x{}\ncell_pixels={}x{}{}\nkitty={}\nreason={}\npreview_attempts={}\nimage_output_limit={}\nimage_placement_limit={}\ninput_limit={}\npixel_limit={}\ndecoder_alloc_limit={} (best effort)\ncache={}\nterminal_validation=see docs/compatibility.md",
            term.tty,
            term.name,
            term.version,
            term.cols,
            term.rows,
            term.cell_width,
            term.cell_height,
            if term.estimated_cell {
                " (estimated)"
            } else {
                ""
            },
            term.kitty,
            term.reason,
            opts.preview_limit,
            preview::OUTPUT_LIMIT,
            preview::PLACEMENT_LIMIT,
            preview::INPUT_LIMIT,
            preview::PIXEL_LIMIT,
            preview::ALLOC_LIMIT,
            if opts.cache_path().is_some() {
                "opt-in (not accessed by diagnose)"
            } else {
                "off"
            }
        )?;
        if let Some(path) = opts.cache_path() {
            writeln!(
                out,
                "cache_directory={}\ncache_namespace={}\ncache_slots={}\ncache_file_bytes_limit={}",
                display::escape(path.as_os_str()),
                cache::NAMESPACE,
                cache::SLOTS,
                cache::STORAGE_LIMIT
            )?;
        }
    }
    let mut failed = false;
    let mut printed = false;
    let mut budget = preview::Budget::new(opts.preview_limit);
    let mut cache = cache::Cache::new(opts.cache_path());
    let mut operands = Vec::new();
    let mut explained = opts.layout_notice().is_some();
    // `dir` holds the entries (empty for file operands); `path` names the
    // listing in diagnostics.
    let mut render = |out: &mut W,
                      dir: &Path,
                      entries: &[entry::Entry],
                      path: &std::ffi::OsStr|
     -> io::Result<()> {
        let choice = layout::choose(entries, &term, opts);
        // An explicit grid request this terminal cannot show gets one notice.
        if choice.unavailable
            && !opts.diagnose
            && !explained
            && let Some(request) = opts.grid_request()
        {
            explained = true;
            let hint = if term.kitty {
                ""
            } else {
                "; --protocol=kitty forces Kitty graphics"
            };
            let _ = writeln!(
                io::stderr().lock(),
                "lsa: {request} ignored: {}{hint}",
                choice.reason
            );
        }
        if opts.diagnose {
            if let layout::Layout::Grid(g) = choice.layout {
                writeln!(
                    out,
                    "\ngrid_frame={}x{} cells\ngrid_pixels={}x{}\ngrid_columns={}",
                    g.image_cols, g.image_rows, g.width, g.height, g.columns
                )?;
            }
            return writeln!(
                out,
                "\npath={}\nentries={}\npreview_candidates={}\nlayout={}\nlayout_reason={}\ncolor={}\nicons={:?}",
                display::escape(path),
                entries.len(),
                entries.iter().filter(|e| e.candidate()).count(),
                choice.layout.name(),
                choice.reason,
                style.color,
                style.icons
            );
        }
        match choice.layout {
            layout::Layout::Grid(geometry) => grid::write(
                out,
                dir,
                entries,
                geometry,
                &style,
                preview::Previews {
                    term: &term,
                    budget: &mut budget,
                    cache: &mut cache,
                },
            ),
            layout::Layout::Columns => if opts.across {
                columns::Plan::across(entries, term.cols, &style)
            } else {
                columns::Plan::new(entries, term.cols, &style)
            }
            .write(out, dir, entries, &style),
            layout::Layout::Long => metadata::write(
                out,
                dir,
                entries,
                opts,
                &style,
                Some(preview::Previews {
                    term: &term,
                    budget: &mut budget,
                    cache: &mut cache,
                }),
            ),
            layout::Layout::Lines => entry::write_text(out, dir, entries, &style),
        }
    };
    for path in &opts.paths {
        let mut listing = entry::list(path, opts);
        if listing.directory {
            let long = layout::choose(&listing.entries, &term, opts).layout == layout::Layout::Long;
            // Dangling links matter where they are styled or could be previewed.
            entry::prepare(
                &mut listing,
                opts,
                long,
                !opts.diagnose && style.needs_mode(),
                style.needs_mode() || term.kitty,
            );
        }
        for error in &listing.errors {
            let _ = writeln!(io::stderr().lock(), "lsa: {error}");
            failed = true;
        }
        if !listing.valid {
            continue;
        }
        if !listing.directory {
            operands.extend(listing.entries);
            continue;
        }
        if !operands.is_empty() {
            render(
                out,
                Path::new(""),
                &operands,
                if operands.len() == 1 {
                    &operands[0].name
                } else {
                    std::ffi::OsStr::new("(file operands)")
                },
            )?;
            operands.clear();
            printed = true;
        }
        if opts.paths.len() > 1 && !opts.diagnose {
            if printed {
                writeln!(out)?;
            }
            style.paint(out, "1;34", &display::escape(path.as_os_str()))?;
            writeln!(out, ":")?;
        }
        render(out, &listing.dir, &listing.entries, path.as_os_str())?;
        printed = true;
    }
    if !operands.is_empty() {
        render(
            out,
            Path::new(""),
            &operands,
            if operands.len() == 1 {
                &operands[0].name
            } else {
                std::ffi::OsStr::new("(file operands)")
            },
        )?;
    }
    if opts.cache_stats {
        out.flush()?;
        let stats = &cache.stats;
        let _ = writeln!(
            io::stderr().lock(),
            "lsa: cache: {} hits, {} misses, {} writes, {} errors",
            stats.hits,
            stats.misses,
            stats.writes,
            stats.errors
        );
    }
    Ok(u8::from(failed))
}
