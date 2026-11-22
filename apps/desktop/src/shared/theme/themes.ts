import type { ThemeName } from '@/shared/api/types';

/**
 * The themes of the window (decision 0028). Each is a set of values for
 * the design tokens, in `shared/styles/themes.css`; this file only names
 * them and says what each is for. The reasoning behind the palettes is in
 * `docs/design/themes.md`.
 */
export interface ThemeEntry {
  name: ThemeName;
  /** The name the chooser shows. */
  label: string;
  /**
   * A palette is plain; a colour-and-white theme is a white ground with one
   * colour; a special palette has a pattern drawn on its ground.
   */
  kind: 'palette' | 'colour' | 'special';
  /** One honest line: what the palette does and on what ground. */
  line: string;
}

/** The theme a window has until the manager chooses another. */
export const DEFAULT_THEME: ThemeName = 'lamplight';

export const THEMES: readonly ThemeEntry[] = [
  {
    name: 'lamplight',
    label: 'Lamplight',
    kind: 'palette',
    line: 'Night or paper, as the system is set. Warm neutrals and one gold.',
  },
  {
    name: 'paper',
    label: 'Paper',
    kind: 'palette',
    line: 'Light. A warm off-white with ink that is not black: the sharpest reading in a bright room.',
  },
  {
    name: 'dusk',
    label: 'Dusk',
    kind: 'palette',
    line: 'Dark. Warm and dim, with little blue, for work in the evening.',
  },
  {
    name: 'slate',
    label: 'Slate',
    kind: 'palette',
    line: 'Dark. A neutral grey, never pure black, and softer contrast for long sessions.',
  },
  {
    name: 'clear',
    label: 'Clear',
    kind: 'palette',
    line: 'Light. The highest contrast: every line of text meets the strictest level of WCAG.',
  },
  {
    name: 'even',
    label: 'Even',
    kind: 'palette',
    line: 'Dark. Statuses differ in lightness as well as hue, so they part without colour vision.',
  },
  {
    name: 'blossom',
    label: 'Blossom',
    kind: 'special',
    line: 'Light. Plum blossoms in faint rose line on a warm ground.',
  },
  {
    name: 'wing',
    label: 'Wing',
    kind: 'special',
    line: 'Dark. Swallows in flight across a night sky.',
  },
  {
    name: 'fern',
    label: 'Fern',
    kind: 'special',
    line: 'Light. Fronds of fern drawn in green on a pale sage ground.',
  },
  {
    name: 'tide',
    label: 'Tide',
    kind: 'special',
    line: 'Dark. Rings of waves, laid row over row, on deep sea water.',
  },
  {
    name: 'rose',
    label: 'Rose',
    kind: 'colour',
    line: 'Light. A white ground with pink for what is chosen, focused or pressed.',
  },
  {
    name: 'delft',
    label: 'Delft',
    kind: 'colour',
    line: 'Light. A white ground with the blue of glazed tiles for what is chosen, focused or pressed.',
  },
  {
    name: 'poppy',
    label: 'Poppy',
    kind: 'colour',
    line: 'Light. A white ground with red for what is chosen, focused or pressed; trouble keeps a darker red.',
  },
];

const NAMES = new Set<string>(THEMES.map((theme) => theme.name));

/** Whether a text names a theme. */
export function isThemeName(text: unknown): text is ThemeName {
  return typeof text === 'string' && NAMES.has(text);
}

/** The theme a text names, or the default one if it names none. */
export function themeOrDefault(text: unknown): ThemeName {
  return isThemeName(text) ? text : DEFAULT_THEME;
}

/**
 * Where the window remembers its theme in the browser's storage, so that
 * it opens in that theme before the backend has answered. The backend
 * keeps the choice; this is only a copy of it.
 */
export const REMEMBERED_THEME_KEY = 'teamwright.theme';

/** The theme the window remembered, or the default one. */
export function rememberedTheme(): ThemeName {
  try {
    return themeOrDefault(globalThis.localStorage?.getItem(REMEMBERED_THEME_KEY));
  } catch {
    // Storage can be turned off; then the window opens in the default.
    return DEFAULT_THEME;
  }
}

/** Remembers a theme in the browser's storage. */
export function rememberTheme(name: ThemeName): void {
  try {
    globalThis.localStorage?.setItem(REMEMBERED_THEME_KEY, name);
  } catch {
    // Without storage the window opens in the default, then turns.
  }
}
