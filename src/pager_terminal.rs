//! Pager-only terminal lifetime. Inline output never initializes this module.
use std::{
    fs::{File, OpenOptions},
    io::{self, IsTerminal, Write},
    mem::MaybeUninit,
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

const SIGNALS: [libc::c_int; 7] = [
    libc::SIGWINCH,
    libc::SIGINT,
    libc::SIGTERM,
    libc::SIGHUP,
    libc::SIGQUIT,
    libc::SIGTSTP,
    libc::SIGCONT,
];
static PENDING: AtomicU32 = AtomicU32::new(0);

extern "C" fn signal_received(signal: libc::c_int) {
    if let Some(i) = SIGNALS.iter().position(|s| *s == signal) {
        // No allocation, I/O, or errno changes in the signal handler.
        PENDING.fetch_or(1 << i, Ordering::Relaxed);
    }
}

struct Signals {
    mask: libc::sigset_t,
    waiting_mask: libc::sigset_t,
    actions: Vec<(libc::c_int, libc::sigaction)>,
}

impl Signals {
    fn install() -> io::Result<Self> {
        // Install before the optional preview worker starts. Block managed signals
        // in this thread during work (the worker inherits that mask), then
        // atomically unblock them in pselect: no check-then-sleep wakeup race,
        // polling timer, self-pipe, or work inside the handlers.
        unsafe {
            let mut set = MaybeUninit::<libc::sigset_t>::zeroed().assume_init();
            libc::sigemptyset(&mut set);
            for signal in SIGNALS {
                libc::sigaddset(&mut set, signal);
            }
            let mut mask = MaybeUninit::<libc::sigset_t>::uninit();
            let error = libc::pthread_sigmask(libc::SIG_BLOCK, &set, mask.as_mut_ptr());
            if error != 0 {
                return Err(io::Error::from_raw_os_error(error));
            }
            let mask = mask.assume_init();
            let mut guard = Self {
                mask,
                waiting_mask: mask,
                actions: Vec::with_capacity(SIGNALS.len()),
            };
            PENDING.store(0, Ordering::Relaxed);
            for signal in SIGNALS {
                libc::sigdelset(&mut guard.waiting_mask, signal);
                let mut action = MaybeUninit::<libc::sigaction>::zeroed().assume_init();
                action.sa_sigaction = signal_received as *const () as usize;
                libc::sigemptyset(&mut action.sa_mask);
                let mut old = MaybeUninit::<libc::sigaction>::uninit();
                if libc::sigaction(signal, &action, old.as_mut_ptr()) != 0 {
                    return Err(io::Error::last_os_error());
                }
                guard.actions.push((signal, old.assume_init()));
            }
            Ok(guard)
        }
    }
}

impl Drop for Signals {
    fn drop(&mut self) {
        // SAFETY: saved actions and mask were initialized by successful calls.
        unsafe {
            for (signal, action) in &self.actions {
                libc::sigaction(*signal, action, std::ptr::null_mut());
            }
            libc::pthread_sigmask(libc::SIG_SETMASK, &self.mask, std::ptr::null_mut());
        }
    }
}

pub enum Event {
    Input(usize),
    Redraw,
    Preview,
    Suspend,
    Exit(u8),
    Timeout,
}

pub struct Session<'a, W: Write> {
    pub out: &'a mut W,
    pub graphics: crate::pager_graphics::Screen,
    original: libc::termios,
    input: File,
    active: bool,
    signals: Signals,
}

pub fn validate() -> io::Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::other(
            "--page requires terminal stdin and stdout; use a normal listing for pipes",
        ));
    }
    if std::env::var_os("TERM").is_some_and(|term| term == "dumb") {
        return Err(io::Error::other(
            "--page requires a cursor-addressable terminal",
        ));
    }
    // Require the same foreground terminal for input, display, and restoration.
    // SAFETY: fstat writes into valid storage; tcgetpgrp/getpgrp take no pointers.
    unsafe {
        let mut input = MaybeUninit::<libc::stat>::uninit();
        let mut output = MaybeUninit::<libc::stat>::uninit();
        if libc::fstat(libc::STDIN_FILENO, input.as_mut_ptr()) != 0
            || libc::fstat(libc::STDOUT_FILENO, output.as_mut_ptr()) != 0
        {
            return Err(io::Error::last_os_error());
        }
        if input.assume_init().st_rdev != output.assume_init().st_rdev
            || libc::tcgetpgrp(libc::STDIN_FILENO) != libc::getpgrp()
        {
            return Err(io::Error::other(
                "--page requires stdin and stdout on the same foreground terminal",
            ));
        }
    }
    Ok(())
}

