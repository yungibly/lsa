mod artwork;
mod cache;
mod cli;
mod columns;
mod display;
mod entry;
mod extended;
mod filetype;
mod grid;
#[cfg(target_os = "macos")]
mod imageio;
mod kitty;
mod layout;
mod metadata;
mod pool;
mod preview;
mod sort;
mod style;
mod svg;
mod terminal;

use std::{
    collections::HashSet,
    ffi::OsString,
    fs,
    io::{self, BufWriter, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

/// Only active ancestor directories retain pending child names. A deep tree
/// never consumes the call stack, and completed listings are released before
/// reading the next directory.
struct DirectoryFrame {
    path: PathBuf,
    children: std::vec::IntoIter<OsString>,
    identity: (u64, u64),
}

fn main() -> std::process::ExitCode {
    // A decoder panic on a preview worker becomes error artwork, like any
    // failed preview; keep it quiet. Other panics still report normally.
    let report = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if std::thread::current().name() != Some(pool::THREAD_NAME) {
            report(info);
        }
    }));
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
            "stdout_tty={}\nterminal={}\nversion={}\ngeometry={}x{}\ncell_pixels={}x{}{}\nkitty={}\nreason={}\npreview_attempts={}\nimage_output_limit={}\nimage_placement_limit={}\ninput_limit={}\npixel_limit={}\ndecoder_alloc_limit={} (best effort)\ndecode_workers={}\nsystem_decoder={}\ncache={}\nterminal_validation=see docs/compatibility.md",
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
            pool::workers(),
            if cfg!(target_os = "macos") {
                "ImageIO for JPEG, HEIF, AVIF, TIFF, JPEG XL, PSD and camera RAW"
            } else {
                "none"
            },
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
    let mut art = preview::Artwork::default();
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
                    art: &mut art,
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
                    art: &mut art,
                }),
            ),
            layout::Layout::Lines => entry::write_text(out, dir, entries, &style),
            layout::Layout::Commas => columns::write_commas(out, dir, entries, term.cols, &style),
        }
    };
    // Classification serves styling, layout and previews; plain pipes use none.
    let classify = term.tty || opts.diagnose || style.needs_mode();
    // Resolve every explicit operand before reading directory contents. Files
    // print together first, in the requested ordering, followed by directories.
    let mut files = entry::Listing::default();
    let mut directories = Vec::new();
    let mut report = |errors: &[String]| {
        for error in errors {
            let _ = writeln!(io::stderr().lock(), "lsa: {error}");
            failed = true;
        }
    };
    for path in &opts.paths {
        let listing = entry::operand(path, opts, classify);
        report(&listing.errors);
        if listing.directory {
            directories.extend(listing.entries);
        } else {
            files.entries.extend(listing.entries);
        }
    }
    if !files.entries.is_empty() {
        let long = layout::choose(&files.entries, &term, opts).layout == layout::Layout::Long;
        entry::prepare(
            &mut files,
            opts,
            long,
            !opts.diagnose && style.needs_mode(),
            style.needs_mode() || term.kitty,
        );
        report(&files.errors);
        render(
            out,
            Path::new(""),
            &files.entries,
            if files.entries.len() == 1 {
                &files.entries[0].name
            } else {
                std::ffi::OsStr::new("(file operands)")
            },
        )?;
        printed = true;
    }
    drop(files);
    entry::sort(&mut directories, opts);
    let mut directories = directories.into_iter();
    let mut frames: Vec<DirectoryFrame> = Vec::new();
    let mut ancestors = HashSet::new();
    'walk: loop {
        let path = loop {
            if let Some(frame) = frames.last_mut() {
                if let Some(name) = frame.children.next() {
                    break frame.path.join(name);
                }
                let finished = frames.pop().unwrap();
                ancestors.remove(&finished.identity);
            } else if let Some(directory) = directories.next() {
                break PathBuf::from(directory.name);
            } else {
                break 'walk;
            }
        };
        let identity = if opts.recursive {
            match fs::metadata(&path) {
                Ok(meta) => {
                    let identity = (meta.dev(), meta.ino());
                    if ancestors.contains(&identity) {
                        report(&[format!(
                            "{}: recursive directory cycle",
                            display::escape(path.as_os_str())
                        )]);
                        continue;
                    }
                    Some(identity)
                }
                Err(error) => {
                    report(&[format!("{}: {error}", display::escape(path.as_os_str()))]);
                    continue;
                }
            }
        } else {
            None
        };
        let mut listing = entry::read_directory(&path, opts, classify);
        if listing.valid {
            let long = layout::choose(&listing.entries, &term, opts).layout == layout::Layout::Long;
            entry::prepare(
                &mut listing,
                opts,
                long,
                !opts.diagnose && style.needs_mode(),
                style.needs_mode() || term.kitty,
            );
        }
        report(&listing.errors);
        if !listing.valid {
            continue;
        }
        if (opts.paths.len() > 1 || opts.recursive) && !opts.diagnose {
            if printed {
                writeln!(out)?;
            }
            style.paint(out, "1;34", &display::escape(path.as_os_str()))?;
            writeln!(out, ":")?;
        }
        render(out, &listing.dir, &listing.entries, path.as_os_str())?;
        printed = true;
        if let Some(identity) = identity {
            let children: Vec<_> = listing
                .entries
                .into_iter()
                .filter(|entry| {
                    entry.kind == entry::Kind::Directory && entry.name != "." && entry.name != ".."
                })
                .map(|entry| entry.name)
                .collect();
            if !children.is_empty() {
                ancestors.insert(identity);
                frames.push(DirectoryFrame {
                    path,
                    children: children.into_iter(),
                    identity,
                });
            }
        }
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
