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

/** The three themes of one colour and white. */
const COLOUR_AND_WHITE = THEMES.filter((entry) => entry.kind === 'colour').map(
  (entry) => entry.name,
);

/** The hue of a colour in degrees, 0 to 360. */
function hue({ red, green, blue }: Colour): number {
  const [r, g, b] = [red / 255, green / 255, blue / 255];
  const high = Math.max(r, g, b);
  const spread = high - Math.min(r, g, b);
  if (spread === 0) return 0;
  const sector =
    high === r ? ((g - b) / spread) % 6 : high === g ? (b - r) / spread + 2 : (r - g) / spread + 4;
  return (sector * 60 + 360) % 360;
}

/** The distance of two hues round the circle. */
function hueGap(a: number, b: number): number {
  const apart = Math.abs(a - b) % 360;
  return apart > 180 ? 360 - apart : apart;
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

  it('shows the black form of a mark on a light ground and the white one on a dark ground', () => {
    for (const theme of FIXED) {
      const isDark = themes.get(theme)?.get('color-scheme') === 'dark';
      expect(themes.get(theme)?.get('--mark-black'), theme).toBe(isDark ? 'none' : 'inline-block');
      expect(themes.get(theme)?.get('--mark-white'), theme).toBe(isDark ? 'inline-block' : 'none');
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

describe('the themes of one colour and white', () => {
  it('are Rose, Delft and Poppy, and each is light', () => {
    expect(COLOUR_AND_WHITE).toEqual(['rose', 'delft', 'poppy']);
    for (const theme of COLOUR_AND_WHITE) {
      expect(themes.get(theme)?.get('color-scheme'), theme).toBe('light');
    }
  });

  it.each(COLOUR_AND_WHITE)('%s has a white or near-white ground, and no pattern', (theme) => {
    // White is 1; the ground may hold a breath of the colour, never more.
    expect(luminance(colourOf(theme, '--ground')), theme).toBeGreaterThan(0.95);
    expect(luminance(colourOf(theme, '--surface')), theme).toBeGreaterThan(0.99);
    expect(luminance(colourOf(theme, '--ground-sunken')), theme).toBeGreaterThan(0.85);
    expect(themes.get(theme)?.get('--pattern')).toBeUndefined();
  });

  it.each(COLOUR_AND_WHITE)(
    '%s keeps its colour for marks, and its words read on the head',
    (theme) => {
      // The focus ring and the line under the active tab are the colour itself.
      expect(themes.get(theme)?.get('--tab-line')).toBe('var(--brass)');
      for (const lower of ['--ground', '--surface', '--ground-sunken']) {
        expect(ratio(theme, '--brass', lower), lower).toBeGreaterThanOrEqual(3);
      }
      // The head and the pressed state are washes of the colour; words on
      // them keep the level AA.
      for (const wash of ['--head-wash', '--pressed', '--hover']) {
        const ground = colourOf(theme, '--ground');
        const washed = over(colourOf(theme, wash), ground);
        for (const word of ['--ink', '--ink-soft', '--ink-faint', '--brass-text']) {
          expect(
            contrastRatio(colourOf(theme, word), washed),
            `${word} on ${wash}`,
          ).toBeGreaterThanOrEqual(4.5);
        }
      }
      for (const word of ['--ink', '--ink-soft', '--ink-faint', '--brass-text', '--danger']) {
        expect(ratio(theme, word, '--ground-sunken'), word).toBeGreaterThanOrEqual(4.5);
      }
    },
  );

  it.each(COLOUR_AND_WHITE)('%s keeps the red of trouble apart from its own colour', (theme) => {
    // Trouble is told from the theme's colour by hue, by at least 20
    // degrees, and is always a red: between 330 and 20 degrees.
    const trouble = hue(colourOf(theme, '--danger'));
    expect(trouble > 330 || trouble < 20, `hue ${trouble}`).toBe(true);
    expect(hueGap(trouble, hue(colourOf(theme, '--brass'))), theme).toBeGreaterThanOrEqual(20);
  });

  it('Poppy tells its red from trouble by lightness as well as hue', () => {
    // The theme's red is warm and light; trouble is cooler and darker, so
    // that the two part in grey too.
    const mark = luminance(colourOf('poppy', '--brass-text'));
    const trouble = luminance(colourOf('poppy', '--danger'));
    expect(trouble).toBeLessThan(mark);
    expect((mark + 0.05) / (trouble + 0.05)).toBeGreaterThanOrEqual(1.2);
  });

  it('statuses part by more than the colour of the theme: each tint is as distinct as in Paper', () => {
    // The status tints are the same in the three, so no status is lost
    // to the theme's own colour: the pigments of a theme never equal its brass.
    for (const theme of COLOUR_AND_WHITE) {
      const brass = colourOf(theme, '--brass');
      for (const pigment of PIGMENTS) {
        const tint = colourOf(theme, `--pigment-${pigment}`);
        expect(JSON.stringify(tint), `${theme} ${pigment}`).not.toBe(JSON.stringify(brass));
      }
    }
  });
});
