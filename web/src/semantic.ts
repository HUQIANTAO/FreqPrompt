/**
 * FreqPrompt v3 — Semantic Guard (TypeScript layer)
 *
 * The Rust/WASM layer extracts semantic slots (Action, Object, Scope, etc.)
 * from prompts. The TypeScript layer is responsible for:
 *
 *   1. Calling the WASM extractor (off main thread via frequency.worker)
 *   2. Computing per-slot similarity between original and candidate
 *   3. Verifying candidates against slot preservation thresholds
 *
 * Similarity computation has two modes:
 *   - **Embedding mode** (preferred): uses a small quantized embedding model
 *     to compute cosine similarity between slot texts. This is what catches
 *     财政政策 → 国税规定 (high embedding similarity is rare for distinct concepts).
 *   - **Heuristic mode** (fallback): character-level Jaccard + word overlap.
 *     Catches obvious mismatches but not semantic drift.
 *
 * The embedding model is loaded lazily and cached. The first verify call
 * may take ~2-3 seconds (model download + warmup); subsequent calls are <50ms.
 */

import { callWasm } from './frequency';

/* ═══════════════ Slot Type Definitions ═══════════════ */

export type SlotKind =
  | 'action' | 'object' | 'scope' | 'modifier'
  | 'qtype' | 'register' | 'placeholder'
  | 'entity' | 'negation' | 'condition';

export interface Slot {
  kind: SlotKind;
  text: string;
  char_start: number;
  char_end: number;
  sub_tag: string | null;
}

export interface SlotThresholds {
  action: number;
  object: number;
  scope: number;
  modifier: number;
  qtype: number;
  register: number;
  entity: number;
  condition: number;
}

export const DEFAULT_THRESHOLDS: SlotThresholds = {
  action: 0.85,
  object: 0.90,
  scope: 0.95,
  modifier: 0.75,
  qtype: 0.95,
  register: 0.80,
  entity: 0.95,
  condition: 0.90,
};

export interface SlotVerdict {
  original: Slot;
  matched: string | null;
  similarity: number;
  passes: boolean;
  threshold: number;
}

export interface VerifyOutput {
  verdicts: SlotVerdict[];
  preservation_score: number;
  passes: boolean;
}

/* ═══════════════ Embedder Interface ═══════════════ */

/**
 * Abstract embedder interface. Any implementation that turns text
 * into a fixed-dimension vector and provides cosine similarity
 * satisfies this contract.
 */
export interface Embedder {
  /** Encode a text to a vector (Float32Array or number[]). */
  encode(text: string): Promise<Float32Array>;
  /** Embedding dimension; checked for compatibility. */
  dim(): number;
  /** Human-readable name for UI display. */
  name(): string;
}

/**
 * Heuristic embedder — character Jaccard + word overlap, no model needed.
 *
 * Useful as a fallback when no real embedder is available. Catches obvious
 * mismatches (different words → low similarity) but not semantic equivalence
 * (synonyms would also score low).
 */
export class HeuristicEmbedder implements Embedder {
  dim(): number { return 64; }
  name(): string { return 'Heuristic (Jaccard)'; }

  async encode(text: string): Promise<Float32Array> {
    const v = new Float32Array(64);
    const normalized = text.toLowerCase().trim();
    if (!normalized) return v;

    // Char n-gram (1-3) into the vector
    const chars = normalized.replace(/\s+/g, '');
    for (let i = 0; i < chars.length; i++) {
      for (let n = 1; n <= 3 && i + n <= chars.length; n++) {
        const ngram = chars.slice(i, i + n);
        const h = simpleHash(ngram);
        v[h % 64] += 1.0 / n;  // shorter n-grams get more weight
      }
    }

    // Word overlap features
    const words = normalized.split(/\s+/);
    for (const word of words) {
      if (word.length < 2) continue;
      const h = simpleHash(word);
      v[h % 64] += 0.5;
    }

    // L2 normalize
    let norm = 0;
    for (let i = 0; i < 64; i++) norm += v[i] * v[i];
    norm = Math.sqrt(norm) || 1;
    for (let i = 0; i < 64; i++) v[i] /= norm;
    return v;
  }
}

function simpleHash(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) {
    h = (h * 31 + s.charCodeAt(i)) | 0;
  }
  return Math.abs(h);
}

/* ═══════════════ Transformers.js Embedder (lazy-loaded) ═══════════════ */

/**
 * Lazily-loaded embedder using @huggingface/transformers.
 * BGE-small-zh-v1.5 quantized (q8) — ~25MB download.
 *
 * Loaded only when explicitly requested via loadTransformersEmbedder().
 */
export class TransformersEmbedder implements Embedder {
  private pipeline: any = null;
  private modelId: string;
  private modelDim: number = 512;

  constructor(modelId: string = 'Xenova/bge-small-zh-v1.5') {
    this.modelId = modelId;
  }

  async ensureLoaded(): Promise<void> {
    if (this.pipeline) return;
    // Dynamic import keeps the heavy dep out of the initial bundle
    const { pipeline, env } = await import('@huggingface/transformers');
    // Use the local cache where possible
    env.allowLocalModels = true;
    env.useBrowserCache = true;
    this.pipeline = await pipeline('feature-extraction', this.modelId, {
      quantized: true,
    });
  }

  dim(): number { return this.modelDim; }
  name(): string { return `Transformers.js (${this.modelId})`; }

