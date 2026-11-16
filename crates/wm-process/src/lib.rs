//! Layer 4: the printer, material, firmware and slicer as a model. Spec §05, §08.
//!
//! # Everything here refuses without a declaration
//!
//! Spec §08. A Layer 4 answer is about a PROCESS, not about a shape: whether a
//! wall is self-supporting depends on the layer height, whether a retraction
//! speed is reachable depends on a firmware acceleration, whether a thickness
//! is what the model says depends on quantization. None of those is a property
//! of the geometry, and every one of them was once assumed.
//!
//! So a missing input is a REFUSAL and never a default. The famous instance is
//! "45 degrees": it is true at 0.2 mm layers with a 0.4 mm nozzle because that
//! ratio happens to be 1, and it is 26.57 degrees at 0.1 mm. Quoting 45 without
//! the layer height is quoting half a fact, and a library that defaults to it
//! is the thing that makes the other half invisible.

use wm_envelope::ProvenanceKind;

pub const LAYER: &str = "wm-process";

/// One declared quantity: its value, where it came from, and who said so.
///
/// THE SOURCE IS NOT DECORATION. `Measured` from a caliper and `Assumed` from
/// a datasheet's typical value are different grounds for the same number, and
/// a derived result inherits the WEAKEST of its inputs. Without the kind
/// travelling with the value, that rule has nothing to read.
#[derive(Debug, Clone, PartialEq)]
pub struct Declared<T> {
    pub value: T,
    pub kind: ProvenanceKind,
    pub source: String,
}

impl<T> Declared<T> {
    pub fn new(value: T, kind: ProvenanceKind, source: impl Into<String>) -> Self {
        Declared {
            value,
            kind,
            source: source.into(),
        }
    }
    pub fn measured(value: T, source: impl Into<String>) -> Self {
        Self::new(value, ProvenanceKind::Measured, source)
    }
    pub fn literature(value: T, source: impl Into<String>) -> Self {
        Self::new(value, ProvenanceKind::Literature, source)
    }
    pub fn assumed(value: T, source: impl Into<String>) -> Self {
        Self::new(value, ProvenanceKind::Assumed, source)
    }
}

/// Why a Layer 4 call could not be answered.
///
/// NAMES THE FIELD AND THE CALLER. "insufficient data" sends somebody reading
/// source; "support needs layer_h, which this process does not declare" sends
/// them to the declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Missing {
    /// A String and not a `&'static str`: the unknown-field case below has to
    /// name a field that is not in any static list. The first version reached
    /// for `Box::leak` to keep the borrow static, which leaks on every typo --
    /// a deliberate leak on an ERROR path, where the caller is already having
    /// a bad day.
    pub wanted: String,
    pub by: &'static str,
}

impl std::fmt::Display for Missing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} needs {}, which this process does not declare; refusing rather than assuming one",
            self.by, self.wanted
        )
    }
}

/// Firmware limits. Separate from the printer because they are the inputs
/// nobody owns: a slicer asks for a speed, the firmware delivers whatever its
/// acceleration allows, and no part of the model mentions the difference.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Firmware {
    /// mm/s^2 for moves of the extruder alone. On the harness printer nobody
    /// set this and Klipper computed 399 from max_accel.
    pub max_extrude_only_accel: Option<Declared<f64>>,
    pub max_accel: Option<Declared<f64>>,
    pub max_velocity: Option<Declared<f64>>,
}

/// The process. Every field is optional because an undeclared field must be
/// REFUSABLE: making them required with defaults would put the assumption
/// back, one layer down.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Process {
    pub material: Option<Declared<String>>,
    /// Young's modulus, MPa.
    pub e_mpa: Option<Declared<f64>>,
    /// Yield stress, MPa.
    pub yield_mpa: Option<Declared<f64>>,
    pub layer_h: Option<Declared<f64>>,
    pub nozzle: Option<Declared<f64>>,
    /// Extrusion width. NOT the nozzle diameter: a 0.4 mm nozzle commonly
    /// extrudes 0.42 to 0.48, and the critical angle reads THIS.
    pub extrusion_width: Option<Declared<f64>>,
    /// Bed size, mm, x by y, and usable z.
    pub bed: Option<Declared<[f64; 3]>>,
    pub firmware: Firmware,
}

