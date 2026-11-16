//! How much material stands between a feature and open air.
//!
//! # The probe this generalizes
//!
//! buildability 66. Each dovetail socket should have had a flank on both
//! sides. A probe box placed just outboard of where each flank belongs was
//! measured: the mouth-side flanks read **ZERO** and the outboard ones read
//! **52.800 mm3**. Same probe, same volume of box, two different answers, and
//! the zeros are the sheared flanks.
//!
//! The reading is only meaningful because the probe's own volume is known:
//! 52.800 is "completely full" for that box, and 0.000 is "nothing there". A
//! probe whose capacity nobody recorded gives a number with no scale.
//!
//! # This builds the probe; the provider measures it
//!
//! Constructing where to look is geometry this crate can do from the tree.
//! Measuring what is there is a kernel operation, and `wm-provider` owns that.
//! Splitting them keeps this testable without a Python environment, and keeps
//! the thing under test the PLACEMENT, which is where the mistakes are.

use crate::bbox::Bbox;

/// Which way to look from a feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    NegX,
    PosX,
    NegY,
    PosY,
    NegZ,
    PosZ,
}

impl Dir {
    fn axis(self) -> usize {
        match self {
            Dir::NegX | Dir::PosX => 0,
            Dir::NegY | Dir::PosY => 1,
            Dir::NegZ | Dir::PosZ => 2,
        }
    }
    fn positive(self) -> bool {
        matches!(self, Dir::PosX | Dir::PosY | Dir::PosZ)
    }
}

/// A box to measure, and the capacity that makes its reading meaningful.
#[derive(Debug, Clone, PartialEq)]
pub struct Probe {
    pub lo: [f64; 3],
    pub hi: [f64; 3],
    /// The probe's own volume. A reading without this has no scale: 52.800 is
    /// only "full" if the box holds 52.800.
    pub capacity: f64,
}

impl Probe {
    /// SCAD for the probe box itself, to be intersected with the model.
    pub fn scad(&self) -> String {
        format!(
            "translate([{}, {}, {}]) cube([{}, {}, {}]);",
            self.lo[0],
            self.lo[1],
            self.lo[2],
            self.hi[0] - self.lo[0],
            self.hi[1] - self.lo[1],
            self.hi[2] - self.lo[2]
        )
    }
}

/// Place a probe immediately outboard of a feature's face.
///
/// `depth` is how far past the face to look. The probe spans the feature's
/// full extent on the other two axes, so it sees the whole flank rather than
/// a strip of it.
///
/// REFUSES AN UNBOUNDED FEATURE. There is nowhere to put a probe relative to
/// a box nobody could compute, and placing one at the origin would measure
/// somewhere else entirely and report a number for it.
pub fn probe_outboard(b: &Bbox, dir: Dir, depth: f64) -> Result<Probe, String> {
    let Bbox::Known { lo, hi, .. } = b else {
        return Err(format!(
            "cannot place a probe relative to {b:?}: an unbounded feature has no face \
             to stand outside of, and a probe at the origin would measure somewhere else \
             and report a number for it"
        ));
    };
    if !depth.is_finite() || depth <= 0.0 {
        return Err(format!(
            "probe depth must be positive and finite, got {depth}"
        ));
    }
    let a = dir.axis();
    let mut plo = *lo;
    let mut phi = *hi;
    if dir.positive() {
        plo[a] = hi[a];
        phi[a] = hi[a] + depth;
    } else {
        phi[a] = lo[a];
        plo[a] = lo[a] - depth;
    }
    let capacity = (0..3).map(|i| phi[i] - plo[i]).product::<f64>();
    // EXPLICIT ABOUT NaN, the same lesson clippy taught in wm-process: a
    // negated comparison catches NaN as a SIDE EFFECT, and a side effect that
    // load-bearing should be written down.
    if !capacity.is_finite() || capacity <= 0.0 {
        return Err(format!(
            "the probe has no volume ({capacity}): a feature flat on another axis gives \
             a box that cannot hold anything, and every reading from it would be 0.000 \
             whatever the material"
        ));
    }
    Ok(Probe {
        lo: plo,
        hi: phi,
        capacity,
    })
}

/// What a measured volume means for this probe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Flank {
    /// Nothing there. The flank is absent, which is the finding.
    Absent,
    /// The probe is entirely inside material.
    Full,
    /// Something, but not everything: the fraction of the probe filled.
    Partial(f64),
}

