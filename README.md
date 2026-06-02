# 🔬 FreqPrompt

**Prompt Optimizer based on Textual Frequency Law**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Deploy](https://github.com/HUQIANTAO/FreqPrompt/actions/workflows/deploy.yml/badge.svg)](https://github.com/HUQIANTAO/FreqPrompt/actions/workflows/deploy.yml)

FreqPrompt optimizes your LLM prompts by rewriting them with higher-frequency expressions — based on the discovery that **LLMs produce more accurate, higher-quality responses when prompts use higher-frequency words** (even when the semantic meaning is identical).

> 📄 Based on [Adam's Law: Textual Frequency Law on Large Language Models](https://arxiv.org/abs/2604.02176v2) (arXiv:2604.02176v2)

## How It Works

1. **Input** your prompt
2. **LLM rewrites** it into 6 high-frequency variants
3. **Rust WASM engine** scores each variant using a 15K-word Zipf frequency dictionary
4. **Best version** is selected and displayed with visual score comparison

$$sfreq(sentence) = \sqrt[K]{\prod_{k=1}^{K} wfreq(word_k)}$$

## Features

- ⚡ **Frequency Scoring** — Zipf-scale scoring (0–8) via Rust/WASM engine
- 📊 **Visual Comparison** — Side-by-side score bars and donut charts
- 📋 **History** — Local history of past optimizations
- 🌓 **Dark Mode** — System-aware theme with manual toggle
- 📤 **One-Click Export** — Copy optimized prompts instantly
- ⌨️ **Keyboard Shortcuts** — `⌘Enter` to optimize, `⌘K` to clear, `⌘,` for settings

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) ≥ 1.70
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) (`cargo install wasm-pack`)
- [Node.js](https://nodejs.org/) ≥ 18
- wasm32 target (`rustup target add wasm32-unknown-unknown`)

### Build & Run

```bash
# 1. Build WASM
wasm-pack build crates/wasm-bridge --target web
cp -r crates/wasm-bridge/pkg web/src/wasm-pkg

# 2. Install & build frontend
cd web
npm install
npm run build:web   # or: npx rolldown -c rolldown.config.mjs

# 3. Serve
cd dist
python3 -m http.server 8888
# Open http://localhost:8888
```

Or use the npm script:

```bash
cd web
npm run serve
```

## Tech Stack

| Layer | Technology |
|---|---|
| Frequency Engine | Rust → WASM (15K word-Zipf dictionary) |
| LLM API | TypeScript (fetch, OpenAI-compatible) |
| Frontend | Vanilla HTML/CSS/TypeScript |
| Bundler | [Rolldown](https://rolldown.rs/) |
| Design | [Cursor](https://cursor.com/) design system via DESIGN.md |

## Project Structure

```
FreqPrompt/
├── .github/workflows/deploy.yml   # GitHub Pages CI
├── DESIGN.md                      # Design system tokens
├── crates/
│   ├── frequency/                 # Frequency calculation core
│   │   └── src/
│   │       ├── lib.rs             # Tokenization, geometric mean, ranking
│   │       └── wordfreq_data.rs   # 15K word-Zipf dictionary
│   └── wasm-bridge/               # WASM bindings
│       └── src/lib.rs             # wasm-bindgen interface
├── web/
│   ├── index.html                 # Main page (Cursor design system)
│   ├── src/
│   │   ├── main.ts                # Entry point & event bindings
│   │   ├── paraphrase.ts          # LLM API client
│   │   ├── frequency.ts           # WASM interface wrapper
│   │   └── ui.ts                  # UI rendering & history
│   ├── rolldown.config.mjs        # Build config
│   └── dist/                      # Build output
└── README.md
```

## API Configuration

FreqPrompt works with any OpenAI-compatible API:

- **Base URL**: Your API endpoint (e.g. `https://api.openai.com/v1`)
- **API Key**: Your API key
- **Model ID**: Model identifier (e.g. `gpt-4o-mini`)

Configuration is stored locally in your browser — no data is ever sent to our servers.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

MIT © FreqPrompt Contributors
