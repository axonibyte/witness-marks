//! The geometry provider client. Pinned to RobertCoop/openscad-mcp (see
//! `provider/PROVIDER.toml`); spoken to over MCP stdio, never imported.
//! Spec §03, §04.
//!
//! # Why a subprocess and not a library
//!
//! Spec §03: consume the provider, do not rebuild it. Speaking to it over its
//! own protocol means the thing under test is the thing that ships, and it
//! means the pin is enforceable: a client can check what it is talking to.
//!
//! # Two version numbers
//!
//! `PROVIDER.toml` carries both, and they are different schemes. `version` is
//! the PyPI distribution version, 0.6.1, which a client never sees over MCP.
//! `initialize` returns `serverInfo`, and that reads 4.2.0. A refusal written
//! against the wrong one refuses every run, so [`Expect`] names the one that
//! can actually be observed.
//!
//! # Diagnostics are never dropped
//!
//! The provider answers with `errors`, `warnings`, `hints` and `deprecated`
//! alongside its numbers. Those are the sentences that say a result is not
//! what it looks like, so [`Diagnostics`] rides with every call and a caller
//! that ignores it has to do so on purpose.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// Present so the crate has one exported symbol and one test from day one.
pub const LAYER: &str = "wm-provider";

/// The six tools this client speaks, named in the milestone's own words.
pub const TOOLS: [&str; 6] = [
    "measure",
    "check",
    "render",
    "validate",
    "scad_eval",
    "get_project_files",
];

/// What the provider must say it is, read from `PROVIDER.toml`.
///
/// A mismatch is a REFUSAL and not a warning: every number this project
/// records is attributed to a pinned kernel, and a different server is a
/// different kernel whatever else it does correctly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expect {
    pub server_name: String,
    pub server_version: String,
    pub protocol: String,
}

impl Expect {
    /// Parse the three expectation lines out of `PROVIDER.toml`.
    ///
    /// REFUSES A FILE THAT NAMES NONE OF THEM rather than returning empty
    /// expectations: an `Expect` with blank fields would match any server at
    /// all, which is the shape of a guard whose blind state and whose safe
    /// state are the same bytes.
    pub fn from_provider_toml(text: &str) -> Result<Self> {
        fn field(text: &str, key: &str) -> Option<String> {
            for line in text.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix(key) {
                    let rest = rest.trim_start();
                    if let Some(rest) = rest.strip_prefix('=') {
                        let rest = rest.trim();
                        if let Some(v) = rest.strip_prefix('"').and_then(|r| r.split('"').next()) {
                            return Some(v.to_string());
                        }
                    }
                }
            }
            None
        }
        let server_name = field(text, "server_info_name");
        let server_version = field(text, "server_info_version");
        let protocol = field(text, "mcp_protocol");
        match (server_name, server_version, protocol) {
            (Some(n), Some(v), Some(p)) => Ok(Expect {
                server_name: n,
                server_version: v,
                protocol: p,
            }),
            _ => bail!(
                "PROVIDER.toml names no server_info_name / server_info_version / mcp_protocol; \
                 refusing rather than accepting any server"
            ),
        }
    }

    fn check(&self, info: &ServerInfo, protocol: &str) -> Result<()> {
        if info.name != self.server_name {
            bail!(
                "provider is {:?}, PROVIDER.toml pins {:?}",
                info.name,
                self.server_name
            );
        }
        if info.version != self.server_version {
            bail!(
                "provider serverInfo version is {:?}, PROVIDER.toml pins {:?}",
                info.version,
                self.server_version
            );
        }
        if protocol != self.protocol {
            bail!(
                "provider speaks MCP {:?}, PROVIDER.toml pins {:?}",
                protocol,
                self.protocol
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// Everything the provider said that was not a number.
///
/// `hints` is the field that matters most and is easiest to drop: an empty
/// result arrives with `hints[0].code == "empty_output"` explaining that a
/// difference may have removed everything. That sentence is the difference
/// between "no geometry" and "no geometry, and here is why".
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Diagnostics {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub hints: Vec<String>,
    pub deprecated: Vec<String>,
}

impl Diagnostics {
    /// Pull the four diagnostic arrays out of a provider payload.
    ///
    /// Entries may be strings or objects; an object is rendered rather than
    /// dropped, because a hint this client cannot parse is still a hint, and
    /// silently discarding it is how a caller comes to believe a clean result.
    pub fn from_payload(v: &Value) -> Self {
        fn arr(v: &Value, key: &str) -> Vec<String> {
            v.get(key)
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .map(|e| match e {
                            Value::String(s) => s.clone(),
                            other => other
                                .get("hint")
                                .or_else(|| other.get("message"))
                                .and_then(Value::as_str)
                                .map(str::to_string)
                                .unwrap_or_else(|| other.to_string()),
                        })
                        .collect()
                })
                .unwrap_or_default()
        }
        Diagnostics {
            errors: arr(v, "errors"),
            warnings: arr(v, "warnings"),
            hints: arr(v, "hints"),
            deprecated: arr(v, "deprecated"),
        }
    }

    pub fn is_clean(&self) -> bool {
        self.errors.is_empty() && self.warnings.is_empty() && self.deprecated.is_empty()
    }
}

/// One `tools/call` answer: the structured payload, the diagnostics that came
/// with it, and whether the provider flagged the call itself as an error.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolAnswer {
    pub tool: String,
    pub payload: Value,
    pub diagnostics: Diagnostics,
    pub is_error: bool,
}

