<p align="center">
  <img src="https://img.shields.io/github/license/HUQIANTAO/FreqPrompt?color=%23f54e00" alt="License: MIT">
  <a href="https://github.com/HUQIANTAO/FreqPrompt/actions/workflows/deploy.yml"><img src="https://github.com/HUQIANTAO/FreqPrompt/actions/workflows/deploy.yml/badge.svg" alt="Deploy"></a>
  <img src="https://img.shields.io/badge/rust-1.70%2B-orange" alt="Rust 1.70+">
  <img src="https://img.shields.io/badge/wasm--pack-latest-blueviolet" alt="wasm-pack">
</p>

<h1 align="center">FreqPrompt</h1>
<p align="center"><strong>Prompt Optimizer via Textual Frequency Law</strong></p>
<p align="center">
  <a href="https://huqiantao.github.io/FreqPrompt/"><strong>Live Demo</strong></a> ·
  <a href="#how-it-works"><strong>How It Works</strong></a> ·
  <a href="#quick-start"><strong>Quick Start</strong></a> ·
  <a href="#architecture"><strong>Architecture</strong></a> ·
  <a href="#contributing"><strong>Contributing</strong></a>
</p>

---

## Overview

**FreqPrompt** rewrites your LLM prompts to use higher-frequency vocabulary — boosting response accuracy and quality without changing semantic meaning.

