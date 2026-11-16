//! Producing a file without leaving a lie behind.
//!
//! # The incident class
//!
//! mcp notes 18 and 26. Every stale-artifact failure in the logs began the
//! same way: a tool errored, and the PREVIOUS run's good file was still
//! sitting at the path. The next read found it, parsed perfectly, and returned
//! a confident number about a model that no longer existed. Four flank probes
//! once reported 52.800 mm3 of material in a box whose radii put it outside
//! the body wall; all four were reading a file from an earlier run.
//!
//! OpenSCAD makes this easy to fall into: an empty result is reported as
//! "Current top level object is empty", it EXITS 0, and it writes NO FILE. So
//! "the file is there" is not evidence that this run produced it.
//!
//! # The two rules
//!
//! 1. **Clear the target before producing.** Absence must mean absence. A
//!    producer that writes over whatever is there cannot distinguish "I wrote
//!    nothing" from "I wrote nothing and something else is here".
//! 2. **A failure returns no path.** Not a path to a partial file, not a path
//!    to the old file, nothing. A `Failed` result carrying a path invites the
//!    next caller to read it, which is precisely how the class propagates.
//!
//! Rule 1 is the one that is easy to skip because it looks redundant, and it
//! is the one that makes rule 2 mean anything: without it, "no path returned"
//! still leaves a readable file at a predictable location.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

/// What a producer returns. There is deliberately no way to build a failed
/// outcome that carries a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Produced {
    /// The producer wrote a file and it is at this path.
    At(PathBuf),
    /// The producer ran and there was nothing to write. EMPTY IS A VALUE: this
    /// is not a failure and must not be reported as one.
    Empty,
    /// The producer failed. No path, by construction.
    Failed(String),
}

impl Produced {
    /// The path, if there is one. `Empty` and `Failed` have none.
    pub fn path(&self) -> Option<&Path> {
        match self {
            Produced::At(p) => Some(p.as_path()),
            _ => None,
        }
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Produced::Failed(_))
    }
}

/// Run a producer against `target`, enforcing both rules.
///
/// `produce` is handed the cleared target path and returns `Ok(true)` if it
/// believes it wrote something, `Ok(false)` for a legitimately empty result,
/// or `Err` for a failure.
///
/// THE PRODUCER'S OWN CLAIM IS NOT TRUSTED. `Ok(true)` with no file on disk is
/// reported as `Empty`, not `At`: a producer that thinks it wrote and did not
/// is exactly the OpenSCAD case, and believing it would hand back a path to
/// nothing. The disk is the authority.
///
/// ON FAILURE THE TARGET IS CLEARED AGAIN, because a producer may have written
/// a partial file before erroring, and a partial file at a predictable path is
/// the same hazard as a stale one.
pub fn produce<F>(target: &Path, produce: F) -> Produced
where
    F: FnOnce(&Path) -> Result<bool>,
{
    if let Err(e) = clear(target) {
        return Produced::Failed(format!("could not clear the target before producing: {e}"));
    }
    match produce(target) {
        Err(e) => {
            let _ = clear(target);
            Produced::Failed(format!("{e:#}"))
        }
        Ok(_claimed) => {
            if target.exists() {
                Produced::At(target.to_path_buf())
            } else {
                Produced::Empty
            }
        }
    }
}

