//! One decoder, one outstanding request/result, and viewport-sized retention.
use crate::{
    cache::{Cache, Stats},
    entry::Entry,
    pager_graphics::{Geometry, Screen},
    preview,
};
use image::RgbaImage;
use std::{
    io::{self, Read, Write},
    ops::Range,
    os::{
        fd::{AsRawFd, RawFd},
        unix::net::UnixStream,
    },
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread,
};

struct Job {
    generation: u64,
    index: usize,
    path: PathBuf,
    width: u32,
    height: u32,
}
struct Completion {
    job: Job,
    image: Option<RgbaImage>,
    stats: Stats,
}
#[derive(Default)]
struct Mailbox {
    job: Option<Job>,
    result: Option<Completion>,
    stop: bool,
}
struct Shared {
    mailbox: Mutex<Mailbox>,
    ready: Condvar,
    generation: AtomicU64,
}

struct Worker {
    shared: Arc<Shared>,
    wake: UnixStream,
}

impl Worker {
    fn new(cache_dir: Option<PathBuf>, generation: u64) -> io::Result<Self> {
        Self::spawn(generation, move |shared, wake| {
            let mut cache = Cache::new(cache_dir.as_deref());
            work(shared, wake, |job| {
                let image = preview::load(&job.path, job.width, job.height, &mut cache).ok();
                (image, cache.stats)
            });
        })
    }

    fn spawn(
        generation: u64,
        run: impl FnOnce(Arc<Shared>, UnixStream) + Send + 'static,
    ) -> io::Result<Self> {
        let (wake, sender) = UnixStream::pair()?;
        wake.set_nonblocking(true)?;
        sender.set_nonblocking(true)?;
        if wake.as_raw_fd() as usize >= libc::FD_SETSIZE {
            return Err(io::Error::other("preview wake fd exceeds select limit"));
        }
        let shared = Arc::new(Shared {
            mailbox: Mutex::new(Mailbox::default()),
            ready: Condvar::new(),
            generation: AtomicU64::new(generation),
        });
        let state = Arc::clone(&shared);
        // Spawn after Session installs the main-thread signal mask. The worker
        // inherits blocked managed signals; only the main pselect unmasks them.
        thread::Builder::new()
            .name("lsa-preview".into())
            .spawn(move || run(state, sender))?;
        Ok(Self { shared, wake })
    }

    fn submit(&self, job: Job) {
        self.shared.mailbox.lock().unwrap().job = Some(job);
        self.shared.ready.notify_one();
    }

