//! Bounded, ordered work: up to `workers` threads process jobs at most
//! `2 * workers` ahead of the consumer, which receives results strictly in job
//! order. Output therefore matches sequential work exactly, while memory stays
//! bounded by the few jobs in flight. A job that panics yields `None`.
use std::{
    panic::{self, AssertUnwindSafe},
    sync::{Condvar, Mutex, MutexGuard, PoisonError},
    thread,
};

/// Worker thread name; the process panic hook keeps these threads quiet.
pub const THREAD_NAME: &str = "lsa-preview";

struct State<T> {
    claimed: usize,
    consumed: usize,
    stop: bool,
    // Outer None: not finished. Inner None: the job panicked.
    results: Vec<Option<Option<T>>>,
}

struct Shared<T> {
    state: Mutex<State<T>>,
    ready: Condvar,
    room: Condvar,
    window: usize,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn run<J, T>(work: &(impl Fn(&J) -> T + Sync), job: &J) -> Option<T> {
    panic::catch_unwind(AssertUnwindSafe(|| work(job))).ok()
}

pub struct Results<'a, J, T, W> {
    shared: &'a Shared<T>,
    jobs: &'a [J],
    work: &'a W,
    next: usize,
    // No worker could be started: compute each result on demand instead.
    inline: bool,
}

impl<J, T, W: Fn(&J) -> T + Sync> Results<'_, J, T, W> {
    /// Whether another result can be taken without waiting.
    pub fn is_ready(&self) -> bool {
        self.next < self.jobs.len()
            && (self.inline || lock(&self.shared.state).results[self.next].is_some())
    }

    /// The next result in job order, waiting for it if necessary.
    pub fn next(&mut self) -> Option<T> {
        let index = self.next;
        self.next += 1;
        if self.inline {
            return run(self.work, &self.jobs[index]);
        }
        let mut state = lock(&self.shared.state);
        loop {
            if let Some(result) = state.results[index].take() {
                state.consumed = index + 1;
                self.shared.room.notify_all();
                return result;
            }
            state = self
                .shared
                .ready
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

// Stops workers when consumption ends early (an output error) or unwinds;
// otherwise a worker waiting for window room would block the scope's join.
struct Stop<'a, T>(&'a Shared<T>);
impl<T> Drop for Stop<'_, T> {
    fn drop(&mut self) {
        lock(&self.0.state).stop = true;
        self.0.room.notify_all();
    }
}

fn worker<J, T>(shared: &Shared<T>, jobs: &[J], work: &(impl Fn(&J) -> T + Sync)) {
    loop {
        let index = {
            let mut state = lock(&shared.state);
            loop {
                if state.stop || state.claimed == jobs.len() {
                    return;
                }
                if state.claimed < state.consumed + shared.window {
                    break;
                }
                state = shared
                    .room
                    .wait(state)
                    .unwrap_or_else(PoisonError::into_inner);
            }
            state.claimed += 1;
            state.claimed - 1
        };
        let result = run(work, &jobs[index]);
        lock(&shared.state).results[index] = Some(result);
        shared.ready.notify_all();
    }
}

/// Process `jobs` with `work` on up to `workers` threads while `consume`
/// takes results in order. Returns once consumption ends and workers stop.
pub fn ordered<J: Sync, T: Send, W: Fn(&J) -> T + Sync, R>(
    jobs: &[J],
    workers: usize,
    work: W,
    consume: impl FnOnce(&mut Results<'_, J, T, W>) -> R,
) -> R {
    let shared = Shared {
        state: Mutex::new(State {
            claimed: 0,
            consumed: 0,
            stop: false,
            results: (0..jobs.len()).map(|_| None).collect(),
        }),
        ready: Condvar::new(),
        room: Condvar::new(),
        window: 2 * workers.max(1),
    };
    thread::scope(|scope| {
        let _stop = Stop(&shared);
        let mut started = 0;
        for _ in 0..workers.min(jobs.len()) {
            // Match the main thread's stack; decoders run here instead.
            let spawned = thread::Builder::new()
                .name(THREAD_NAME.into())
                .stack_size(8 << 20)
                .spawn_scoped(scope, || worker(&shared, jobs, &work));
            if spawned.is_err() {
                break;
            }
            started += 1;
        }
        consume(&mut Results {
            shared: &shared,
            jobs,
            work: &work,
            next: 0,
            inline: started == 0,
        })
    })
}

/// Workers for preview decoding: bounded so memory stays a small multiple of
/// one decode, even on machines with many cores.
pub fn workers() -> usize {
    thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };

    #[test]
    fn results_arrive_in_job_order_with_bounded_lookahead() {
        let jobs: Vec<u64> = (0..64).collect();
        let started = AtomicUsize::new(0);
        let consumed = AtomicUsize::new(0);
        let ahead = AtomicUsize::new(0);
        let results = ordered(
            &jobs,
            4,
            |&job| {
                let claimed = started.fetch_add(1, Ordering::SeqCst) + 1;
                ahead.fetch_max(claimed - consumed.load(Ordering::SeqCst), Ordering::SeqCst);
                // Later jobs finish first, so ordering is actually exercised.
                thread::sleep(Duration::from_micros((64 - job) * 50));
                job * 3
            },
            |results| {
                (0..jobs.len())
                    .map(|_| {
                        let value = results.next();
                        consumed.fetch_add(1, Ordering::SeqCst);
                        value
                    })
                    .collect::<Vec<_>>()
            },
        );
        assert_eq!(
            results,
            jobs.iter().map(|j| Some(j * 3)).collect::<Vec<_>>()
        );
        // The window is twice the worker count: never further ahead than that.
        assert!(ahead.load(Ordering::SeqCst) <= 8 + 1);
    }

    #[test]
    fn panicking_jobs_yield_none_and_early_exit_stops_workers() {
        let jobs: Vec<usize> = (0..10).collect();
        let results = ordered(
            &jobs,
            3,
            |&job| {
                assert!(job != 4, "decoder bug");
                job
            },
            |results| (0..10).map(|_| results.next()).collect::<Vec<_>>(),
        );
        assert_eq!(results[4], None);
        assert_eq!(results.iter().flatten().count(), 9);
        // Consuming only part of the work returns promptly; no job runs past
        // the lookahead window after consumption stops.
        let jobs: Vec<usize> = (0..1000).collect();
        let ran = AtomicUsize::new(0);
        let first = ordered(
            &jobs,
            2,
            |&job| {
                ran.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(1));
                job
            },
            |results| results.next(),
        );
        assert_eq!(first, Some(0));
        assert!(ran.load(Ordering::SeqCst) <= 1 + 4 + 2);
    }

    #[test]
    fn empty_and_single_job_lists() {
        let none: [u8; 0] = [];
        assert_eq!(ordered(&none, 4, |&j| j, |_| 7), 7);
        assert_eq!(
            ordered(&[5], 4, |&j| j + 1, |r| (r.next(), r.is_ready())),
            (Some(6), false)
        );
        assert_eq!(
            ordered(&[5, 6], 0, |&j| j, |r| (r.next(), r.next())),
            (Some(5), Some(6))
        );
    }
}
