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
