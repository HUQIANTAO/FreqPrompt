//! FreqPrompt v3 — Domain Adaptation
//!
//! Custom frequency tables built from user-uploaded corpora.
//! Hybrid scoring: `α · general_zipf + β · domain_zipf`.

pub mod freq_table;
pub mod hybrid;

pub use freq_table::{DomainFreqTable, WordFreq};
pub use hybrid::{hybrid_score, hybrid_sentence_score, HybridConfig};
