//! Jobs, so a long call is not a lost turn.
//!
//! # Why
//!
//! The provider's own timeout is 30 to 3600 seconds and then a TIMEOUT error.
//! A full overhang scan on the harness took 76 seconds. A call that blocks for
//! that long and then times out costs a turn and leaves nothing behind except
//! the temptation to retry it, which costs another.
//!
//! So anything that can exceed a few seconds starts a job and returns a handle
//! at once. The work happens on a thread; the caller asks `status` when it
//! wants to.
//!
//! # The failure that matters
//!
//! A worker that PANICS must not leave its job `Running` for ever. A job stuck
//! in `Running` is indistinguishable from a job that is simply slow, so a
//! caller waits, then waits more, and the thing it is waiting for died minutes
//! ago. Every worker's end is recorded, panic included, because an absence of
//! news must never be the representation of a death.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// A job's identity. Opaque and monotonic; ids are never reused, so a stale
/// handle names a job that is GONE rather than someone else's work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JobId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Still working. `note` is whatever the worker last reported.
    Running { note: String },
    /// Finished, with what it produced rendered as a string.
    Done { value: String },
    /// Finished badly. A panic lands here too.
    Failed { why: String },
    /// No such job. NOT an error and NOT running: a handle that names nothing
    /// is its own answer, and conflating it with either of those is how a
    /// caller comes to wait on a job that never existed.
    Unknown,
}

impl Status {
    pub fn is_running(&self) -> bool {
        matches!(self, Status::Running { .. })
    }
    pub fn is_finished(&self) -> bool {
        matches!(self, Status::Done { .. } | Status::Failed { .. })
    }
}

#[derive(Debug, Default)]
struct Registry {
    jobs: HashMap<JobId, Status>,
    labels: HashMap<JobId, String>,
}

/// A place to start jobs and ask after them.
#[derive(Debug, Clone, Default)]
pub struct Jobs {
    inner: Arc<Mutex<Registry>>,
    next: Arc<AtomicU64>,
}

/// Handed to a worker so it can say where it has got to.
#[derive(Clone)]
pub struct Progress {
    id: JobId,
    inner: Arc<Mutex<Registry>>,
}

impl Progress {
    /// Report where the work has reached. Ignored once the job has finished,
    /// so a late report from a dying thread cannot resurrect it.
    pub fn note(&self, text: impl Into<String>) {
        if let Ok(mut r) = self.inner.lock() {
            if let Some(s) = r.jobs.get_mut(&self.id) {
                if s.is_running() {
                    *s = Status::Running { note: text.into() };
                }
            }
        }
    }
}

