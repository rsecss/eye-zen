import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [svelte(), tailwindcss()],

  resolve: {
    alias: { $lib: resolve(__dirname, 'src/lib') },
  },

  clearScreen: false,

  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    watch: { ignored: ['**/src-tauri/**'] },
  },

  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        tip: resolve(__dirname, 'tip.html'),
        'tip-minimal': resolve(__dirname, 'tip-minimal.html'),
        tray: resolve(__dirname, 'tray.html'),
      },
    },
  },
}));
