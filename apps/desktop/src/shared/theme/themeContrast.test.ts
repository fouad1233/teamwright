import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import {
  contrastRatio,
  luminance,
  over,
  parseColour,
  tokensOfThemes,
  type Colour,
} from './contrast';
import { THEMES } from './themes';

const css = readFileSync(fileURLToPath(new URL('../styles/themes.css', import.meta.url)), 'utf8');
const themes = tokensOfThemes(css);

const PIGMENTS = ['gray', 'blue', 'purple', 'amber', 'green', 'red', 'pink', 'teal'] as const;

/** The tokens of colour every theme must set, so that none of Lamplight's shows through. */
const COLOUR_TOKENS = [
  '--ground',
  '--ground-sunken',
  '--surface',
  '--surface-raised',
  '--rule',
  '--rule-strong',
  '--hover',
  '--pressed',
  '--backdrop',
  '--ink',
  '--ink-soft',
  '--ink-faint',
  '--ink-inverse',
  '--brass',
  '--brass-text',
  '--brass-rgb',
  '--brass-soft',
  '--umber',
  '--primary-hover',
  '--danger',
  '--danger-soft',
  '--lamp-on-ink',
  '--sun-from',
  '--sun-to',
  '--shadow-overlay',
  '--shadow-lifted',
  ...PIGMENTS.flatMap((pigment) => [
    `--pigment-${pigment}`,
    `--pigment-${pigment}-text`,
    `--pigment-${pigment}-wash`,
  ]),
];

function colourOf(theme: string, token: string): Colour {
  const value = themes.get(theme)?.get(token);
  const colour = value === undefined ? null : parseColour(value);
  if (colour === null) throw new Error(`${theme} has no colour for ${token}`);
  return colour;
}

/** The contrast of a token on another, the upper one laid over the lower one. */
function ratio(theme: string, upper: string, lower: string): number {
  const below = colourOf(theme, lower);
  return contrastRatio(over(colourOf(theme, upper), below), below);
}

const FIXED = THEMES.filter((entry) => entry.name !== 'lamplight').map((entry) => entry.name);

describe('the contrast of colours', () => {
  it('reads colours as the style sheets write them', () => {
    expect(parseColour('#12100e')).toEqual({ red: 18, green: 16, blue: 14, alpha: 1 });
    expect(parseColour('#fff')).toEqual({ red: 255, green: 255, blue: 255, alpha: 1 });
    expect(parseColour('rgb(232 224 208 / 9%)')).toEqual({
      red: 232,
      green: 224,
      blue: 208,
      alpha: 0.09,
    });
    expect(parseColour('var(--umber)')).toBeNull();
  });

  it('measures as WCAG 2 does', () => {
    const black = parseColour('#000000');
    const white = parseColour('#ffffff');
    if (black === null || white === null) throw new Error('unread');
    expect(contrastRatio(black, white)).toBeCloseTo(21, 5);
    expect(contrastRatio(white, white)).toBe(1);
    expect(luminance(parseColour('#808080') ?? black)).toBeCloseTo(0.2159, 3);
    // Half of white over black is a middle grey.
    expect(over({ ...white, alpha: 0.5 }, black).red).toBeCloseTo(127.5);
  });
});

