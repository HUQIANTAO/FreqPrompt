use frequency;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
struct FrequencyResult {
    text: String,
    zipf_score: f64,
}

#[derive(Serialize)]
struct OptimizeResult {
    original: FrequencyResult,
    optimized: FrequencyResult,
    candidates: Vec<FrequencyResult>,
}

/// Score a list of sentences and return the results sorted by frequency (highest first).
/// Input: JSON array of strings, e.g. ["sentence1", "sentence2", ...]
/// Output: JSON array of {text, zipf_score} sorted by zipf_score descending.
#[wasm_bindgen]
pub fn score_sentences(json_input: &str) -> String {
    let sentences: Vec<String> =
        serde_json::from_str(json_input).unwrap_or_else(|_| vec![]);
    let scored = frequency::score_sentences(&sentences);
    let results: Vec<FrequencyResult> = scored
        .into_iter()
        .map(|(text, zipf_score)| FrequencyResult { text, zipf_score })
        .collect();
    serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string())
}

/// Optimize: given an original sentence and a list of paraphrase candidates,
/// compute frequency scores for all (including the original), and return the best.
///
/// Input JSON: {"original": "...", "candidates": ["...", "..."]}
/// Output JSON: {"original": {text, zipf_score}, "optimized": {text, zipf_score}, "candidates": [...]}
#[wasm_bindgen]
pub fn optimize_prompt(json_input: &str) -> String {
    #[derive(serde::Deserialize)]
    struct OptimizeInput {
        original: String,
        candidates: Vec<String>,
    }

    let input: OptimizeInput =
        serde_json::from_str(json_input).unwrap_or_else(|_| OptimizeInput {
            original: String::new(),
            candidates: vec![],
        });

    // Score original
    let original_score = frequency::sentence_zipf(&input.original);

    // Score candidates
    let mut all: Vec<FrequencyResult> = input
        .candidates
        .iter()
        .map(|c| FrequencyResult {
            text: c.clone(),
            zipf_score: frequency::sentence_zipf(c),
        })
        .collect();

    // Add original
    all.push(FrequencyResult {
        text: input.original.clone(),
        zipf_score: original_score,
    });

    // Sort by score descending
    all.sort_by(|a, b| b.zipf_score.partial_cmp(&a.zipf_score).unwrap_or(std::cmp::Ordering::Equal));

    let optimized = all.first().map(|r| FrequencyResult {
        text: r.text.clone(),
        zipf_score: r.zipf_score,
    }).unwrap_or(FrequencyResult {
        text: String::new(),
        zipf_score: 0.0,
    });

    let result = OptimizeResult {
        original: FrequencyResult {
            text: input.original,
            zipf_score: original_score,
        },
        optimized,
        candidates: all,
    };

    serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string())
}
