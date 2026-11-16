//! The envelope every result travels in. Spec §07.
//!
//! The one rule the whole project exists to enforce: **`Empty` is a value,
//! never an absence.** A query that found nothing returns `status: "empty"`
//! with a timestamp and the hash of the model it ran against. It never
//! returns a missing file, a missing field, or a silence.

use serde::{Deserialize, Serialize};

/// What kind of answer this is. `Failed` never carries a path to an artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Empty,
    Refused,
    Failed,
}

/// Where a number came from. A derived value inherits the WEAKEST of its inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProvenanceKind {
    Measured,
    Literature,
    Derived,
    Assumed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub model_hash: String,
    pub params_hash: String,
    pub kernel: String,
    pub timestamp: String,
}

/// Known-positive controls run beside the real query. A probe that cannot bite
/// reports `bit == 0`, and the caller is told rather than left to infer it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Controls {
    pub ran: u32,
    pub bit: u32,
}

/// What was actually measured, so a pass can never share its representation
/// with "I measured nothing": a verdict whose subject has zero extent is
/// `Refused`, not `Ok`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Subject {
    pub bbox: [[f64; 3]; 2],
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub status: Status,
    pub value: Option<T>,
    pub subject: Option<Subject>,
    pub provenance: Provenance,
    pub controls: Controls,
    pub warnings: Vec<String>,
}

impl<T> Envelope<T> {
    /// An empty result. The value is `None` **and** the status says so; a
    /// consumer that only looks at `value` still cannot mistake this for a
    /// crash, because a crash is `Failed`.
    pub fn empty(provenance: Provenance, controls: Controls) -> Self {
        Envelope {
            status: Status::Empty,
            value: None,
            subject: None,
            provenance,
            controls,
            warnings: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prov() -> Provenance {
        Provenance {
            model_hash: "m".into(),
            params_hash: "p".into(),
            kernel: "openscad 2021.01".into(),
            timestamp: "2026-11-14T00:00:00Z".into(),
        }
    }

    /// Spec §07, and seven logged incidents: emptiness is a serialized VALUE.
    /// Mutation check: change `Status::Empty` to `Status::Ok` in `empty()` and
    /// this fails; delete the `status` field and it fails to compile.
    #[test]
    fn empty_is_a_value_on_the_wire() {
        let e: Envelope<f64> = Envelope::empty(prov(), Controls { ran: 1, bit: 1 });
        let json = serde_json::to_value(&e).expect("serializes");
        assert_eq!(json["status"], "empty");
        assert!(
            json.get("status").is_some(),
            "status must be present, not absent"
        );
        assert_eq!(json["value"], serde_json::Value::Null);
    }

    /// Provenance ordering is the weakest-input rule: Assumed > Derived > Literature > Measured.
    #[test]
    fn weakest_provenance_is_the_maximum() {
        use ProvenanceKind::*;
        let inputs = [Measured, Literature, Assumed, Derived];
        assert_eq!(inputs.iter().max(), Some(&Assumed));
        assert_eq!([Measured, Literature].iter().max(), Some(&Literature));
    }
}

// ---------------------------------------------------------------------------
// Deriving a status from what the provider said. Spec §07.
// ---------------------------------------------------------------------------

/// The facts a status is derived from, stated as a struct so the derivation is
/// a pure function of named inputs rather than a chain of `if let` over a JSON
/// blob. `wm-provider` fills this in; this crate does not depend on it, so the
/// rule lives below the client rather than inside it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Observed<'a> {
    /// The provider's own `empty` flag. NOT a zero volume: a 2D result or a
    /// degenerate solid measures 0.0 and is present.
    pub empty: bool,
    /// The provider flagged the call itself as an error.
    pub is_error: bool,
    /// `validate`'s verdict, where there was one.
    pub valid: Option<bool>,
    /// A value the model could not resolve. The provider writes `UNRESOLVED`
    /// where a variable had no binding, and a number derived from one of those
    /// is not a measurement.
    pub unresolved: bool,
    pub errors: &'a [String],
    /// What was actually measured. `None` means the provider reported no
    /// extent at all.
    pub bbox: Option<[[f64; 3]; 2]>,
}

impl Status {
    /// Derive the verdict. The order of these tests is the whole content of
    /// the function, so it is stated rather than implied:
    ///
    /// 1. **A call that errored is `Failed`.** Nothing it returned is a
    ///    measurement, including anything that looks like one.
    /// 2. **An unresolved input is `Refused`.** A number computed from a
    ///    binding that did not exist is not wrong, it is unanswerable, and
    ///    those are different and must stay different.
    /// 3. **`valid: false` is `Refused`.** The model did not compile cleanly;
    ///    the kernel's output is about something other than what was asked.
    /// 4. **A subject with ZERO EXTENT is `Refused`, never `Ok`.** This is the
    ///    rule the milestone names, and it is here rather than at a call site
    ///    because it was missed at a call site: the overhang scan reported
    ///    PASS on a part 80 mm outside the probe box (mcp note 30). A verdict
    ///    about nothing is not a pass.
    /// 5. **The provider's empty flag is `Empty`,** which is a value.
    /// 6. Otherwise `Ok`.
    ///
    /// EMPTY IS CHECKED AFTER ZERO EXTENT ON PURPOSE. A genuinely empty result
    /// also has no extent, and the honest word for it is `Empty` -- but a
    /// result that claims geometry while measuring nothing is the dangerous
    /// one, so the ordering only reaches `Empty` when the provider said so.
    pub fn derive(o: &Observed<'_>) -> Status {
        if o.is_error || !o.errors.is_empty() {
            return Status::Failed;
        }
        if o.unresolved {
            return Status::Refused;
        }
        if o.valid == Some(false) {
            return Status::Refused;
        }
        if o.empty {
            return Status::Empty;
        }
        match o.bbox {
            None => Status::Refused,
            Some(b) if zero_extent(&b) => Status::Refused,
            Some(_) => Status::Ok,
        }
    }
}

/// True when a bounding box has no extent in at least one axis.
///
/// ONE FLAT AXIS IS ENOUGH. A box that is 10 x 10 x 0 has volume zero and
/// nothing can be measured through it, so treating only the all-zero box as
/// degenerate would let a flattened subject through as `Ok`.
pub fn zero_extent(bbox: &[[f64; 3]; 2]) -> bool {
    (0..3).any(|i| (bbox[1][i] - bbox[0][i]).abs() <= f64::EPSILON)
}

impl Subject {
    /// Build a subject from a bounding box, hashing it so two verdicts about
    /// the same extent are comparable.
    pub fn from_bbox(bbox: [[f64; 3]; 2]) -> Self {
        let text = format!("{bbox:?}");
        Subject {
            bbox,
            hash: sha256_hex(text.as_bytes()),
        }
    }