describe('the themes', () => {
  it('every theme of the chooser but Lamplight is in the style sheet, and nothing else', () => {
    expect([...themes.keys()].sort()).toEqual([...FIXED].sort());
  });

  it.each(FIXED)('%s sets every token of colour', (theme) => {
    const tokens = themes.get(theme);
    for (const token of COLOUR_TOKENS) expect(tokens?.has(token), token).toBe(true);
  });

  it.each(FIXED)('%s has text that reads at the level AA of WCAG 2', (theme) => {
    for (const lower of ['--ground', '--surface', '--surface-raised']) {
      expect(ratio(theme, '--ink', lower)).toBeGreaterThanOrEqual(4.5);
      expect(ratio(theme, '--ink-soft', lower)).toBeGreaterThanOrEqual(4.5);
      expect(ratio(theme, '--ink-faint', lower)).toBeGreaterThanOrEqual(4.5);
      expect(ratio(theme, '--brass-text', lower)).toBeGreaterThanOrEqual(4.5);
      expect(ratio(theme, '--danger', lower)).toBeGreaterThanOrEqual(4.5);
    }
    expect(ratio(theme, '--brass', '--ground')).toBeGreaterThanOrEqual(3);
  });

  it.each(FIXED)('%s has status tints that read on their ground', (theme) => {
    for (const pigment of PIGMENTS) {
      // A mark of the solid tint needs 3 to 1, a word in the tint 4.5.
      expect(ratio(theme, `--pigment-${pigment}`, '--ground'), pigment).toBeGreaterThanOrEqual(3);
      expect(ratio(theme, `--pigment-${pigment}`, '--surface'), pigment).toBeGreaterThanOrEqual(3);
      expect(
        ratio(theme, `--pigment-${pigment}-text`, '--surface'),
        pigment,
      ).toBeGreaterThanOrEqual(4.5);
      // The word on its own wash, as a status sits in a card.
      const surface = colourOf(theme, '--surface');
      const wash = over(colourOf(theme, `--pigment-${pigment}-wash`), surface);
      const word = colourOf(theme, `--pigment-${pigment}-text`);
      expect(contrastRatio(word, wash), pigment).toBeGreaterThanOrEqual(4.5);
    }
  });

  it('Clear reads at the level AAA, 7 to 1, in every word', () => {
    for (const lower of ['--ground', '--surface']) {
      for (const upper of ['--ink', '--ink-soft', '--ink-faint', '--brass-text', '--danger']) {
        expect(ratio('clear', upper, lower), upper).toBeGreaterThanOrEqual(7);
      }
      for (const pigment of PIGMENTS) {
        expect(ratio('clear', `--pigment-${pigment}-text`, lower), pigment).toBeGreaterThanOrEqual(
          7,
        );
      }
    }
  });

  it('no dark theme sets pure white on pure black, and none goes past 15 to 1', () => {
    for (const theme of FIXED) {
      if (themes.get(theme)?.get('color-scheme') !== 'dark') continue;
      expect(luminance(colourOf(theme, '--ground')), theme).toBeGreaterThan(0);
      expect(luminance(colourOf(theme, '--ink')), theme).toBeLessThan(1);
      expect(ratio(theme, '--ink', '--ground'), theme).toBeLessThanOrEqual(15);
    }
  });

  it('Slate is grey, not black, and of medium contrast', () => {
    const ground = colourOf('slate', '--ground');
    expect(luminance(ground)).toBeGreaterThan(0.015);
    expect(ratio('slate', '--ink', '--ground')).toBeLessThanOrEqual(10);
  });

  it('Dusk is warm and dim: its large fields hold less blue than red', () => {
    for (const token of ['--ground', '--ground-sunken', '--surface', '--ink', '--ink-soft']) {
      const colour = colourOf('dusk', token);
      expect(colour.blue, token).toBeLessThan(colour.red * 0.9);
    }
    // Dimmer ink than Lamplight's at night, which is #e8e0d0.
    expect(luminance(colourOf('dusk', '--ink'))).toBeLessThan(
      luminance(parseColour('#e8e0d0') ?? colourOf('dusk', '--ink')),
    );
  });

  it('Even tells its tints apart by lightness, a step of at least 1.2 to 1 between any two', () => {
    const lightness = PIGMENTS.map((pigment) =>
      luminance(colourOf('even', `--pigment-${pigment}`)),
    ).sort((a, b) => a - b);
    lightness.slice(1).forEach((lighter, step) => {
      const darker = lightness[step] ?? 0;
      expect((lighter + 0.05) / (darker + 0.05)).toBeGreaterThanOrEqual(1.2);
    });
  });

  it('two special palettes are light and two dark, and only they have a pattern', () => {
    const specials = THEMES.filter((entry) => entry.kind === 'special').map((entry) => entry.name);
    const modes = specials.map((name) => themes.get(name)?.get('color-scheme'));
    expect(modes.filter((mode) => mode === 'light')).toHaveLength(2);
    expect(modes.filter((mode) => mode === 'dark')).toHaveLength(2);
    for (const name of FIXED) {
      const pattern = themes.get(name)?.get('--pattern');
      expect(pattern?.startsWith('url("data:image/svg+xml,') ?? false, name).toBe(
        specials.includes(name),
      );
    }
  });
});
