import { resolve, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { copyFileSync, mkdirSync, readdirSync, existsSync, rmSync, statSync } from 'node:fs';

const __dirname = dirname(fileURLToPath(import.meta.url));

function copyDirSync(src, dst) {
  mkdirSync(dst, { recursive: true });
  for (const entry of readdirSync(src)) {
    const srcPath = join(src, entry);
    const dstPath = join(dst, entry);
    if (statSync(srcPath).isDirectory()) {
      copyDirSync(srcPath, dstPath);
    } else {
      copyFileSync(srcPath, dstPath);
    }
  }
}

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
        const wasmSrcDir = resolve(__dirname, 'src', 'wasm-pkg');
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

        // Copy styles.css
        copyFileSync(
          resolve(__dirname, 'styles.css'),
          resolve(distDir, 'styles.css')
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

        // Copy ontologies (v3 Sprint 2)
        const ontoSrcDir = resolve(__dirname, '..', 'ontologies');
        const ontoDstDir = resolve(distDir, 'ontologies');
        if (existsSync(ontoSrcDir)) {
          copyDirSync(ontoSrcDir, ontoDstDir);
        }

        console.log('✅ Built + copied assets to dist/');
      },
    },
  ],
};
