use frequency;
use semantic;
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

/// Score a list of sentences (auto-detects language).
/// Input: JSON array of strings, e.g. ["sentence1", "sentence2", ...]
/// Output: JSON array of {text, zipf_score} sorted by zipf_score descending.
#[wasm_bindgen]
pub fn score_sentences(json_input: &str) -> String {
    let sentences: Vec<String> = serde_json::from_str(json_input).unwrap_or_default();
    let scored = frequency::score_sentences(&sentences);
    let results: Vec<FrequencyResult> = scored
        .into_iter()
        .map(|(text, zipf_score)| FrequencyResult { text, zipf_score })
        .collect();
    serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string())
}

/// Optimize: given an original sentence and paraphrase candidates,
/// compute frequency scores (arithmetic mean) and return the best.
///
/// Language is auto-detected from the original text.
/// Works with both English (whitespace tokenizer) and Chinese (FMM tokenizer).
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

    let input: OptimizeInput = serde_json::from_str(json_input).unwrap_or_else(|_| OptimizeInput {
        original: String::new(),
        candidates: vec![],
    });

    let lang = frequency::detect_language(&input.original);
    let original_score = frequency::sentence_zipf_lang(&input.original, lang);

    let mut all: Vec<FrequencyResult> = input
        .candidates
        .iter()
        .map(|c| FrequencyResult {
            text: c.clone(),
            zipf_score: frequency::sentence_zipf_lang(c, lang),
        })
        .collect();

    all.push(FrequencyResult {
        text: input.original.clone(),
        zipf_score: original_score,
    });

    all.sort_by(|a, b| {
        b.zipf_score
            .partial_cmp(&a.zipf_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let optimized = all
        .first()
        .map(|r| FrequencyResult {
            text: r.text.clone(),
            zipf_score: r.zipf_score,
        })
        .unwrap_or(FrequencyResult {
            text: String::new(),
            zipf_score: 0.0,
        });

    let result = OptimizeResult {
        original: FrequencyResult { text: input.original, zipf_score: original_score },
        optimized,
        candidates: all,
    };

    serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string())
}

/// Return the N lowest-scoring tokens in a sentence.
/// Used for targeted word-level refinement: the frontend sends
/// these low-frequency words back to the LLM so it can replace them.
///
/// Input JSON: {"sentence": "...", "n": 5}
/// Output JSON: array of {text, zipf_score} sorted by score ascending.
#[wasm_bindgen]
pub fn lowest_tokens(json_input: &str) -> String {
    #[derive(serde::Deserialize)]
    struct LowTokensInput {
        sentence: String,
        n: usize,
    }

    let input: LowTokensInput = serde_json::from_str(json_input).unwrap_or_else(|_| {
        LowTokensInput { sentence: String::new(), n: 5 }
    });

    let tokens = frequency::lowest_tokens(&input.sentence, input.n);
    let results: Vec<FrequencyResult> = tokens
        .into_iter()
        .map(|(text, zipf_score)| FrequencyResult { text, zipf_score })
        .collect();
    serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string())
}

/// Tokenize a sentence and return per-token (word, zipf_score) pairs.
/// Input: a single sentence string.
/// Output: JSON array of {text, zipf_score} in token order.
#[wasm_bindgen]
pub fn tokenize_and_score(json_input: &str) -> String {
    let sentence: String = serde_json::from_str(json_input).unwrap_or_default();
    let tokens = frequency::tokenize_and_score(&sentence);
    let results: Vec<FrequencyResult> = tokens
        .into_iter()
        .map(|(text, zipf_score)| FrequencyResult { text, zipf_score })
        .collect();
    serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string())
}

/// Detect the language of a text. Returns "zh" or "en".
#[wasm_bindgen]
pub fn detect_language(text: &str) -> String {
    frequency::detect_language(text).to_string()
}

/* ═══════════════════════════════════════════════
   SEMANTIC GUARD (FreqPrompt v3 Sprint 1)
   ═══════════════════════════════════════════════ */

/// Extract semantic slots from a prompt.
/// Pass `lang` as "auto", "zh", or "en".
/// Returns JSON array of {kind, text, char_start, char_end, sub_tag}.
#[wasm_bindgen]
pub fn extract_slots(text: &str, lang: &str) -> String {
    let slots = semantic::extract_slots(text, lang);
    serde_json::to_string(&slots).unwrap_or_else(|_| "[]".to_string())
}

