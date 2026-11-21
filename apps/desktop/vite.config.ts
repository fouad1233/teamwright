import { fileURLToPath } from 'node:url';

import react from '@vitejs/plugin-react';
import { defineConfig, type Plugin } from 'vitest/config';

/**
 * The mode in which `npm run build:demo` builds the interface for the
 * public demo on the website (decision 0032). `src/shared/lib/publicDemo.ts`
 * reads the same name.
 */
const PUBLIC_DEMO_MODE = 'public-demo';

// Set by the Tauri CLI when the interface is served to a physical device.
const tauriDevHost = process.env.TAURI_DEV_HOST;

/**
 * The page of the public demo says what it is, to a visitor and to a
 * search engine. The desktop program's page is left as it is.
 */
function publicDemoPage(): Plugin {
  return {
    name: 'teamwright-public-demo-page',
    transformIndexHtml: (html) =>
      html
        .replace('<title>Teamwright</title>', '<title>Teamwright, the demo in your browser</title>')
        .replace(
          '<meta name="color-scheme"',
          '<meta name="description" content="Try the interface of Teamwright in your browser, on a sample project. Nothing is saved and no AI runs here." />\n    <meta name="color-scheme"',
        ),
  };
}

export default defineConfig(({ mode }) => {
  // `npm run build:demo` builds the interface for the public demo on the
  // website (decision 0032). Every other build is the desktop program's.
  const isPublicDemo = mode === PUBLIC_DEMO_MODE;

  return {
    plugins: isPublicDemo ? [react(), publicDemoPage()] : [react()],
    // The demo is served from a folder of the website whose address is not
    // known here, so it names its files relative to its page. The desktop
    // shell serves the interface from its root.
    base: isPublicDemo ? './' : '/',
    build: {
      outDir: isPublicDemo ? 'dist-demo' : 'dist',
    },
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
  };
});
