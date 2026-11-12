import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      // The GUI talks to the manager's API during development. In production the manager
      // serves the built bundle, so there is one origin and no CORS configuration.
      '/api': {
        target: process.env.ORCHESTRATOR_MANAGER ?? 'http://127.0.0.1:8081',
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: 'dist',
    sourcemap: true,
    rollupOptions: {
      output: {
        // Keep the vendor bundle separate so a dependency change does not invalidate the
        // whole cache.
        manualChunks: {
          vendor: ['react', 'react-dom', 'react-router-dom'],
        },
      },
    },
  },
});
