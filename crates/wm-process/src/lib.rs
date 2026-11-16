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

pub mod support;
pub mod wire;

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

// ---------------------------------------------------------------------------
// Quantization. A thickness is a whole number of layers.
// ---------------------------------------------------------------------------

/// What a nominal thickness actually becomes when printed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantized {
    pub nominal: f64,
    pub layers: u32,
    /// `layers * layer_h`: the thickness that will exist.
    pub actual: f64,
    /// `actual - nominal`, signed. Negative means thinner than asked.
    pub error: f64,
    /// `(actual / nominal)^3 - 1`, the change in BENDING STIFFNESS, signed.
    /// Negative is a loss.
    pub stiffness_error: f64,
}

impl Quantized {
    /// True when the nominal thickness was already a whole number of layers.
    pub fn is_exact(&self) -> bool {
        self.error.abs() <= 1e-9
    }
}

/// Quantize a thickness in the BENDING direction to whole layers.
///
/// # Why the cubed error is the number that matters
///
/// Bending stiffness goes as thickness CUBED, so a quantization that looks
/// like rounding noise is not. The harness incident: a 0.9 mm leaf at 0.20 mm
/// layers is 4.5 layers, which cannot exist. It prints as 4 layers, 0.80 mm --
/// an 11.1% loss of thickness and a **29.77% loss of stiffness**. The clamp
/// that was designed on 0.9 mm delivered about seven tenths of its force, and
/// nothing in the model said so.
///
/// Reporting only `error` would show 0.1 mm and invite the reader to shrug.
///
/// # Rounding is to NEAREST, and that is a choice worth stating
///
/// A slicer lays whole layers; 4.5 goes to 4 or 5 depending on the slicer and
/// the surrounding geometry. Nearest is reported as the expectation, and
/// [`quantize_floor`] is available for the conservative reading. Ties go DOWN,
/// because the thinner outcome is the one that loses force, and a tool whose
/// optimistic rounding hides a 30% loss is worse than no tool.
pub fn quantize(p: &Process, nominal: f64) -> Result<Quantized, Missing> {
    let h = p.need("layer_h", "quantize")?.value;
    quantize_with(h, nominal, false)
}

/// Quantize downward: the conservative reading, never more material than asked.
pub fn quantize_floor(p: &Process, nominal: f64) -> Result<Quantized, Missing> {
    let h = p.need("layer_h", "quantize")?.value;
    quantize_with(h, nominal, true)
}

fn quantize_with(h: f64, nominal: f64, floor: bool) -> Result<Quantized, Missing> {
    if !h.is_finite() || h <= 0.0 || !nominal.is_finite() || nominal <= 0.0 {
        return Err(Missing {
            wanted: format!("a positive, finite layer_h and nominal (got {h} and {nominal})"),
            by: "quantize",
        });
    }
    let exact = nominal / h;
    // TIES GO DOWN. `f64::round` rounds half AWAY from zero, so 4.5 would
    // become 5 and the 0.9 mm incident would report no loss at all -- the
    // optimistic answer, for the case that produced the rule.
    let n = if floor {
        exact.floor()
    } else {
        let r = exact.fract();
        if (r - 0.5).abs() <= 1e-9 {
            exact.floor()
        } else {
            exact.round()
        }
    };
    let layers = n.max(1.0) as u32;
    let actual = f64::from(layers) * h;
    Ok(Quantized {
        nominal,
        layers,
        actual,
        error: actual - nominal,
        stiffness_error: (actual / nominal).powi(3) - 1.0,
    })
}

#[cfg(test)]
mod quantize_tests {
    use super::*;

    fn at(h: f64) -> Process {
        let mut p = Process::new();
        p.layer_h = Some(Declared::measured(h, "the profile"));
        p
    }

    /// THE HARNESS INCIDENT, as the vector. 0.9 mm at 0.20 is 4.5 layers,
    /// which cannot exist; it prints 0.80 and loses 29.77% of its stiffness.
    #[test]
    fn the_point_nine_leaf_loses_thirty_percent_of_its_stiffness() {
        let q = quantize(&at(0.20), 0.9).expect("declared");
        assert_eq!(q.layers, 4, "4.5 layers cannot exist and ties go DOWN");
        assert!((q.actual - 0.80).abs() < 1e-12);
        assert!((q.error - -0.10).abs() < 1e-12, "0.1 mm thinner");
        assert!(
            (q.stiffness_error - -0.297_668).abs() < 1e-6,
            "cubed, that is a 29.77% loss, got {}",
            q.stiffness_error
        );
        assert!(!q.is_exact());
    }