    fn take(&mut self) -> io::Result<Option<Completion>> {
        let mut bytes = [0; 16];
        loop {
            match self.wake.read(&mut bytes) {
                Ok(0) => return Err(io::Error::other("preview worker stopped")),
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(self.shared.mailbox.lock().unwrap().result.take())
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let mut mailbox = self.shared.mailbox.lock().unwrap();
        mailbox.stop = true;
        mailbox.job = None;
        mailbox.result = None;
        self.shared.generation.fetch_add(1, Ordering::Relaxed);
        self.shared.ready.notify_one();
        // Do not join an in-progress decoder/filesystem call. This is an owned
        // thread, never another process; process exit terminates any remaining work.
    }
}

fn work(
    shared: Arc<Shared>,
    mut wake: UnixStream,
    mut load: impl FnMut(&Job) -> (Option<RgbaImage>, Stats),
) {
    loop {
        let job = {
            let mut mailbox = shared.mailbox.lock().unwrap();
            while mailbox.job.is_none() && !mailbox.stop {
                mailbox = shared.ready.wait(mailbox).unwrap();
            }
            if mailbox.stop {
                return;
            }
            mailbox.job.take().unwrap()
        };
        let (mut image, stats) = if shared.generation.load(Ordering::Relaxed) == job.generation {
            load(&job)
        } else {
            (None, Stats::default())
        };
        if shared.generation.load(Ordering::Relaxed) != job.generation {
            image = None;
        }
        let mut mailbox = shared.mailbox.lock().unwrap();
        if mailbox.stop {
            return;
        }
        mailbox.result = Some(Completion { job, image, stats });
        drop(mailbox);
        // At most one completion is outstanding. A full notification socket
        // already wakes the main loop; the bounded mailbox holds the actual result.
        if let Err(e) = wake.write_all(&[1])
            && e.kind() != io::ErrorKind::WouldBlock
        {
            return;
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub geometry: Geometry,
    pub range: Range<usize>,
}
enum Pixels {
    Ready(RgbaImage),
    Unavailable,
    Limited,
}
struct Record {
    index: usize,
    pixels: Pixels,
    number: Option<u32>,
}

pub struct Previews {
    view: Option<View>,
    records: Vec<Record>,
    worker: Option<Worker>,
    cache_dir: Option<PathBuf>,
    generation: u64,
    busy: bool,
    disabled: bool,
    attempt_limit: usize,
    attempts_left: usize,
    pub stats: Stats,
    pub stale: usize,
}

impl Previews {
    pub fn new(attempts: usize, cache_dir: Option<PathBuf>) -> Self {
        Self {
            view: None,
            records: Vec::new(),
            worker: None,
            cache_dir,
            generation: 0,
            busy: false,
            disabled: false,
            attempt_limit: attempts,
            attempts_left: attempts,
            stats: Stats::default(),
            stale: 0,
        }
    }

    pub fn fd(&self) -> Option<RawFd> {
        self.worker.as_ref().map(|w| w.wake.as_raw_fd())
    }

    pub fn sync(
        &mut self,
        view: Option<View>,
        entries: usize,
        screen: &mut Screen,
        out: &mut impl Write,
    ) -> io::Result<()> {
        if self.view == view {
            return Ok(());
        }
        // A user-driven viewport change opens a new bounded batch. Selection
        // and completion redraws of the same viewport cannot renew its budget.
        screen.begin_view();
        self.generation += 1;
        if let Some(worker) = &self.worker {
            worker
                .shared
                .generation
                .store(self.generation, Ordering::Relaxed);
        }
        let compatible = self
            .view
            .as_ref()
            .zip(view.as_ref())
            .is_some_and(|(a, b)| a.geometry == b.geometry);
        let wanted = view
            .as_ref()
            .map(|v| margin(&v.range, entries))
            .unwrap_or(0..0);
        let visible = view.as_ref().map(|v| v.range.clone()).unwrap_or(0..0);
        for record in &mut self.records {
            if (!compatible || !visible.contains(&record.index))
                && let Some(number) = record.number.take()
            {
                screen.remove(out, number)?;
            }
        }
        self.records.retain(|r| {
            compatible && wanted.contains(&r.index) && !matches!(r.pixels, Pixels::Limited)
        });
        self.attempts_left = self.attempt_limit.saturating_sub(self.records.len());
        self.view = view;
        Ok(())
    }

    pub fn invalidate(&mut self, screen: &mut Screen, out: &mut impl Write) -> io::Result<()> {
        self.sync(None, 0, screen, out)
    }

    pub fn placeholder(&self, index: usize, available: usize) -> &'static str {
        match self
            .records
            .iter()
            .find(|r| r.index == index)
            .map(|r| &r.pixels)
        {
            Some(Pixels::Ready(_)) => "",
            Some(Pixels::Unavailable) => "[no preview]",
            Some(Pixels::Limited) => "[limit]",
            None if self.disabled => "[no preview]",
            None if self.attempts_left == 0
                || self
                    .view
                    .as_ref()
                    .is_some_and(|v| v.geometry.upload_bound() > available) =>
            {
                "[limit]"
            }
            None => "[loading]",
        }
    }

    pub fn paint(&mut self, screen: &mut Screen, out: &mut impl Write) -> io::Result<bool> {
        let Some(view) = &self.view else {
            return Ok(false);
        };
        let could_upload = screen.remaining() >= view.geometry.upload_bound();
        let mut changed = false;
        for record in &mut self.records {
            if !view.range.contains(&record.index) {
                continue;
            }
            let Pixels::Ready(image) = &record.pixels else {
                continue;
            };
            let (row, col) = view.geometry.position(record.index - view.range.start);
            if let Some(number) = record.number {
                if !screen.move_to(out, number, view.geometry, row, col)? {
                    screen.remove(out, number)?;
                    record.number = None;
                    record.pixels = Pixels::Limited;
                    changed = true;
                }
            } else {
                record.number = screen.upload(out, image, view.geometry, row, col)?;
                if record.number.is_none() {
                    record.pixels = Pixels::Limited;
                    changed = true;
                }
            }
        }
        // The last successful upload can exhaust the budget. Render the remaining
        // placeholders once more so they say [limit] instead of staying [loading].
        Ok(changed || (could_upload && screen.remaining() < view.geometry.upload_bound()))
    }

    pub fn schedule(&mut self, entries: &[Entry], selected: usize, available: usize) -> bool {
        if self.busy || self.disabled || self.attempts_left == 0 {
            return false;
        }
        let Some(view) = &self.view else {
            return false;
        };
        if view.geometry.upload_bound() > available {
            return false;
        }
        let wanted = margin(&view.range, entries.len());
        let index = std::iter::once(selected)
            .filter(|i| view.range.contains(i))
            .chain(view.range.clone())
            .chain(wanted)
            .find(|i| entries[*i].candidate() && !self.records.iter().any(|r| r.index == *i));
        let Some(index) = index else {
            return false;
        };
        if self.worker.is_none() {
            match Worker::new(self.cache_dir.clone(), self.generation) {
                Ok(worker) => self.worker = Some(worker),
                Err(_) => {
                    self.disabled = true;
                    return true;
                }
            }
        }
        self.attempts_left -= 1;
        self.busy = true;
        self.worker.as_ref().unwrap().submit(Job {
            generation: self.generation,
            index,
            path: entries[index].path.clone(),
            width: view.geometry.width,
            height: view.geometry.height,
        });
        false
    }

    pub fn complete(&mut self) {
        let result = self.worker.as_mut().unwrap().take();
        match result {
            Ok(Some(result)) => {
                self.busy = false;
                // Cache counters are cumulative; skipped stale jobs return zeros.
                if result.stats.hits + result.stats.misses + result.stats.errors > 0 {
                    self.stats = result.stats;
                }
                if result.job.generation != self.generation {
                    self.stale += 1;
                    return;
                }
                self.records.push(Record {
                    index: result.job.index,
                    number: None,
                    pixels: result.image.map_or(Pixels::Unavailable, Pixels::Ready),
                });
            }
            Ok(None) => {}
            Err(_) => {
                self.disabled = true;
                self.busy = false;
                self.worker = None;
            }
        }
    }
}

fn margin(range: &Range<usize>, entries: usize) -> Range<usize> {
    range.start.saturating_sub(1)..range.end.saturating_add(1).min(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };

    #[test]
    fn scheduler_prioritizes_selection_and_rejects_old_view_results() {
        let (entered, started) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let worker = Worker::spawn(0, move |shared, wake| {
            work(shared, wake, |job| {
                entered.send(job.index).unwrap();
                wait.recv().unwrap();
                (
                    Some(RgbaImage::new(job.width, job.height)),
                    Stats::default(),
                )
            });
        })
        .unwrap();
        let entries: Vec<_> = (0..40)
            .map(|i| Entry {
                path: format!("image-{i}.png").into(),
                name: format!("image-{i}.png").into(),
                kind: crate::entry::Kind::File,
                metadata: None,
            })
            .collect();
        let mut previews = Previews::new(4, None);
        previews.worker = Some(worker);
        let geometry = Geometry {
            columns: 3,
            tile: 24,
            lines: 3,
            width: 176,
            height: 80,
        };
        let view = |range| Some(View { geometry, range });
        let mut screen = Screen::default();
        let mut out = Vec::new();
        previews
            .sync(view(0..9), entries.len(), &mut screen, &mut out)
            .unwrap();
        previews.schedule(&entries, 5, screen.remaining());
        assert_eq!(started.recv_timeout(Duration::from_secs(2)).unwrap(), 5);
        previews.schedule(&entries, 6, screen.remaining());
        assert_eq!(previews.attempts_left, 3); // No second queued request.
        previews
            .sync(view(20..29), entries.len(), &mut screen, &mut out)
            .unwrap();
        release.send(()).unwrap();
        let finish = |previews: &mut Previews| {
            let deadline = Instant::now() + Duration::from_secs(2);
            while previews.busy {
                previews.complete();
                assert!(Instant::now() < deadline);
                thread::sleep(Duration::from_millis(1));
            }
        };
        finish(&mut previews);
        assert_eq!(previews.stale, 1);
        assert!(previews.records.is_empty() && out.is_empty());
        previews.schedule(&entries, 28, screen.remaining());
        assert_eq!(started.recv_timeout(Duration::from_secs(2)).unwrap(), 28);
        release.send(()).unwrap();
        finish(&mut previews);
        assert_eq!(previews.records.len(), 1);
        previews.paint(&mut screen, &mut out).unwrap();
        assert!(previews.records[0].number.is_some());
        previews
            .sync(view(25..34), entries.len(), &mut screen, &mut out)
            .unwrap();
        assert_eq!(previews.records.len(), 1); // Overlap keeps pixels and image number.
        let before = screen.remaining();
        previews.paint(&mut screen, &mut out).unwrap();
        assert!(screen.remaining() < before); // Only a bounded move command.
        previews
            .sync(view(0..9), entries.len(), &mut screen, &mut out)
            .unwrap();
        assert!(previews.records.is_empty());
    }

    #[test]
    fn slow_worker_discards_stale_pixels_and_drop_never_joins() {
        let (entered, started) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let mut worker = Worker::spawn(1, move |shared, wake| {
            work(shared, wake, |job| {
                entered.send(()).unwrap();
                wait.recv().unwrap();
                (
                    Some(RgbaImage::new(job.width, job.height)),
                    Stats::default(),
                )
            })
        })
        .unwrap();
        let job = |generation| Job {
            generation,
            index: 3,
            path: "unused".into(),
            width: 10,
            height: 10,
        };
        worker.submit(job(1));
        started.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.shared.generation.store(2, Ordering::Relaxed);
        release.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Some(result) = worker.take().unwrap() {
                assert_eq!(result.job.generation, 1);
                assert!(result.image.is_none());
                break;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(1));
        }
        worker.submit(job(2));
        started.recv_timeout(Duration::from_secs(2)).unwrap();
        let begin = Instant::now();
        drop(worker);
        assert!(begin.elapsed() < Duration::from_millis(100));
        release.send(()).unwrap();
    }
}
