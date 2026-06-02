import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { copyFileSync, mkdirSync, existsSync, readdirSync } from 'node:fs';

const __dirname = dirname(fileURLToPath(import.meta.url));

/** @type {import('rolldown').RolldownConfig} */
export default {
  input: resolve(__dirname, 'src/main.ts'),
  output: {
    dir: resolve(__dirname, 'dist'),
    format: 'esm',
    entryFileNames: 'bundle.js',
  },
  resolve: {
    extensions: ['.ts', '.js'],
  },
  // Keep wasm-pkg imports as runtime dynamic imports
  external: (id) => {
    return id.includes('wasm-pkg/');
  },
  plugins: [
    {
      name: 'copy-assets',
      async buildEnd() {
        const distDir = resolve(__dirname, 'dist');
        mkdirSync(distDir, { recursive: true });

        // Copy index.html
        copyFileSync(
          resolve(__dirname, 'index.html'),
          resolve(distDir, 'index.html')
        );

        // Copy wasm-pkg from src/ to dist/
        const wasmSrcDir = resolve(__dirname, 'src', 'wasm-pkg');
        const wasmDstDir = resolve(distDir, 'wasm-pkg');
        mkdirSync(wasmDstDir, { recursive: true });

        for (const file of readdirSync(wasmSrcDir)) {
          copyFileSync(
            resolve(wasmSrcDir, file),
            resolve(wasmDstDir, file)
          );
        }

        console.log('✅ Copied index.html and wasm-pkg to dist/');
      },
    },
  ],
};