It is based on the discovery in [Adam's Law: Textual Frequency Law on Large Language Models](https://arxiv.org/abs/2604.02176v2) (arXiv:2604.02176v2):

> *When semantically equivalent prompts are phrased with higher-frequency words, LLMs consistently produce more accurate, higher-quality responses.*

FreqPrompt operationalizes this finding as a practical tool: you input a prompt → an LLM generates frequency-optimized rewrites → a Rust/WASM engine scores each variant against a 15,000-word Zipf dictionary → the best version is presented with visual comparison.

---

## How It Works

### The Frequency Law

The TFL paper establishes a robust correlation between *word frequency* and *LLM response quality*. The intuition is straightforward: LLMs are trained on internet-scale text, and higher-frequency expressions appear in more training contexts, leading to more reliable internal representations.

FreqPrompt computes a **sentence-level frequency score** using the geometric mean of individual word frequencies:

$$S_{freq}(s) = \sqrt[K]{\prod_{k=1}^{K} f(w_k)}$$

where $f(w_k)$ is the Zipf-scale frequency (0–8) of the $k$-th word in sentence $s$, and $K$ is the number of content words (stop words excluded).

The score is reported on the standard **Zipf scale** (0–8), where:
- **0–3**: Rare, domain-specific vocabulary
- **4–5**: Common everyday language
- **6–8**: High-frequency, broadly understood expressions

### Chinese Language Support (中文支持)

FreqPrompt supports Chinese prompts with a dedicated pipeline:

| Component | English | Chinese |
|---|---|---|
| **Detection** | CJK ratio ≤ 30% | CJK ratio > 30% |
| **Tokenizer** | Whitespace + ASCII punctuation | Forward Maximum Matching (FMM) with our 1,792-word frequency dictionary |
| **Frequency DB** | 15,000 English word-Zipf pairs | 1,792 Chinese word-Zipf pairs (BCC/SUBTLEX-CH norms) |
| **Default unknown** | 1.0 | 0.8 (steeper Zipf slope for Chinese: s ≈ 1.3 vs s ≈ 1.0) |
| **LLM prompt** | English paraphrase system prompt | Chinese paraphrase system prompt (现代标准汉语) |

**Segmentation approach:** We use a pure-Rust Forward Maximum Matching (FMM) algorithm. Unlike jieba/THULAC which require C compilation (blocking WASM targets), FMM is dependency-free. The frequency dictionary serves double duty — as both the word list for segmentation and the lookup table for scoring. For unknown multi-character words, we fall back to the geometric mean of individual character frequencies.

**Zipf parameters:** Chinese word frequency distributions are more skewed than English. The Zipf exponent s is approximately 1.3 for Chinese (vs 1.0 for English), meaning high-frequency words are even more dominant. We account for this by:
- Using a lower default score for unknown words (0.8 vs 1.0)
- Calibrating frequency values from Chinese-specific corpora (BCC, SUBTLEX-CH)
- Character-level fallback for compound words not in the dictionary

### Optimization Pipeline

```
┌──────────────┐     ┌─────────────────┐     ┌──────────────────┐
│  User Input  │ ──▶ │  LLM generates   │ ──▶ │  Rust/WASM       │
│  (prompt)    │     │  6 paraphrases   │     │  frequency score │
└──────────────┘     └─────────────────┘     └────────┬─────────┘
                                                      │
                                                      ▼
┌──────────────┐     ┌─────────────────┐     ┌──────────────────┐
│  Display     │ ◀── │  Select highest  │ ◀── │  Score & rank    │
│  results     │     │  scoring version │     │  all candidates  │
└──────────────┘     └─────────────────┘     └──────────────────┘
```

1. **Input** — User provides a prompt (English works best; dictionary is English-only)
2. **Paraphrase Generation** — An LLM (configurable; any OpenAI-compatible API) generates 6 semantically equivalent rewrites using higher-frequency vocabulary
3. **Frequency Scoring** — A Rust-compiled WASM module tokenizes each version, looks up each word in a 15K-entry Zipf dictionary, and computes the geometric mean score
4. **Selection & Display** — The highest-scoring version is highlighted with visual score comparison, delta from original, and a ranked candidate list

---

## Architecture

### Technology Stack

| Layer | Technology | Role |
|---|---|---|
| **Frequency Engine** | Rust → WebAssembly | Core computation: tokenization, dictionary lookup, geometric mean |
| **LLM Client** | TypeScript (fetch API) | OpenAI-compatible API calls for paraphrase generation |
| **UI Framework** | Vanilla HTML/CSS/TypeScript | Zero-framework reactive UI with dark mode |
| **Bundler** | [Rolldown](https://rolldown.rs/) | Fast ESM bundler (Rust-based, Rolldown team) |
| **Design System** | Cursor DESIGN.md | Token-based design with warm-cream canvas + Cursor Orange accent |
| **CI/CD** | GitHub Actions | Automated build (Rust + WASM + npm) and deploy to Pages |

### WASM Engine (`crates/frequency`)

The core scoring engine lives in a Rust library compiled to WebAssembly:

```
crates/
├── frequency/              # Pure Rust frequency library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Public API: tokenize, score, rank
│       └── wordfreq_data.rs # 15,000-word Zipf frequency dictionary
└── wasm-bridge/            # wasm-bindgen glue layer
    ├── Cargo.toml
    └── src/lib.rs          # Exports to JavaScript
```

**Key implementation details:**

- **Tokenizer**: Splits on whitespace, strips punctuation, lowercases, and filters a 200+ word stop-word list. Only content words contribute to the score.
- **Dictionary**: 15,000 English words with Zipf-scale frequencies (0.0–8.0). Stored as a compile-time `phf::Map` (perfect hash function) for O(1) lookup with zero runtime overhead.
- **Geometric Mean**: Computed as $\exp(\frac{1}{K} \sum \ln f(w_k))$. Using log-space avoids floating-point underflow for long sentences.
- **WASM Interface**: Exposes `score_sentence(text: &str) -> f64` and `rank_candidates(original: &str, candidates: Vec<String>) -> JsValue` via `wasm-bindgen`.

### Frontend (`web/`)

```
web/
├── index.html              # Single-page app shell + CSS design system
├── src/
│   ├── main.ts             # Entry point, event bindings, keyboard shortcuts
│   ├── paraphrase.ts       # LLM API client (OpenAI-compatible /chat/completions)
│   ├── frequency.ts        # WASM bridge — loads .wasm, wraps score/rank calls
│   └── ui.ts               # DOM rendering, history (localStorage), theme toggle
├── rolldown.config.mjs     # Build config (TS → ESM, asset copy)
├── tsconfig.json
└── package.json
```

**Design tokens** follow the [Cursor](https://cursor.com/) design system (`DESIGN.md`):

| Token | Value | Usage |
|---|---|---|
| `--brand` | `#f54e00` | Primary CTAs, brand accent |
| `--bg` | `#f7f7f4` | Warm cream page canvas |
| `--surface` | `#ffffff` | Card backgrounds |
| `--text` | `#26251e` | Display & body ink |
| `--hairline` | `#e6e5e0` | 1px borders (no shadows) |

Fonts: **Inter** (body/display at weight 400, magazine-editorial voice) + **JetBrains Mono** (all code surfaces and score values).

---

## Quick Start

### Prerequisites

- **Rust** ≥ 1.70 ([rustup](https://rustup.rs/))
- **wasm-pack** (`cargo install wasm-pack`)
- **wasm32 target** (`rustup target add wasm32-unknown-unknown`)
- **Node.js** ≥ 18 ([nvm](https://github.com/nvm-sh/nvm) recommended)

### Local Development

```bash
# Clone
git clone https://github.com/HUQIANTAO/FreqPrompt.git
cd FreqPrompt

# Build WASM
wasm-pack build crates/wasm-bridge --target web
mkdir -p web/src/wasm-pkg
cp -r crates/wasm-bridge/pkg/* web/src/wasm-pkg/

# Build frontend
cd web
npm install
npx rolldown -c rolldown.config.mjs

# Serve
cd dist
python3 -m http.server 8888
# → http://localhost:8888
```

### One-Command Dev Server

```bash
cd web && npm run serve
```

---

## API Configuration

FreqPrompt works with any **OpenAI-compatible** API endpoint. The app is **fully client-side** — your API key is stored in `localStorage` and never leaves your browser.

| Field | Example | Notes |
|---|---|---|
| Base URL | `https://api.openai.com/v1` | Any OpenAI-compatible endpoint |
| API Key | `sk-...` | Stored in browser localStorage only |
| Model ID | `gpt-4o-mini` | The model used for paraphrase generation |

**Supported providers**: OpenAI, Anthropic (via compatible proxy), Groq, Together AI, DeepSeek, Ollama (local), and any other `/v1/chat/completions` endpoint.

---

## Keyboard Shortcuts

| Key | Action |
|---|---|
| `⌘/Ctrl + Enter` | Optimize prompt |
| `⌘/Ctrl + K` | Clear input |
| `⌘/Ctrl + ,` | Open API settings |
| `Esc` | Close any modal/drawer |

---

## Project Structure

```
FreqPrompt/
├── .github/
│   └── workflows/
│       └── deploy.yml             # CI: Rust → WASM → npm → Pages
├── crates/
│   ├── frequency/                 # Core frequency library
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs             # Tokenize, geometric mean, rank
│   │       └── wordfreq_data.rs   # 15K word-Zipf phf::Map
│   └── wasm-bridge/               # WASM binding layer
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs             # wasm_bindgen exports
├── web/
│   ├── index.html                 # SPA shell + design system CSS
│   ├── src/
│   │   ├── main.ts                # Entry & event handling
│   │   ├── paraphrase.ts          # LLM API client
│   │   ├── frequency.ts           # WASM bridge
│   │   └── ui.ts                  # Rendering, history, theme
│   ├── rolldown.config.mjs        # Bundler config
│   ├── tsconfig.json
│   └── package.json
├── DESIGN.md                      # Cursor design system tokens
├── Cargo.toml                     # Rust workspace root
├── LICENSE                        # MIT
└── README.md
```

---

## Design Decisions

### Why Rust/WASM instead of pure JavaScript?

The frequency dictionary is 15,000 entries. JavaScript object lookup is fast, but the WASM approach provides:

- **Type-safe, zero-copy** dictionary via `phf` compile-time perfect hashing
- **Deterministic performance** — no JIT warm-up variance
- **Smaller bundle** — 15K entries compile to ~200 KB WASM (vs ~500 KB minified JSON)
- **Reusability** — the `frequency` crate can be used in CLI tools, servers, or other WASM projects

### Why vanilla HTML/CSS/TypeScript?

No React, no Vue, no Tailwind. The app is a focused single-purpose tool:

- **Zero runtime dependencies** — faster load, no framework churn
- **Small bundle** — 18 KB gzipped total
- **Direct DOM control** — fine-grained transitions and SVG rendering
- **No build step needed for iteration** (just Rolldown for TS)

### Why Rolldown instead of Webpack/Vite?

[Rolldown](https://rolldown.rs/) is the Rust-based bundler used by Vite's upcoming version. For this project, it provides:

- Sub-100ms cold builds (vs 2–5s with Vite/webpack)
- Native ESM output with tree-shaking
- Simple plugin API (just a `copy-assets` hook)
- Single 1.2 MB binary dependency (vs 200+ MB for Vite)

---

## Contributing

Contributions are welcome — whether bug fixes, features, documentation, or design improvements.

### Development Workflow

```bash
# 1. Fork & clone
git clone https://github.com/YOUR_USERNAME/FreqPrompt.git
cd FreqPrompt

# 2. Build WASM (re-run after Rust changes)
wasm-pack build crates/wasm-bridge --target web
mkdir -p web/src/wasm-pkg
cp -r crates/wasm-bridge/pkg/* web/src/wasm-pkg/

# 3. Dev server
cd web
npm install
npx rolldown -c rolldown.config.mjs --watch &
python3 -m http.server 8888 -d dist
```

### Before Submitting

- Rust changes: `cargo test` (in each crate)
- TypeScript: `npx rolldown -c rolldown.config.mjs` (must succeed)
- UI: Test in Chrome, Firefox, and Safari
- Mobile: Test at ≤768px viewport

### Commit Convention

```
type: brief description

feat: add batch optimization support
fix: handle empty LLM response gracefully
docs: document WASM build process
style: align score card padding to 16px
```

---

## Related Work

- **Adam's Law** ([arXiv:2604.02176v2](https://arxiv.org/abs/2604.02176v2)) — the paper that inspired this project
- **DSPy** — framework for algorithmically optimizing LM prompts
- **PromptLayer** — prompt management and observability platform
- **Langfuse** — open-source LLM engineering platform with prompt versioning

---

## License

MIT © FreqPrompt Contributors — see [LICENSE](LICENSE) for details.