    pub fn is_degenerate(&self) -> bool {
        zero_extent(&self.bbox)
    }
}

impl Provenance {
    /// Hash the model text and the parameters, and record the kernel that
    /// answered. A number without these cannot be invalidated later, which is
    /// the whole of failure class C.
    pub fn of(model: &str, params: &str, kernel: &str, timestamp: &str) -> Self {
        Provenance {
            model_hash: sha256_hex(model.as_bytes()),
            params_hash: sha256_hex(params.as_bytes()),
            kernel: kernel.to_string(),
            timestamp: timestamp.to_string(),
        }
    }
}

/// Lowercase hex SHA-256.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// RFC 3339 UTC from Unix epoch seconds.
///
/// Hand-written because the alternative is a date dependency for one line of
/// output, and because the civil-from-days algorithm is short enough to test
/// against known instants rather than trusted.
pub fn rfc3339_utc(epoch_secs: i64) -> String {
    let days = epoch_secs.div_euclid(86_400);
    let secs = epoch_secs.rem_euclid(86_400);
    // Howard Hinnant's civil_from_days, shifted to a March-based year.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

#[cfg(test)]
mod derive_tests {
    use super::*;

    fn obs<'a>(errors: &'a [String]) -> Observed<'a> {
        Observed {
            bbox: Some([[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]]),
            errors,
            ..Default::default()
        }
    }

    const NONE: &[String] = &[];

    #[test]
    fn a_measured_solid_is_ok() {
        assert_eq!(Status::derive(&obs(NONE)), Status::Ok);
    }

    /// THE RULE THIS ISSUE EXISTS FOR. The overhang scan reported PASS on a
    /// part 80 mm outside the probe box: a verdict about nothing read as a
    /// clean verdict.
    #[test]
    fn a_subject_with_zero_extent_is_refused_never_ok() {
        let flat = Observed {
            bbox: Some([[0.0, 0.0, 0.0], [10.0, 10.0, 0.0]]),
            ..obs(NONE)
        };
        assert_eq!(
            Status::derive(&flat),
            Status::Refused,
            "a flattened subject is not a pass"
        );
        let nothing = Observed {
            bbox: None,
            ..obs(NONE)
        };
        assert_eq!(Status::derive(&nothing), Status::Refused);
    }