impl<'a, W: Write> Session<'a, W> {
    pub fn enter(out: &'a mut W) -> io::Result<Self> {
        validate()?;
        let mut original = MaybeUninit::<libc::termios>::uninit();
        // SAFETY: tcgetattr initializes original on success.
        if unsafe { libc::tcgetattr(libc::STDIN_FILENO, original.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // Own an independent nonblocking file description. Changing stdin flags
        // could also make stdout/the shell nonblocking when they share an open fd.
        let input = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open("/dev/tty")?;
        if input.as_raw_fd() as usize >= libc::FD_SETSIZE {
            return Err(io::Error::other("pager input exceeds select fd limit"));
        }
        let mut session = Self {
            out,
            graphics: crate::pager_graphics::Screen::default(),
            original: unsafe { original.assume_init() },
            input,
            active: false,
            signals: Signals::install()?,
        };
        session.activate()?;
        Ok(session)
    }

    fn activate(&mut self) -> io::Result<()> {
        let mut raw = self.original;
        // Keep signal characters (including Ctrl-C and Ctrl-Z) functional.
        // A nonblocking read handles input flushed between readiness and read.
        unsafe { libc::cfmakeraw(&mut raw) };
        raw.c_lflag |= libc::ISIG;
        raw.c_cc[libc::VMIN] = 1;
        raw.c_cc[libc::VTIME] = 0;
        set_attributes(&raw)?;
        self.active = true; // Drop must unwind even a partially written sequence.
        self.out.write_all(b"\x1b[?1049h\x1b[?25l\x1b[?2004h")?;
        self.out.flush()
    }

    pub fn restore(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }
        // Restore termios even if output/flush fails; retry in Drop on error.
        let cleanup = self.graphics.clear(self.out);
        let output = self
            .out
            .write_all(b"\x1b[0m\x1b[?2004l\x1b[?25h\x1b[?1049l")
            .and_then(|()| self.out.flush());
        let attributes = set_attributes(&self.original);
        if cleanup.is_ok() && output.is_ok() && attributes.is_ok() {
            self.active = false;
        }
        cleanup.and(output).and(attributes)
    }

    pub fn suspend(&mut self) -> io::Result<()> {
        self.restore()?;
        // SIGSTOP works even for an orphan process group; handlers/mask remain
        // installed while stopped. SIGCONT resumes here, then requests a redraw.
        if unsafe { libc::raise(libc::SIGSTOP) } != 0 {
            return Err(io::Error::last_os_error());
        }
        validate()?;
        self.activate()
    }

    pub fn wait(
        &self,
        buffer: &mut [u8],
        timeout: Option<Duration>,
        preview_fd: Option<libc::c_int>,
    ) -> io::Result<Event> {
        loop {
            let pending = PENDING.swap(0, Ordering::Relaxed);
            for (i, signal) in SIGNALS.iter().enumerate().take(5).skip(1) {
                if pending & (1 << i) != 0 {
                    return Ok(Event::Exit((128 + signal) as u8));
                }
            }
            if pending & (1 << 5) != 0 {
                return Ok(Event::Suspend);
            }
            if pending != 0 {
                return Ok(Event::Redraw);
            }
            // SAFETY: all pointers refer to initialized buffers for the call lifetime;
            // the input fd was checked against FD_SETSIZE. pselect temporarily
            // restores the waiting mask and reblocks signals before returning.
            unsafe {
                let fd = self.input.as_raw_fd();
                let mut fds = MaybeUninit::<libc::fd_set>::zeroed().assume_init();
                libc::FD_ZERO(&mut fds);
                libc::FD_SET(fd, &mut fds);
                if let Some(preview) = preview_fd {
                    libc::FD_SET(preview, &mut fds);
                }
                let deadline = timeout.map(|t| libc::timespec {
                    tv_sec: t.as_secs() as libc::time_t,
                    tv_nsec: t.subsec_nanos().into(),
                });
                let result = libc::pselect(
                    fd.max(preview_fd.unwrap_or(fd)) + 1,
                    &mut fds,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    deadline.as_ref().map_or(std::ptr::null(), |t| t),
                    &self.signals.waiting_mask,
                );
                if result < 0 {
                    let error = io::Error::last_os_error();
                    if error.kind() == io::ErrorKind::Interrupted {
                        // Consume signal flags before any input or redraw.
                        continue;
                    }
                    return Err(error);
                }
                if PENDING.load(Ordering::Relaxed) != 0 {
                    continue;
                }
                if result == 0 {
                    return Ok(Event::Timeout);
                }
                // Input wins over completions when both are ready, so viewport
                // changes can invalidate results before they are displayed.
                if !libc::FD_ISSET(fd, &fds) {
                    return Ok(Event::Preview);
                }
                let n = libc::read(fd, buffer.as_mut_ptr().cast(), buffer.len());
                if n < 0 {
                    let error = io::Error::last_os_error();
                    return if error.kind() == io::ErrorKind::WouldBlock {
                        Ok(Event::Input(0))
                    } else {
                        Err(error)
                    };
                }
                if n == 0 {
                    return Ok(Event::Exit(0));
                }
                return Ok(Event::Input(n as usize));
            }
        }
    }
}

impl<W: Write> Drop for Session<'_, W> {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

fn set_attributes(attributes: &libc::termios) -> io::Result<()> {
    loop {
        // TCSANOW avoids flushing queued input or waiting for terminal output.
        if unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, attributes) } == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}
