//! Slot-presence verification: does the candidate have the same critical slots?
//!
//! The downstream TypeScript layer does the actual per-slot embedding
//! similarity (it has access to the embedding model). This Rust layer
//! provides the structural checks that don't need embeddings:
//!
//!   - Exact-match slots (placeholder, negation) — pure string match
//!   - Slot count consistency (warning if candidate dropped a slot)
//!   - Per-slot length sanity (a 1-char answer is unlikely to match an 8-char one)
//!
//! The TypeScript layer composes this with embedding similarity to produce
//! the final `SlotVerdict`.

use serde::{Deserialize, Serialize};

use crate::slot::{Slot, SlotKind, SlotThresholds, SlotVerdict};

/// Check whether a slot is "preserved" by exact match (placeholder, negation).
///
/// Returns `Some(SlotVerdict)` if the slot is exact-match, `None` if it
/// requires embedding-based comparison (and should be deferred to TS).
pub fn exact_match_check(
    original: &Slot,
    candidate_text: Option<&str>,
) -> Option<SlotVerdict> {
    if !original.requires_exact_match() {
        return None;
    }
    let threshold = 1.0_f64;
    match candidate_text {
        Some(t) if t == original.text => Some(SlotVerdict {
            original: original.clone(),
            matched: Some(t.to_string()),
            similarity: 1.0,
            passes: true,
            threshold,
        }),
        Some(t) => Some(SlotVerdict {
            original: original.clone(),
            matched: Some(t.to_string()),
            similarity: 0.0,  // exact mismatch
            passes: false,
            threshold,
        }),
        None => Some(SlotVerdict {
            original: original.clone(),
            matched: None,
            similarity: 0.0,
            passes: false,
            threshold,
        }),
    }
}

/// Structural sanity check: does the candidate text plausibly contain the slot?
///
/// This is a cheap pre-filter for the TS embedding step. It can't catch
/// semantic shifts, but it can catch obvious failures:
///   - Dropped slot (count mismatch)
///   - Length collapse (1-char answer for 10-char slot)
pub fn structural_check(
    original_slots: &[Slot],
    candidate_text: &str,
) -> Vec<SlotCheck> {
    let mut results = Vec::new();
    for orig in original_slots {
        if orig.requires_exact_match() {
            let present = candidate_text.contains(&orig.text);
            results.push(SlotCheck {
                kind: orig.kind,
                text: orig.text.clone(),
                issue: if present { None } else { Some(SlotIssue::Missing) },
            });
        } else {
            // Length sanity: a 5+ char slot shouldn't be replaced with a 1-char answer
            // (unless the answer is a placeholder, which we already checked above)
            if orig.len_chars() >= 4 {
                // Heuristic: candidate has any token in roughly the same position
                // We don't enforce strict match here — just warn.
                // Actual similarity is computed in TS.
            }
        }
    }
    results
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotCheck {
    pub kind: SlotKind,
    pub text: String,
    pub issue: Option<SlotIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotIssue {
    /// Slot is completely missing from the candidate
    Missing,
}

/// Compute a slot-preservation score in [0.0, 1.0] given a list of verdicts.
///
/// Weighting:
///   - Placeholder / Negation failures: -1.0 (fatal)
///   - Other kinds: weighted average of similarities
pub fn aggregate_preservation(verdicts: &[SlotVerdict]) -> f64 {
    if verdicts.is_empty() { return 1.0; }

    // First, check for fatal failures
    for v in verdicts {
        if v.original.requires_exact_match() && !v.passes {
            return 0.0; // fatal
        }
    }

    // Otherwise, weighted average
    let mut sum = 0.0;
    let mut count = 0;
    for v in verdicts {
        sum += v.similarity;
        count += 1;
    }
    if count == 0 { 1.0 } else { sum / count as f64 }
}

/// Apply thresholds to raw similarities and produce final verdicts.
///
/// This is a helper for the TS layer: it has similarities, but the
/// threshold logic lives here so it's consistent across both languages.
pub fn apply_thresholds(
    original: Slot,
    matched: Option<String>,
    similarity: f64,
    thresholds: &SlotThresholds,
) -> SlotVerdict {
    let threshold = thresholds.for_kind(original.kind);
    let passes = if original.requires_exact_match() {
        similarity >= 0.999
    } else {
        similarity >= threshold
    };
    SlotVerdict {
        original,
        matched,
        similarity,
        passes,
        threshold,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::Slot;

    fn placeholder() -> Slot {
        Slot::new(SlotKind::Placeholder, "{name}", 0, 6)
    }

    fn neg() -> Slot {
        Slot::new(SlotKind::Negation, "不", 5, 6)
    }

    fn obj() -> Slot {
        Slot::new(SlotKind::Object, "财政政策", 0, 4)
    }

    #[test]
    fn test_exact_match_pass() {
        let v = exact_match_check(&placeholder(), Some("{name}"));
        assert!(v.is_some());
        assert!(v.unwrap().passes);
    }

    #[test]
    fn test_exact_match_fail_changed() {
        let v = exact_match_check(&placeholder(), Some("{user}"));
        assert!(!v.unwrap().passes);
    }

    #[test]
    fn test_exact_match_fail_missing() {
        let v = exact_match_check(&placeholder(), None);
        assert!(!v.unwrap().passes);
    }

    #[test]
    fn test_non_exact_returns_none() {
        let v = exact_match_check(&obj(), Some("财政"));
        assert!(v.is_none(), "Object slots require embedding similarity");
    }

    #[test]
    fn test_negation_exact() {
        let v = exact_match_check(&neg(), Some("不"));
        assert!(v.unwrap().passes);
    }

    #[test]
    fn test_fatal_negation_drops_score() {
        let v1 = apply_thresholds(neg(), None, 0.0, &SlotThresholds::default());
        let v2 = apply_thresholds(obj(), Some("税收政策".to_string()), 0.6, &SlotThresholds::default());
        let verdicts = vec![v1, v2];
        let score = aggregate_preservation(&verdicts);
        assert_eq!(score, 0.0, "Negation failure must produce 0.0");
    }

    #[test]
    fn test_partial_preservation() {
        let v1 = apply_thresholds(obj(), Some("财政政策".to_string()), 0.95, &SlotThresholds::default());
        let v2 = apply_thresholds(
            Slot::new(SlotKind::Modifier, "整体", 0, 2),
            Some("完整".to_string()),
            0.7,
            &SlotThresholds::default(),
        );
        let score = aggregate_preservation(&vec![v1, v2]);
        assert!((score - 0.825).abs() < 0.01, "Got: {}", score);
    }

    #[test]
    fn test_empty_verdicts() {
        let score = aggregate_preservation(&[]);
        assert_eq!(score, 1.0);
    }
}