    /// One flat axis is enough; a box with volume zero cannot be measured
    /// through whatever its other two axes say.
    #[test]
    fn one_flat_axis_is_degenerate() {
        assert!(zero_extent(&[[0.0, 0.0, 0.0], [10.0, 10.0, 0.0]]));
        assert!(zero_extent(&[[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]]));
        assert!(!zero_extent(&[[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]]));
    }

    #[test]
    fn the_providers_empty_flag_is_empty_not_refused() {
        let e = Observed {
            empty: true,
            bbox: Some([[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]]),
            ..obs(NONE)
        };
        assert_eq!(
            Status::derive(&e),
            Status::Empty,
            "a result the provider called empty is Empty, not Refused"
        );
    }

    #[test]
    fn an_errored_call_is_failed_whatever_else_it_returned() {
        let e = Observed {
            is_error: true,
            ..obs(NONE)
        };
        assert_eq!(Status::derive(&e), Status::Failed);
        let msgs = vec!["ERROR: parse error".to_string()];
        assert_eq!(Status::derive(&obs(&msgs)), Status::Failed);
    }

    /// An unresolved binding is UNANSWERABLE, not wrong, and the two must not
    /// share a representation.
    #[test]
    fn an_unresolved_input_is_refused_not_failed() {
        let u = Observed {
            unresolved: true,
            ..obs(NONE)
        };
        assert_eq!(Status::derive(&u), Status::Refused);
    }

    #[test]
    fn an_invalid_model_is_refused() {
        let v = Observed {
            valid: Some(false),
            ..obs(NONE)
        };
        assert_eq!(Status::derive(&v), Status::Refused);
        let ok = Observed {
            valid: Some(true),
            ..obs(NONE)
        };
        assert_eq!(Status::derive(&ok), Status::Ok);
    }

    /// A failure outranks everything, including emptiness: a call that errored
    /// returned nothing trustworthy, and "empty" would read as an answer.
    #[test]
    fn failed_outranks_empty() {
        let both = Observed {
            is_error: true,
            empty: true,
            ..obs(NONE)
        };
        assert_eq!(Status::derive(&both), Status::Failed);
    }

    #[test]
    fn a_subject_carries_its_box_and_a_hash_of_it() {
        let s = Subject::from_bbox([[0.0, 0.0, 0.0], [1.0, 2.0, 3.0]]);
        assert_eq!(s.hash.len(), 64, "sha-256 hex is 64 characters");
        assert!(!s.is_degenerate());
        let same = Subject::from_bbox([[0.0, 0.0, 0.0], [1.0, 2.0, 3.0]]);
        assert_eq!(s.hash, same.hash, "the same extent hashes the same");
        let other = Subject::from_bbox([[0.0, 0.0, 0.0], [1.0, 2.0, 4.0]]);
        assert_ne!(s.hash, other.hash, "a different extent hashes differently");
    }

    /// A KNOWN VECTOR, so the hash is checked against the algorithm and not
    /// merely against itself: SHA-256 of the empty input.
    #[test]
    fn sha256_matches_a_published_vector() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn provenance_hashes_the_model_and_the_params() {
        let p = Provenance::of(
            "cube(10);",
            "{}",
            "openscad 2026.10.14",
            "2026-11-16T00:00:00Z",
        );
        assert_eq!(p.model_hash, sha256_hex(b"cube(10);"));
        assert_eq!(p.params_hash, sha256_hex(b"{}"));
        assert_ne!(
            p.model_hash, p.params_hash,
            "two different inputs must not share a hash"
        );
        assert_eq!(p.kernel, "openscad 2026.10.14");
    }

    /// Known instants, so the date arithmetic is checked rather than trusted.
    #[test]
    fn rfc3339_matches_known_instants() {
        assert_eq!(rfc3339_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339_utc(1_000_000_000), "2001-09-09T01:46:40Z");
        // a leap day, which is where a hand-written calendar goes wrong
        assert_eq!(rfc3339_utc(1_709_164_800), "2024-02-29T00:00:00Z");
        assert_eq!(rfc3339_utc(1_763_251_200), "2025-11-16T00:00:00Z");
    }
}
