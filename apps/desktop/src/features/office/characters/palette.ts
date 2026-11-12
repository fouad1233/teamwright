import type { Color } from '@/shared/api/types';

/*
 * The colors of the people in the office (decision 0019). A character's
 * pixels are the one place in the interface where colors are data, and they
 * all live here, by name.
 *
 * They are tuned to Lamplight: warm skins and hair, cloth in the umbers and
 * parchments of the palette, metal that leans to gilt. The shirt is the
 * member's own pigment and the gold is the brass of the theme; both are read
 * from the tokens, so they follow the theme like the rest of the program.
 */

/** A color and the darker one used for its shadow side. */
export interface Tone {
  base: string;
  shade: string;
}

/** The skins a character may have, from light to deep. */
export const SKINS: readonly Tone[] = [
  { base: '#f1d2b6', shade: '#d6ac8b' },
  { base: '#e2b48c', shade: '#c08e66' },
  { base: '#c99567', shade: '#a5734a' },
  { base: '#a36c45', shade: '#7f5233' },
  { base: '#7c4f33', shade: '#5f3a24' },
];

/** The hair a character may have. */
export const HAIRS: readonly Tone[] = [
  // Nearly black, a little warm.
  { base: '#3a2e27', shade: '#251d18' },
  // Chestnut.
  { base: '#6e4128', shade: '#4f2d1b' },
  // Auburn.
  { base: '#9a4f2c', shade: '#73391e' },
  // Honey.
  { base: '#c59a58', shade: '#9f7840' },
  // Gray, for somebody who has been at it a while.
  { base: '#a39c90', shade: '#7e776d' },
  // Flax.
  { base: '#dcc595', shade: '#b8a172' },
];

/** The cloth, metal and paper the characters wear and work with. */
export const MATERIALS = {
  /** Trousers: dark umber-gray, so the legs do not compete with the shirt. */
  trousers: { base: '#4d433b', shade: '#3a322c' },
  shoes: { base: '#2b231e', shade: '#2b231e' },
  eyes: { base: '#211a15', shade: '#211a15' },
  /** The lab coat of a test engineer: parchment. */
  coat: { base: '#ece4d4', shade: '#c4b9a5' },
  /** The apron of a mechanical engineer: dark leather, so that it stands apart from any shirt. */
  apron: { base: '#5c4029', shade: '#45301f' },
  /** The cardigan of a technical writer: oat. */
  cardigan: { base: '#ab9a7e', shade: '#8b7b62' },
  /** The sweater vest of a researcher: moss. */
  vest: { base: '#6f6b4b', shade: '#56533a' },
  /** The beret of a designer: wine. */
  beret: { base: '#7d3439', shade: '#5c2429' },
  /** The long coat of the Advisor: the umber of the palette, deepened towards ink. */
  robe: { base: '#4f4038', shade: '#3a2f29' },
  /** Steel: a laptop, a caliper, the frame of glasses. */
  metal: { base: '#a8a296', shade: '#78726a' },
  metalLight: { base: '#d3cdc0', shade: '#d3cdc0' },
  /** Paper: a notebook, a plan, a clipboard. */
  paper: { base: '#f3ede0', shade: '#d4cab6' },
  /** The lines written on paper. */
  writing: { base: '#5d5044', shade: '#5d5044' },
  /** A printed circuit board. */
  board: { base: '#2f6a4b', shade: '#22513a' },
  /** The black of a chip, a tablet, a screen at rest. */
  black: { base: '#2a2420', shade: '#1e1916' },
  /** Wood: tables, a counter, a shelf. */
  wood: { base: '#8c6b4b', shade: '#6c5139' },
  woodTop: { base: '#a8845e', shade: '#a8845e' },
  /** Light that falls from a screen or a lamp: gilt, made bright. */
  glow: { base: '#f4dda6', shade: '#e2c27c' },
  /** Steam and smoke. */
  steam: { base: '#d9d1c2', shade: '#bdb4a4' },
  /** Red: the cap of a pen, a test that failed, the coffee machine's light. */
  red: { base: '#c4553f', shade: '#9b3f2e' },
  /** Coffee. */
  coffee: { base: '#4a2f20', shade: '#4a2f20' },
  /** A plant by the window. */
  leaf: { base: '#5a8a55', shade: '#436b40' },
} as const satisfies Record<string, Tone>;

export type MaterialName = keyof typeof MATERIALS;

/** The theme a character is drawn for. */
export type Theme = 'night' | 'paper';

/**
 * The line round a character. At night a dark line would vanish into the
 * ground, so it is a warm umber that holds the shape like a rim of light;
 * on paper it is ink.
 */
export const OUTLINES: Readonly<Record<Theme, string>> = {
  night: '#3a2f27',
  paper: '#2a211b',
};

/** The shadow a character casts on the floor, as a color with its own alpha. */
export const FLOOR_SHADOWS: Readonly<Record<Theme, string>> = {
  night: 'rgb(0 0 0 / 45%)',
  paper: 'rgb(61 47 37 / 16%)',
};

/** Mixes a color of the form #rrggbb towards black by a share from 0 to 1. */
export function darken(color: string, share: number): string {
  const match = /^#([0-9a-f]{6})$/i.exec(color.trim());
  if (!match?.[1]) {
    return color;
  }
  const value = Number.parseInt(match[1], 16);
  const channel = (shift: number) =>
    Math.round(((value >> shift) & 255) * (1 - share))
      .toString(16)
      .padStart(2, '0');
  return `#${channel(16)}${channel(8)}${channel(0)}`;
}

/** The colors of the theme a character takes from the tokens. */
export interface ThemeColors {
  theme: Theme;
  /** The member's pigment, by its name. */
  pigments: Readonly<Record<Color, string>>;
  /** The gold of the theme, for marks. */
  brass: string;
}
