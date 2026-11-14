//! Layer 4: printer, material, firmware and slicer as a model. The slicer criterion is unsupported(z) = slice(z) MINUS offset(slice(z-h), +d), d = layer_h/tan(theta). Spec §05.
//!
//! Skeleton. Each public item added here arrives with the test that would
//! have caught its absence, and a mutation check recorded in the PR.

/// Present so the crate has one exported symbol and one test from day one.
pub const LAYER: &str = "wm-process";

#[cfg(test)]
mod tests {
    #[test]
    fn layer_is_named() {
        assert!(!super::LAYER.is_empty());
    }
}
