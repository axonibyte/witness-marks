//! The REAL client driven against a stub server.
//!
//! WHY A STUB AND NOT THE PROVIDER. The unit tests in the crate cover parsing
//! and the version comparison; none of them drives `start`, `request` or
//! `call` against a process at all. Pointing those at the pinned provider
//! would make them need uv, a resolved Python environment and OpenSCAD, which
//! means they would not run here, would not run in CI, and would therefore not
//! run. The stub makes the CLIENT the thing under test, which is the half this
//! crate owns.
//!
//! Each stub mode exists because the behaviour it fakes breaks a naive client:
//! a non-JSON banner line, a notification arriving before the answer, a server
//! that is not the pinned one, and a server that stops answering.

use std::time::Duration;
use wm_provider::{Expect, Provider, Spawn};

const TOML: &str = r#"
server_info_name = "OpenSCAD MCP Server"
server_info_version = "4.2.0"
mcp_protocol = "2024-11-05"
"#;

fn spawn(mode: &str, secs: u64) -> Spawn {
    // The stub is beside this test file, so the path is relative to the crate.
    let stub = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/stubs/server.py");
    // THE MODE GOES IN ARGV, NOT THE ENVIRONMENT. It was `set_var`, and cargo
    // runs these as threads in ONE process, so all nine tests shared the
    // variable: four failed in a batch and all nine passed when run alone,
    // which is what a race between test threads looks like from the outside.
    Spawn {
        program: "python3".into(),
        args: vec![stub.into(), mode.into()],
        cwd: None,
        timeout: Duration::from_secs(secs),
    }
}

fn expect() -> Expect {
    Expect::from_provider_toml(TOML).expect("expectations parse")
}

#[test]
fn a_pinned_server_starts_and_reports_itself() {
    let mut p = Provider::start(&spawn("plain", 20), &expect()).expect("starts");
    assert_eq!(p.server().name, "OpenSCAD MCP Server");
    assert_eq!(p.server().version, "4.2.0");
    assert_eq!(p.protocol(), "2024-11-05");
    let tools = p.list_tools().expect("lists tools");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "measure");
}

/// The real provider prints a security warning to stdout before it answers.
/// A client that parses the first line as JSON dies on it.
#[test]
fn a_non_json_banner_line_does_not_break_the_handshake() {
    let mut p = Provider::start(&spawn("banner", 20), &expect()).expect("starts past the banner");
    assert_eq!(p.server().version, "4.2.0");
    assert_eq!(p.list_tools().expect("lists").len(), 1);
}

/// A notification arriving before the answer must not be mistaken for it.
/// The stub sends one, plus a result under a DIFFERENT id carrying -1.0, so a
/// client that takes the first line it can parse reports the wrong number.
#[test]
fn an_interleaved_notification_is_not_mistaken_for_the_answer() {
    let mut p = Provider::start(&spawn("interleave", 20), &expect()).expect("starts");
    let a = p.measure("cube(10);").expect("measures");
    assert_eq!(
        a.payload["volume"], 1000.0,
        "the answer must be the one matching our id, not the first line that parsed"
    );
}

#[test]
fn the_structured_payload_and_its_diagnostics_both_arrive() {
    let mut p = Provider::start(&spawn("plain", 20), &expect()).expect("starts");
    let a = p.measure("cube(10);").expect("measures");
    assert_eq!(a.payload["volume"], 1000.0, "structuredContent wins");
    assert_eq!(a.diagnostics.warnings, vec!["a stub warning"]);
    assert_eq!(a.diagnostics.hints, vec!["a stub hint"]);
    assert!(!a.diagnostics.is_clean());
    assert!(!a.is_error);
}

/// An argument the provider rejects comes back as prose with `isError`, and
/// the prose must survive: a caller left with an empty payload cannot read
/// what it did wrong.
#[test]
fn an_error_result_keeps_its_prose() {
    let mut p = Provider::start(&spawn("texterror", 20), &expect()).expect("starts");
    let a = p.measure("cube([10,10,10)").expect("call returns");
    assert!(a.is_error, "isError must survive");
    assert!(
        a.payload["text"]
            .as_str()
            .unwrap_or("")
            .contains("validation error"),
        "the prose must survive, got {}",
        a.payload
    );
}

/// A SERVER THAT STOPS ANSWERING IS A TIMEOUT, NOT A HANG. Without a bound
/// this test would never return, which is the point: a blocked read with no
/// bound is a lost turn.
#[test]
fn a_silent_server_times_out_rather_than_hanging() {
    let mut p = Provider::start(&spawn("silent", 2), &expect()).expect("starts");
    let err = p.list_tools().expect_err("must time out");
    assert!(
        err.to_string().contains("did not answer"),
        "the error must name the timeout: {err}"
    );
}

#[test]
fn a_server_with_the_wrong_version_is_refused_at_startup() {
    let err = Provider::start(&spawn("wrongver", 20), &expect()).expect_err("must refuse");
    let s = format!("{err:#}");
    assert!(s.contains("9.9.9"), "name what it found: {s}");
    assert!(s.contains("refusing"), "say it is refusing: {s}");
}

#[test]
fn a_server_with_the_wrong_name_is_refused_at_startup() {
    let err = Provider::start(&spawn("wrongname", 20), &expect()).expect_err("must refuse");
    assert!(format!("{err:#}").contains("Some Other Server"));
}

#[test]
fn a_server_with_the_wrong_protocol_is_refused_at_startup() {
    let err = Provider::start(&spawn("wrongproto", 20), &expect()).expect_err("must refuse");
    assert!(format!("{err:#}").contains("2099-01-01"));
}

/// The pinning reaches the provider as `variables`, and the stub echoes the
/// arguments back so the test can see what was actually sent.
///
/// WITHOUT THIS ARM the pinning plan is computed and could simply never be
/// attached to the call, and every unit test about the plan would still pass.
#[test]
fn view_state_is_pinned_in_the_arguments_that_are_actually_sent() {
    let mut p = Provider::start(&spawn("echoargs", 20), &expect()).expect("starts");
    let src = "/* [View] */\nexplode = 80;\nshow_frame = true;\n/* [Can] */\ncan_dia = 66;\n";
    let (a, plan) = p.measure_pinned(src, &[]).expect("measures");
    let sent = &a.payload["echoed_arguments"];
    assert_eq!(
        sent["variables"]["explode"], 0,
        "explode must be pinned to 0 in the call itself, got {sent}"
    );
    assert!(
        sent["variables"].get("show_frame").is_none(),
        "a visibility flag must not be pinned: {sent}"
    );
    assert!(
        sent["variables"].get("can_dia").is_none(),
        "a real dimension must not be pinned: {sent}"
    );
    assert!(plan.notes().iter().any(|n| n.contains("explode")));
}

/// A snippet with no view state must send NO `variables` key at all, rather
/// than an empty object: an empty override is still an override, and the
/// provider is entitled to treat it as one.
#[test]
fn a_model_without_view_state_sends_no_variables_at_all() {
    let mut p = Provider::start(&spawn("echoargs", 20), &expect()).expect("starts");
    let (a, plan) = p.measure_pinned("cube(10);", &[]).expect("measures");
    assert!(plan.pinned.is_empty());
    assert!(
        a.payload["echoed_arguments"].get("variables").is_none(),
        "no view state means no variables key: {}",
        a.payload["echoed_arguments"]
    );
}
