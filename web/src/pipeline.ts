/**
 * FreqPrompt v3 — Multi-Layer Pipeline (Sprint 3)
 *
 * Orchestrates the 5-layer scoring pipeline:
 *   1. Frequency (Zipf)
 *   2. Collocation (bigram/trigram PMI)
 *   3. Complexity (shorter = better)
 *   4. Slot preservation (from semantic guard)
 *   5. Domain relevance (from domain adaptation)
 */

import { callWasm } from './frequency';

/* ═══════════════ Types ═══════════════ */

export interface LayerScores {
  frequency: number;
  collocation: number;
  complexity: number;
  slot_preservation: number;
  domain_relevance: number;
  final_score: number;
}

export interface ScoringWeights {
  frequency: number;
  collocation: number;
  complexity: number;
  slot_preservation: number;
  domain_relevance: number;
}

export const DEFAULT_WEIGHTS: ScoringWeights = {
  frequency: 0.40,
  collocation: 0.20,
  complexity: 0.15,
  slot_preservation: 0.15,
  domain_relevance: 0.10,
};

/* ═══════════════ Weight Persistence (IndexedDB) ═══════════════ */

const DB_NAME = 'freqprompt';
const STORE_NAME = 'settings';

async function getDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, 2);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME);
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

export async function loadWeights(): Promise<ScoringWeights> {
  try {
    const db = await getDb();
    return new Promise((resolve) => {
      const tx = db.transaction(STORE_NAME, 'readonly');
      const store = tx.objectStore(STORE_NAME);
      const req = store.get('scoring_weights');
      req.onsuccess = () => resolve(req.result || DEFAULT_WEIGHTS);
      req.onerror = () => {
        console.warn('[FreqPrompt] IndexedDB read error (loadWeights):', req.error);
        resolve(DEFAULT_WEIGHTS);
      };
    });
  } catch (err) {
    console.warn('[FreqPrompt] IndexedDB open error (loadWeights):', err);
    return DEFAULT_WEIGHTS;
  }
}

export async function saveWeights(weights: ScoringWeights): Promise<void> {
  try {
    const db = await getDb();
    const tx = db.transaction(STORE_NAME, 'readwrite');
    const store = tx.objectStore(STORE_NAME);
    store.put(weights, 'scoring_weights');
  } catch (err) {
    console.warn('[FreqPrompt] IndexedDB write error (saveWeights):', err);
  }
}

/* ═══════════════ Multi-Layer Scoring ═══════════════ */

/**
 * Compute multi-layer score for a sentence.
 */
export async function computeMultiLayerScore(
  sentence: string,
  slotPreservation: number = 1.0,
  domainRelevance: number = 0.0,
  weights?: ScoringWeights
): Promise<LayerScores> {
  const w = weights || await loadWeights();
  return callWasm('multi_layer_score', [{
    sentence,
    slot_preservation: slotPreservation,
    domain_relevance: domainRelevance,
    weights: w,
  }]);
}

/**
 * Score multiple candidates and return sorted by final_score descending.
 */
export async function scoreCandidates(
  candidates: string[],
  slotPreservations: number[],
  domainRelevance: number = 0.0,
  weights?: ScoringWeights
): Promise<(LayerScores & { text: string })[]> {
  const w = weights || await loadWeights();
  const results = await Promise.all(
    candidates.map(async (text, i) => {
      const scores = await computeMultiLayerScore(
        text,
        slotPreservations[i] ?? 1.0,
        domainRelevance,
        w
      );
      return { text, ...scores };
    })
  );

  return results.sort((a, b) => b.final_score - a.final_score);
}

/* ═══════════════ Weight Config UI ═══════════════ */

/**
 * Render weight configuration sliders into a container element.
 */
export function renderWeightConfig(
  container: HTMLElement,
  onChange: (weights: ScoringWeights) => void
): void {
  const labels: Record<keyof ScoringWeights, string> = {
    frequency: '频率',
    collocation: '搭配',
    complexity: '简洁',
    slot_preservation: '语义保真',
    domain_relevance: '领域相关',
  };

  loadWeights().then(weights => {
    const form = document.createElement('div');
    form.className = 'weight-config';

    for (const [key, label] of Object.entries(labels)) {
      const k = key as keyof ScoringWeights;
      const row = document.createElement('div');
      row.className = 'weight-row';
      row.innerHTML = `
        <label class="weight-label">${label}</label>
        <input type="range" min="0" max="100" value="${Math.round(weights[k] * 100)}"
               class="weight-slider" data-key="${k}">
        <span class="weight-value">${Math.round(weights[k] * 100)}%</span>
      `;
      form.appendChild(row);
    }

    // Normalize button
    const normBtn = document.createElement('button');
    normBtn.className = 'btn-secondary weight-norm-btn';
    normBtn.textContent = '归一化';
    normBtn.onclick = () => {
      const sliders = form.querySelectorAll<HTMLInputElement>('.weight-slider');
      const newWeights: ScoringWeights = { ...DEFAULT_WEIGHTS };
      sliders.forEach(s => {
        (newWeights as any)[s.dataset.key!] = parseInt(s.value) / 100;
      });
      // Normalize
      const sum = Object.values(newWeights).reduce((a, b) => a + b, 0);
      if (sum > 0) {
        for (const k of Object.keys(newWeights) as (keyof ScoringWeights)[]) {
          (newWeights as any)[k] = (newWeights as any)[k] / sum;
        }
      }
      // Update sliders
      sliders.forEach(s => {
        const k = s.dataset.key as keyof ScoringWeights;
        s.value = String(Math.round(newWeights[k] * 100));
        s.nextElementSibling!.textContent = `${Math.round(newWeights[k] * 100)}%`;
      });
      saveWeights(newWeights);
      onChange(newWeights);
    };
    form.appendChild(normBtn);

    // Event listeners
    form.addEventListener('input', (e) => {
      const target = e.target as HTMLInputElement;
      if (target.classList.contains('weight-slider')) {
        const key = target.dataset.key as keyof ScoringWeights;
        weights[key] = parseInt(target.value) / 100;
        target.nextElementSibling!.textContent = `${target.value}%`;
        saveWeights(weights);
        onChange(weights);
      }
    });

    container.appendChild(form);
  });
}
