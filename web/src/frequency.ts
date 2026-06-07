/**
 * FreqPrompt v2 — Frequency computation client.
 *
 * All WASM calls are dispatched to a Web Worker to keep the main
 * thread (and UI) responsive even for large prompts.
 *
 * The public API matches the previous version so consumers (main.ts, ui.ts)
 * don't need to change.
 */

export interface FrequencyResult {
  text: string;
  zipf_score: number;
}

export interface OptimizeOutput {
  original: FrequencyResult;
  optimized: FrequencyResult;
  candidates: FrequencyResult[];
}

export interface LowToken {
  text: string;
  zipf_score: number;
}

/* ═══════════════ Worker RPC ═══════════════ */

let worker: Worker | null = null;
let nextRequestId = 1;
let wasmReady = false;
const pending = new Map<number, { resolve: (v: any) => void; reject: (e: any) => void }>();

function ensureWorker(): Worker {
  if (worker) return worker;
  // Use the bundled worker file (rolldown outputs frequency.worker.js)
  worker = new Worker(new URL('./frequency.worker.js', import.meta.url), { type: 'module' });
  worker.addEventListener('message', (e: MessageEvent) => {
    const { id, ok, data, error } = e.data;
    if (id === -1) return; // worker-ready signal, ignore
    if (id === -2) { wasmReady = true; updateWasmLoading(false); return; } // wasm-ready
    const p = pending.get(id);
    if (!p) return;
    pending.delete(id);
    if (ok) p.resolve(data);
    else p.reject(new Error(error));
  });
  worker.addEventListener('error', (e) => {
    console.error('[frequency.worker] error:', e);
  });
  // Show loading indicator during initial WASM init
  updateWasmLoading(true);
  return worker;
}

/** Update a WASM loading indicator in the UI. */
function updateWasmLoading(loading: boolean): void {
  const el = document.getElementById('wasm-loading');
  if (el) el.style.display = loading ? '' : 'none';
}

function call<T = any>(op: string, args: any[]): Promise<T> {
  const w = ensureWorker();
  const id = nextRequestId++;
  return new Promise<T>((resolve, reject) => {
    pending.set(id, { resolve, reject });
    w.postMessage({ id, op, args });
  });
}

/* ═══════════════ Public API ═══════════════ */

export async function optimizePrompt(
  original: string,
  candidates: string[]
): Promise<OptimizeOutput> {
  return call<OptimizeOutput>('optimize_prompt', [{ original, candidates }]);
}

export async function scoreSentences(
  sentences: string[]
): Promise<FrequencyResult[]> {
  return call<FrequencyResult[]>('score_sentences', [sentences]);
}

export async function tokenizeAndScore(
  sentence: string
): Promise<FrequencyResult[]> {
  return call<FrequencyResult[]>('tokenize_and_score', [sentence]);
}

export async function lowestTokens(
  sentence: string,
  n: number = 8
): Promise<LowToken[]> {
  return call<LowToken[]>('lowest_tokens', [{ sentence, n }]);
}

export async function detectLanguage(text: string): Promise<string> {
  return call<string>('detect_language', [text]);
}

/* ═══════════════ Internal Worker Bridge ═══════════════ */

/**
 * Generic call into the WASM worker. Used by v3 modules (semantic, etc.)
 * that share the same worker transport. Exported as a public API.
 */
export async function callWasm<T = any>(op: string, args: any[]): Promise<T> {
  return call<T>(op, args);
}
