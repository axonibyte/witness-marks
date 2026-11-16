//! Layer E: the one edit-layer tool the provider does not have.
//!
//! The rest of Layer E -- model CRUD with content-hash etags, typed parameter
//! injection by source-wrapping, `scad_eval`, module listing -- exists
//! upstream and is better than what this project would write. Geometric diff
//! does not exist upstream, and it is the one that catches the edit that
//! silently did nothing.

pub mod diff;
pub mod wire;
