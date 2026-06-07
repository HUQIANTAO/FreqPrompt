/* tslint:disable */
/* eslint-disable */

/**
 * Batch substitution check: validate multiple (original, candidate) pairs.
 * Input JSON: {"pairs": [["财政政策", "税收政策"], ...], "ontology_json": "..."}
 * Output JSON: [{"original": "...", "candidate": "...", "verdict": "Narrowing", ...}]
 */
export function batch_check_substitutions(json_input: string): string;

/**
 * Build a domain frequency table from tokenized text.
 * Input JSON: {"name": "finance", "lang": "zh", "tokens": ["财政", "政策", ...]}
 * Output JSON: the DomainFreqTable as JSON
 */
export function build_domain_freq_table(json_input: string): string;

/**
 * Check structural sanity of a candidate (cheap pre-filter, no embedding).
 * Returns JSON array of {kind, text, issue} for any failed checks.
 */
export function check_candidate_structure(json_input: string): string;

/**
 * Check if substituting `original` with `candidate` is semantically safe.
 * Input JSON: {"original": "财政政策", "candidate": "国税规定", "ontology_json": "..."}
 * Output JSON: {"verdict": "Narrowing", "is_safe": false, "label": "语义收窄"}
 */
export function check_substitution(json_input: string): string;

/**
 * Return default scoring weights as JSON.
 */
export function default_scoring_weights(): string;

/**
 * Default slot thresholds as JSON (for UI display / configuration).
 */
export function default_slot_thresholds(): string;

/**
 * Detect the domain of a text using keyword matching.
 * Input JSON: {"text": "...", "lang": "zh"}
 * Output JSON: {"domain": "finance", "confidence": 0.35, "matched_keywords": ["财政", ...]}
 */
export function detect_domain(json_input: string): string;

/**
 * Detect the language of a text. Returns "zh" or "en".
 */
export function detect_language(text: string): string;

/**
 * Extract semantic slots from a prompt.
 * Pass `lang` as "auto", "zh", or "en".
 * Returns JSON array of {kind, text, char_start, char_end, sub_tag}.
 */
export function extract_slots(text: string, lang: string): string;

/**
 * Compute hybrid score (general + domain) for a sentence.
 * Input JSON: {"sentence": "...", "domain_table_json": "...", "alpha": 0.6, "beta": 0.4}
 * Output JSON: {"general_score": 6.0, "domain_score": 7.5, "hybrid_score": 6.6}
 */
export function hybrid_sentence_score(json_input: string): string;

/**
 * Return the N lowest-scoring tokens in a sentence.
 * Used for targeted word-level refinement: the frontend sends
 * these low-frequency words back to the LLM so it can replace them.
 *
 * Input JSON: {"sentence": "...", "n": 5}
 * Output JSON: array of {text, zipf_score} sorted by score ascending.
 */
export function lowest_tokens(json_input: string): string;

/**
 * Compute multi-layer score for a sentence.
 * Input JSON: {"sentence": "...", "slot_preservation": 1.0, "domain_relevance": 0.5}
 * Output JSON: {"frequency": 7.0, "collocation": 0.8, "complexity": 0.6, ...}
 */
export function multi_layer_score(json_input: string): string;

/**
 * Optimize: given an original sentence and paraphrase candidates,
 * compute frequency scores (arithmetic mean) and return the best.
 *
 * Language is auto-detected from the original text.
 * Works with both English (whitespace tokenizer) and Chinese (FMM tokenizer).
 *
 * Input JSON: {"original": "...", "candidates": ["...", "..."]}
 * Output JSON: {"original": {text, zipf_score}, "optimized": {text, zipf_score}, "candidates": [...]}
 */
export function optimize_prompt(json_input: string): string;

/**
 * Score a list of sentences (auto-detects language).
 * Input: JSON array of strings, e.g. ["sentence1", "sentence2", ...]
 * Output: JSON array of {text, zipf_score} sorted by zipf_score descending.
 */
export function score_sentences(json_input: string): string;

/**
 * Tokenize a sentence and return per-token (word, zipf_score) pairs.
 * Input: a single sentence string.
 * Output: JSON array of {text, zipf_score} in token order.
 */
export function tokenize_and_score(json_input: string): string;

/**
 * Verify slot preservation between original and candidate.
 * Returns JSON object with:
 *   - `verdicts`: array of {original, matched, similarity, passes, threshold}
 *   - `preservation_score`: aggregated score in [0.0, 1.0]
 *   - `passes`: whether the candidate preserves all critical slots
 *
 * `similarities` is a JSON array parallel to the original slots:
 *   [{"text": "财政政策", "similarity": 0.92}, ...]
 * For exact-match slots (placeholder, negation), similarity is ignored
 * and string equality is used.
 */
export function verify_slots(json_input: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly batch_check_substitutions: (a: number, b: number) => [number, number];
    readonly build_domain_freq_table: (a: number, b: number) => [number, number];
    readonly check_candidate_structure: (a: number, b: number) => [number, number];
    readonly check_substitution: (a: number, b: number) => [number, number];
    readonly default_scoring_weights: () => [number, number];
    readonly default_slot_thresholds: () => [number, number];
    readonly detect_domain: (a: number, b: number) => [number, number];
    readonly detect_language: (a: number, b: number) => [number, number];
    readonly extract_slots: (a: number, b: number, c: number, d: number) => [number, number];
    readonly hybrid_sentence_score: (a: number, b: number) => [number, number];
    readonly lowest_tokens: (a: number, b: number) => [number, number];
    readonly multi_layer_score: (a: number, b: number) => [number, number];
    readonly optimize_prompt: (a: number, b: number) => [number, number];
    readonly score_sentences: (a: number, b: number) => [number, number];
    readonly tokenize_and_score: (a: number, b: number) => [number, number];
    readonly verify_slots: (a: number, b: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