    /// REPORTING ONLY THE THICKNESS ERROR INVITES A SHRUG. 0.1 mm sounds like
    /// nothing; the cube is what makes it a third of the clamp.
    #[test]
    fn the_cubed_error_is_far_larger_than_the_thickness_error() {
        let q = quantize(&at(0.20), 0.9).expect("declared");
        let thickness_frac = (q.error / q.nominal).abs();
        assert!(
            q.stiffness_error.abs() > thickness_frac * 2.5,
            "11.1% of thickness is 29.8% of stiffness: {thickness_frac} vs {}",
            q.stiffness_error
        );
    }

    /// TIES GO DOWN, and this is the arm that pins it. `f64::round` rounds
    /// half away from zero, so 4.5 would become 5, `actual` would be 1.0, and
    /// the incident would report a stiffness GAIN.
    #[test]
    fn a_tie_rounds_down_rather_than_reporting_a_gain() {
        let q = quantize(&at(0.20), 0.9).expect("declared");
        assert!(
            q.stiffness_error < 0.0,
            "the 0.9 mm case must report a LOSS, got {}",
            q.stiffness_error
        );
    }

    #[test]
    fn an_exact_multiple_has_no_error_at_all() {
        let q = quantize(&at(0.20), 0.8).expect("declared");
        assert_eq!(q.layers, 4);
        assert!(q.is_exact());
        assert!(q.error.abs() < 1e-12 && q.stiffness_error.abs() < 1e-12);
    }

    /// Rounding UP is a real outcome too, and it is a stiffness GAIN. A tool
    /// that only ever reported losses would be wrong half the time.
    #[test]
    fn rounding_up_reports_a_gain() {
        let q = quantize(&at(0.20), 0.75).expect("declared");
        assert_eq!(q.layers, 4, "3.75 rounds to 4");
        assert!(q.actual > q.nominal);
        assert!(q.stiffness_error > 0.0, "thicker is stiffer");
    }

    /// The conservative reading never gives more material than asked.
    #[test]
    fn the_floor_reading_never_exceeds_the_nominal() {
        let q = quantize_floor(&at(0.20), 0.75).expect("declared");
        assert_eq!(q.layers, 3);
        assert!(q.actual <= q.nominal);
        assert!(q.stiffness_error < 0.0);
    }

    /// A thickness under one layer still prints as ONE layer. Reporting zero
    /// would say the feature does not exist, when what actually happens is
    /// that it comes out far too thick.
    #[test]
    fn a_sub_layer_thickness_becomes_one_layer_not_zero() {
        let q = quantize(&at(0.20), 0.05).expect("declared");
        assert_eq!(q.layers, 1);
        assert!((q.actual - 0.20).abs() < 1e-12);
        assert!(
            q.stiffness_error > 50.0,
            "0.05 asked, 0.20 printed, 64x stiffer"
        );
    }

    #[test]
    fn it_refuses_without_a_declared_layer_height() {
        assert_eq!(
            quantize(&Process::new(), 0.9)
                .expect_err("must refuse")
                .wanted,
            "layer_h"
        );
    }

    #[test]
    fn a_nonpositive_or_nan_nominal_is_refused() {
        for n in [0.0, -0.9, f64::NAN] {
            assert!(quantize(&at(0.2), n).is_err(), "{n} must be refused");
        }
    }
}

// ---------------------------------------------------------------------------
// Firmware: commanded is not achieved. Spec §05.
// ---------------------------------------------------------------------------

/// What actually happens when a speed is asked for.
///
/// `commanded` and `achieved` are separate fields on purpose. A result that
/// returned one number would be read as the speed, and the whole point is that
/// there are two of them and they differ.
#[derive(Debug, Clone, PartialEq)]
pub struct Reachable {
    /// What the slicer asked for.
    pub commanded: f64,
    /// What the move actually reaches.
    pub achieved: f64,
    /// Which limit decided. `None` means the command governed: nothing got in
    /// its way.
    pub governed_by: Option<String>,
    /// `achieved / commanded`. 1.0 when the command was met.
    pub fraction: f64,
}

