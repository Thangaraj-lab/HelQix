import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'node:path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@helqix/visual-runtime': resolve(__dirname, '../../14-visual-runtime/engines/visual-runtime/src/index.ts'),
    },
  },
  server: { host: '127.0.0.1', port: 5178, strictPort: true },
  preview: { host: '127.0.0.1', port: 4178, strictPort: true },
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes('/node_modules/react/') || id.includes('/node_modules/react-dom/') || id.includes('/node_modules/scheduler/')) return 'framework';
          if (id.includes('/node_modules/three/')) return 'three-core';
          if (id.includes('/14-visual-runtime/')) return 'visual-runtime';
          if (id.includes('/node_modules/lucide-react/')) return 'icons';
        },
      },
    },
  },
});
