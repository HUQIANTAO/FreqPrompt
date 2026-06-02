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

async function initWasm(): Promise<void> {
  if (wasm) return;
  if (initPromise) return initPromise;
  initPromise = (async () => {
    const mod = await import('./wasm-pkg/wasm_bridge.js');
    await mod.default();
    wasm = mod;
  })();
  return initPromise;
}

interface Request {
  id: number;
  op: string;
  args: any[];
}

self.addEventListener('message', async (e: MessageEvent<Request>) => {
  const { id, op, args } = e.data;
  try {
    await initWasm();
    let data: any;
    switch (op) {
      case 'optimize_prompt': {
        const [input] = args;
        const resultJson = wasm.optimize_prompt(JSON.stringify(input));
        data = JSON.parse(resultJson);
        break;
      }
      case 'score_sentences': {
        const [sentences] = args;
        const resultJson = wasm.score_sentences(JSON.stringify(sentences));
        data = JSON.parse(resultJson);
        break;
      }
      case 'tokenize_and_score': {
        const [sentence] = args;
        const resultJson = wasm.tokenize_and_score(JSON.stringify(sentence));
        data = JSON.parse(resultJson);
        break;
      }
      case 'lowest_tokens': {
        const [input] = args;
        const resultJson = wasm.lowest_tokens(JSON.stringify(input));
        data = JSON.parse(resultJson);
        break;
      }
      case 'detect_language': {
        data = wasm.detect_language(args[0]);
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
