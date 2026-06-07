/**
 * FreqPrompt v2 — Web Worker for WASM frequency computations.
 *
 * Loads the Rust/WASM module off the main thread, exposes the same
 * API surface as the main-thread module via a postMessage protocol.
 *
 * Request format:  { id: number, op: string, args: any[] }
 * Response format: { id: number, ok: boolean, data?: any, error?: string }
 */

let wasm: any = null;
let initPromise: Promise<void> | null = null;
let wasmReady = false;

async function initWasm(): Promise<void> {
  if (wasm) return;
  if (initPromise) return initPromise;
  initPromise = (async () => {
    const mod = await import('./wasm-pkg/wasm_bridge.js');
    await mod.default();
    wasm = mod;
    wasmReady = true;
    (self as any).postMessage({ id: -2, ok: true, data: 'wasm-ready' });
  })();
  return initPromise;
}

interface Request {
  id: number;
  op: string;
  args: any[];
}

/** Parse WASM JSON result, detecting silent deserialization failures. */
function parseWasmResult(op: string, resultJson: string, inputs?: unknown): any {
  if (!resultJson || resultJson === '[]' || resultJson === '{}' || resultJson === 'null') {
    console.warn(`[FreqPrompt Worker] ${op} returned empty/suspicious result`, { inputs, resultJson });
  }
  return JSON.parse(resultJson);
}

self.addEventListener('message', async (e: MessageEvent<Request>) => {
  const { id, op, args } = e.data;
  try {
    await initWasm();
    let data: any;
    switch (op) {
      case 'optimize_prompt': {
        const [input] = args;
        data = parseWasmResult(op, wasm.optimize_prompt(JSON.stringify(input)), input);
        break;
      }
      case 'score_sentences': {
        const [sentences] = args;
        data = parseWasmResult(op, wasm.score_sentences(JSON.stringify(sentences)), sentences);
        break;
      }
      case 'tokenize_and_score': {
        const [sentence] = args;
        data = parseWasmResult(op, wasm.tokenize_and_score(JSON.stringify(sentence)), sentence);
        break;
      }
      case 'lowest_tokens': {
        const [input] = args;
        data = parseWasmResult(op, wasm.lowest_tokens(JSON.stringify(input)), input);
        break;
      }
      case 'detect_language': {
        data = wasm.detect_language(args[0]);
        break;
      }
      // ── FreqPrompt v3: Semantic Guard ──
      case 'extract_slots': {
        const [text, lang] = args;
        data = parseWasmResult(op, wasm.extract_slots(text, lang));
        break;
      }
      case 'verify_slots': {
        const [input] = args;
        data = parseWasmResult(op, wasm.verify_slots(JSON.stringify(input)));
        break;
      }
      case 'check_candidate_structure': {
        const [input] = args;
        data = parseWasmResult(op, wasm.check_candidate_structure(JSON.stringify(input)));
        break;
      }
      case 'default_slot_thresholds': {
        data = parseWasmResult(op, wasm.default_slot_thresholds());
        break;
      }
      // ── FreqPrompt v3 Sprint 2: Ontology Guard ──
      case 'check_substitution': {
        const [input] = args;
        data = parseWasmResult(op, wasm.check_substitution(JSON.stringify(input)));
        break;
      }
      case 'batch_check_substitutions': {
        const [input] = args;
        data = parseWasmResult(op, wasm.batch_check_substitutions(JSON.stringify(input)));
        break;
      }
      case 'detect_domain': {
        const [input] = args;
        data = parseWasmResult(op, wasm.detect_domain(JSON.stringify(input)));
        break;
      }
      // ── FreqPrompt v3 Sprint 3: Multi-Layer Scorer ──
      case 'multi_layer_score': {
        const [input] = args;
        data = parseWasmResult(op, wasm.multi_layer_score(JSON.stringify(input)));
        break;
      }
      case 'default_scoring_weights': {
        data = parseWasmResult(op, wasm.default_scoring_weights());
        break;
      }
      // ── FreqPrompt v3 Sprint 4: Domain Adaptation ──
      case 'build_domain_freq_table': {
        const [input] = args;
        data = parseWasmResult(op, wasm.build_domain_freq_table(JSON.stringify(input)));
        break;
      }
      case 'hybrid_sentence_score': {
        const [input] = args;
        data = parseWasmResult(op, wasm.hybrid_sentence_score(JSON.stringify(input)));
        break;
      }
      default:
        throw new Error(`Unknown op: ${op}`);
    }
    (self as any).postMessage({ id, ok: true, data });
  } catch (err: any) {
    (self as any).postMessage({ id, ok: false, error: err.message || String(err) });
  }
});

// Signal that the worker is ready (even before WASM init)
(self as any).postMessage({ id: -1, ok: true, data: 'worker-ready' });