impl Reachable {
    /// True when the firmware, not the command, decided the outcome.
    pub fn was_capped(&self) -> bool {
        self.governed_by.is_some()
    }
}

/// The fastest a retraction of `length_mm` can actually go.
///
/// # The physics, and why length is the governing term
///
/// A retraction accelerates from rest and decelerates to rest over its whole
/// length, so its peak is `v = sqrt(a * L)`. The acceleration is
/// `max_extrude_only_accel`, which applies to moves of the extruder alone.
/// Nothing in that expression is the commanded speed: a SHORTER retraction is
/// a SLOWER one, which is the opposite of the intuition that short moves are
/// quick.
///
/// # The harness instance
///
/// Nobody set `max_extrude_only_accel` on the printer, so Klipper computed 399
/// mm/s^2 from `max_accel`. The PETG profile asks for 40 mm/s at 0.8 mm and
/// gets **17.87**. Every retraction speed above about 20 mm/s on that machine
/// is decorative, and no part of the slicer's output says so.
pub fn reachable_retract_speed(
    p: &Process,
    commanded: f64,
    length_mm: f64,
) -> Result<Reachable, Missing> {
    let a = p
        .need("max_extrude_only_accel", "reachable_retract_speed")?
        .value;
    if !a.is_finite()
        || a <= 0.0
        || !length_mm.is_finite()
        || length_mm <= 0.0
        || !commanded.is_finite()
        || commanded <= 0.0
    {
        return Err(Missing {
            wanted: format!(
                "positive, finite accel, length and command (got {a}, {length_mm}, {commanded})"
            ),
            by: "reachable_retract_speed",
        });
    }
    let ceiling = (a * length_mm).sqrt();
    let (achieved, governed_by) = if commanded <= ceiling {
        (commanded, None)
    } else {
        (
            ceiling,
            Some(format!(
                "max_extrude_only_accel {a} mm/s^2 over {length_mm} mm: v = sqrt(a*L)"
            )),
        )
    };
    Ok(Reachable {
        commanded,
        achieved,
        fraction: achieved / commanded,
        governed_by,
    })
}

#[cfg(test)]
mod firmware_tests {
    use super::*;

    fn klipper_default() -> Process {
        let mut p = Process::new();
        p.firmware.max_extrude_only_accel = Some(Declared::measured(
            399.0,
            "Klipper computed it from max_accel; nobody set it (kobra-neo.md)",
        ));
        p
    }

    /// THE THREE PUBLISHED ROWS, recomputed rather than copied: the note
    /// rounds to 15.5, 17.9 and 20.0.
    #[test]
    fn the_harness_printers_reachable_retract_speeds() {
        for (len, want) in [(0.6, 15.472_6), (0.8, 17.866_2), (1.0, 19.975_0)] {
            let r = reachable_retract_speed(&klipper_default(), 40.0, len).expect("declared");
            assert!(
                (r.achieved - want).abs() < 1e-3,
                "{len} mm should reach {want}, got {}",
                r.achieved
            );
        }
    }

    /// THE HEADLINE. The PETG profile asks 40 at 0.8 mm and gets 17.87, which
    /// is 45% of what it asked for, and the slicer says nothing.
    #[test]
    fn forty_commanded_is_seventeen_achieved_and_the_limit_is_named() {
        let r = reachable_retract_speed(&klipper_default(), 40.0, 0.8).expect("declared");
        assert_eq!(r.commanded, 40.0);
        assert!((r.achieved - 17.866_2).abs() < 1e-3);
        assert!(r.was_capped());
        let why = r.governed_by.clone().expect("capped means a reason");
        assert!(
            why.contains("max_extrude_only_accel") && why.contains("399"),
            "the limit must be named with its value: {why}"
        );
        assert!(
            (r.fraction - 0.4466).abs() < 1e-3,
            "it got 45% of what it asked: {}",
            r.fraction
        );
    }

