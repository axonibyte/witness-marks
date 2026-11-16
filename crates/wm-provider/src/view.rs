//! View state: parameters that move or hide geometry for DISPLAY, and are a
//! live input to every measurement taken through the same file.
//!
//! # The incident
//!
//! mcp note 30. `can_cutter.scad` carries `explode = 80`, which pulls the
//! loose parts clear so a slicer can lay them flat. An overhang scan run
//! through that file reported PASS -- on empty space, because the part it
//! meant to scan had been displaced 80 mm out of the probe box. Nothing in the
//! call said `explode`; it is a top-level assignment in the file, and the
//! measurement inherited it.
//!
//! # Two kinds, and they need OPPOSITE treatment
//!
//! The issue that commissioned this says "pin them to zero". That is right for
//! one kind and destroys the other:
//!
//! - **Displacement** is numeric and moves geometry: `explode`, `stage_out`,
//!   a spin angle. Zero is the measuring position, and pinning it is the fix.
//! - **Visibility** is boolean and selects what renders at all: `show_frame`,
//!   `show_stage`. Pinning those to zero hides everything and measures an
//!   empty model -- the same failure in a new costume. WHICH PART IS BEING
//!   MEASURED IS THE CALLER'S QUESTION, so these are REPORTED and left alone.
//!
//! Reporting the untouched ones matters as much as pinning the others: a
//! caller who measures with `show_stage = false` still measured the wrong
//! thing, and the envelope should let them see that they did.

use serde_json::{Map, Value};

/// Why a parameter was taken for view state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// It sits under a Customizer group named for the view.
    Group,
    /// Its name is one the convention reserves.
    Name,
    /// The caller said so.
    Declared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Numeric, moves geometry. Pinned to zero.
    Displacement,
    /// Boolean, selects what renders. Reported, never pinned.
    Visibility,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub kind: Kind,
    pub why: Why,
    pub declared_value: String,
}

/// Customizer groups whose contents are view state, lowercased.
const VIEW_GROUPS: [&str; 4] = ["view", "display", "preview", "debug"];

/// Names the convention reserves, whatever group they sit in.
fn name_is_view(name: &str) -> Option<Kind> {
    let n = name.to_ascii_lowercase();
    if n.starts_with("show_") || n.starts_with("hide_") {
        return Some(Kind::Visibility);
    }
    if n == "explode" || n.starts_with("explode_") || n.ends_with("_explode") {
        return Some(Kind::Displacement);
    }
    // $vpr / $vpt / $vpd are OpenSCAD's own camera variables.
    if n.starts_with("$vp") {
        return Some(Kind::Displacement);
    }
    None
}

fn kind_of_value(v: &str) -> Kind {
    let t = v.trim();
    if t == "true" || t == "false" {
        Kind::Visibility
    } else {
        Kind::Displacement
    }
}

/// Find the view parameters declared at the top level of a SCAD file.
///
/// GROUP MEMBERSHIP IS INHERITED UNTIL THE NEXT GROUP, which is how the
/// Customizer itself reads a file: `/* [View] */` opens a section and every
/// assignment after it belongs to that section until another `/* [...] */`
/// opens the next. A detector that only looked at names would miss
/// `stage_out`, which is view state here and is named like a dimension.
pub fn detect(source: &str, declared: &[&str]) -> Vec<Param> {
    let mut out: Vec<Param> = Vec::new();
    let mut in_view_group = false;
    for raw in source.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("/* [") {
            if let Some(label) = rest.split(']').next() {
                let l = label.trim().to_ascii_lowercase();
                in_view_group = VIEW_GROUPS.iter().any(|g| l == *g || l.starts_with(*g));
            }
            continue;
        }
        // a top-level assignment: NAME = VALUE ;
        let Some(eq) = line.find('=') else { continue };
        let name = line[..eq].trim();
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
            || name.chars().next().is_some_and(|c| c.is_ascii_digit())
        {
            continue;
        }
        let value = line[eq + 1..]
            .split(';')
            .next()
            .unwrap_or("")
            .split("//")
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if value.is_empty() {
            continue;
        }
        let by_name = name_is_view(name);
        let declared_here = declared.contains(&name);
        let (kind, why) = if declared_here {
            (kind_of_value(&value), Why::Declared)
        } else if let Some(k) = by_name {
            (k, Why::Name)
        } else if in_view_group {
            (kind_of_value(&value), Why::Group)
        } else {
            continue;
        };
        if !out.iter().any(|p| p.name == name) {
            out.push(Param {
                name: name.to_string(),
                kind,
                why,
                declared_value: value,
            });
        }
    }
    out
}

/// What a measurement call should do about them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pinning {
    /// name -> the value forced, for the provider's `variables` argument.
    pub pinned: Map<String, Value>,
    /// Flagged, deliberately NOT pinned, and reported so a caller can see
    /// that the subject may not be what they meant.
    pub reported: Vec<String>,
}