impl Process {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read a declared f64 or refuse, naming the caller.
    pub fn need(&self, field: &'static str, by: &'static str) -> Result<&Declared<f64>, Missing> {
        let got = match field {
            "e_mpa" => self.e_mpa.as_ref(),
            "yield_mpa" => self.yield_mpa.as_ref(),
            "layer_h" => self.layer_h.as_ref(),
            "nozzle" => self.nozzle.as_ref(),
            "extrusion_width" => self.extrusion_width.as_ref(),
            "max_extrude_only_accel" => self.firmware.max_extrude_only_accel.as_ref(),
            "max_accel" => self.firmware.max_accel.as_ref(),
            "max_velocity" => self.firmware.max_velocity.as_ref(),
            // AN UNKNOWN FIELD NAME IS A REFUSAL, NOT AN ABSENCE. A typo would
            // otherwise read as "the process does not declare it", which is a
            // true sentence about the wrong thing.
            other => {
                return Err(Missing {
                    wanted: format!("{other} (no such field: this is a bug in the caller)"),
                    by,
                })
            }
        };
        got.ok_or_else(|| Missing {
            wanted: field.to_string(),
            by,
        })
    }

    /// The weakest provenance among the named fields, which is what any result
    /// derived from them inherits.
    ///
    /// AN ABSENT FIELD CONTRIBUTES NOTHING HERE ON PURPOSE: callers reach this
    /// only after `need` has already refused on anything missing, and silently
    /// treating absence as `Assumed` would let a refusal become a weak answer.
    pub fn weakest(&self, fields: &[&'static str]) -> Option<ProvenanceKind> {
        fields
            .iter()
            .filter_map(|f| self.need(f, "weakest").ok().map(|d| d.kind))
            .max()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn petg() -> Process {
        let mut p = Process::new();
        p.material = Some(Declared::measured("PETG".to_string(), "the spool"));
        p.e_mpa = Some(Declared::literature(2000.0, "datasheet, typical"));
        p.layer_h = Some(Declared::measured(0.15, "the profile in use"));
        p.nozzle = Some(Declared::measured(0.4, "the installed nozzle"));
        p.extrusion_width = Some(Declared::assumed(0.4, "slicer default, unverified"));
        p
    }

    #[test]
    fn layer_is_named() {
        assert!(!LAYER.is_empty());
    }

    #[test]
    fn a_declared_field_is_readable_with_its_provenance() {
        let p = petg();
        let d = p.need("layer_h", "support").expect("declared");
        assert_eq!(d.value, 0.15);
        assert_eq!(d.kind, ProvenanceKind::Measured);
        assert_eq!(d.source, "the profile in use");
    }

    /// THE RULE OF THIS LAYER. A missing input is refused and never defaulted.
    #[test]
    fn an_undeclared_field_is_refused_and_names_the_caller() {
        let p = petg();
        let e = p.need("yield_mpa", "beam margin").expect_err("must refuse");
        assert_eq!(e.wanted, "yield_mpa");
        assert_eq!(e.by, "beam margin");
        let said = e.to_string();
        assert!(said.contains("refusing rather than assuming"), "{said}");
        assert!(
            said.contains("beam margin") && said.contains("yield_mpa"),
            "the refusal must name both the caller and the field: {said}"
        );
    }

    /// A typo in a field name is a BUG IN THE CALLER, and saying "the process
    /// does not declare it" would be a true sentence about the wrong thing.
    #[test]
    fn an_unknown_field_name_says_it_is_a_caller_bug() {
        let p = petg();
        let e = p.need("layer_height", "support").expect_err("must refuse");
        assert!(
            e.wanted.contains("no such field"),
            "a misspelled field must not read as an undeclared one: {}",
            e.wanted
        );
    }

    /// A derived result inherits the WEAKEST of its inputs: an assumed
    /// extrusion width beside a measured layer height makes the answer assumed.
    #[test]
    fn the_weakest_provenance_wins() {
        let p = petg();
        assert_eq!(
            p.weakest(&["layer_h", "extrusion_width"]),
            Some(ProvenanceKind::Assumed),
            "measured beside assumed is assumed"
        );
        assert_eq!(
            p.weakest(&["layer_h", "nozzle"]),
            Some(ProvenanceKind::Measured)
        );
        assert_eq!(
            p.weakest(&["layer_h", "e_mpa"]),
            Some(ProvenanceKind::Literature)
        );
    }

    /// An empty process declares nothing, and `weakest` over nothing is None
    /// rather than a cheerful `Measured`.
    #[test]
    fn an_empty_process_has_no_provenance_to_report() {
        let p = Process::new();
        assert_eq!(p.weakest(&["layer_h"]), None);
        assert!(p.need("layer_h", "anything").is_err());
    }

    /// Extrusion width is NOT the nozzle diameter, and the critical angle
    /// reads the width. Conflating them is how 45 degrees becomes folklore.
    #[test]
    fn extrusion_width_is_its_own_field() {
        let mut p = petg();
        p.extrusion_width = Some(Declared::measured(0.46, "measured on a single wall"));
        assert_ne!(
            p.need("extrusion_width", "t").expect("declared").value,
            p.need("nozzle", "t").expect("declared").value,
            "a 0.4 nozzle commonly extrudes wider than 0.4"
        );
    }
}
