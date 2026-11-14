//! The geometry provider client. Pinned to RobertCoop/openscad-mcp (see provider/PROVIDER.toml); spoken to over MCP stdio, never imported. Spec §03, §04.
//!
//! Skeleton. Each public item added here arrives with the test that would
//! have caught its absence, and a mutation check recorded in the PR.

/// Present so the crate has one exported symbol and one test from day one.
pub const LAYER: &str = "wm-provider";

#[cfg(test)]
mod tests {
    #[test]
    fn layer_is_named() {
        assert!(!super::LAYER.is_empty());
    }
}
