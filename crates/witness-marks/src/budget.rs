//! A budget on our own tool surface.
//!
//! ADOPTED FROM THE PROVIDER, which keeps one for the same reason: the schema
//! of every registered tool is paid on EVERY request, and tool-selection
//! accuracy degrades as it grows. A surface is not a thing anyone notices
//! growing, which is why it needs a number and a test rather than judgment.
//!
//! THE REFERENCE POINT IS MEASURED, not guessed. The pinned provider advertises
//! 12 tools totalling **19174 bytes** of serialized schema, read off the wire
//! on 2026-11-16 (`cargo run -p wm-provider --example smoke`). Its single
//! largest tool, `render`, is 2263 bytes. Those are the numbers the caps below
//! are set against.
//!
//! THE CAPS ARE PROVISIONAL AND SAY SO. Witness Marks registers no tools yet,
//! so no cap here has been earned by a real tool pressing against it. They are
//! placed now because the issue's own point is that the test exists FIRST: a
//! budget introduced after the surface is already large is a budget written
//! around whatever happened.

use serde_json::Value;

/// Total serialized bytes across every registered tool.
///
/// Set at the provider's measured 19174 rounded up to 20000. Witness Marks
/// owns strictly more layers than the provider does, so an equal budget is a
/// deliberate constraint rather than an allowance.
pub const TOTAL_CAP: usize = 20_000;

/// Serialized bytes for any one tool, at the provider's largest (`render`,
/// 2263) rounded up to 2500.
pub const PER_TOOL_CAP: usize = 2_500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Surface {
    pub total: usize,
    pub per_tool: Vec<(String, usize)>,
}

/// Measure the serialized size of each tool and the whole set.
///
/// SERIALIZED, NOT STRUCTURAL. What costs a request is the bytes on the wire,
/// so the measurement is `to_string().len()` of each tool object rather than a
/// count of fields, which would miss a tool whose descriptions are essays.
pub fn measure(tools: &[Value]) -> Surface {
    let per_tool: Vec<(String, usize)> = tools
        .iter()
        .map(|t| {
            let name = t
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("<unnamed>")
                .to_string();
            (name, t.to_string().len())
        })
        .collect();
    Surface {
        total: per_tool.iter().map(|(_, n)| n).sum(),
        per_tool,
    }
}

/// Every way this surface is over budget, named. Empty means within budget.
///
/// RETURNS ALL OF THEM rather than the first. A caller told only about the
/// worst tool fixes it, re-runs, and is told about the next one; a list says
/// how much work there is.
pub fn over_budget(s: &Surface) -> Vec<String> {
    let mut out = Vec::new();
    if s.total > TOTAL_CAP {
        out.push(format!(
            "the whole surface is {} bytes, over the {TOTAL_CAP} cap by {}",
            s.total,
            s.total - TOTAL_CAP
        ));
    }
    for (name, n) in &s.per_tool {
        if *n > PER_TOOL_CAP {
            out.push(format!(
                "tool {name:?} is {n} bytes, over the {PER_TOOL_CAP} per-tool cap by {}",
                n - PER_TOOL_CAP
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tool(name: &str, pad: usize) -> Value {
        json!({"name": name, "description": "x".repeat(pad), "inputSchema": {"type": "object"}})
    }

    #[test]
    fn the_measurement_is_the_serialized_bytes() {
        let t = tool("measure", 10);
        let s = measure(std::slice::from_ref(&t));
        assert_eq!(s.per_tool.len(), 1);
        assert_eq!(s.per_tool[0].0, "measure");
        assert_eq!(
            s.per_tool[0].1,
            t.to_string().len(),
            "the size must be what goes on the wire"
        );
        assert_eq!(s.total, s.per_tool[0].1);
    }

    /// A tool whose DESCRIPTION is an essay costs the same as one whose schema
    /// is, and a structural count would miss it.
    #[test]
    fn a_long_description_counts_against_the_budget() {
        let small = measure(&[tool("a", 10)]);
        let large = measure(&[tool("a", 5_000)]);
        assert!(large.total > small.total + 4_000);
        assert!(
            !over_budget(&large).is_empty(),
            "a 5000-character description must breach the per-tool cap"
        );
    }

    /// THE CONTROL. Without an arm that sees the budget REFUSE, every other arm
    /// here is equally satisfied by a function that always returns no
    /// complaints, which is exactly what it would do today: we register no
    /// tools, so measuring the real surface alone proves nothing.
    #[test]
    fn the_budget_bites_and_names_what_breached_it() {
        let over = measure(&[tool("huge", PER_TOOL_CAP + 1), tool("small", 5)]);
        let why = over_budget(&over);
        assert!(!why.is_empty(), "the budget must refuse");
        assert!(
            why.iter().any(|w| w.contains("huge")),
            "it must name the tool: {why:?}"
        );
        assert!(
            !why.iter().any(|w| w.contains("small")),
            "it must not blame the innocent one: {why:?}"
        );
    }

    #[test]
    fn every_breach_is_reported_not_only_the_first() {
        let over = measure(&[
            tool("a", PER_TOOL_CAP + 1),
            tool("b", PER_TOOL_CAP + 1),
            tool("c", TOTAL_CAP),
        ]);
        let why = over_budget(&over);
        assert!(
            why.len() >= 3,
            "the total and both tools must each be named: {why:?}"
        );
        assert!(why.iter().any(|w| w.contains("whole surface")));
    }

    #[test]
    fn a_surface_within_budget_has_nothing_to_say() {
        assert!(over_budget(&measure(&[tool("a", 10), tool("b", 20)])).is_empty());
    }

    /// OUR REAL SURFACE, and the honest reading of it today.
    ///
    /// It is ZERO because no tool is registered yet, and this arm asserts that
    /// rather than passing quietly: when the first tool lands, this number
    /// changes and the assertion below is what makes someone look at it.
    #[test]
    fn our_own_surface_is_measured_and_is_currently_empty() {
        let listed = crate::dispatch("tools/list", None).expect("tools/list answers");
        let tools = listed
            .get("tools")
            .and_then(Value::as_array)
            .expect("tools/list returns an array");
        let s = measure(tools);
        assert_eq!(
            tools.len(),
            0,
            "witness-marks registers no tools yet; when that changes, re-read \
             the caps in this module against the provider's measured 19174"
        );
        assert_eq!(s.total, 0);
        assert!(over_budget(&s).is_empty());
    }

    /// The provider's measured surface fits the caps it inspired, which is the
    /// check that they are not arbitrary.
    #[test]
    fn the_caps_admit_the_provider_they_were_set_from() {
        assert!(
            TOTAL_CAP >= 19_174,
            "the cap must admit the provider's measured surface"
        );
        assert!(PER_TOOL_CAP >= 2_263, "and its largest single tool");
    }
}
