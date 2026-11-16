//! Layer 2: assembly. Spec §05.
//!
//! A Layer 1 call answers about ONE solid. Assembly is about PAIRS and about
//! TIME: whether a part can be put in, what stops it first, and whether every
//! mating pair has the sign the design intended.

pub mod fits;
pub mod travel;

pub const LAYER: &str = "wm-asm";

#[cfg(test)]
mod tests {
    #[test]
    fn layer_is_named() {
        assert!(!super::LAYER.is_empty());
    }
}
