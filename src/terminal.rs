use crate::{
    cli::{Options, Protocol},
    display::escape,
};
use std::{
    env,
    io::{self, IsTerminal},
};

pub struct Terminal {
    pub tty: bool,
    pub kitty: bool,
    pub reason: &'static str,
    pub name: String,
    pub version: String,
    pub cols: usize,
    pub rows: usize,
    pub cell_width: u32,
    pub cell_height: u32,
    pub estimated_cell: bool,
}

impl Terminal {
    pub fn detect(opts: &Options) -> Self {
        let tty = io::stdout().is_terminal();
        let name = env::var_os("TERM_PROGRAM").unwrap_or_default();
        let term = env::var_os("TERM").unwrap_or_default();
        let mux = ["TMUX", "STY", "ZELLIJ"]
            .iter()
            .any(|key| env::var_os(key).is_some())
            || term.as_encoded_bytes().starts_with(b"screen")
            || term.as_encoded_bytes().starts_with(b"tmux");
        let known = term != "dumb"
            && (name == "ghostty"
                || name == "kitty"
                || term == "xterm-kitty"
                || term == "xterm-ghostty");
        let (kitty, reason) = select(tty, opts, mux, known);
        let mut result = Self {
            tty,
            kitty,
            reason,
            name: escape(&name),
            version: escape(&env::var_os("TERM_PROGRAM_VERSION").unwrap_or_default()),
            cols: 80,
            rows: 24,
            cell_width: 8,
            cell_height: 16,
            estimated_cell: true,
        };
        if tty {
            let mut size = std::mem::MaybeUninit::<libc::winsize>::zeroed();
            // SAFETY: ioctl writes a winsize to a valid pointer; no terminal mode changes.
            if unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, size.as_mut_ptr()) } == 0
            {
                let size = unsafe { size.assume_init() };
                if size.ws_col > 0 {
                    result.cols = usize::from(size.ws_col);
                }
                if size.ws_row > 0 {
                    result.rows = usize::from(size.ws_row);
                }
                if size.ws_col > 0
                    && size.ws_row > 0
                    && size.ws_xpixel >= size.ws_col
                    && size.ws_ypixel >= size.ws_row
                {
                    result.cell_width = u32::from(size.ws_xpixel) / u32::from(size.ws_col);
                    result.cell_height = u32::from(size.ws_ypixel) / u32::from(size.ws_row);
                    result.estimated_cell = false;
                }
            }
        }
        result
    }
}

fn select(tty: bool, opts: &Options, mux: bool, known: bool) -> (bool, &'static str) {
    if !tty {
        (false, "stdout is not a terminal")
    } else if opts.no_images || opts.one || opts.long || opts.protocol == Protocol::None {
        (false, "text requested")
    } else if opts.protocol == Protocol::Kitty {
        (true, "explicit Kitty override")
    } else if mux {
        (false, "multiplexer: graphics not auto-enabled")
    } else if known {
        (true, "Kitty/Ghostty environment hint")
    } else {
        (false, "unknown terminal")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conservative_detection() {
        let mut opts = Options::default();
        assert!(!select(false, &opts, false, true).0);
        assert!(!select(true, &opts, true, true).0);
        assert!(!select(true, &opts, false, false).0);
        assert!(select(true, &opts, false, true).0);
        opts.protocol = Protocol::Kitty;
        assert!(select(true, &opts, true, false).0);
        assert!(!select(false, &opts, false, true).0);
        opts.no_images = true;
        assert!(!select(true, &opts, false, true).0);
    }
}
