mod cli;
mod display;
mod entry;
mod grid;
mod kitty;
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
    let term = terminal::Terminal::detect(opts);
    if opts.diagnose {
        writeln!(
            out,
            "stdout_tty={}\nterminal={}\nversion={}\ngeometry={}x{}\ncell_pixels={}x{}{}\nkitty={}\nreason={}\nlayout={}\npreview_attempts={}\nimage_output_limit={}\ninput_limit={}\npixel_limit={}\ndecoder_alloc_limit={} (best effort)\ncache=none\nterminal_validation=pending",
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
            if term.grid(opts) { "grid" } else { "text" },
            opts.preview_limit,
            preview::OUTPUT_LIMIT,
            preview::INPUT_LIMIT,
            preview::PIXEL_LIMIT,
            preview::ALLOC_LIMIT
        )?;
        return Ok(0);
    }
    let mut failed = false;
    let mut printed = false;
    let mut budget = preview::Budget::new(opts.preview_limit);
    for path in &opts.paths {
        let listing = entry::list(path, opts);
        for error in listing.errors {
            let _ = writeln!(io::stderr().lock(), "lsa: {error}");
            failed = true;
        }
        if !listing.valid {
            continue;
        }
        if opts.paths.len() > 1 && listing.directory {
            if printed {
                writeln!(out)?;
            }
            writeln!(out, "{}:", display::escape(path.as_os_str()))?;
        }
        if term.grid(opts) {
            grid::write(out, &listing.entries, &term, &mut budget)?;
        } else {
            entry::write_text(out, &listing.entries, opts)?;
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
    Ok(u8::from(failed))
}
