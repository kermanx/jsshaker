import { defineConfig } from 'vite'
import React from '@vitejs/plugin-react'
import TreeShake from '../plugin'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    React(),
    TreeShake(),
  ],
  define: {
    'process.env.NODE_ENV': '"production"',
  },
  build: {
    rollupOptions: {
      external: ['react', 'react/jsx-runtime', 'classnames'],
    }
  }
})
