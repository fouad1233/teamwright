import { fileURLToPath } from 'node:url';

import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

// Set by the Tauri CLI when the interface is served to a physical device.
const tauriDevHost = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      // `@/shared/ui/Button` instead of `../../../shared/ui/Button`.
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  // Keep the Rust compiler's output on screen when the Tauri CLI runs Vite.
  clearScreen: false,
  server: {
    // The desktop shell loads the interface from this port, so it is fixed.
    port: 1420,
    strictPort: true,
    host: tauriDevHost ?? false,
    hmr: tauriDevHost ? { protocol: 'ws', host: tauriDevHost, port: 1421 } : undefined,
    watch: {
      // Changes to the Rust code are the Tauri CLI's business, not Vite's.
      ignored: ['**/src-tauri/**'],
    },
  },
  test: {
    include: ['src/**/*.test.ts'],
  },
});
