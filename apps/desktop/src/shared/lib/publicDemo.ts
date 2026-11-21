/** The mode of Vite in which the public demo is built; `vite.config.ts` names it too. */
const PUBLIC_DEMO_MODE = 'public-demo';

/**
 * Whether this is the build of the public demo on the website, rather than
 * the desktop program or the demo of `npm run dev`.
 *
 * Vite writes the mode into the code as text, so in every other build this
 * is false before the code is minified, and what only the public demo uses
 * is left out of the bundle.
 */
export const isPublicDemo: boolean = import.meta.env.MODE === PUBLIC_DEMO_MODE;

/** The website of Teamwright. */
export const WEBSITE_ADDRESS = 'https://fouad1233.github.io/teamwright/';

/** Where the installers of Teamwright are published. */
export const RELEASES_ADDRESS = 'https://github.com/fouad1233/teamwright/releases';
