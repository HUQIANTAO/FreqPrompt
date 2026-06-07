//! Hybrid scoring: blend general Zipf with domain-specific Zipf.

use serde::{Deserialize, Serialize};

/// Configuration for hybrid scoring weights.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridConfig {
    /// Weight for general (global) Zipf frequency
    pub alpha: f64,
    /// Weight for domain-specific Zipf frequency
    pub beta: f64,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            alpha: 0.6,
            beta: 0.4,
        }
    }
}

/// Compute hybrid score for a word given both general and domain Zipf values.
///
/// If the word is not in the domain table, falls back to general-only.
pub fn hybrid_score(
    general_zipf: f64,
    domain_zipf: Option<f64>,
    config: &HybridConfig,
) -> f64 {
    match domain_zipf {
        Some(dz) => config.alpha * general_zipf + config.beta * dz,
        None => general_zipf,
    }
}

/// Compute hybrid sentence score: geometric mean of per-word hybrid scores.
pub fn hybrid_sentence_score(
    general_zipfs: &[f64],
    domain_zipfs: &[Option<f64>],
    config: &HybridConfig,
) -> f64 {
    assert_eq!(general_zipfs.len(), domain_zipfs.len());
    if general_zipfs.is_empty() {
        return 0.0;
    }

    let log_sum: f64 = general_zipfs
        .iter()
        .zip(domain_zipfs.iter())
        .map(|(gz, dz)| {
            let h = hybrid_score(*gz, *dz, config);
            (h + 1e-12).ln()
        })
        .sum();

    (log_sum / general_zipfs.len() as f64).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_with_domain() {
        let config = HybridConfig::default(); // alpha=0.6, beta=0.4
        let score = hybrid_score(6.0, Some(8.0), &config);
        // 0.6 * 6.0 + 0.4 * 8.0 = 3.6 + 3.2 = 6.8
        assert!((score - 6.8).abs() < 0.01);
    }

    #[test]
    fn test_hybrid_without_domain() {
        let config = HybridConfig::default();
        let score = hybrid_score(6.0, None, &config);
        assert!((score - 6.0).abs() < 0.01);
    }

    #[test]
    fn test_hybrid_sentence() {
        let config = HybridConfig::default();
        let general = vec![6.0, 7.0, 5.0];
        let domain = vec![Some(8.0), None, Some(6.0)];
        let score = hybrid_sentence_score(&general, &domain, &config);
        // Should be geometric mean of hybrid scores
        assert!(score > 0.0);
        assert!(score < 10.0);
    }

    #[test]
    fn test_domain_boosts_score() {
        let config = HybridConfig::default();
        let general = vec![5.0, 5.0];
        let domain_none: Vec<Option<f64>> = vec![None, None];
        let domain_some = vec![Some(9.0), Some(9.0)];

        let score_without = hybrid_sentence_score(&general, &domain_none, &config);
        let score_with = hybrid_sentence_score(&general, &domain_some, &config);
        // Domain-specific frequencies are higher, so hybrid should be higher
        assert!(score_with > score_without);
    }
}
