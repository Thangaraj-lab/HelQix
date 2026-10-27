/// <reference types="vitest" />
import { defineConfig, loadEnv } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

// The API key never reaches the browser. The browser calls /api/*; this dev server forwards to
// HELQIX_API_URL and adds "Authorization: Bearer $HELQIX_API_KEY" on the server side. Neither variable
// starts with VITE_, so Vite does not put them in the bundle. Put them in .env.local (git-ignored).
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '');
  const target = env.HELQIX_API_URL || 'http://127.0.0.1:8080';
  const key = env.HELQIX_API_KEY || '';
  return {
    plugins: [react(), tailwindcss()],
    // Not host: true. The dev server (and the proxy that holds the key) listens on this machine only.
    server: {
      port: 5173,
      proxy: {
        '/api': {
          target,
          changeOrigin: true,
          rewrite: (path: string) => path.replace(/^\/api/, ''),
          configure: proxy => {
            proxy.on('proxyReq', req => { if (key) req.setHeader('authorization', `Bearer ${key}`); });
          }
        }
      }
    },
    test: {
      environment: 'jsdom',
      include: ['src/**/*.test.{ts,tsx}']
    }
  };
});