    /// COMMANDED AND ACHIEVED ARE SEPARATE FIELDS. A single number would be
    /// read as "the speed", and the whole finding is that there are two.
    #[test]
    fn a_commanded_value_is_never_reported_as_achieved() {
        let r = reachable_retract_speed(&klipper_default(), 40.0, 0.8).expect("declared");
        assert_ne!(
            r.commanded, r.achieved,
            "the two must not collapse into one number"
        );
        assert!(r.achieved < r.commanded);
    }

    /// A command the machine can meet is met, and nothing is said to have
    /// governed it. Reporting a cap here would be as wrong as hiding one.
    #[test]
    fn a_reachable_command_is_delivered_and_nothing_governed_it() {
        let r = reachable_retract_speed(&klipper_default(), 10.0, 0.8).expect("declared");
        assert_eq!(r.achieved, 10.0);
        assert!(!r.was_capped());
        assert!(r.governed_by.is_none());
        assert!((r.fraction - 1.0).abs() < 1e-12);
    }

    /// A SHORTER RETRACTION IS A SLOWER ONE, which is the opposite of the
    /// intuition that short moves are quick.
    #[test]
    fn a_shorter_retraction_reaches_a_lower_speed() {
        let p = klipper_default();
        let short = reachable_retract_speed(&p, 40.0, 0.4).expect("declared");
        let long = reachable_retract_speed(&p, 40.0, 1.6).expect("declared");
        assert!(
            short.achieved < long.achieved,
            "sqrt(a*L) rises with L: {} vs {}",
            short.achieved,
            long.achieved
        );
    }

    /// Raising the acceleration is the fix the note proposes, and it makes the
    /// commanded speed reachable.
    #[test]
    fn raising_the_acceleration_makes_the_command_reachable() {
        let mut p = klipper_default();
        p.firmware.max_extrude_only_accel = Some(Declared::assumed(2000.0, "the proposed setting"));
        let r = reachable_retract_speed(&p, 40.0, 0.8).expect("declared");
        assert!(
            !r.was_capped(),
            "2000 mm/s^2 over 0.8 mm reaches {}, above the 40 asked",
            (2000.0f64 * 0.8).sqrt()
        );
        assert_eq!(r.achieved, 40.0);
    }

    #[test]
    fn it_refuses_without_the_declared_acceleration() {
        let e = reachable_retract_speed(&Process::new(), 40.0, 0.8).expect_err("must refuse");
        assert_eq!(e.wanted, "max_extrude_only_accel");
        assert_eq!(e.by, "reachable_retract_speed");
    }

