/**
 * WASM frequency computation wrapper.
 * Imports from the wasm-pack generated module in wasm-pkg/.
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

// Re-export the wasm module functions, with typed wrappers.
// We lazy-load the WASM module on first use.

let wasmReady = false;

async function ensureWasm(): Promise<void> {
  if (wasmReady) return;

  // Dynamic import of the wasm-pack module
  const mod = await import('./wasm-pkg/wasm_bridge.js');
  // Call init to load the WASM binary
  await mod.default();
  wasmReady = true;
}

export async function optimizePrompt(
  original: string,
  candidates: string[]
): Promise<OptimizeOutput> {
  await ensureWasm();

  // Import again to get the functions (they're available after init)
  const mod = await import('./wasm-pkg/wasm_bridge.js');
  const input = JSON.stringify({ original, candidates });
  const resultJson = mod.optimize_prompt(input);
  return JSON.parse(resultJson) as OptimizeOutput;
}

export async function scoreSentences(
  sentences: string[]
): Promise<FrequencyResult[]> {
  await ensureWasm();

  const mod = await import('./wasm-pkg/wasm_bridge.js');
  const input = JSON.stringify(sentences);
  const resultJson = mod.score_sentences(input);
  return JSON.parse(resultJson) as FrequencyResult[];
}
