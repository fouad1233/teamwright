import type { ThemeName } from '@/shared/api/types';

import { DEFAULT_THEME } from './themes';

/**
 * The event the window sends when its theme changed, for what draws with
 * its own colours (the office, the portraits) to draw again.
 */
export const THEME_CHANGED = 'teamwright:theme-changed';

/** The theme the window shows now. */
export function shownTheme(): ThemeName {
  return (document.documentElement.dataset.theme as ThemeName | undefined) ?? DEFAULT_THEME;
}

/**
 * Whether the window is light now. Lamplight follows the system; every
 * other theme is one mode. The root's `color-scheme` says which.
 */
export function isLightWindow(): boolean {
  if (typeof getComputedStyle !== 'function') return false;
  return getComputedStyle(document.documentElement).colorScheme.trim().startsWith('light');
}

/**
 * Calls back when the window turns light or dark: when the theme changes,
 * or, in Lamplight, when the system's setting does.
 */
export function subscribeToWindowLook(onChange: () => void): () => void {
  const scheme =
    typeof window.matchMedia === 'function'
      ? window.matchMedia('(prefers-color-scheme: light)')
      : undefined;
  scheme?.addEventListener('change', onChange);
  window.addEventListener(THEME_CHANGED, onChange);
  return () => {
    scheme?.removeEventListener('change', onChange);
    window.removeEventListener(THEME_CHANGED, onChange);
  };
}

/** How long the window takes to cross from one theme to the next: `--duration-crossing`. */
const CROSSING_MS = 400;

function setTheme(name: ThemeName): void {
  const root = document.documentElement;
  if (name === DEFAULT_THEME) {
    delete root.dataset.theme;
  } else {
    root.dataset.theme = name;
  }
  window.dispatchEvent(new Event(THEME_CHANGED));
}

/** What a web view that knows view transitions offers, and an older one does not. */
type CrossingDocument = Document & {
  startViewTransition?: (update: () => Promise<void>) => { finished: Promise<void> };
};

/**
 * Shows a theme. With `smoothly`, the whole window crosses over in about
 * 400 ms on an eased curve: as one picture where the web view knows view
 * transitions, by easing every colour where it does not. With motion
 * turned down it changes at once.
 */
export function applyTheme(name: ThemeName, smoothly = false): void {
  if (name === shownTheme()) return;
  const reduced =
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (!smoothly || reduced) {
    setTheme(name);
    return;
  }
  const root = document.documentElement;
  const crossing = document as CrossingDocument;
  if (typeof crossing.startViewTransition === 'function') {
    root.dataset.themeCrossing = 'picture';
    const transition = crossing.startViewTransition(async () => {
      setTheme(name);
      // Lets React draw what follows the theme before the new picture is taken.
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
    void transition.finished.finally(() => {
      delete root.dataset.themeCrossing;
    });
    return;
  }
  root.dataset.themeCrossing = 'colours';
  setTheme(name);
  window.setTimeout(() => {
    delete root.dataset.themeCrossing;
  }, CROSSING_MS + 50);
}
