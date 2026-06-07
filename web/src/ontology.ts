/**
 * FreqPrompt v3 — Ontology Guard (Sprint 2)
 *
 * Loads domain ontologies and validates that candidate substitutions
 * don't silently narrow meaning (e.g., 财政政策 → 国税规定).
 */

import { callWasm } from './frequency';

/* ═══════════════ Types ═══════════════ */

export interface SubstitutionResult {
  original: string;
  candidate: string;
  verdict: string;       // "Allowed", "Narrowing", "Widening", "CrossBranch", "Unrelated"
  is_safe: boolean;
  label: string;         // Chinese label
}

export interface DomainDetection {
  domain: string;
  confidence: number;
  matched_keywords: string[];
}

/* ═══════════════ Ontology Cache ═══════════════ */

const ontologyCache = new Map<string, string>();

/**
 * Load an ontology JSON file by domain name.
 * Tries: /ontologies/{domain}_{lang}.json
 */
export async function loadOntology(domain: string, lang: string): Promise<string> {
  const key = `${domain}_${lang}`;
  if (ontologyCache.has(key)) return ontologyCache.get(key)!;

  const url = `/ontologies/${domain}_${lang}.json`;
  try {
    const resp = await fetch(url);
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
    const json = await resp.text();
    ontologyCache.set(key, json);
    return json;
  } catch {
    // Try general ontology as fallback
    if (domain !== 'general') {
      return loadOntology('general', lang);
    }
    return '{}';
  }
}

/* ═══════════════ Substitution Check ═══════════════ */

/**
 * Check if replacing `original` with `candidate` is semantically safe.
 */
export async function checkSubstitution(
  original: string,
  candidate: string,
  ontologyJson: string
): Promise<SubstitutionResult> {
  return callWasm('check_substitution', [{
    original,
    candidate,
    ontology_json: ontologyJson,
  }]);
}

/**
 * Batch check multiple substitution pairs.
 */
export async function batchCheckSubstitutions(
  pairs: [string, string][],
  ontologyJson: string
): Promise<SubstitutionResult[]> {
  return callWasm('batch_check_substitutions', [{
    pairs,
    ontology_json: ontologyJson,
  }]);
}

/* ═══════════════ Domain Detection ═══════════════ */

/**
 * Detect which domain a prompt belongs to.
 */
export async function detectDomain(text: string, lang: string): Promise<DomainDetection | null> {
  const result = await callWasm<DomainDetection>('detect_domain', [{ text, lang }]);
  return result?.domain ? result : null;
}

/* ═══════════════ Substitution Analysis for UI ═══════════════ */

/**
 * Analyze a candidate's word changes against the ontology.
 * Returns only the unsafe substitutions.
 */
export async function analyzeCandidateChanges(
  originalText: string,
  candidateText: string,
  ontologyJson: string
): Promise<SubstitutionResult[]> {
  // Simple word-level diff: find changed words
  const origWords = originalText.split(/[\s,，。！？、；：""''（）\[\]【】]+/).filter(Boolean);
  const candWords = candidateText.split(/[\s,，。！？、；：""''（）\[\]【】]+/).filter(Boolean);

  // Find words in candidate that differ from original
  const origSet = new Set(origWords);
  const changedPairs: [string, string][] = [];

  for (const cw of candWords) {
    if (!origSet.has(cw)) {
      // Try to find which original word this might be replacing
      // Simple heuristic: check if any orig word is a substring or vice versa
      for (const ow of origWords) {
        if (!candWords.includes(ow)) {
          changedPairs.push([ow, cw]);
          break;
        }
      }
    }
  }

  if (changedPairs.length === 0) return [];

  const results = await batchCheckSubstitutions(changedPairs, ontologyJson);
  return results.filter(r => !r.is_safe);
}