/// Verify slot preservation between original and candidate.
/// Returns JSON object with:
///   - `verdicts`: array of {original, matched, similarity, passes, threshold}
///   - `preservation_score`: aggregated score in [0.0, 1.0]
///   - `passes`: whether the candidate preserves all critical slots
///
/// `similarities` is a JSON array parallel to the original slots:
///   [{"text": "财政政策", "similarity": 0.92}, ...]
/// For exact-match slots (placeholder, negation), similarity is ignored
/// and string equality is used.
#[wasm_bindgen]
pub fn verify_slots(json_input: &str) -> String {
    #[derive(serde::Deserialize)]
    struct SlotInput {
        text: String,
        similarity: f64,
    }
    #[derive(serde::Deserialize)]
    struct VerifyInput {
        original_slots: Vec<semantic::Slot>,
        similarities: Vec<SlotInput>,
        thresholds: Option<semantic::SlotThresholds>,
    }
    #[derive(serde::Serialize)]
    struct VerifyOutput {
        verdicts: Vec<semantic::SlotVerdict>,
        preservation_score: f64,
        passes: bool,
    }

    let input: VerifyInput = match serde_json::from_str(json_input) {
        Ok(v) => v,
        Err(_) => {
            return serde_json::to_string(&VerifyOutput {
                verdicts: vec![],
                preservation_score: 1.0,
                passes: true,
            })
            .unwrap_or_else(|_| "{}".to_string());
        }
    };

    let thresholds = input.thresholds.unwrap_or_default();

    let mut verdicts = Vec::with_capacity(input.original_slots.len());
    for (i, orig) in input.original_slots.iter().enumerate() {
        let (matched_text, similarity) = input
            .similarities
            .get(i)
            .map(|s| (Some(s.text.clone()), s.similarity))
            .unwrap_or((None, 0.0));

        // For exact-match slots, override similarity based on text equality
        let effective_similarity = if orig.requires_exact_match() {
            match &matched_text {
                Some(t) if t == &orig.text => 1.0,
                _ => 0.0,
            }
        } else {
            similarity
        };

        verdicts.push(semantic::apply_thresholds(
            orig.clone(),
            matched_text,
            effective_similarity,
            &thresholds,
        ));
    }

    let preservation_score = semantic::aggregate_preservation(&verdicts);
    let passes = verdicts.iter().all(|v| v.passes);

    let output = VerifyOutput {
        verdicts,
        preservation_score,
        passes,
    };
    serde_json::to_string(&output).unwrap_or_else(|_| "{}".to_string())
}

/// Check structural sanity of a candidate (cheap pre-filter, no embedding).
/// Returns JSON array of {kind, text, issue} for any failed checks.
#[wasm_bindgen]
pub fn check_candidate_structure(json_input: &str) -> String {
    #[derive(serde::Deserialize)]
    struct CheckInput {
        original_slots: Vec<semantic::Slot>,
        candidate_text: String,
    }

    let input: CheckInput = serde_json::from_str(json_input).unwrap_or(CheckInput {
        original_slots: vec![],
        candidate_text: String::new(),
    });

    let checks = semantic::structural_check(&input.original_slots, &input.candidate_text);
    serde_json::to_string(&checks).unwrap_or_else(|_| "[]".to_string())
}

/// Default slot thresholds as JSON (for UI display / configuration).
#[wasm_bindgen]
pub fn default_slot_thresholds() -> String {
    serde_json::to_string(&semantic::SlotThresholds::default())
        .unwrap_or_else(|_| "{}".to_string())
}

/* ═══════════════════════════════════════════════
   ONTOLOGY GUARD (FreqPrompt v3 Sprint 2)
   ═══════════════════════════════════════════════ */

/// Check if substituting `original` with `candidate` is semantically safe.
/// Input JSON: {"original": "财政政策", "candidate": "国税规定", "ontology_json": "..."}
/// Output JSON: {"verdict": "Narrowing", "is_safe": false, "label": "语义收窄"}
#[wasm_bindgen]
pub fn check_substitution(json_input: &str) -> String {
    #[derive(serde::Deserialize)]
    struct SubInput {
        original: String,
        candidate: String,
        ontology_json: String,
    }
    #[derive(serde::Serialize)]
    struct SubOutput {
        verdict: String,
        is_safe: bool,
        label: String,
    }

    let input: SubInput = match serde_json::from_str(json_input) {
        Ok(v) => v,
        Err(_) => {
            return serde_json::to_string(&SubOutput {
                verdict: "Unrelated".into(),
                is_safe: true,
                label: "无本体关系".into(),
            })
            .unwrap_or_else(|_| "{}".into());
        }
    };

    let onto = match ontology::Ontology::from_json(&input.ontology_json) {
        Ok(o) => o,
        Err(_) => {
            return serde_json::to_string(&SubOutput {
                verdict: "Unrelated".into(),
                is_safe: true,
                label: "本体加载失败".into(),
            })
            .unwrap_or_else(|_| "{}".into());
        }
    };

    let verdict = ontology::can_substitute(&input.original, &input.candidate, &onto);
    let output = SubOutput {
        verdict: format!("{:?}", verdict),
        is_safe: verdict.is_safe(),
        label: verdict.label().to_string(),
    };
    serde_json::to_string(&output).unwrap_or_else(|_| "{}".into())
}