    #[test]
    fn nonpositive_and_nan_inputs_are_refused() {
        let p = klipper_default();
        for (c, l) in [
            (0.0, 0.8),
            (40.0, 0.0),
            (-1.0, 0.8),
            (f64::NAN, 0.8),
            (40.0, f64::NAN),
        ] {
            assert!(
                reachable_retract_speed(&p, c, l).is_err(),
                "command {c} length {l} must be refused"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Bed fit and export. Spec §05.
// ---------------------------------------------------------------------------

/// Whether a part fits the bed, and what to do if it does not.
#[derive(Debug, Clone, PartialEq)]
pub struct BedFit {
    pub fits_as_is: bool,
    /// Degrees about Z that would make it fit, where a rotation helps.
    /// `None` means either it already fits or no rotation about Z can help.
    pub suggested_rotation_deg: Option<f64>,
    /// Which axis is over, and by how much, for the orientation as given.
    pub over_by_mm: [f64; 3],
}

/// Does a bounding box fit the declared bed, and would turning it help?
///
/// ROTATION IS TRIED ABOUT Z ONLY, and that is a stated limit rather than an
/// oversight: turning a part onto another face changes which surfaces need
/// support and which direction its layers run, so it is a DESIGN decision and
/// not a packing one. Suggesting it here would be answering a question nobody
/// asked with consequences this function cannot see.
///
/// The 45-degree diagonal is checked because a part too long for the bed often
/// fits corner to corner, which is the one rotation worth suggesting without
/// knowing anything about the part.
pub fn fits_bed(p: &Process, size_mm: [f64; 3]) -> Result<BedFit, Missing> {
    let bed = p.bed.as_ref().ok_or(Missing {
        wanted: "bed".to_string(),
        by: "fits_bed",
    })?;
    for (i, v) in size_mm.iter().enumerate() {
        if !v.is_finite() || *v <= 0.0 {
            return Err(Missing {
                wanted: format!("a positive, finite size on axis {i} (got {v})"),
                by: "fits_bed",
            });
        }
    }
    let b = bed.value;
    let over = [
        (size_mm[0] - b[0]).max(0.0),
        (size_mm[1] - b[1]).max(0.0),
        (size_mm[2] - b[2]).max(0.0),
    ];
    let fits = over.iter().all(|o| *o <= 0.0);
    if fits {
        return Ok(BedFit {
            fits_as_is: true,
            suggested_rotation_deg: None,
            over_by_mm: over,
        });
    }
    // HEIGHT IS NOT NEGOTIABLE BY A Z ROTATION. If z is over, no turn about z
    // changes it, and suggesting one would be advice that cannot work.
    let suggested = if over[2] > 0.0 {
        None
    } else {
        let d = 45.0f64.to_radians();
        let (c, s) = (d.cos(), d.sin());
        let w = size_mm[0] * c + size_mm[1] * s;
        let h = size_mm[0] * s + size_mm[1] * c;
        if w <= b[0] && h <= b[1] {
            Some(45.0)
        } else {
            None
        }
    };
    Ok(BedFit {
        fits_as_is: false,
        suggested_rotation_deg: suggested,
        over_by_mm: over,
    })
}

/// Mesh export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshFormat {
    /// The default, and the only one anybody should ask for.
    BinaryStl,
    /// Available, never default. Roughly five times the bytes for the same
    /// triangles: the logged mistake was a **111 MB** export of a model whose
    /// binary form is a few megabytes, written because ASCII was the default
    /// and nobody chose it.
    AsciiStl,
    Off,
    ThreeMf,
}

impl Default for MeshFormat {
    /// BINARY, ALWAYS. This impl is the whole fix for mcp-surface's
    /// "ascii is a 111 MB mistake": a default that has to be overridden to get
    /// the bad outcome, rather than chosen to avoid it.
    fn default() -> Self {
        MeshFormat::BinaryStl
    }
}

impl MeshFormat {
    pub fn is_binary(self) -> bool {
        matches!(self, MeshFormat::BinaryStl | MeshFormat::ThreeMf)
    }

    /// The flag OpenSCAD wants for this format.
    pub fn openscad_export_format(self) -> &'static str {
        match self {
            MeshFormat::BinaryStl => "binstl",
            MeshFormat::AsciiStl => "asciistl",
            MeshFormat::Off => "off",
            MeshFormat::ThreeMf => "3mf",
        }
    }
}

#[cfg(test)]
mod bed_tests {
    use super::*;

    fn kobra() -> Process {
        let mut p = Process::new();
        p.bed = Some(Declared::measured(
            [220.0, 220.0, 250.0],
            "the Kobra Neo's usable volume",
        ));
        p
    }

    #[test]
    fn a_part_within_the_bed_fits_and_needs_no_advice() {
        let f = fits_bed(&kobra(), [100.0, 50.0, 30.0]).expect("declared");
        assert!(f.fits_as_is);
        assert_eq!(f.suggested_rotation_deg, None);
        assert_eq!(f.over_by_mm, [0.0, 0.0, 0.0]);
    }

    /// A part too long for the bed often fits corner to corner, which is the
    /// one rotation worth suggesting without knowing anything about the part.
    #[test]
    fn a_long_thin_part_is_offered_the_diagonal() {
        let f = fits_bed(&kobra(), [280.0, 20.0, 30.0]).expect("declared");
        assert!(!f.fits_as_is);
        assert_eq!(f.suggested_rotation_deg, Some(45.0));
        assert!(f.over_by_mm[0] > 59.0, "60 mm over in x");
    }

    /// HEIGHT IS NOT NEGOTIABLE BY A Z ROTATION, and suggesting one would be
    /// advice that cannot work.
    #[test]
    fn a_part_too_tall_gets_no_rotation_advice() {
        let f = fits_bed(&kobra(), [100.0, 100.0, 300.0]).expect("declared");
        assert!(!f.fits_as_is);
        assert_eq!(
            f.suggested_rotation_deg, None,
            "no turn about z makes a part shorter"
        );
        assert!(f.over_by_mm[2] > 49.0);
    }

    /// A part too big in both axes is not rescued by turning it, and saying
    /// "try 45 degrees" would waste somebody's afternoon.
    #[test]
    fn a_part_too_big_in_both_axes_gets_no_rotation_advice() {
        let f = fits_bed(&kobra(), [300.0, 300.0, 30.0]).expect("declared");
        assert!(!f.fits_as_is);
        assert_eq!(f.suggested_rotation_deg, None);
    }

    #[test]
    fn it_refuses_without_a_declared_bed() {
        assert_eq!(
            fits_bed(&Process::new(), [10.0, 10.0, 10.0])
                .expect_err("must refuse")
                .wanted,
            "bed"
        );
    }

    /// THE DEFAULT IS BINARY. The logged mistake was a 111 MB ASCII export
    /// written because ASCII was the default and nobody chose it; the fix is a
    /// default that has to be overridden to get the bad outcome.
    #[test]
    fn the_export_default_is_binary() {
        assert_eq!(MeshFormat::default(), MeshFormat::BinaryStl);
        assert!(MeshFormat::default().is_binary());
        assert_eq!(MeshFormat::default().openscad_export_format(), "binstl");
    }

    /// ASCII is available and is never arrived at by accident.
    #[test]
    fn ascii_exists_but_must_be_asked_for() {
        assert!(!MeshFormat::AsciiStl.is_binary());
        assert_ne!(MeshFormat::default(), MeshFormat::AsciiStl);
        assert_eq!(MeshFormat::AsciiStl.openscad_export_format(), "asciistl");
    }

    #[test]
    fn a_nonpositive_or_nan_size_is_refused() {
        for s in [
            [0.0, 10.0, 10.0],
            [10.0, f64::NAN, 10.0],
            [-1.0, 10.0, 10.0],
        ] {
            assert!(fits_bed(&kobra(), s).is_err(), "{s:?} must be refused");
        }
    }
}

// ---------------------------------------------------------------------------
// The slicer handoff. Spec §05.
// ---------------------------------------------------------------------------

/// How to invoke the slicer, built rather than typed.
///
/// # Three quirks, all of them measured against OrcaSlicer 2.3.2
///
/// **It dispatches on the FILE EXTENSION, not on content.** A perfectly good
/// binary STL named `stage.binstl` -- which is what OpenSCAD's
/// `--export-format binstl` invites -- is rejected with "Unknown file format.
/// Input file must have .stl, .obj, .amf(.xml) extension." The same bytes
/// named `.stl` load fine. So the export name is part of the handoff and not
/// a detail.
///
/// **Its facet count is not the file's facet count.** `--info` on the vendored
/// stage reports 788 facets where the file holds 806. The difference is
/// exactly the 18 ZERO-AREA triangles OpenSCAD's export contains, which the
/// slicer drops on load. Volume agrees to six significant figures
/// (40805.5117 against 40805.4868) and so does the bounding box. A facet count
/// is a property of a file AND of the reader that loaded it.
///
/// **The CLI matches the LEAF preset name where the GUI walks the inherits
/// chain.** A profile that resolves in the GUI through inheritance may not
/// resolve on the command line, and the failure reads as a missing preset
/// rather than as an unresolved parent.
#[derive(Debug, Clone, PartialEq)]
pub struct SliceRequest {
    /// Path to the mesh. MUST end in one of the extensions the CLI accepts.
    pub mesh_path: String,
    /// Process and machine settings files, in the CLI's `;`-joined form.
    pub settings: Vec<String>,
    pub output_dir: String,
}

/// What `SliceRequest` refuses before spending a subprocess on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SliceRefusal {
    /// The mesh name has an extension the CLI will not dispatch on.
    UnacceptableExtension {
        got: String,
    },
    MissingMesh,
    MissingOutputDir,
}

impl std::fmt::Display for SliceRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SliceRefusal::UnacceptableExtension { got } => write!(
                f,
                "the slicer dispatches on the file EXTENSION and will refuse {got:?}: it \
                 accepts .stl, .obj, .amf and .xml. A binary STL named .binstl is rejected \
                 for its name, not its contents. Rename the export before handing it over."
            ),
            SliceRefusal::MissingMesh => write!(f, "no mesh path given"),
            SliceRefusal::MissingOutputDir => write!(f, "no output directory given"),
        }
    }
}