impl Jobs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start work and return at once.
    ///
    /// THE WORKER'S END IS ALWAYS RECORDED. `catch_unwind` is not defensive
    /// decoration here: without it a panicking worker leaves its job
    /// `Running`, which reads exactly like a job that is merely slow.
    pub fn start<F>(&self, label: &str, f: F) -> JobId
    where
        F: FnOnce(Progress) -> Result<String, String> + Send + 'static,
    {
        let id = JobId(self.next.fetch_add(1, Ordering::SeqCst));
        if let Ok(mut r) = self.inner.lock() {
            r.jobs.insert(
                id,
                Status::Running {
                    note: "started".into(),
                },
            );
            r.labels.insert(id, label.to_string());
        }
        let inner = Arc::clone(&self.inner);
        let progress = Progress {
            id,
            inner: Arc::clone(&inner),
        };
        std::thread::spawn(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(progress)));
            let status = match outcome {
                Ok(Ok(value)) => Status::Done { value },
                Ok(Err(why)) => Status::Failed { why },
                Err(p) => {
                    let what = p
                        .downcast_ref::<&str>()
                        .map(|s| (*s).to_string())
                        .or_else(|| p.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "a panic with no message".into());
                    Status::Failed {
                        why: format!("the worker panicked: {what}"),
                    }
                }
            };
            if let Ok(mut r) = inner.lock() {
                r.jobs.insert(id, status);
            }
        });
        id
    }

    pub fn status(&self, id: JobId) -> Status {
        match self.inner.lock() {
            Ok(r) => r.jobs.get(&id).cloned().unwrap_or(Status::Unknown),
            // A POISONED LOCK IS A FAILURE, NOT A SILENCE. Reporting Unknown
            // here would say "no such job" about a job that exists.
            Err(_) => Status::Failed {
                why: "the job registry is poisoned; a previous worker panicked while holding it"
                    .into(),
            },
        }
    }

    pub fn label(&self, id: JobId) -> Option<String> {
        self.inner.lock().ok()?.labels.get(&id).cloned()
    }

    pub fn running(&self) -> usize {
        match self.inner.lock() {
            Ok(r) => r.jobs.values().filter(|s| s.is_running()).count(),
            Err(_) => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    fn settle(jobs: &Jobs, id: JobId) -> Status {
        for _ in 0..600 {
            let s = jobs.status(id);
            if s.is_finished() {
                return s;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("job {id:?} never finished");
    }

    /// NOTHING BLOCKS. The whole point: `start` returns while the work is
    /// still going. A release gate holds the worker open so the assertion is
    /// about ordering and not about being faster than the work.
    #[test]
    fn start_returns_while_the_work_is_still_running() {
        let jobs = Jobs::new();
        let (tx, rx) = mpsc::channel::<()>();
        let t0 = Instant::now();
        let id = jobs.start("slow", move |_p| {
            rx.recv().ok();
            Ok("done at last".into())
        });
        assert!(
            t0.elapsed() < Duration::from_secs(2),
            "start must not wait for the work"
        );
        assert!(
            jobs.status(id).is_running(),
            "the job is still going when the handle is already in hand"
        );
        tx.send(()).expect("release the worker");
        assert_eq!(
            settle(&jobs, id),
            Status::Done {
                value: "done at last".into()
            }
        );
    }

    #[test]
    fn a_worker_can_report_where_it_has_got_to() {
        let jobs = Jobs::new();
        let (tx, rx) = mpsc::channel::<()>();
        let id = jobs.start("scan", move |p| {
            p.note("layer 41 of 207");
            rx.recv().ok();
            Ok("finished".into())
        });
        for _ in 0..400 {
            if let Status::Running { note } = jobs.status(id) {
                if note.contains("layer 41") {
                    tx.send(()).ok();
                    settle(&jobs, id);
                    return;
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("the progress note never arrived");
    }

    /// THE FAILURE THAT MATTERS. A job left `Running` by a dead worker is
    /// indistinguishable from a slow one, so the caller waits for something
    /// that died minutes ago.
    #[test]
    fn a_panicking_worker_becomes_failed_and_not_running_for_ever() {
        let jobs = Jobs::new();
        let id = jobs.start("doomed", |_p| panic!("the kernel went away"));
        match settle(&jobs, id) {
            Status::Failed { why } => {
                assert!(why.contains("panicked"), "say that it panicked: {why}");
                assert!(why.contains("kernel went away"), "and what it said: {why}");
            }
            other => panic!("a panicking worker must be Failed, got {other:?}"),
        }
        assert_eq!(
            jobs.running(),
            0,
            "and it must not still be counted running"
        );
    }

    #[test]
    fn an_error_from_the_worker_is_failed_with_its_reason() {
        let jobs = Jobs::new();
        let id = jobs.start("refused", |_p| Err("the provider refused".into()));
        assert_eq!(
            settle(&jobs, id),
            Status::Failed {
                why: "the provider refused".into()
            }
        );
    }

    /// A handle that names nothing is ITS OWN ANSWER. Reporting Running would
    /// have a caller wait for ever; reporting Failed would have them report a
    /// failure that never happened.
    #[test]
    fn an_unknown_handle_is_unknown_not_running_and_not_failed() {
        let jobs = Jobs::new();
        let s = jobs.status(JobId(9_999));
        assert_eq!(s, Status::Unknown);
        assert!(!s.is_running());
        assert!(!s.is_finished());
    }

    /// Ids are never reused, so a stale handle names a job that is GONE rather
    /// than someone else's work.
    #[test]
    fn ids_are_unique_across_jobs() {
        let jobs = Jobs::new();
        let ids: Vec<JobId> = (0..8)
            .map(|_| jobs.start("x", |_p| Ok("ok".into())))
            .collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "every id is distinct");
        for id in ids {
            settle(&jobs, id);
        }
    }

    /// A late note from a worker that has already finished must not put the
    /// job back into Running.
    #[test]
    fn a_late_progress_note_cannot_resurrect_a_finished_job() {
        let jobs = Jobs::new();
        let (tx, rx) = mpsc::channel::<Progress>();
        let id = jobs.start("quick", move |p| {
            tx.send(p).ok();
            Ok("over".into())
        });
        let p = rx.recv().expect("the worker hands out its reporter");
        let done = settle(&jobs, id);
        assert!(done.is_finished());
        p.note("still going, honest");
        assert_eq!(jobs.status(id), done, "a finished job stays finished");
    }
}
