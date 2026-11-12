import type { Color } from '@/shared/api/types';

import { OUTFITS, type Look } from './characterModel';
import {
  darken,
  FLOOR_SHADOWS,
  HAIRS,
  MATERIALS,
  OUTLINES,
  SKINS,
  type Theme,
  type ThemeColors,
  type Tone,
} from './palette';
import { OUTLINE, type Sprite } from './spriteModel';
import type { Garment } from './sprites';

/*
 * Paints the pictures of spriteModel.ts: every letter gets its color for a
 * character and a theme, and every pixel becomes a square of whole device
 * pixels, so that nothing is ever blurred by scaling.
 */

/** The colors of a character's letters. */
export type Colors = ReadonlyMap<string, string>;

const PIGMENTS: readonly Color[] = [
  'gray',
  'blue',
  'purple',
  'amber',
  'green',
  'red',
  'pink',
  'teal',
];

/** Reads the colors a character takes from the tokens of the theme that is shown. */
export function readThemeColors(theme: Theme): ThemeColors {
  const style = getComputedStyle(document.documentElement);
  const token = (name: string, fallback: string) => style.getPropertyValue(name).trim() || fallback;
  const pigments = Object.fromEntries(
    PIGMENTS.map((color) => [color, token(`--pigment-${color}`, '#8a857a')]),
  ) as Record<Color, string>;
  return { theme, pigments, brass: token('--brass', '#c9a96e') };
}

/** The theme the window shows now. */
export function currentTheme(): Theme {
  return typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-color-scheme: light)').matches
    ? 'paper'
    : 'night';
}

/** The letters of the body each garment paints, and with what. */
function garmentLetters(garment: Garment, shirt: Tone, gilt: Tone): Record<string, Tone> {
  const all = (tone: Tone) => ({ c: tone, b: tone, l: tone, n: tone });
  switch (garment) {
    case 'shirt':
      return all(shirt);
    case 'labCoat':
      return { ...all(MATERIALS.coat), n: shirt };
    case 'apron':
      return { ...all(shirt), n: MATERIALS.apron, b: MATERIALS.apron };
    case 'cardigan':
      return { ...all(MATERIALS.cardigan), n: shirt };
    case 'vest':
      return { ...all(MATERIALS.vest), l: shirt, n: shirt };
    case 'jacket':
      // The manager's jacket is the gold of the keystone, over a shirt of paper.
      return { ...all(gilt), n: MATERIALS.paper };
    case 'robe':
      return { ...all(MATERIALS.robe), n: MATERIALS.paper };
  }
}

/** The color of every letter for one character in one theme. */
export function colorsOf(look: Look, theme: ThemeColors): Colors {
  const skin = SKINS[look.skin] ?? SKINS[0] ?? MATERIALS.paper;
  const hair = HAIRS[look.hair] ?? HAIRS[0] ?? MATERIALS.black;
  const shirtBase = look.shirt === 'brass' ? theme.brass : theme.pigments[look.shirt];
  const shirt = { base: shirtBase, shade: darken(shirtBase, 0.28) };
  const gilt = { base: theme.brass, shade: darken(theme.brass, 0.28) };
  const body = garmentLetters(OUTFITS[look.role].garment, shirt, gilt);
  const colors = new Map<string, string>([
    ['s', skin.base],
    ['z', skin.shade],
    ['e', MATERIALS.eyes.base],
    ['h', hair.base],
    ['j', hair.shade],
    ['p', MATERIALS.trousers.base],
    ['q', MATERIALS.trousers.shade],
    ['k', MATERIALS.shoes.base],
    ['1', MATERIALS.beret.base],
    ['2', MATERIALS.beret.shade],
    ['m', MATERIALS.metal.base],
    ['M', MATERIALS.metal.shade],
    ['i', MATERIALS.metalLight.base],
    ['x', MATERIALS.black.base],
    ['X', MATERIALS.black.shade],
    ['w', MATERIALS.paper.base],
    ['v', MATERIALS.paper.shade],
    ['t', MATERIALS.writing.base],
    ['g', MATERIALS.board.base],
    ['G', MATERIALS.board.shade],
    ['o', MATERIALS.wood.base],
    ['O', MATERIALS.wood.shade],
    ['T', MATERIALS.woodTop.base],
    ['y', MATERIALS.glow.base],
    ['Y', MATERIALS.glow.shade],
    ['f', MATERIALS.steam.base],
    ['r', MATERIALS.red.base],
    ['R', MATERIALS.red.shade],
    ['u', MATERIALS.coffee.base],
    ['a', MATERIALS.leaf.base],
    ['A', MATERIALS.leaf.shade],
    ['$', theme.brass],
    [OUTLINE, OUTLINES[theme.theme]],
  ]);
  // The body: each part and its shadow.
  const shadows: Record<string, string> = { c: 'd', b: 'B', l: 'L', n: 'n' };
  for (const [letter, tone] of Object.entries(body)) {
    colors.set(letter, tone.base);
    const shadow = shadows[letter];
    if (shadow && shadow !== letter) {
      colors.set(shadow, tone.shade);
    }
  }
  return colors;
}

/** The colors of things that belong to nobody: the furniture of the room. */
export function roomColors(theme: ThemeColors): Colors {
  return colorsOf(
    { role: 'default', skin: 0, hair: 0, hairStyle: 'short', shirt: 'gray', seed: 0 },
    theme,
  );
}

/** The shadow a figure casts on the floor, in the theme. */
export function floorShadow(theme: Theme): string {
  return FLOOR_SHADOWS[theme];
}

/**
 * Paints a picture onto a canvas of its own, every pixel a square of `unit`
 * device pixels. Letters without a color are left empty.
 */
export function paintSprite(sprite: Sprite, colors: Colors, unit: number): HTMLCanvasElement {
  const canvas = document.createElement('canvas');
  canvas.width = sprite.width * unit;
  canvas.height = sprite.height * unit;
  const context = canvas.getContext('2d');
  if (!context) {
    return canvas;
  }
  sprite.rows.forEach((row, y) => {
    for (let x = 0; x < row.length; x += 1) {
      const color = colors.get(row[x] ?? '.');
      if (color !== undefined) {
        context.fillStyle = color;
        context.fillRect(x * unit, y * unit, unit, unit);
      }
    }
  });
  return canvas;
}

/**
 * How many device pixels one pixel of a character takes: twice the
 * pixels of the screen at its scale, rounded down, so that a character is
 * never larger than it was drawn to be and always lands on whole pixels.
 */
export function deviceUnit(cssUnit: number, devicePixelRatio: number): number {
  return Math.max(1, Math.floor(cssUnit * devicePixelRatio));
}

/** A store of painted pictures, so that each is painted once per theme and scale. */
export class SpriteCache {
  private readonly pictures = new Map<string, HTMLCanvasElement>();

  get(key: string, paint: () => HTMLCanvasElement): HTMLCanvasElement {
    const found = this.pictures.get(key);
    if (found) {
      return found;
    }
    const painted = paint();
    this.pictures.set(key, painted);
    return painted;
  }

  clear(): void {
    this.pictures.clear();
  }
}
