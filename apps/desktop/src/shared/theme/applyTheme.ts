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

/** Shows a theme. */
export function applyTheme(name: ThemeName): void {
  if (name === shownTheme()) return;
  const root = document.documentElement;
  if (name === DEFAULT_THEME) {
    delete root.dataset.theme;
  } else {
    root.dataset.theme = name;
  }
  window.dispatchEvent(new Event(THEME_CHANGED));
}
