//! Witness Marks: hands and a tape measure for agents that model in OpenSCAD.
//!
//! MCP over stdio, one JSON object per line, no framework, `serde_json` only --
//! the same shape as lychgate-mcp. Handles `initialize`, `tools/list`,
//! `tools/call` and `ping`; ignores notifications (no `id`).

use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

pub mod budget;

const PROTOCOL: &str = "2025-06-18";

fn dispatch(method: &str, _params: Option<&Value>) -> Result<Value, (i64, String)> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "witness-marks", "version": env!("CARGO_PKG_VERSION") }
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": [] })),
        "tools/call" => Err((-32601, "no tools registered yet".into())),
        other => Err((-32601, format!("method not found: {other}"))),
    }
}

/// One request line in, one response line out. Pure, so it can be tested
/// without a process.
pub fn handle_line(line: &str) -> Option<String> {
    let msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return Some(json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":format!("parse error: {e}")}}).to_string()),
    };
    let id = msg.get("id")?.clone(); // a notification has no id: no reply
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let body = match dispatch(method, msg.get("params")) {
        Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
        Err((code, message)) => {
            json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
        }
    };
    Some(body.to_string())
}

fn main() -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut out = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = handle_line(&line) {
            out.write_all(reply.as_bytes())?;
            out.write_all(b"\n")?;
            out.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_answers_with_server_info() {
        let r = handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
            .expect("reply");
        let v: Value = serde_json::from_str(&r).expect("json");
        assert_eq!(v["id"], 1);
        assert_eq!(v["result"]["serverInfo"]["name"], "witness-marks");
    }

    /// A notification carries no id and must get no reply -- replying to one
    /// desynchronizes the client. Mutation check: replace the `?` on `id` with
    /// a default and this fails.
    #[test]
    fn notifications_get_no_reply() {
        assert!(handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
    }

    #[test]
    fn unknown_method_is_a_jsonrpc_error_not_a_crash() {
        let r = handle_line(r#"{"jsonrpc":"2.0","id":7,"method":"nope"}"#).expect("reply");
        let v: Value = serde_json::from_str(&r).expect("json");
        assert_eq!(v["error"]["code"], -32601);
    }
}
