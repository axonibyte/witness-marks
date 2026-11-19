//! Parse a REAL section reply from the REAL provider with the REAL parser
//! (rook's F11, thread:wm-review/15).
//!
//! WHY THIS EXISTS, and it is the whole of the finding: `wire::section_from`
//! wanted per-contour objects carrying centroid, area, bbox and vertices. The
//! pinned provider returns none of that -- `contours` is a list of POINT
//! LISTS, with area, centroid and bbox once at the top level. So the parser
//! had NEVER PARSED A REAL ANSWER. Every fixture was hand-built by me, nothing
//! outside the tests called it, and the shape I wrote down was never checked
//! against the instrument. That is this project's founding defect -- a figure
//! written down and not checked -- committed in the code meant to catch it.
//!
//! A test cannot close that, because a test runs against a recorded reply and
//! the recording is the thing that was wrong. Only speaking to the provider
//! can. So this runs the pair end to end, deliberately, like `smoke.rs` and
//! `claim-in-model.rs`, and `tools/tests/vectors-arms.sh` drives it.
//!
//! usage: cargo run -p wm-provider --example section-from-provider -- [provider-dir]

use anyhow::{bail, Result};
use serde_json::json;
use wm_model::regions::{Axis, Comparison, Plane, Section};
use wm_model::wire::section_from;
use wm_provider::{Provider, Spawn};

fn main() -> Result<()> {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "provider".into());
    let spawn = Spawn::pinned(&dir);
    let expect = wm_provider::Expect::from_provider_toml(&std::fs::read_to_string(
        std::path::Path::new(&dir).join("PROVIDER.toml"),
    )?)?;
    let mut p = Provider::start(&spawn, &expect)?;

    // TWO CUBES, so the cut has TWO contours and the pairing has something to
    // pair. One cube would make a one-contour answer, where a parser that
    // dropped contours would still look right.
    let model = "cube([10,10,10]); translate([20,0,0]) cube([10,10,10]);";
    let answer = p
        .call(
            "measure",
            json!({"scad_content": model, "mode": "section", "section_offset": 5.0}),
        )
        .map_err(|e| anyhow::anyhow!(e))?;
    let reply = &answer.payload;
    println!("reply keys  {:?}", {
        let mut k: Vec<_> = reply
            .as_object()
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default();
        k.sort();
        k
    });

    let plane = Plane {
        axis: Axis::Z,
        offset_mm: 5.0,
    };
    let section = section_from(reply, plane).map_err(|e| anyhow::anyhow!(e))?;
    let contours = match &section {
        Section::Cut { contours, .. } => contours,
        Section::Unresolved { why, .. } => bail!("the provider would not resolve the cut: {why}"),
    };
    println!("contours    {}", contours.len());
    for (n, c) in contours.iter().enumerate() {
        println!(
            "  {n}: area {:.4} centroid [{:.4}, {:.4}] vertices {:?} hole {}",
            c.area_mm2, c.centroid_mm[0], c.centroid_mm[1], c.vertices, c.is_hole
        );
    }

    // AND THE PARSED CONTOURS GO THROUGH THE DIFF, against themselves, which
    // must report no change. A parse that produced garbage would usually fail
    // here too -- an unpairable contour reads as Removed plus Added.
    let d = wm_model::regions::diff_sections(
        &section,
        &section,
        Comparison::ContourIdentity {
            centroid_tol_mm: 0.01,
            area_tol_mm2: 0.01,
        },
    )
    .map_err(|e| anyhow::anyhow!(e))?;
    println!(
        "self-diff   unchanged={} regions={}",
        d.unchanged(),
        d.regions.len()
    );
    if !d.unchanged() {
        bail!("a section did not compare equal to ITSELF: {:?}", d.regions);
    }

    // The provider's own aggregate, against the sum this parse computed. The
    // parser checks this too; printing it is how a reader sees the agreement
    // rather than taking it on trust.
    let reported = reply.get("area").and_then(|v| v.as_f64());
    let summed: f64 = contours.iter().map(|c| c.area_mm2).sum();
    println!("area        provider {reported:?} parsed {summed}");
    println!("-- a real section reply parsed, diffed and agreed with its own aggregate");
    Ok(())
}
