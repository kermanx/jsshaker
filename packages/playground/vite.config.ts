import Unocss from 'unocss/vite'
import { defineConfig } from 'vite'
import Vue from '@vitejs/plugin-vue'
import Wasm from 'vite-plugin-wasm'

export default defineConfig({
  plugins: [Vue(), Unocss(), Wasm()],
  optimizeDeps: {
    exclude: [
      'jsshaker',
    ],
    include: [
      'monaco-editor/esm/vs/editor/editor.worker',
      'monaco-editor/esm/vs/language/typescript/ts.worker',
    ],
  },
  build: {
    target: 'esnext',
  },
  resolve: {
    alias: {
      'jsshaker': 'jsshaker/jsshaker.wasi-browser.js',
    }
  },
  define: {
    'process.env.NODE_DEBUG_NATIVE': 'undefined',
  }
})
