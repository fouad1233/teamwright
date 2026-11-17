import { describe, expect, it } from 'vitest';

import { DEFAULT_THEME, THEMES, isThemeName, themeOrDefault } from './themes';

describe('the catalogue of themes', () => {
  it('has ten themes, Lamplight first, five palettes and four special ones besides it', () => {
    expect(THEMES).toHaveLength(10);
    expect(THEMES[0]?.name).toBe(DEFAULT_THEME);
    expect(new Set(THEMES.map((theme) => theme.name)).size).toBe(10);
    expect(THEMES.filter((theme) => theme.kind === 'palette')).toHaveLength(6);
    expect(THEMES.filter((theme) => theme.kind === 'special')).toHaveLength(4);
  });

  it('falls back to Lamplight for a name it does not know', () => {
    expect(isThemeName('tide')).toBe(true);
    expect(isThemeName('neon')).toBe(false);
    expect(themeOrDefault('fern')).toBe('fern');
    expect(themeOrDefault('neon')).toBe('lamplight');
    expect(themeOrDefault(null)).toBe('lamplight');
  });

  it('says in every line which mode the theme is, except Lamplight, which follows the system', () => {
    for (const theme of THEMES.slice(1)) {
      expect(theme.line, theme.name).toMatch(/^(Light|Dark)\. /);
    }
  });
});
