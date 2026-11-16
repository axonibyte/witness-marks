//! Layer 1b: queries over the construction tree -- eats, flank, coincident. Every Layer 1 call runs on the RESULT; class G defects live in the operations. Spec §05.
//!
//! Skeleton. Each public item added here arrives with the test that would
//! have caught its absence, and a mutation check recorded in the PR.

/// Present so the crate has one exported symbol and one test from day one.
pub mod tree;

pub const LAYER: &str = "wm-csg";

#[cfg(test)]
mod tests {
    #[test]
    fn layer_is_named() {
        assert!(!super::LAYER.is_empty());
    }
}
