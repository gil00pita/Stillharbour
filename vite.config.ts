import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';

export default defineConfig(({ mode }) => ({
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
      ...(mode === 'desktop-test' && process.env.STILLHARBOUR_REACT_PROFILE === '1'
        ? { 'react-dom/client': 'react-dom/profiling' }
        : {}),
    },
  },
  plugins: [react()],
  server: {
    host: '127.0.0.1',
    port: Number(process.env.STILLHARBOUR_PORT ?? 1420),
    strictPort: true,
  },
  clearScreen: false,
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test-setup.ts'],
    include: ['src/**/*.test.tsx', 'tests/desktop/**/*.test.mjs'],
  },
}));
