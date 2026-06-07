//! Ontology graph: concepts with hypernym/hyponym/sibling relations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single concept in the ontology.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    /// Unique identifier (e.g., "finance:fiscal_policy")
    pub id: String,
    /// Display text (e.g., "财政政策")
    pub text: String,
    /// Parent concepts (broader terms)
    #[serde(default)]
    pub hypernyms: Vec<String>,
    /// Child concepts (narrower terms)
    #[serde(default)]
    pub hyponyms: Vec<String>,
    /// Sibling concepts (same level, different branch)
    #[serde(default)]
    pub siblings: Vec<String>,
    /// Domain this concept belongs to
    #[serde(default)]
    pub domain: String,
    /// Alias forms (alternative surface forms for the same concept)
    #[serde(default)]
    pub aliases: Vec<String>,
}

/// The full ontology graph, indexed by concept text for fast lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ontology {
    /// Domain name (e.g., "finance", "general")
    pub domain: String,
    /// Language ("zh" or "en")
    pub lang: String,
    /// Concepts indexed by their text
    pub concepts: HashMap<String, Concept>,
}

impl Ontology {
    /// Load from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Look up a concept by text or alias.
    pub fn find(&self, text: &str) -> Option<&Concept> {
        // Direct lookup
        if let Some(c) = self.concepts.get(text) {
            return Some(c);
        }
        // Alias lookup
        self.concepts.values().find(|c| c.aliases.contains(&text.to_string()))
    }

    /// Check if `a` is a hypernym (parent) of `b`.
    pub fn is_hypernym_of(&self, a: &str, b: &str) -> bool {
        if let Some(concept_b) = self.find(b) {
            // Direct hypernym
            if concept_b.hypernyms.iter().any(|h| h == a) {
                return true;
            }
            // Recursive: check if any hypernym of b is a, or a is hypernym of that hypernym
            for hypernym in &concept_b.hypernyms {
                if self.is_hypernym_of(a, hypernym) {
                    return true;
                }
            }
        }
        false
    }

    /// Check if `a` is a hyponym (child) of `b`.
    pub fn is_hyponym_of(&self, a: &str, b: &str) -> bool {
        self.is_hypernym_of(b, a)
    }

    /// Check if `a` and `b` are siblings (share a direct hypernym).
    pub fn are_siblings(&self, a: &str, b: &str) -> bool {
        if let (Some(ca), Some(cb)) = (self.find(a), self.find(b)) {
            // Direct sibling reference
            if ca.siblings.iter().any(|s| s == b) || cb.siblings.iter().any(|s| s == a) {
                return true;
            }
            // Share a hypernym
            for ha in &ca.hypernyms {
                if cb.hypernyms.contains(ha) {
                    return true;
                }
            }
        }
        false
    }

    /// Check if two concepts are the same (direct match or alias).
    pub fn is_same_concept(&self, a: &str, b: &str) -> bool {
        if a == b {
            return true;
        }
        // Check if they resolve to the same concept
        match (self.find(a), self.find(b)) {
            (Some(ca), Some(cb)) => ca.id == cb.id,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_ontology() -> Ontology {
        let json = r#"{
            "domain": "finance",
            "lang": "zh",
            "concepts": {
                "财政政策": {
                    "id": "finance:fiscal_policy",
                    "text": "财政政策",
                    "hypernyms": ["经济政策"],
                    "hyponyms": ["税收政策", "支出政策", "公债政策"],
                    "siblings": ["货币政策"],
                    "domain": "finance",
                    "aliases": ["财政举措", "财政手段"]
                },
                "税收政策": {
                    "id": "finance:tax_policy",
                    "text": "税收政策",
                    "hypernyms": ["财政政策"],
                    "hyponyms": ["国税政策", "地税政策"],
                    "siblings": ["支出政策", "公债政策"],
                    "domain": "finance",
                    "aliases": ["税收规定"]
                },
                "国税政策": {
                    "id": "finance:national_tax_policy",
                    "text": "国税政策",
                    "hypernyms": ["税收政策"],
                    "hyponyms": [],
                    "siblings": ["地税政策"],
                    "domain": "finance",
                    "aliases": ["国税规定"]
                },
                "支出政策": {
                    "id": "finance:spending_policy",
                    "text": "支出政策",
                    "hypernyms": ["财政政策"],
                    "hyponyms": [],
                    "siblings": ["税收政策", "公债政策"],
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
    fn test_find_direct() {
        let onto = make_test_ontology();
        assert!(onto.find("财政政策").is_some());
        assert!(onto.find("不存在").is_none());
    }

    #[test]
    fn test_find_alias() {
        let onto = make_test_ontology();
        assert!(onto.find("国税规定").is_some()); // alias of 国税政策
        assert!(onto.find("财政举措").is_some()); // alias of 财政政策
    }

    #[test]
    fn test_hypernym() {
        let onto = make_test_ontology();
        assert!(onto.is_hypernym_of("财政政策", "税收政策"));
        assert!(onto.is_hypernym_of("财政政策", "国税政策")); // transitive
        assert!(onto.is_hypernym_of("经济政策", "国税政策")); // 2 hops
        assert!(!onto.is_hypernym_of("税收政策", "财政政策")); // reverse
    }

    #[test]
    fn test_sibling() {
        let onto = make_test_ontology();
        assert!(onto.are_siblings("税收政策", "支出政策"));
        assert!(onto.are_siblings("货币政策", "财政政策"));
        assert!(!onto.are_siblings("财政政策", "国税政策")); // parent-child, not siblings
    }

    #[test]
    fn test_same_concept() {
        let onto = make_test_ontology();
        assert!(onto.is_same_concept("财政政策", "财政政策"));
        assert!(onto.is_same_concept("国税政策", "国税规定")); // alias
        assert!(!onto.is_same_concept("财政政策", "税收政策"));
    }
}
