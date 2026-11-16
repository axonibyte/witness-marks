//! Drive the REAL pinned provider once, and print what it said.
//!
//! NOT A TEST, DELIBERATELY. It needs `uv`, a resolved Python environment and
//! OpenSCAD, so putting it in `cargo test` would make a suite that cannot run
//! in CI and therefore does not run at all. The protocol tests use a stub and
//! cover the CLIENT; this covers the PAIR, once, by hand, with its output
//! recorded in the commit the way the first `reaper up` was.
//!
//! usage: cargo run -p wm-provider --example smoke -- <provider-dir>

use anyhow::Result;
use wm_provider::{Expect, Provider, Spawn};

fn main() -> Result<()> {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "provider".into());
    let toml = std::fs::read_to_string(format!("{dir}/PROVIDER.toml"))?;
    let expect = Expect::from_provider_toml(&toml)?;
    println!(
        "pinned to {} {} over MCP {}",
        expect.server_name, expect.server_version, expect.protocol
    );

    let mut p = Provider::start(&Spawn::pinned(&dir), &expect)?;
    println!(
        "started:   {} {} speaking {}",
        p.server().name,
        p.server().version,
        p.protocol()
    );

    let tools = p.list_tools()?;
    let total: usize = tools.iter().map(|t| t.to_string().len()).sum();
    println!(
        "tools:     {} advertised, {total} bytes of serialized schema",
        tools.len()
    );
    for want in wm_provider::TOOLS {
        let have = tools.iter().any(|t| t["name"] == want);
        println!(
            "           {want:<18} {}",
            if have { "present" } else { "ABSENT" }
        );
    }

    let a = p.measure("cube([10,10,10]);")?;
    println!(
        "measure:   volume {} components {} watertight {} empty {}",
        a.payload["volume"],
        a.payload["component_count"],
        a.payload["is_watertight"],
        a.is_empty_result()
    );

    let e = p.measure("intersection() { cube(5); translate([50,0,0]) cube(5); }")?;
    println!(
        "empty:     empty={} volume {} hints {:?}",
        e.is_empty_result(),
        e.payload["volume"],
        e.diagnostics.hints
    );

    let two = p.measure("cube(5); translate([20,0,0]) cube(5);")?;
    println!(
        "components: two disjoint cubes -> {}",
        two.payload["component_count"]
    );
    Ok(())
}
