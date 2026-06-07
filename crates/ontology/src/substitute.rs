//! Substitution validator: can term A be replaced by term B?

use crate::graph::Ontology;
use serde::{Deserialize, Serialize};

/// Verdict on whether a substitution is semantically safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubstitutionVerdict {
    /// Same concept or true synonym — safe.
    Allowed,
    /// A is broader than B (parent→child): narrows meaning. BLOCKED.
    Narrowing,
    /// B is broader than A (child→parent): widens meaning. Usually safe but noted.
    Widening,
    /// No ontological relation found — can't validate, assume allowed.
    Unrelated,
    /// Both terms exist but are in different branches — risky.
    CrossBranch,
}

impl SubstitutionVerdict {
    pub fn is_safe(&self) -> bool {
        matches!(self, Self::Allowed | Self::Widening | Self::Unrelated)
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Allowed => "等价替换",
            Self::Narrowing => "语义收窄（父→子）",
            Self::Widening => "语义扩展（子→父）",
            Self::Unrelated => "无本体关系",
            Self::CrossBranch => "跨分支替换",
        }
    }
}

/// Check if replacing `original` with `candidate` is semantically safe.
///
/// Logic:
/// 1. Same concept or alias → Allowed
/// 2. original is hypernym of candidate → Narrowing (BLOCKED)
/// 3. candidate is hypernym of original → Widening (allowed but noted)
/// 4. Siblings → CrossBranch (risky, blocked)
/// 5. Not in ontology → Unrelated (allowed, no info to block)
pub fn can_substitute(original: &str, candidate: &str, ontology: &Ontology) -> SubstitutionVerdict {
    // Fast path: identical
    if original == candidate {
        return SubstitutionVerdict::Allowed;
    }

    let orig_concept = ontology.find(original);
    let cand_concept = ontology.find(candidate);

    match (orig_concept, cand_concept) {
        (Some(_), Some(_)) => {
            // Both in ontology — check relations
            if ontology.is_same_concept(original, candidate) {
                SubstitutionVerdict::Allowed
            } else if ontology.is_hypernym_of(original, candidate) {
                // original is parent of candidate: narrowing
                SubstitutionVerdict::Narrowing
            } else if ontology.is_hypernym_of(candidate, original) {
                // candidate is parent of original: widening
                SubstitutionVerdict::Widening
            } else if ontology.are_siblings(original, candidate) {
                // Same level but different branch
                SubstitutionVerdict::CrossBranch
            } else {
                // Both in ontology but no direct relation
                SubstitutionVerdict::Unrelated
            }
        }
        (Some(_), None) | (None, Some(_)) => {
            // One in ontology, one not — can't validate
            SubstitutionVerdict::Unrelated
        }
        (None, None) => {
            // Neither in ontology — no info
            SubstitutionVerdict::Unrelated
        }
    }
}

/// Batch check: validate a list of (original, candidate) pairs.
/// Returns only the verdicts that are NOT safe.
pub fn check_substitutions(
    pairs: &[(&str, &str)],
    ontology: &Ontology,
) -> Vec<(String, String, SubstitutionVerdict)> {
    pairs
        .iter()
        .filter_map(|(orig, cand)| {
            let verdict = can_substitute(orig, cand, ontology);
            if !verdict.is_safe() {
                Some((orig.to_string(), cand.to_string(), verdict))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Ontology;

    fn make_ontology() -> Ontology {
        let json = r#"{
            "domain": "finance",
            "lang": "zh",
            "concepts": {
                "财政政策": {
                    "id": "finance:fiscal_policy",
                    "text": "财政政策",
                    "hypernyms": ["经济政策"],
                    "hyponyms": ["税收政策", "支出政策"],
                    "siblings": ["货币政策"],
                    "domain": "finance",
                    "aliases": ["财政举措"]
                },
                "税收政策": {
                    "id": "finance:tax_policy",
                    "text": "税收政策",
                    "hypernyms": ["财政政策"],
                    "hyponyms": ["国税政策"],
                    "siblings": ["支出政策"],
                    "domain": "finance",
                    "aliases": ["国税规定"]
                },
                "国税政策": {
                    "id": "finance:national_tax",
                    "text": "国税政策",
                    "hypernyms": ["税收政策"],
                    "hyponyms": [],
                    "siblings": [],
                    "domain": "finance",
                    "aliases": []
                },
                "经济政策": {
                    "id": "finance:economic_policy",
                    "text": "经济政策",
                    "hypernyms": [],
                    "hyponyms": ["财政政策", "货币政策"],
                    "siblings": [],
                    "domain": "finance",
                    "aliases": []
                },
                "货币政策": {
                    "id": "finance:monetary_policy",
                    "text": "货币政策",
                    "hypernyms": ["经济政策"],
                    "hyponyms": [],
                    "siblings": ["财政政策"],
                    "domain": "finance",
                    "aliases": []
                }
            }
        }"#;
        Ontology::from_json(json).unwrap()
    }

    #[test]
    fn test_same_concept_allowed() {
        let onto = make_ontology();
        assert_eq!(can_substitute("财政政策", "财政政策", &onto), SubstitutionVerdict::Allowed);
    }

    #[test]
    fn test_alias_allowed() {
        let onto = make_ontology();
        assert_eq!(can_substitute("财政政策", "财政举措", &onto), SubstitutionVerdict::Allowed);
    }

    #[test]
    fn test_narrowing_blocked() {
        let onto = make_ontology();
        // 财政政策 → 税收政策 = parent→child = narrowing
        assert_eq!(can_substitute("财政政策", "税收政策", &onto), SubstitutionVerdict::Narrowing);
        // 财政政策 → 国税政策 = grandparent→grandchild
        assert_eq!(can_substitute("财政政策", "国税政策", &onto), SubstitutionVerdict::Narrowing);
    }

    #[test]
    fn test_widening() {
        let onto = make_ontology();
        // 税收政策 → 财政政策 = child→parent = widening
        assert_eq!(can_substitute("税收政策", "财政政策", &onto), SubstitutionVerdict::Widening);
    }

    #[test]
    fn test_sibling_cross_branch() {
        let onto = make_ontology();
        // 财政政策 ↔ 货币政策 = siblings
        assert_eq!(can_substitute("财政政策", "货币政策", &onto), SubstitutionVerdict::CrossBranch);
    }

    #[test]
    fn test_unrelated_term() {
        let onto = make_ontology();
        // "说明" not in ontology → Unrelated
        assert_eq!(can_substitute("财政政策", "说明", &onto), SubstitutionVerdict::Unrelated);
    }

    #[test]
    fn test_batch_check() {
        let onto = make_ontology();
        let pairs = vec![
            ("财政政策", "税收政策"),   // narrowing
            ("财政政策", "财政举措"),   // allowed (alias)
            ("说明", "解释"),           // unrelated (not in ontology)
        ];
        let blocked = check_substitutions(&pairs, &onto);
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].2, SubstitutionVerdict::Narrowing);
    }
}