impl ToolAnswer {
    /// The provider's own emptiness flag, which is a VALUE and not an absence.
    pub fn is_empty_result(&self) -> bool {
        self.payload
            .get("empty")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }
}

/// How to start the provider. Separable so the protocol can be driven against
/// a stub: the client under test must be the real one, and the only way to do
/// that without a Python environment is to let the command be chosen.
#[derive(Debug, Clone)]
pub struct Spawn {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub timeout: Duration,
}

impl Spawn {
    /// The pinned provider, run through uv from the `provider/` directory so
    /// the lockfile decides what is installed.
    pub fn pinned(provider_dir: &str) -> Self {
        Spawn {
            program: "uv".into(),
            args: ["run", "--frozen", "--no-dev", "openscad-mcp"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            cwd: Some(provider_dir.to_string()),
            timeout: Duration::from_secs(120),
        }
    }
}

/// A live provider process.
pub struct Provider {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<String>,
    next_id: i64,
    timeout: Duration,
    server: ServerInfo,
    protocol: String,
}

impl Provider {
    /// Spawn, `initialize`, and REFUSE if it is not the pinned server.
    pub fn start(spawn: &Spawn, expect: &Expect) -> Result<Self> {
        let mut cmd = Command::new(&spawn.program);
        cmd.args(&spawn.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if let Some(dir) = &spawn.cwd {
            cmd.current_dir(dir);
        }
        let mut child = cmd
            .spawn()
            .with_context(|| format!("spawning provider {:?}", spawn.program))?;
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("no stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("no stdout"))?;

        // A READER THREAD, so a provider that says nothing is a TIMEOUT rather
        // than a hang. A blocked read with no bound is the shape of a lost
        // turn, which the async-jobs issue in this milestone exists to fix at
        // the tool level; here it is the floor under it.
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });

        let mut p = Provider {
            child,
            stdin,
            rx,
            next_id: 0,
            timeout: spawn.timeout,
            server: ServerInfo {
                name: String::new(),
                version: String::new(),
            },
            protocol: String::new(),
        };

        let init = p.request(
            "initialize",
            json!({
                "protocolVersion": expect.protocol,
                "capabilities": {},
                "clientInfo": {"name": "witness-marks", "version": env!("CARGO_PKG_VERSION")},
            }),
        )?;
        let result = init
            .get("result")
            .ok_or_else(|| anyhow!("initialize returned no result: {init}"))?;
        p.server = serde_json::from_value(
            result
                .get("serverInfo")
                .cloned()
                .ok_or_else(|| anyhow!("initialize returned no serverInfo"))?,
        )?;
        p.protocol = result
            .get("protocolVersion")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        expect
            .check(&p.server, &p.protocol)
            .context("the provider is not the pinned one; refusing")?;
        p.notify("notifications/initialized", json!({}))?;
        Ok(p)
    }

    pub fn server(&self) -> &ServerInfo {
        &self.server
    }

    pub fn protocol(&self) -> &str {
        &self.protocol
    }

    fn send(&mut self, v: &Value) -> Result<()> {
        writeln!(self.stdin, "{v}")?;
        self.stdin.flush()?;
        Ok(())
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        let v = json!({"jsonrpc": "2.0", "method": method, "params": params});
        self.send(&v)
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        let v = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        self.send(&v)?;
        // READ UNTIL THE MATCHING ID. A server may interleave notifications,
        // and taking the first line as the answer is how a client comes to
        // report one call's result for another's question.
        loop {
            let line = match self.rx.recv_timeout(self.timeout) {
                Ok(l) => l,
                Err(RecvTimeoutError::Timeout) => {
                    bail!("provider did not answer {method} within {:?}", self.timeout)
                }
                Err(RecvTimeoutError::Disconnected) => {
                    bail!("provider closed its stdout during {method}")
                }
            };
            let v: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue, // not JSON: banner or log line, not an answer
            };
            if v.get("id").and_then(Value::as_i64) == Some(id) {
                if let Some(err) = v.get("error") {
                    bail!("provider returned an error for {method}: {err}");
                }
                return Ok(v);
            }
        }
    }

