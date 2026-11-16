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

// ---------------------------------------------------------------------------
// The critical angle. Spec §08.
// ---------------------------------------------------------------------------

/// The shallowest angle from horizontal that is self-supporting, in degrees.
///
/// `theta = atan(layer_h / (0.5 * extrusion_width))`
///
/// A WALL IS SELF-SUPPORTING WHEN ITS PER-LAYER STEP OUTWARD STAYS UNDER ABOUT
/// HALF AN EXTRUSION WIDTH, so each bead still lands on the one below. That
/// makes the threshold a property of the PRINT and not of the model, which is
/// the whole reason it belongs in Layer 4.
///
/// # 45 degrees is folklore with one true case
///
/// 0.2 mm layers at 0.4 mm width make the ratio exactly 1, and atan(1) is 45.
/// The number is correct there and nowhere else by right: 0.1 mm layers at the
/// same width hold 26.57 degrees, and 0.15 mm hold 36.87. Quoting 45 without
/// the layer height is quoting half a fact.
///
/// # It reads the WIDTH, not the nozzle
///
/// A 0.4 mm nozzle commonly extrudes 0.42 to 0.48. Reading the nozzle instead
/// would be a different, slightly wrong number that looks just as plausible.
pub fn critical_angle_deg(p: &Process) -> Result<f64, Missing> {
    let h = p.need("layer_h", "critical_angle")?.value;
    let w = p.need("extrusion_width", "critical_angle")?.value;
    // A NON-POSITIVE OR NON-FINITE INPUT IS NOT A SHALLOW ANGLE. atan of a
    // zero or negative ratio returns a perfectly good number that means
    // nothing here, so it is refused rather than reported.
    //
    // `is_finite` IS CHECKED FIRST AND EXPLICITLY. This was `!(h > 0.0)`,
    // which catches NaN as a side effect of the negation -- clippy refused it
    // as hard to read, and clippy was right twice over: the NaN case was
    // load-bearing and invisible. A plain `h <= 0.0` would let NaN through,
    // since every comparison with NaN is false.
    if !h.is_finite() || !w.is_finite() || h <= 0.0 || w <= 0.0 {
        return Err(Missing {
            wanted: format!("a positive layer_h and extrusion_width (got {h} and {w})"),
            by: "critical_angle",
        });
    }
    Ok((h / (0.5 * w)).atan().to_degrees())
}

/// Is a surface at `angle_deg` from horizontal self-supporting under `p`?
///
/// REFUSES WITHOUT A DECLARED PROCESS, which is the point of the function
/// existing beside the angle: the tempting call is "is 40 degrees fine?" and
/// the honest answer is "under which layer height?".
pub fn is_self_supporting(p: &Process, angle_deg: f64) -> Result<bool, Missing> {
    Ok(angle_deg >= critical_angle_deg(p)?)
}

#[cfg(test)]
mod critical_angle_tests {
    use super::*;

    fn proc_with(h: f64, w: f64) -> Process {
        let mut p = Process::new();
        p.layer_h = Some(Declared::measured(h, "the profile"));
        p.extrusion_width = Some(Declared::measured(w, "the profile"));
        p
    }

    /// Three hand-computed points, including the one that made the folklore.
    #[test]
    fn the_angle_is_a_property_of_the_print() {
        let cases = [
            (0.2, 0.4, 45.000_000),
            (0.1, 0.4, 26.565_051),
            (0.15, 0.4, 36.869_898),
        ];
        for (h, w, want) in cases {
            let got = critical_angle_deg(&proc_with(h, w)).expect("declared");
            assert!(
                (got - want).abs() < 1e-5,
                "{h} mm at {w} mm should be {want} deg, got {got}"
            );
        }
    }

    /// 45 IS TRUE IN EXACTLY ONE CASE. The arm exists to pin that the value
    /// moves with the layer height rather than sitting at the folklore number.
    #[test]
    fn forty_five_is_not_a_constant() {
        let coarse = critical_angle_deg(&proc_with(0.2, 0.4)).expect("declared");
        let fine = critical_angle_deg(&proc_with(0.1, 0.4)).expect("declared");
        assert!((coarse - 45.0).abs() < 1e-9);
        assert!(
            fine < coarse - 15.0,
            "finer layers tolerate SHALLOWER overhangs: {fine} should be well under {coarse}"
        );
    }

    /// It reads the extrusion width. A 0.4 nozzle extruding 0.46 gives a
    /// different, lower angle, and reading the nozzle would look just as
    /// plausible while being wrong.
    #[test]
    fn it_reads_the_width_and_not_the_nozzle() {
        let mut p = proc_with(0.2, 0.46);
        p.nozzle = Some(Declared::measured(0.4, "the installed nozzle"));
        let got = critical_angle_deg(&p).expect("declared");
        assert!(
            (got - 45.0).abs() > 1e-3,
            "a 0.46 width must not give the 0.4-nozzle answer, got {got}"
        );
        let want = (0.2f64 / (0.5 * 0.46)).atan().to_degrees();
        assert!((got - want).abs() < 1e-9);
    }

    #[test]
    fn it_refuses_without_a_declared_layer_height() {
        let mut p = Process::new();
        p.extrusion_width = Some(Declared::measured(0.4, "the profile"));
        let e = critical_angle_deg(&p).expect_err("must refuse");
        assert_eq!(e.wanted, "layer_h");
        assert_eq!(e.by, "critical_angle");
    }

    #[test]
    fn it_refuses_without_a_declared_width() {
        let mut p = Process::new();
        p.layer_h = Some(Declared::measured(0.2, "the profile"));
        assert_eq!(
            critical_angle_deg(&p).expect_err("must refuse").wanted,
            "extrusion_width"
        );
    }

    /// A zero or negative input yields a perfectly good atan that means
    /// nothing. Refused rather than reported.
    #[test]
    fn a_nonpositive_input_is_refused_not_reported_as_zero_degrees() {
        for (h, w) in [(0.0, 0.4), (0.2, 0.0), (-0.2, 0.4), (f64::NAN, 0.4)] {
            let e = critical_angle_deg(&proc_with(h, w)).expect_err("must refuse");
            assert!(e.wanted.contains("positive"), "{}", e.wanted);
        }
    }

    /// The tempting question is "is 40 degrees fine?" and the honest answer is
    /// "under which layer height?" -- so the verdict refuses too.
    #[test]
    fn a_self_supporting_verdict_refuses_without_a_process() {
        assert!(is_self_supporting(&Process::new(), 40.0).is_err());
        let p = proc_with(0.15, 0.4);
        assert!(
            is_self_supporting(&p, 40.0).expect("declared"),
            "40 > 36.87"
        );
        assert!(
            !is_self_supporting(&p, 30.0).expect("declared"),
            "30 < 36.87"
        );
    }

    /// 40 degrees is self-supporting at 0.15 mm and NOT at 0.1 mm, on the same
    /// geometry. One verdict, two answers, and the process is the difference.
    #[test]
    fn the_same_angle_gets_opposite_verdicts_under_different_processes() {
        assert!(!is_self_supporting(&proc_with(0.2, 0.4), 44.0).expect("declared"));
        assert!(is_self_supporting(&proc_with(0.1, 0.4), 44.0).expect("declared"));
    }
}
