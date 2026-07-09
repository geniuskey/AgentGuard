import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    // Tauri loads a static SPA build; there is no Node server.
    adapter: adapter({
      fallback: 'index.html'
    })
  }
};

export default config;
