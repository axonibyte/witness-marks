//! What parts a model holds, and which of their exports are behind the source.
//!
//! # Why
//!
//! The harness exported its parts with a hand-maintained loop: a list of
//! module names in a shell script, kept in step with the model by whoever
//! remembered. A part added to the model and not to the loop is simply never
//! exported, and nothing says so.
//!
//! And an export that WAS made is only as good as the source it was made from.
//! A claim, a render or an STL produced before the last edit is failure class
//! C -- a derived number, invalidated elsewhere, still presented as current.
//!
//! # Unknown is not current
//!
//! The distinction this module exists to keep is between *this export matches
//! its source*, *this export is behind*, and *there is no export at all*. The
//! third one is the easiest to lose, because an absent record and a matching
//! record both produce "nothing to do". They are not the same, and only one of
//! them means the artifact can be trusted.

use std::collections::BTreeMap;

use wm_envelope::sha256_hex;

/// How a part was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Found {
    /// A `show_<name>` flag, the convention this harness uses.
    ShowFlag,
    /// A module whose name the caller recognized as a part.
    Module,
    /// The caller named it.
    Declared,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub name: String,
    pub found: Found,
}

/// Enumerate the printable parts of a model.
///
/// THE THREE ROUTES ARE DELIBERATE AND ORDERED. A declared list wins, because
/// the caller knows things the file does not say. `show_` flags come next:
/// they are this harness's own convention and they are what the hand-written
/// loop was tracking. Module names are last and are only consulted for names
/// the caller supplied, because every model is full of modules that are not
/// parts and guessing from all of them would produce a list nobody can use.
pub fn parts(source: &str, declared: &[&str], module_parts: &[&str]) -> Vec<Part> {
    let mut out: Vec<Part> = Vec::new();
    let mut push = |name: String, found: Found| {
        if !out.iter().any(|p| p.name == name) {
            out.push(Part { name, found });
        }
    };
    for d in declared {
        push((*d).to_string(), Found::Declared);
    }
    for line in source.lines() {
        let line = line.trim();
        let Some(eq) = line.find('=') else { continue };
        let name = line[..eq].trim();
        if let Some(part) = name.strip_prefix("show_") {
            if !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                push(part.to_string(), Found::ShowFlag);
            }
        }
    }
    for m in module_parts {
        let needle = format!("module {m}(");
        if source.contains(&needle) {
            push((*m).to_string(), Found::Module);
        }
    }
    out
}

/// What is known about one part's export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Freshness {
    /// The export was made from this exact source.
    Current,
    /// The export was made from different source. Carries both hashes so the
    /// report can say so rather than merely asserting it.
    Behind { exported_from: String, now: String },
    /// No export has ever been recorded. NOT the same as current.
    NeverExported,
}

impl Freshness {
    /// Can an artifact for this part be trusted as describing the source?
    ///
    /// ONLY `Current`. `NeverExported` answers false, which is the whole point
    /// of its existing: an absent record and a matching record both look like
    /// "nothing to do", and only one of them means the artifact is good.
    pub fn is_trustworthy(&self) -> bool {
        matches!(self, Freshness::Current)
    }
}

/// A record of what was exported from what.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Exports {
    by_part: BTreeMap<String, String>,
}

impl Exports {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `part` was exported from this source text.
    pub fn record(&mut self, part: &str, source: &str) {
        self.by_part
            .insert(part.to_string(), sha256_hex(source.as_bytes()));
    }

    pub fn freshness(&self, part: &str, source: &str) -> Freshness {
        let now = sha256_hex(source.as_bytes());
        match self.by_part.get(part) {
            None => Freshness::NeverExported,
            Some(h) if *h == now => Freshness::Current,
            Some(h) => Freshness::Behind {
                exported_from: h.clone(),
                now,
            },
        }
    }

