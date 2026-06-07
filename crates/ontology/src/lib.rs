//! FreqPrompt v3 — Ontology Guard
//!
//! Prevents parent↔child substitution (财政政策 → 国税规定) by validating
//! that candidate replacements stay within the same semantic level.

pub mod graph;
pub mod substitute;
pub mod domain;

pub use graph::{Ontology, Concept};
pub use substitute::{can_substitute, SubstitutionVerdict};
pub use domain::{detect_domain, DomainMatch};
