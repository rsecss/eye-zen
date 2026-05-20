import { readFileSync, realpathSync } from 'node:fs';
import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';
import tailwindcss from '@tailwindcss/vite';

const host = process.env.TAURI_DEV_HOST;
const pkg = JSON.parse(readFileSync(resolve(__dirname, 'package.json'), 'utf8'));
const nodeModulesRoot = realpathSync(resolve(__dirname, 'node_modules'));

export default defineConfig(async () => ({
  plugins: [svelte(), svelteTesting(), tailwindcss()],

  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },

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
    fs: { allow: [__dirname, nodeModulesRoot] },
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

  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test-setup.ts'],
  },
}));
