import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { copyFileSync, mkdirSync, readdirSync, existsSync, rmSync } from 'node:fs';

const __dirname = dirname(fileURLToPath(import.meta.url));

/** @type {import('rolldown').RolldownConfig} */
export default {
  input: {
    main: resolve(__dirname, 'src/main.ts'),
    'frequency.worker': resolve(__dirname, 'src/frequency.worker.ts'),
  },
  output: {
    dir: resolve(__dirname, 'dist'),
    format: 'esm',
    entryFileNames: '[name].js',
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
        const wasmSrcDir = resolve(__dirname, 'wasm-pkg');
        const wasmDstDir = resolve(distDir, 'wasm-pkg');

        // Clean and recreate dist
        if (existsSync(distDir)) {
          rmSync(distDir, { recursive: true, force: true });
        }
        mkdirSync(distDir, { recursive: true });
        mkdirSync(wasmDstDir, { recursive: true });

        // Copy index.html
        copyFileSync(
          resolve(__dirname, 'index.html'),
          resolve(distDir, 'index.html')
        );

        // Copy wasm-pkg
        if (existsSync(wasmSrcDir)) {
          for (const file of readdirSync(wasmSrcDir)) {
            copyFileSync(
              resolve(wasmSrcDir, file),
              resolve(wasmDstDir, file)
            );
          }
        }

        console.log('✅ Built + copied assets to dist/');
      },
    },
  ],
};
