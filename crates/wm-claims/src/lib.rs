//! Layer 5: assertions that re-evaluate on every change, staleness propagation, dominance, fan-out. Spec §05, §06.
//!
//! Skeleton. Each public item added here arrives with the test that would
//! have caught its absence, and a mutation check recorded in the PR.

/// Present so the crate has one exported symbol and one test from day one.
pub mod claim;
pub mod dominance;
pub mod fanout;
pub mod provenance;
pub mod refusals;
pub mod stale;

pub const LAYER: &str = "wm-claims";

#[cfg(test)]
mod tests {
    #[test]
    fn layer_is_named() {
        assert!(!super::LAYER.is_empty());
    }
}