/// Extensions OrcaSlicer 2.3.2 will dispatch on, from its own refusal message.
pub const SLICER_EXTENSIONS: [&str; 4] = ["stl", "obj", "amf", "xml"];

impl SliceRequest {
    /// Build the argv, or refuse.
    ///
    /// REFUSES BEFORE SPENDING A SUBPROCESS. The extension check in particular
    /// is cheap here and costs a process launch and a confusing error message
    /// if it is left to the slicer.
    pub fn argv(&self) -> Result<Vec<String>, SliceRefusal> {
        if self.mesh_path.trim().is_empty() {
            return Err(SliceRefusal::MissingMesh);
        }
        if self.output_dir.trim().is_empty() {
            return Err(SliceRefusal::MissingOutputDir);
        }
        let ext = self
            .mesh_path
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if !SLICER_EXTENSIONS.contains(&ext.as_str()) {
            return Err(SliceRefusal::UnacceptableExtension {
                got: self.mesh_path.clone(),
            });
        }
        let mut v = vec!["--slice".to_string(), "0".to_string()];
        if !self.settings.is_empty() {
            v.push("--load-settings".into());
            v.push(self.settings.join(";"));
        }
        v.push("--outputdir".into());
        v.push(self.output_dir.clone());
        v.push(self.mesh_path.clone());
        Ok(v)
    }
}