/// Read a measured volume against the probe's capacity.
///
/// A FRACTION, NOT A RAW VOLUME. 52.800 means nothing on its own; 52.800 out
/// of 52.800 means the flank is intact and 52.800 out of 500 means it is
/// mostly air. The logged incident only reads as a finding because the two
/// probes had the SAME capacity and returned 0.000 against 52.800.
pub fn read(p: &Probe, measured: f64, tol: f64) -> Result<Flank, String> {
    if !measured.is_finite() || measured < 0.0 {
        return Err(format!(
            "a measured volume must be finite and non-negative, got {measured}"
        ));
    }
    if measured > p.capacity + tol {
        return Err(format!(
            "measured {measured} in a probe that holds {}: more material than the box \
             can contain means the probe and the measurement are not of the same thing",
            p.capacity
        ));
    }
    if measured <= tol {
        return Ok(Flank::Absent);
    }
    if (p.capacity - measured).abs() <= tol {
        return Ok(Flank::Full);
    }
    Ok(Flank::Partial(measured / p.capacity))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feature() -> Bbox {
        Bbox::known([0.0, 0.0, 0.0], [4.0, 9.0, 15.0])
    }

    #[test]
    fn a_probe_sits_immediately_outboard_of_the_chosen_face() {
        let p = probe_outboard(&feature(), Dir::PosX, 2.0).expect("valid");
        assert_eq!(p.lo, [4.0, 0.0, 0.0], "it starts AT the face");
        assert_eq!(p.hi, [6.0, 9.0, 15.0]);
        let n = probe_outboard(&feature(), Dir::NegX, 2.0).expect("valid");
        assert_eq!(n.hi, [0.0, 9.0, 15.0], "and the other side ends at it");
        assert_eq!(n.lo, [-2.0, 0.0, 0.0]);
    }

    /// IT SPANS THE WHOLE FLANK, not a strip of it: the other two axes keep
    /// the feature's full extent.
    #[test]
    fn the_probe_spans_the_features_full_extent_on_the_other_axes() {
        let p = probe_outboard(&feature(), Dir::PosX, 2.0).expect("valid");
        assert_eq!(p.hi[1] - p.lo[1], 9.0);
        assert_eq!(p.hi[2] - p.lo[2], 15.0);
        assert_eq!(p.capacity, 2.0 * 9.0 * 15.0);
    }

    /// THE INCIDENT. Two probes of the SAME capacity, one reading 0.000 and
    /// one reading 52.800. The zero is the sheared flank, and it only reads as
    /// a finding because the capacity is known and shared.
    #[test]
    fn the_sheared_flank_reads_absent_where_the_surviving_one_reads_full() {
        let p = Probe {
            lo: [0.0, 0.0, 0.0],
            hi: [2.0, 4.0, 6.6],
            capacity: 52.8,
        };
        assert_eq!(read(&p, 0.0, 1e-9).expect("valid"), Flank::Absent);
        assert_eq!(read(&p, 52.8, 1e-9).expect("valid"), Flank::Full);
    }

    /// A FRACTION, NOT A RAW VOLUME. The same 52.800 in a bigger probe is
    /// mostly air, and reporting the raw number would read identically.
    #[test]
    fn the_same_volume_means_different_things_in_different_probes() {
        let small = Probe {
            lo: [0.0; 3],
            hi: [2.0, 4.0, 6.6],
            capacity: 52.8,
        };
        let large = Probe {
            lo: [0.0; 3],
            hi: [10.0, 10.0, 5.0],
            capacity: 500.0,
        };
        assert_eq!(read(&small, 52.8, 1e-9).expect("valid"), Flank::Full);
        match read(&large, 52.8, 1e-9).expect("valid") {
            Flank::Partial(f) => assert!((f - 0.1056).abs() < 1e-4, "got {f}"),
            other => panic!("the same volume is mostly air here, got {other:?}"),
        }
    }

    /// MORE MATERIAL THAN THE BOX HOLDS means the probe and the measurement
    /// are not of the same thing, which is a bug and not a very full flank.
    #[test]
    fn a_reading_larger_than_the_probe_is_refused() {
        let p = Probe {
            lo: [0.0; 3],
            hi: [2.0, 4.0, 6.6],
            capacity: 52.8,
        };
        let e = read(&p, 60.0, 1e-9).expect_err("must refuse");
        assert!(e.contains("not of the same thing"), "{e}");
    }

    /// AN UNBOUNDED FEATURE HAS NO FACE TO STAND OUTSIDE OF, and a probe at
    /// the origin would measure somewhere else and report a number for it.
    #[test]
    fn an_unbounded_feature_cannot_be_probed() {
        let e = probe_outboard(&Bbox::Unknown("a rotate_extrude".into()), Dir::PosX, 2.0)
            .expect_err("must refuse");
        assert!(e.contains("somewhere else"), "{e}");
        assert!(probe_outboard(&Bbox::Empty, Dir::PosX, 2.0).is_err());
    }

    /// A FEATURE FLAT ON ANOTHER AXIS gives a probe that cannot hold
    /// anything, and every reading from it would be 0.000 whatever the
    /// material -- an Absent verdict for every flank in the model.
    #[test]
    fn a_flat_feature_gives_a_probe_with_no_volume_and_is_refused() {
        let flat = Bbox::known([0.0, 0.0, 0.0], [4.0, 9.0, 0.0]);
        let e = probe_outboard(&flat, Dir::PosX, 2.0).expect_err("must refuse");
        assert!(e.contains("whatever the material"), "{e}");
    }

    #[test]
    fn the_scad_places_the_box_where_the_probe_is() {
        let p = probe_outboard(&feature(), Dir::PosX, 2.0).expect("valid");
        let s = p.scad();
        assert!(s.contains("translate([4, 0, 0])"), "{s}");
        assert!(s.contains("cube([2, 9, 15])"), "{s}");
    }

    #[test]
    fn a_nonpositive_depth_is_refused() {
        for d in [0.0, -2.0, f64::NAN] {
            assert!(probe_outboard(&feature(), Dir::PosX, d).is_err(), "{d}");
        }
    }
}
