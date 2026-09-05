mod browser;
mod browser_terminal;
mod cache;
mod cli;
mod columns;
mod display;
mod entry;
mod grid;
mod kitty;
mod layout;
mod metadata;
mod preview;
mod terminal;

use std::io::{self, BufWriter, Write};

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

fn run(opts: &cli::Options, out: &mut impl Write) -> io::Result<u8> {
    if opts.help {
        write!(out, "{}", cli::HELP)?;
        return Ok(0);
    }
    if opts.version {
        writeln!(out, "lsa {}", env!("CARGO_PKG_VERSION"))?;
        return Ok(0);
    }
    if opts.browse {
        return browser::run(opts, out);
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
    if opts.diagnose {
        writeln!(
            out,
            "stdout_tty={}\nterminal={}\nversion={}\ngeometry={}x{}\ncell_pixels={}x{}{}\nkitty={}\nreason={}\npreview_attempts={}\nimage_output_limit={}\ninput_limit={}\npixel_limit={}\ndecoder_alloc_limit={} (best effort)\ncache={}\nterminal_validation=see docs/compatibility.md",
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
    for path in &opts.paths {
        let listing = entry::list(path, opts);
        for error in listing.errors {
            let _ = writeln!(io::stderr().lock(), "lsa: {error}");
            failed = true;
        }
        if !listing.valid {
            continue;
        }
        let choice = layout::choose(&listing.entries, &term, opts);
        if opts.diagnose {
            writeln!(
                out,
                "\npath={}\nentries={}\npreview_candidates={}\nlayout={}\nlayout_reason={}\nestimated_grid_rows={}",
                display::escape(path.as_os_str()),
                listing.entries.len(),
                listing.entries.iter().filter(|e| e.candidate()).count(),
                choice.layout.name(),
                choice.reason,
                grid::Geometry::new(&term)
                    .map(|g| g.output_rows(&listing.entries).to_string())
                    .unwrap_or_else(|| "unavailable".into())
            )?;
            continue;
        }
        if opts.paths.len() > 1 && listing.directory {
            if printed {
                writeln!(out)?;
            }
            writeln!(out, "{}:", display::escape(path.as_os_str()))?;
        }
        match choice.layout {
            layout::Layout::Grid(geometry) => {
                grid::write(out, &listing.entries, geometry, &mut budget, &mut cache)?
            }
            layout::Layout::Columns => columns::write(out, &listing.entries, term.cols)?,
            layout::Layout::Long | layout::Layout::Lines => {
                entry::write_text(out, &listing.entries, opts)?
            }
        }
        printed = true;
    }
    if budget.failed > 0 || budget.limited > 0 {
        out.flush()?;
        let _ = writeln!(
            io::stderr().lock(),
            "lsa: previews: {} shown, {} unavailable, {} limited; all entries listed",
            budget.shown,
            budget.failed,
            budget.limited
        );
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