    /// Every part that cannot be trusted, with the reason, in a stable order.
    ///
    /// REPORTS NEVER-EXPORTED PARTS TOO. A report that listed only the stale
    /// ones would be empty for a model nobody has ever exported, which reads
    /// exactly like a model that is fully up to date.
    pub fn stale(&self, parts: &[Part], source: &str) -> Vec<String> {
        let mut out = Vec::new();
        for p in parts {
            match self.freshness(&p.name, source) {
                Freshness::Current => {}
                Freshness::NeverExported => out.push(format!(
                    "{}: never exported, so there is no artifact to trust",
                    p.name
                )),
                Freshness::Behind { exported_from, now } => out.push(format!(
                    "{}: exported from {}, source is now {}",
                    p.name,
                    &exported_from[..12],
                    &now[..12]
                )),
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../tests/fixtures/can_cutter/can_cutter.scad");

    #[test]
    fn the_harnesss_parts_are_found_from_its_show_flags() {
        let ps = parts(FIXTURE, &[], &[]);
        let names: Vec<&str> = ps.iter().map(|p| p.name.as_str()).collect();
        for want in ["frame", "stage", "floor", "blade", "can"] {
            assert!(names.contains(&want), "{want} missing from {names:?}");
        }
        assert!(ps.iter().all(|p| p.found == Found::ShowFlag));
    }

    /// Module names are consulted only for names the caller supplied. Every
    /// model is full of modules that are not parts, and a list nobody can use
    /// is worse than no list.
    #[test]
    fn a_module_is_a_part_only_when_the_caller_names_it() {
        let ps = parts(FIXTURE, &[], &["blade_stage"]);
        let p = ps
            .iter()
            .find(|p| p.name == "blade_stage")
            .expect("named modules are looked up");
        assert_eq!(p.found, Found::Module);
        let without = parts(FIXTURE, &[], &[]);
        assert!(
            !without.iter().any(|p| p.name == "blade_stage"),
            "unnamed modules are not guessed at"
        );
    }

    #[test]
    fn a_declared_part_wins_and_needs_no_evidence_in_the_file() {
        let ps = parts("", &["gasket"], &[]);
        assert_eq!(ps.len(), 1);
        assert_eq!(ps[0].found, Found::Declared);
    }

    #[test]
    fn an_export_from_this_source_is_current() {
        let mut e = Exports::new();
        e.record("frame", FIXTURE);
        assert_eq!(e.freshness("frame", FIXTURE), Freshness::Current);
        assert!(e.freshness("frame", FIXTURE).is_trustworthy());
        // SCOPED TO THE ONE PART ON PURPOSE. Passing the fixture here would
        // also pull in its five show_ flags, none of which was recorded, so
        // the report would rightly be non-empty -- my first version of this
        // arm asserted otherwise and failed, which was the arm being wrong
        // rather than the code.
        let only_frame = Part {
            name: "frame".into(),
            found: Found::Declared,
        };
        assert!(
            e.stale(std::slice::from_ref(&only_frame), FIXTURE)
                .is_empty(),
            "a part exported from this exact source is not stale"
        );
    }

    #[test]
    fn an_edit_puts_the_export_behind_and_the_report_says_both_hashes() {
        let mut e = Exports::new();
        e.record("frame", FIXTURE);
        let edited = format!("{FIXTURE}\n// one more line\n");
        match e.freshness("frame", &edited) {
            Freshness::Behind { exported_from, now } => {
                assert_ne!(exported_from, now);
                assert_eq!(exported_from, sha256_hex(FIXTURE.as_bytes()));
            }
            other => panic!("an edited source must leave the export behind, got {other:?}"),
        }
        let report = e.stale(&parts(&edited, &["frame"], &[]), &edited);
        assert!(report.iter().any(|r| r.starts_with("frame:")), "{report:?}");
    }

    /// THE DISTINCTION THIS MODULE EXISTS FOR. An absent record and a matching
    /// record both look like "nothing to do", and only one of them means the
    /// artifact can be trusted.
    #[test]
    fn a_part_never_exported_is_not_current() {
        let e = Exports::new();
        assert_eq!(e.freshness("frame", FIXTURE), Freshness::NeverExported);
        assert!(
            !e.freshness("frame", FIXTURE).is_trustworthy(),
            "never exported is not up to date"
        );
    }

    /// A model nobody has ever exported must not produce an EMPTY staleness
    /// report, because an empty report reads as fully up to date.
    #[test]
    fn a_model_with_no_exports_at_all_reports_every_part() {
        let e = Exports::new();
        let ps = parts(FIXTURE, &[], &[]);
        let report = e.stale(&ps, FIXTURE);
        assert_eq!(
            report.len(),
            ps.len(),
            "every part must be named, not silently omitted"
        );
        assert!(report.iter().all(|r| r.contains("never exported")));
    }

    #[test]
    fn a_part_added_after_the_export_loop_is_still_reported() {
        let mut e = Exports::new();
        e.record("frame", FIXTURE);
        let ps = parts(FIXTURE, &["gasket"], &[]);
        let report = e.stale(&ps, FIXTURE);
        assert!(
            report.iter().any(|r| r.starts_with("gasket:")),
            "the part the hand-written loop would have missed: {report:?}"
        );
        assert!(
            !report.iter().any(|r| r.starts_with("frame:")),
            "and the one that is current is not noise: {report:?}"
        );
    }
}
