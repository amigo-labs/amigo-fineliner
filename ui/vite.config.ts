import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  // The wasm-pack bundle locates its .wasm via import.meta.url; exclude it from
  // dependency pre-bundling so that URL resolution works in dev and build.
  optimizeDeps: {
    exclude: ['fineliner-wasm'],
  },
});