/// Detect the domain of a text using keyword matching.
/// Input JSON: {"text": "...", "lang": "zh"}
/// Output JSON: {"domain": "finance", "confidence": 0.35, "matched_keywords": ["财政", ...]}
#[wasm_bindgen]
pub fn detect_domain(json_input: &str) -> String {
    #[derive(serde::Deserialize)]
    struct DomainInput {
        text: String,
        lang: String,
    }
    #[derive(serde::Serialize)]
    struct DomainOutput {
        domain: String,
        confidence: f64,
        matched_keywords: Vec<String>,
    }

    let input: DomainInput = match serde_json::from_str(json_input) {
        Ok(v) => v,
        Err(_) => {
            return "{}".to_string();
        }
    };

    let profiles = if input.lang == "zh" {
        ontology::domain::default_zh_profiles()
    } else {
        ontology::domain::default_en_profiles()
    };

    match ontology::detect_domain(&input.text, &profiles) {
        Some(dm) => {
            let output = DomainOutput {
                domain: dm.domain,
                confidence: dm.confidence,
                matched_keywords: dm.matched_keywords,
            };
            serde_json::to_string(&output).unwrap_or_else(|_| "{}".into())
        }
        None => "{}".to_string(),
    }
}

/// Batch substitution check: validate multiple (original, candidate) pairs.
/// Input JSON: {"pairs": [["财政政策", "税收政策"], ...], "ontology_json": "..."}
/// Output JSON: [{"original": "...", "candidate": "...", "verdict": "Narrowing", ...}]
#[wasm_bindgen]
pub fn batch_check_substitutions(json_input: &str) -> String {
    #[derive(serde::Deserialize)]
    struct BatchInput {
        pairs: Vec<(String, String)>,
        ontology_json: String,
    }
    #[derive(serde::Serialize)]
    struct SubResult {
        original: String,
        candidate: String,
        verdict: String,
        is_safe: bool,
        label: String,
    }

    let input: BatchInput = match serde_json::from_str(json_input) {
        Ok(v) => v,
        Err(_) => return "[]".to_string(),
    };

    let onto = match ontology::Ontology::from_json(&input.ontology_json) {
        Ok(o) => o,
        Err(_) => return "[]".to_string(),
    };

    let results: Vec<SubResult> = input
        .pairs
        .iter()
        .map(|(orig, cand)| {
            let verdict = ontology::can_substitute(orig, cand, &onto);
            SubResult {
                original: orig.clone(),
                candidate: cand.clone(),
                verdict: format!("{:?}", verdict),
                is_safe: verdict.is_safe(),
                label: verdict.label().to_string(),
            }
        })
        .collect();

    serde_json::to_string(&results).unwrap_or_else(|_| "[]".into())
}

/* ═══════════════════════════════════════════════
   MULTI-LAYER SCORER (FreqPrompt v3 Sprint 3)
   ═══════════════════════════════════════════════ */

/// Compute multi-layer score for a sentence.
/// Input JSON: {"sentence": "...", "slot_preservation": 1.0, "domain_relevance": 0.5}
/// Output JSON: {"frequency": 7.0, "collocation": 0.8, "complexity": 0.6, ...}
#[wasm_bindgen]
pub fn multi_layer_score(json_input: &str) -> String {
    #[derive(serde::Deserialize)]
    struct ScoreInput {
        sentence: String,
        slot_preservation: Option<f64>,
        domain_relevance: Option<f64>,
        weights: Option<frequency::multi_layer::ScoringWeights>,
    }

    let input: ScoreInput = match serde_json::from_str(json_input) {
        Ok(v) => v,
        Err(_) => return "{}".to_string(),
    };

    let lang = frequency::detect_language(&input.sentence);
    let tokens = if lang == "zh" {
        frequency::tokenize_cn(&input.sentence)
    } else {
        frequency::tokenize_en(&input.sentence)
    };

    // Layer 1: Frequency
    let freq_score = frequency::sentence_zipf_lang(&input.sentence, lang);

    // Layer 2: Collocation (bigram hit rate)
    let collocation = frequency::multi_layer::collocation_score(
        &tokens,
        &|bg| {
            if lang == "zh" {
                frequency::bigram_zipf_cn(bg)
            } else {
                frequency::bigram_zipf(bg)
            }
        },
        5.0,
    );

    // Layer 3: Complexity (shorter = better)
    let avg_len = frequency::multi_layer::avg_word_length(&tokens);
    let complexity = frequency::multi_layer::complexity_score(tokens.len(), avg_len);

    let layers = frequency::multi_layer::LayerScores {
        frequency: freq_score,
        collocation,
        complexity,
        slot_preservation: input.slot_preservation.unwrap_or(1.0),
        domain_relevance: input.domain_relevance.unwrap_or(0.0),
    };

    let weights = input.weights.unwrap_or_default();
    let final_score = frequency::multi_layer::multi_layer_score(&layers, &weights);

    #[derive(Serialize)]
    struct ScoreOutput {
        frequency: f64,
        collocation: f64,
        complexity: f64,
        slot_preservation: f64,
        domain_relevance: f64,
        final_score: f64,
    }

    let output = ScoreOutput {
        frequency: layers.frequency,
        collocation: layers.collocation,
        complexity: layers.complexity,
        slot_preservation: layers.slot_preservation,
        domain_relevance: layers.domain_relevance,
        final_score,
    };
    serde_json::to_string(&output).unwrap_or_else(|_| "{}".into())
}