impl Pinning {
    pub fn plan(params: &[Param]) -> Self {
        let mut p = Pinning::default();
        for prm in params {
            match prm.kind {
                Kind::Displacement => {
                    p.pinned.insert(prm.name.clone(), Value::from(0));
                }
                Kind::Visibility => p.reported.push(format!(
                    "{} is view state ({:?}) and was left at the caller's value {:?}: \
                     which part is measured is the caller's question",
                    prm.name, prm.why, prm.declared_value
                )),
            }
        }
        p
    }

    /// One line per pinned parameter, for the envelope's warnings, so the
    /// pinning is reported rather than silently applied. A measurement that
    /// quietly changed its own inputs is not better than one that inherited
    /// them.
    pub fn notes(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .pinned
            .keys()
            .map(|k| format!("{k} is view state and was pinned to 0 for this measurement"))
            .collect();
        v.extend(self.reported.iter().cloned());
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../tests/fixtures/can_cutter/can_cutter.scad");

    #[test]
    fn the_real_incident_file_declares_explode_and_it_is_found() {
        let found = detect(FIXTURE, &[]);
        let explode = found
            .iter()
            .find(|p| p.name == "explode")
            .expect("explode must be found in the file the incident happened in");
        assert_eq!(explode.kind, Kind::Displacement);
        assert_eq!(
            explode.declared_value, "80",
            "and its declared value is the 80 mm that moved the part out of the probe box"
        );
    }

    /// `stage_out` is view state and is named like a dimension. A name-only
    /// detector misses it; the Customizer group is what catches it.
    #[test]
    fn a_view_parameter_named_like_a_dimension_is_caught_by_its_group() {
        let found = detect(FIXTURE, &[]);
        let p = found
            .iter()
            .find(|p| p.name == "stage_out")
            .expect("stage_out sits under /* [View] */");
        assert_eq!(p.why, Why::Group, "its NAME gives nothing away");
    }

    /// A dimension in another group must NOT be swept up. `can_dia` is a real
    /// measurement input and pinning it to zero would destroy the model.
    #[test]
    fn a_real_dimension_outside_the_view_group_is_untouched() {
        let found = detect(FIXTURE, &[]);
        for name in ["can_dia", "wall", "blade_bite", "layer_h"] {
            assert!(
                !found.iter().any(|p| p.name == name),
                "{name} is a real input and must not be taken for view state"
            );
        }
    }

    /// The distinction the issue's wording glosses: pinning a visibility flag
    /// to zero hides the part and measures an empty model, which is the same
    /// failure in a new costume.
    #[test]
    fn visibility_flags_are_reported_and_never_pinned() {
        let found = detect(FIXTURE, &[]);
        let plan = Pinning::plan(&found);
        assert!(
            plan.pinned.contains_key("explode"),
            "displacement is pinned"
        );
        for flag in ["show_frame", "show_stage", "show_can"] {
            assert!(
                !plan.pinned.contains_key(flag),
                "{flag} must NOT be pinned: that would measure an empty model"
            );
            assert!(
                plan.reported.iter().any(|r| r.starts_with(flag)),
                "{flag} must still be reported"
            );
        }
    }

    #[test]
    fn pinned_values_are_zero_and_the_pinning_is_reported() {
        let plan = Pinning::plan(&detect(FIXTURE, &[]));
        assert_eq!(plan.pinned["explode"], Value::from(0));
        assert!(
            plan.notes()
                .iter()
                .any(|n| n.contains("explode") && n.contains("pinned to 0")),
            "the pinning must be reported, not silently applied: {:?}",
            plan.notes()
        );
    }

    /// A caller may name a parameter the conventions do not reach.
    #[test]
    fn a_caller_can_declare_one_the_conventions_miss() {
        let src = "/* [Geometry] */\nwobble = 12;\n";
        assert!(detect(src, &[]).is_empty(), "nothing marks it");
        let found = detect(src, &["wobble"]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].why, Why::Declared);
        assert_eq!(found[0].kind, Kind::Displacement);
    }

    #[test]
    fn a_group_ends_at_the_next_group() {
        let src = "/* [View] */\nexplode = 5;\nspin = 3;\n/* [Can] */\ncan_dia = 66;\n";
        let found = detect(src, &[]);
        assert!(found.iter().any(|p| p.name == "spin"));
        assert!(
            !found.iter().any(|p| p.name == "can_dia"),
            "the next group closes the view section"
        );
    }

    #[test]
    fn openscad_camera_variables_are_view_state_anywhere() {
        let found = detect("/* [Geometry] */\n$vpr = [55, 0, 25];\n", &[]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, Kind::Displacement);
        assert_eq!(found[0].why, Why::Name);
    }
}