/// Remove the target if it is there. Absent is success.
pub fn clear(target: &Path) -> Result<()> {
    match std::fs::remove_file(target) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(anyhow!("removing {}: {e}", target.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp(name: &str) -> PathBuf {
        let mut d = std::env::temp_dir();
        d.push(format!("wm-artifact-{}-{name}", std::process::id()));
        d
    }

    #[test]
    fn a_producer_that_writes_gets_its_path_back() {
        let t = tmp("writes");
        let got = produce(&t, |p| {
            fs::write(p, b"solid")?;
            Ok(true)
        });
        assert_eq!(got, Produced::At(t.clone()));
        assert!(got.path().is_some());
        let _ = fs::remove_file(&t);
    }

    /// THE RULE. A failure returns no path at all.
    #[test]
    fn a_failure_returns_no_path() {
        let t = tmp("fails");
        let got = produce(&t, |_| Err(anyhow!("the kernel refused")));
        assert!(got.is_failed());
        assert_eq!(got.path(), None, "a failure must not hand back a path");
        match got {
            Produced::Failed(why) => assert!(why.contains("kernel refused"), "{why}"),
            _ => panic!("expected a failure"),
        }
    }

    /// THE INCIDENT, REPRODUCED. A good file from an earlier run is sitting at
    /// the target when a later run fails. Without rule 1 the next read finds
    /// it and reports a number about a model that no longer exists.
    #[test]
    fn a_previous_runs_file_does_not_survive_a_failure() {
        let t = tmp("stale");
        fs::write(&t, b"a good file from the last run").expect("seeds the stale file");
        assert!(t.exists());
        let got = produce(&t, |_| Err(anyhow!("this run failed")));
        assert!(got.is_failed());
        assert!(
            !t.exists(),
            "the previous run's file must NOT be left where the next call reads it"
        );
    }

    /// THE INCIDENT ITSELF, AND THE ARM THAT WAS MISSING. A stale file is at
    /// the target and this run produces NOTHING -- the OpenSCAD empty case
    /// exactly, which exits 0 and writes no file. Without the clear BEFORE
    /// producing, the old file is still there, `target.exists()` is true, and
    /// the last run's artifact is handed back as this run's result. That is
    /// mcp note 26: four flank probes reporting 52.800 mm3 of material that
    /// was not there.
    ///
    /// This arm exists because a mutant found its absence. Removing the
    /// clear-before left the suite green, since the stale-file test above is
    /// ALSO satisfied by the clear that runs after a failure. Only a run that
    /// SUCCEEDS while writing nothing can tell the two clears apart.
    #[test]
    fn a_stale_file_is_not_handed_back_as_an_empty_runs_result() {
        let t = tmp("stale-empty");
        fs::write(&t, b"the previous run's good file").expect("seeds the stale file");
        let got = produce(&t, |_| Ok(false));
        assert_eq!(
            got,
            Produced::Empty,
            "this run wrote nothing, so the answer is Empty -- not a path to the last run's file"
        );
        assert_eq!(got.path(), None);
        assert!(!t.exists(), "and the stale file is gone");
    }

    /// A producer may write a partial file and then error. A partial file at a
    /// predictable path is the same hazard as a stale one.
    #[test]
    fn a_partial_file_written_before_the_error_is_removed() {
        let t = tmp("partial");
        let got = produce(&t, |p| {
            fs::write(p, b"half an ST")?;
            Err(anyhow!("died halfway"))
        });
        assert!(got.is_failed());
        assert!(!t.exists(), "a partial file must not be left behind");
    }

    /// EMPTY IS A VALUE. OpenSCAD writes no file and exits 0 for an empty
    /// result, and that is not a failure.
    #[test]
    fn an_empty_result_is_empty_not_failed_and_has_no_path() {
        let t = tmp("empty");
        let got = produce(&t, |_| Ok(false));
        assert_eq!(got, Produced::Empty);
        assert_eq!(got.path(), None);
        assert!(!got.is_failed(), "empty is not a failure");
    }

    /// THE PRODUCER'S CLAIM IS NOT THE AUTHORITY. A producer that returns
    /// `Ok(true)` having written nothing is the OpenSCAD case exactly, and
    /// believing it would hand back a path to a file that is not there.
    #[test]
    fn a_producer_that_claims_success_without_writing_is_empty_not_at() {
        let t = tmp("liar");
        let got = produce(&t, |_| Ok(true));
        assert_eq!(
            got,
            Produced::Empty,
            "the disk decides, not the producer's claim"
        );
        assert_eq!(got.path(), None);
    }

    /// And the mirror: a producer that says `false` while HAVING written is
    /// reported `At`. The file exists; denying it would leave an orphan no
    /// caller knows about.
    #[test]
    fn a_producer_that_denies_writing_but_did_is_reported_at() {
        let t = tmp("modest");
        let got = produce(&t, |p| {
            fs::write(p, b"solid")?;
            Ok(false)
        });
        assert_eq!(got, Produced::At(t.clone()));
        let _ = fs::remove_file(&t);
    }

    #[test]
    fn clearing_an_absent_target_succeeds() {
        let t = tmp("absent");
        let _ = fs::remove_file(&t);
        clear(&t).expect("absent is success");
    }
}