    /// The provider's advertised tools, with their serialized schemas.
    pub fn list_tools(&mut self) -> Result<Vec<Value>> {
        let v = self.request("tools/list", json!({}))?;
        Ok(v.get("result")
            .and_then(|r| r.get("tools"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default())
    }

    /// Call one tool. The structured payload is preferred over the text
    /// rendering, and the diagnostics ride along either way.
    pub fn call(&mut self, tool: &str, args: Value) -> Result<ToolAnswer> {
        let v = self.request("tools/call", json!({"name": tool, "arguments": args}))?;
        let result = v
            .get("result")
            .ok_or_else(|| anyhow!("tools/call returned no result"))?;
        let is_error = result
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let payload = Self::payload_of(result);
        let diagnostics = Diagnostics::from_payload(&payload);
        Ok(ToolAnswer {
            tool: tool.to_string(),
            payload,
            diagnostics,
            is_error,
        })
    }

    /// Prefer `structuredContent`; fall back to parsing the text block, which
    /// this server also emits as JSON. A text block that is NOT JSON is kept
    /// as a string under `text` rather than discarded.
    fn payload_of(result: &Value) -> Value {
        if let Some(sc) = result.get("structuredContent") {
            if !sc.is_null() {
                return sc.clone();
            }
        }
        if let Some(arr) = result.get("content").and_then(Value::as_array) {
            for c in arr {
                if let Some(t) = c.get("text").and_then(Value::as_str) {
                    if let Ok(v) = serde_json::from_str::<Value>(t) {
                        return v;
                    }
                    let mut m = Map::new();
                    m.insert("text".into(), Value::String(t.to_string()));
                    return Value::Object(m);
                }
            }
        }
        Value::Null
    }

    pub fn measure(&mut self, scad: &str) -> Result<ToolAnswer> {
        self.call("measure", json!({"scad_content": scad}))
    }

    pub fn validate(&mut self, scad: &str) -> Result<ToolAnswer> {
        self.call("validate", json!({"scad_content": scad}))
    }

    pub fn scad_eval(&mut self, scad: &str, exprs: &[&str]) -> Result<ToolAnswer> {
        self.call(
            "scad_eval",
            json!({"scad_content": scad, "expressions": exprs}),
        )
    }
}

/// Hand-written rather than derived, and it prints what the client is TALKING
/// TO. A test that asserts a refusal needs `Debug`, and the useful thing to see
/// when a refusal does not happen is which server answered.
impl std::fmt::Debug for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Provider")
            .field("server", &self.server)
            .field("protocol", &self.protocol)
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl Drop for Provider {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_is_named() {
        assert!(!LAYER.is_empty());
    }

    const TOML: &str = r#"
[provider]
name = "openscad-mcp"
version = "0.6.1"
server_info_name = "OpenSCAD MCP Server"
server_info_version = "4.2.0"
mcp_protocol = "2024-11-05"
"#;

    #[test]
    fn expectations_come_from_the_file() {
        let e = Expect::from_provider_toml(TOML).expect("parses");
        assert_eq!(e.server_name, "OpenSCAD MCP Server");
        assert_eq!(e.server_version, "4.2.0");
        assert_eq!(e.protocol, "2024-11-05");
    }

    /// A PROVIDER.toml that names none of the three must REFUSE. An `Expect`
    /// with blank fields would match any server, which is a guard whose blind
    /// state and whose safe state are the same bytes.
    #[test]
    fn a_file_without_expectations_is_refused() {
        let err =
            Expect::from_provider_toml("[provider]\nname = \"x\"\n").expect_err("must refuse");
        assert!(
            err.to_string().contains("refusing"),
            "the refusal must say so: {err}"
        );
    }

    /// The PyPI version is NOT the serverInfo version, and a client that
    /// compared against 0.6.1 would refuse every real run.
    #[test]
    fn the_pypi_version_is_not_the_server_version() {
        let e = Expect::from_provider_toml(TOML).expect("parses");
        assert_ne!(e.server_version, "0.6.1");
        assert_eq!(e.server_version, "4.2.0");
    }

    fn info(name: &str, version: &str) -> ServerInfo {
        ServerInfo {
            name: name.into(),
            version: version.into(),
        }
    }

    #[test]
    fn the_pinned_server_is_accepted() {
        let e = Expect::from_provider_toml(TOML).expect("parses");
        e.check(&info("OpenSCAD MCP Server", "4.2.0"), "2024-11-05")
            .expect("the pinned server must be accepted");
    }

    #[test]
    fn a_different_version_is_refused() {
        let e = Expect::from_provider_toml(TOML).expect("parses");
        let err = e
            .check(&info("OpenSCAD MCP Server", "4.3.0"), "2024-11-05")
            .expect_err("a different version must be refused");
        assert!(
            err.to_string().contains("4.3.0"),
            "name what it found: {err}"
        );
    }

    #[test]
    fn a_different_server_is_refused() {
        let e = Expect::from_provider_toml(TOML).expect("parses");
        let err = e
            .check(&info("Some Other Server", "4.2.0"), "2024-11-05")
            .expect_err("a different server must be refused");
        assert!(err.to_string().contains("Some Other Server"), "{err}");
    }

    #[test]
    fn a_different_protocol_is_refused() {
        let e = Expect::from_provider_toml(TOML).expect("parses");
        let err = e
            .check(&info("OpenSCAD MCP Server", "4.2.0"), "2025-06-18")
            .expect_err("a different protocol must be refused");
        assert!(err.to_string().contains("2025-06-18"), "{err}");
    }

    /// The emptiness hint is the sentence that says WHY there is no geometry,
    /// and it arrives as an object rather than a string.
    #[test]
    fn hints_survive_as_objects() {
        let payload = json!({
            "empty": true,
            "errors": [],
            "warnings": ["a warning as a plain string"],
            "hints": [{"code": "empty_output", "hint": "A difference() may have removed everything."}],
            "deprecated": [],
        });
        let d = Diagnostics::from_payload(&payload);
        assert_eq!(d.hints.len(), 1);
        assert!(
            d.hints[0].contains("difference()"),
            "the hint's text must survive, got {:?}",
            d.hints
        );
        assert_eq!(d.warnings, vec!["a warning as a plain string"]);
        assert!(!d.is_clean(), "a warning means not clean");
    }

    /// A payload with no diagnostic arrays is CLEAN, not malformed: the
    /// provider omits them when there is nothing to say.
    #[test]
    fn absent_diagnostics_are_clean_not_missing() {
        let d = Diagnostics::from_payload(&json!({"volume": 1000.0}));
        assert!(d.is_clean());
        assert!(d.errors.is_empty() && d.hints.is_empty());
    }

    /// Empty is a VALUE on the provider's wire too, and this is the flag the
    /// envelope's `empty` status is derived from.
    #[test]
    fn empty_is_read_from_the_flag_not_from_a_zero() {
        let empty = ToolAnswer {
            tool: "measure".into(),
            payload: json!({"empty": true, "volume": 0.0, "triangle_count": 0}),
            diagnostics: Diagnostics::default(),
            is_error: false,
        };
        assert!(empty.is_empty_result());
        // A REAL ZERO-VOLUME ANSWER IS NOT THE SAME THING. A 2D result or a
        // degenerate solid can measure 0.0 and be present; only the flag says
        // the model evaluated to no geometry.
        let zero = ToolAnswer {
            payload: json!({"volume": 0.0, "triangle_count": 0}),
            ..empty.clone()
        };
        assert!(
            !zero.is_empty_result(),
            "a zero volume is not the empty flag"
        );
    }

    #[test]
    fn structured_content_wins_over_the_text_rendering() {
        let result = json!({
            "structuredContent": {"volume": 1000.0},
            "content": [{"type": "text", "text": "{\"volume\": 999.0}"}],
        });
        assert_eq!(Provider::payload_of(&result)["volume"], 1000.0);
    }

    /// A text block that is not JSON must be KEPT. The provider renders
    /// argument-validation failures as prose, and discarding it leaves a
    /// caller with an error it cannot read.
    #[test]
    fn a_non_json_text_block_is_kept_not_dropped() {
        let result = json!({
            "content": [{"type": "text", "text": "1 validation error for call[measure]"}],
            "isError": true,
        });
        let p = Provider::payload_of(&result);
        assert!(
            p["text"]
                .as_str()
                .unwrap_or("<no text field at all>")
                .contains("validation error"),
            "got {p}"
        );
    }

    #[test]
    fn the_six_tools_are_named() {
        assert_eq!(TOOLS.len(), 6);
        assert!(TOOLS.contains(&"measure") && TOOLS.contains(&"get_project_files"));
    }
}