#[cfg(test)]
mod slicer_tests {
    use super::*;

    fn req() -> SliceRequest {
        SliceRequest {
            mesh_path: "/tmp/stage.stl".into(),
            settings: vec![
                "/profiles/process.json".into(),
                "/profiles/machine.json".into(),
            ],
            output_dir: "/tmp/out".into(),
        }
    }

    #[test]
    fn the_argv_carries_the_settings_joined_by_semicolons() {
        let v = req().argv().expect("valid");
        let joined = v.join(" ");
        assert!(joined.contains("--slice 0"));
        assert!(
            joined.contains("/profiles/process.json;/profiles/machine.json"),
            "the CLI wants one semicolon-joined argument: {joined}"
        );
        assert!(joined.contains("--outputdir /tmp/out"));
        assert_eq!(v.last().map(String::as_str), Some("/tmp/stage.stl"));
    }

    /// MEASURED AGAINST OrcaSlicer 2.3.2: a perfectly good binary STL named
    /// `.binstl` -- which `--export-format binstl` invites -- is rejected with
    /// "Unknown file format". The same bytes named `.stl` load fine.
    #[test]
    fn a_binstl_extension_is_refused_before_a_process_is_spent() {
        let bad = SliceRequest {
            mesh_path: "/tmp/stage.binstl".into(),
            ..req()
        };
        let e = bad.argv().expect_err("must refuse");
        assert_eq!(
            e,
            SliceRefusal::UnacceptableExtension {
                got: "/tmp/stage.binstl".into()
            }
        );
        let said = e.to_string();
        assert!(
            said.contains("not its contents"),
            "the refusal must say the name is the problem: {said}"
        );
    }

    #[test]
    fn the_accepted_extensions_are_the_ones_the_cli_names() {
        for ok in ["a.stl", "a.obj", "a.amf", "a.xml", "A.STL"] {
            assert!(
                SliceRequest {
                    mesh_path: ok.into(),
                    ..req()
                }
                .argv()
                .is_ok(),
                "{ok} must be accepted"
            );
        }
        for bad in ["a.3mf", "a.off", "a.binstl", "a"] {
            assert!(
                SliceRequest {
                    mesh_path: bad.into(),
                    ..req()
                }
                .argv()
                .is_err(),
                "{bad} must be refused"
            );
        }
    }

    #[test]
    fn a_request_with_no_settings_still_builds() {
        let v = SliceRequest {
            settings: vec![],
            ..req()
        }
        .argv()
        .expect("valid");
        assert!(!v.join(" ").contains("--load-settings"));
    }

    #[test]
    fn empty_paths_are_refused() {
        assert_eq!(
            SliceRequest {
                mesh_path: "  ".into(),
                ..req()
            }
            .argv(),
            Err(SliceRefusal::MissingMesh)
        );
        assert_eq!(
            SliceRequest {
                output_dir: String::new(),
                ..req()
            }
            .argv(),
            Err(SliceRefusal::MissingOutputDir)
        );
    }
}
