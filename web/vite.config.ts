import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// `base: './'` makes the build work from any GitHub Pages sub-path
// (https://<user>.github.io/<repo>/) without configuration.
export default defineConfig({
  base: './',
  plugins: [react()],
  build: {
    target: 'es2022',
    chunkSizeWarningLimit: 1600,
  },
  server: { port: 5173 },
});
