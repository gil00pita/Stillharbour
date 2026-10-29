import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig(({ mode }) => ({
  resolve: {
    alias:
      mode === 'desktop-test' && process.env.STILLHARBOUR_REACT_PROFILE === '1'
        ? { 'react-dom/client': 'react-dom/profiling' }
        : {},
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