  async encode(text: string): Promise<Float32Array> {
    await this.ensureLoaded();
    const out = await this.pipeline(text, { pooling: 'mean', normalize: true });
    return new Float32Array(out.data as ArrayLike<number>);
  }
}

/* ═══════════════ Similarity Computation ═══════════════ */

export function cosineSimilarity(a: Float32Array, b: Float32Array): number {
  if (a.length !== b.length) return 0;
  let dot = 0, na = 0, nb = 0;
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i];
    na += a[i] * a[i];
    nb += b[i] * b[i];
  }
  const denom = Math.sqrt(na) * Math.sqrt(nb);
  return denom === 0 ? 0 : dot / denom;
}

/* ═══════════════ Slot Extraction (WASM) ═══════════════ */

/**
 * Extract slots from a prompt using the Rust/WASM extractor.
 * Dispatches via the existing Web Worker — non-blocking.
 */
export async function extractSlots(text: string, lang: string = 'auto'): Promise<Slot[]> {
  return callWasm<Slot[]>('extract_slots', [text, lang]);
}

/**
 * Cheap structural check: does the candidate contain exact-match slots?
 * Useful as a pre-filter before expensive embedding calls.
 */
export async function checkStructure(
  originalSlots: Slot[],
  candidateText: string
): Promise<{ kind: SlotKind; text: string; issue: string | null }[]> {
  return callWasm('check_candidate_structure', [{
    original_slots: originalSlots,
    candidate_text: candidateText,
  }]);
}

/* ═══════════════ Candidate Verification ═══════════════ */

/**
 * Match each original slot to a candidate slot (if any) and compute similarity.
 * For exact-match slots (placeholder, negation), uses string equality.
 * For others, uses the embedder.
 */
async function matchSlots(
  originalSlots: Slot[],
  candidateText: string,
  embedder: Embedder
): Promise<{ text: string; similarity: number }[]> {
  // Extract slots from the candidate (WASM call)
  const candLang = candidateText.match(/[一-鿿]/) ? 'zh' : 'en';
  const candSlots = await extractSlots(candidateText, candLang);

  const results: { text: string; similarity: number }[] = [];

  for (const orig of originalSlots) {
    // 1) Try exact match first (cheapest)
    if (orig.kind === 'placeholder' || orig.kind === 'negation') {
      // For exact match, find a literal occurrence in candidate
      const present = candidateText.includes(orig.text);
      results.push({
        text: present ? orig.text : '',
        similarity: present ? 1.0 : 0.0,
      });
      continue;
    }

    // 2) Find the most similar candidate slot of the same kind
    const sameKind = candSlots.filter(s => s.kind === orig.kind);
    if (sameKind.length === 0) {
      // No candidate slot of this kind — assume dropped
      results.push({ text: '', similarity: 0.0 });
      continue;
    }

    // Encode original and candidate slots
    const origEmb = await embedder.encode(orig.text);
    let bestSim = -1;
    let bestText = sameKind[0].text;
    for (const cand of sameKind) {
      const candEmb = await embedder.encode(cand.text);
      const sim = cosineSimilarity(origEmb, candEmb);
      if (sim > bestSim) {
        bestSim = sim;
        bestText = cand.text;
      }
    }
    results.push({ text: bestText, similarity: bestSim });
  }
  return results;
}

/**
 * Full verification: extract slots from original, match against candidate,
 * apply thresholds, return verdicts + overall pass/fail.
 *
 * @param originalText  The user's original prompt
 * @param candidateText The LLM-generated paraphrase to evaluate
 * @param embedder      Embedding model for similarity (HeuristicEmbedder is built-in)
 * @param thresholds    Per-slot-kind similarity thresholds (defaults provided)
 */
export async function verifyCandidate(
  originalText: string,
  candidateText: string,
  embedder: Embedder = new HeuristicEmbedder(),
  thresholds: SlotThresholds = DEFAULT_THRESHOLDS
): Promise<VerifyOutput> {
  const lang = originalText.match(/[一-鿿]/) ? 'zh' : 'en';
  const originalSlots = await extractSlots(originalText, lang);
  if (originalSlots.length === 0) {
    return { verdicts: [], preservation_score: 1.0, passes: true };
  }

  const matches = await matchSlots(originalSlots, candidateText, embedder);

  return callWasm<VerifyOutput>('verify_slots', [{
    original_slots: originalSlots,
    similarities: matches,
    thresholds,
  }]);
}

/**
 * Filter a list of candidates, keeping only those that preserve critical slots.
 * Returns the survivors in their original order, with verdict info attached.
 */
export async function filterPreservingCandidates(
  originalText: string,
  candidates: string[],
  embedder: Embedder = new HeuristicEmbedder(),
  thresholds: SlotThresholds = DEFAULT_THRESHOLDS
): Promise<{ text: string; passes: boolean; score: number; output: VerifyOutput }[]> {
  const results: { text: string; passes: boolean; score: number; output: VerifyOutput }[] = [];
  for (const cand of candidates) {
    try {
      const output = await verifyCandidate(originalText, cand, embedder, thresholds);
      results.push({
        text: cand,
        passes: output.passes,
        score: output.preservation_score,
        output,
      });
    } catch (e) {
      // On error, include with passes=false to surface
      results.push({
        text: cand,
        passes: false,
        score: 0,
        output: { verdicts: [], preservation_score: 0, passes: false },
      });
    }
  }
  return results;
}