/// Return default scoring weights as JSON.
#[wasm_bindgen]
pub fn default_scoring_weights() -> String {
    serde_json::to_string(&frequency::multi_layer::ScoringWeights::default())
        .unwrap_or_else(|_| "{}".into())
}

/* ═══════════════════════════════════════════════
   DOMAIN ADAPTATION (FreqPrompt v3 Sprint 4)
   ═══════════════════════════════════════════════ */

/// Build a domain frequency table from tokenized text.
/// Input JSON: {"name": "finance", "lang": "zh", "tokens": ["财政", "政策", ...]}
/// Output JSON: the DomainFreqTable as JSON
#[wasm_bindgen]
pub fn build_domain_freq_table(json_input: &str) -> String {
    #[derive(serde::Deserialize)]
    struct DomainInput {
        name: String,
        lang: String,
        tokens: Vec<String>,
    }

    let input: DomainInput = match serde_json::from_str(json_input) {
        Ok(v) => v,
        Err(_) => return "{}".to_string(),
    };

    let table = domain::DomainFreqTable::from_tokens(input.name, input.lang, &input.tokens);
    table.to_json()
}

/// Compute hybrid score (general + domain) for a sentence.
/// Input JSON: {"sentence": "...", "domain_table_json": "...", "alpha": 0.6, "beta": 0.4}
/// Output JSON: {"general_score": 6.0, "domain_score": 7.5, "hybrid_score": 6.6}
#[wasm_bindgen]
pub fn hybrid_sentence_score(json_input: &str) -> String {
    #[derive(serde::Deserialize)]
    struct HybridInput {
        sentence: String,
        domain_table_json: String,
        alpha: Option<f64>,
        beta: Option<f64>,
    }

    let input: HybridInput = match serde_json::from_str(json_input) {
        Ok(v) => v,
        Err(_) => return "{}".to_string(),
    };

    let lang = frequency::detect_language(&input.sentence);
    let tokens = if lang == "zh" {
        frequency::tokenize_cn(&input.sentence)
    } else {
        frequency::tokenize_en(&input.sentence)
    };

    let domain_table = match domain::DomainFreqTable::from_json(&input.domain_table_json) {
        Ok(t) => t,
        Err(_) => return "{}".to_string(),
    };

    let config = domain::HybridConfig {
        alpha: input.alpha.unwrap_or(0.6),
        beta: input.beta.unwrap_or(0.4),
    };

    let general_zipfs: Vec<f64> = tokens
        .iter()
        .map(|t| {
            if lang == "zh" {
                frequency::word_zipf_cn(t)
            } else {
                frequency::word_zipf(t)
            }
        })
        .collect();

    let domain_zipfs: Vec<Option<f64>> = tokens
        .iter()
        .map(|t| domain_table.zipf(t))
        .collect();

    let general_score = frequency::sentence_zipf_lang(&input.sentence, lang);
    let hybrid = domain::hybrid_sentence_score(&general_zipfs, &domain_zipfs, &config);

    #[derive(Serialize)]
    struct HybridOutput {
        general_score: f64,
        hybrid_score: f64,
        token_count: usize,
        domain_coverage: f64,
    }

    let coverage = domain_zipfs.iter().filter(|d| d.is_some()).count() as f64
        / domain_zipfs.len().max(1) as f64;

    let output = HybridOutput {
        general_score,
        hybrid_score: hybrid,
        token_count: tokens.len(),
        domain_coverage: coverage,
    };
    serde_json::to_string(&output).unwrap_or_else(|_| "{}".into())
}
