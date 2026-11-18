//! Evaluate a REAL claim in the REAL model's scope, through the REAL provider
//! (issue 60).
//!
//! NOT A TEST, DELIBERATELY -- the same reason `smoke.rs` is not one. It needs
//! `uv`, a resolved Python environment and OpenSCAD, so as a test it would be
//! a suite that cannot run in CI and therefore does not run at all. The parse
//! and every refusal are covered in CI by `wm_claims::scad`'s own arms against
//! recorded replies; this covers THE PAIR, once, by hand, with its output
//! recorded in the commit.
//!
//! WHAT IT PROVES THAT A FIXTURE CANNOT: that `scad_eval` reads the harness's
//! own symbols, that `variables` overrides them, and therefore that
//! perturbation -- the whole basis of the dependency index -- works through a
//! transport this project does not control.
//!
//! usage: cargo run -p wm-provider --example claim-in-model -- [provider-dir] [model]

use anyhow::Result;
use std::collections::BTreeMap;
use wm_claims::claim::{
    declaration_gap_with, derived_inputs_with, verify_with, Claim, Expression, Input, Inputs,
};
use wm_claims::index::{Index, Source};
use wm_claims::scad::ScadEval;
use wm_provider::{scad_eval_in_model, Expect, Provider, Spawn};

fn main() -> Result<()> {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "provider".into());
    let model = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "/data/projects/tech/designs/arts/can_cutter.scad".into());
    let toml = std::fs::read_to_string(format!("{dir}/PROVIDER.toml"))?;
    let expect = Expect::from_provider_toml(&toml)?;
    let mut p = Provider::start(&Spawn::pinned(&dir), &expect)?;
    println!(
        "provider  {} {} over MCP {}",
        p.server().name,
        p.server().version,
        p.protocol()
    );
    println!("model     {model}");

    // THE CLAIM LIVES IN THE MODEL. `bore_r` is the harness's own symbol; this
    // claim names it and nothing else, which is the point of issue 60 -- the
    // number is not restated here.
    let claim = Claim {
        name: "bore_clears_the_can".into(),
        declared_inputs: vec!["can_dia".into()],
        range: (33.0, 34.0),
        unit: "mm".into(),
        because: "the bore must clear a 66 mm can with slip, and stay inside the body wall".into(),
        expression: Expression::Scad {
            source: "bore_r".into(),
        },
    };
    let inputs = Inputs::new()
        .with("can_dia", Input::measured(66.0, "the can, calipered"))
        .with("can_clear", Input::measured(0.6, "the model"));

    let ev = ScadEval::new(|expr: &str, vars: &BTreeMap<String, f64>| {
        scad_eval_in_model(&mut p, &model, expr, vars).map_err(|e| e.to_string())
    });

    let v = verify_with(&claim, &inputs, &ev);
    println!("verify    {:?}  value {:?}", v.status, v.value);
    println!("weakest   {:?}", v.weakest_input);

    // PERTURBATION THROUGH THE PROVIDER. This is the measurement that decides
    // whether the dependency index can be built over a model-scope claim at
    // all.
    let used = derived_inputs_with(&claim, &inputs, &ev);
    println!("reads     {used:?}");
    let gap = declaration_gap_with(&claim, &inputs, &ev);
    println!("declared and not read: {:?}", gap.unread);
    println!("read and not declared: {:?}", gap.undeclared);

    let ix = Index::build_with(std::slice::from_ref(&claim), &inputs, &ev, Source::ScadEval)
        .map_err(anyhow::Error::msg)?;
    println!(
        "index     source {:?}, elasticity floor {:.0e} at probe {:.0e}",
        ix.source(),
        ix.source()
            .elasticity_floor(wm_claims::claim::PROBE_FRACTION),
        wm_claims::claim::PROBE_FRACTION
    );
    println!("inert     {:?}", ix.inert());

    // THE FOUNDING DEFECT, CHECKED. `can_clear` is deliberately the WRONG
    // name -- the harness's symbol is `can_clearance` -- so this run shows the
    // misspelling being named rather than silently reading as "an input this
    // claim does not depend on".
    let unwarned = ev.unwarned();
    if unwarned.is_empty() {
        println!("overrides every override overwrote a file-scope assignment");
    } else {
        println!(
            "overrides {unwarned:?} overwrote NOTHING -- a wrong name, or not a \
             file-scope constant"
        );
    }
    Ok(())
}
