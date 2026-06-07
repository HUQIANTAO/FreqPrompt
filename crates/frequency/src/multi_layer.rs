//! Multi-layer scoring: combines frequency, collocation, complexity, and more.
//!
//! Layers:
//! 1. Frequency — TF-IDF weighted Zipf mean (existing)
//! 2. Collocation — bigram/trigram PMI score
//! 3. Complexity — sentence length + avg word length (shorter = better for LLM)
//! 4. Slot preservation — placeholder (filled by semantic crate externally)
//! 5. Domain relevance — placeholder (filled by domain crate externally)

use serde::{Deserialize, Serialize};

/// Per-layer score components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerScores {
    /// Frequency layer: TF-IDF weighted Zipf mean (0-10 scale)
    pub frequency: f64,
    /// Collocation layer: bigram/trigram PMI (0-1 scale)
    pub collocation: f64,
    /// Complexity layer: inverse of syntactic complexity (0-1, higher=simpler)
    pub complexity: f64,
    /// Slot preservation: average slot similarity (0-1, filled externally)
    pub slot_preservation: f64,
    /// Domain relevance: domain-specific frequency boost (0-1, filled externally)
    pub domain_relevance: f64,
}

/// Configurable weights for each layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    pub frequency: f64,
    pub collocation: f64,
    pub complexity: f64,
    pub slot_preservation: f64,
    pub domain_relevance: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            frequency: 0.40,
            collocation: 0.20,
            complexity: 0.15,
            slot_preservation: 0.15,
            domain_relevance: 0.10,
        }
    }
}

impl ScoringWeights {
    /// Validate weights sum to ~1.0.
    pub fn validate(&self) -> bool {
        let sum = self.frequency
            + self.collocation
            + self.complexity
            + self.slot_preservation
            + self.domain_relevance;
        (sum - 1.0).abs() < 0.01
    }

    /// Normalize weights to sum to 1.0.
    pub fn normalize(&mut self) {
        let sum = self.frequency
            + self.collocation
            + self.complexity
            + self.slot_preservation
            + self.domain_relevance;
        if sum > 0.0 {
            self.frequency /= sum;
            self.collocation /= sum;
            self.complexity /= sum;
            self.slot_preservation /= sum;
            self.domain_relevance /= sum;
        }
    }
}

/// Compute multi-layer score from layer scores and weights.
pub fn multi_layer_score(scores: &LayerScores, weights: &ScoringWeights) -> f64 {
    weights.frequency * scores.frequency
        + weights.collocation * scores.collocation
        + weights.complexity * scores.complexity
        + weights.slot_preservation * scores.slot_preservation
        + weights.domain_relevance * scores.domain_relevance
}

/// Compute complexity score for a sentence.
/// Shorter sentences with simpler words score higher.
///
/// Formula: 1.0 / (1.0 + length_penalty + depth_penalty)
/// - length_penalty: (token_count / 20)^0.5
/// - depth_penalty: (avg_word_length / 5)^0.5
pub fn complexity_score(token_count: usize, avg_word_len: f64) -> f64 {
    let length_penalty = (token_count as f64 / 20.0).powf(0.5);
    let depth_penalty = (avg_word_len / 5.0).powf(0.5);
    1.0 / (1.0 + length_penalty + depth_penalty)
}

/// Compute collocation score: fraction of adjacent token pairs that form known bigrams.
pub fn collocation_score(
    tokens: &[String],
    bigram_lookup: &dyn Fn(&str) -> f64,
    threshold: f64,
) -> f64 {
    if tokens.len() < 2 {
        return 1.0; // single token or empty = trivially good
    }

    let mut known = 0;
    let total = tokens.len() - 1;

    for i in 0..total {
        let bigram = format!("{} {}", tokens[i], tokens[i + 1]);
        let score = bigram_lookup(&bigram);
        if score > threshold {
            known += 1;
        }
    }

    known as f64 / total as f64
}

/// Compute average word length (in characters) for a token list.
pub fn avg_word_length(tokens: &[String]) -> f64 {
    if tokens.is_empty() {
        return 0.0;
    }
    let total_chars: usize = tokens.iter().map(|t| t.chars().count()).sum();
    total_chars as f64 / tokens.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_layer_score_default_weights() {
        let scores = LayerScores {
            frequency: 7.0,
            collocation: 0.8,
            complexity: 0.6,
            slot_preservation: 1.0,
            domain_relevance: 0.5,
        };
        let weights = ScoringWeights::default();
        let score = multi_layer_score(&scores, &weights);
        // 0.4*7.0 + 0.2*0.8 + 0.15*0.6 + 0.15*1.0 + 0.1*0.5
        // = 2.8 + 0.16 + 0.09 + 0.15 + 0.05 = 3.25
        assert!((score - 3.25).abs() < 0.01);
    }

    #[test]
    fn test_complexity_score() {
        // Short, simple sentence
        let simple = complexity_score(5, 3.0);
        // Long, complex sentence
        let complex = complexity_score(30, 8.0);
        assert!(simple > complex);
        assert!(simple > 0.3);
        assert!(complex < 0.3);
    }

    #[test]
    fn test_collocation_score_all_known() {
        let tokens: Vec<String> = vec!["machine", "learning"]
            .into_iter()
            .map(String::from)
            .collect();
        let lookup = |_: &str| 7.0; // all bigrams known
        let score = collocation_score(&tokens, &lookup, 5.0);
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_collocation_score_partial() {
        let tokens: Vec<String> = vec!["a", "b", "c"]
            .into_iter()
            .map(String::from)
            .collect();
        let lookup = |bg: &str| {
            if bg == "a b" { 7.0 } else { 2.0 } // only "a b" is known
        };
        let score = collocation_score(&tokens, &lookup, 5.0);
        assert!((score - 0.5).abs() < 0.01); // 1 of 2 bigrams
    }

    #[test]
    fn test_avg_word_length() {
        let tokens: Vec<String> = vec!["hello", "world", "hi"]
            .into_iter()
            .map(String::from)
            .collect();
        let avg = avg_word_length(&tokens);
        assert!((avg - 4.0).abs() < 0.01); // (5+5+2)/3 = 4.0
    }

    #[test]
    fn test_weights_normalize() {
        let mut weights = ScoringWeights {
            frequency: 4.0,
            collocation: 2.0,
            complexity: 1.5,
            slot_preservation: 1.5,
            domain_relevance: 1.0,
        };
        weights.normalize();
        assert!(weights.validate());
        assert!((weights.frequency - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_slot_preservation_boosts_score() {
        let scores_low = LayerScores {
            frequency: 7.0,
            collocation: 0.8,
            complexity: 0.6,
            slot_preservation: 0.3, // bad preservation
            domain_relevance: 0.5,
        };
        let scores_high = LayerScores {
            frequency: 7.0,
            collocation: 0.8,
            complexity: 0.6,
            slot_preservation: 1.0, // perfect preservation
            domain_relevance: 0.5,
        };
        let weights = ScoringWeights::default();
        let low = multi_layer_score(&scores_low, &weights);
        let high = multi_layer_score(&scores_high, &weights);
        assert!(high > low);
    }
}
