/**
 * Contrast of colours as WCAG 2 measures it, and a reader of the values a
 * theme gives the tokens. Pure: the tests of the themes use it on
 * `themes.css` itself, so that what the document claims is what ships.
 */

/** A colour as red, green and blue from 0 to 255, and how opaque it is, from 0 to 1. */
export interface Colour {
  red: number;
  green: number;
  blue: number;
  alpha: number;
}

/** Reads `#rrggbb`, `#rgb` or `rgb(r g b / a%)`; anything else is null. */
export function parseColour(text: string): Colour | null {
  const value = text.trim();
  const short = /^#([0-9a-f])([0-9a-f])([0-9a-f])$/i.exec(value);
  const hex = short
    ? short.slice(1).map((digit) => digit + digit)
    : /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(value)?.slice(1);
  if (hex) {
    const [red = '0', green = '0', blue = '0'] = hex;
    return {
      red: parseInt(red, 16),
      green: parseInt(green, 16),
      blue: parseInt(blue, 16),
      alpha: 1,
    };
  }
  const functional = /^rgb\(\s*(\d+)\s+(\d+)\s+(\d+)\s*(?:\/\s*([\d.]+)(%?)\s*)?\)$/i.exec(value);
  if (functional) {
    const [, red, green, blue, share, percent] = functional;
    const alpha = share === undefined ? 1 : Number(share);
    return {
      red: Number(red),
      green: Number(green),
      blue: Number(blue),
      alpha: percent === '%' ? alpha / 100 : alpha,
    };
  }
  return null;
}

/** A colour laid over an opaque one, as the screen shows it. */
export function over(top: Colour, below: Colour): Colour {
  const mix = (a: number, b: number) => a * top.alpha + b * (1 - top.alpha);
  return {
    red: mix(top.red, below.red),
    green: mix(top.green, below.green),
    blue: mix(top.blue, below.blue),
    alpha: 1,
  };
}

/** The relative luminance of an opaque colour, from 0 (black) to 1 (white). */
export function luminance(colour: Colour): number {
  const linear = (channel: number) => {
    const share = channel / 255;
    return share <= 0.04045 ? share / 12.92 : ((share + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * linear(colour.red) + 0.7152 * linear(colour.green) + 0.0722 * linear(colour.blue);
}

/** The contrast ratio of two opaque colours, from 1 to 21, whichever is lighter. */
export function contrastRatio(first: Colour, second: Colour): number {
  const light = Math.max(luminance(first), luminance(second));
  const dark = Math.min(luminance(first), luminance(second));
  return (light + 0.05) / (dark + 0.05);
}

/**
 * The tokens each theme of a style sheet sets, by the theme's name: every
 * custom property inside a rule whose selector names `data-theme='…'`.
 */
export function tokensOfThemes(css: string): Map<string, Map<string, string>> {
  const themes = new Map<string, Map<string, string>>();
  const withoutComments = css.replace(/\/\*[\s\S]*?\*\//g, '');
  const rule = /([^{}]+)\{([^{}]*)\}/g;
  for (const [, selector = '', body = ''] of withoutComments.matchAll(rule)) {
    const name = /\[data-theme='([a-z]+)'\]/.exec(selector)?.[1];
    if (!name) continue;
    const tokens = themes.get(name) ?? new Map<string, string>();
    for (const declaration of body.split(';')) {
      const colon = declaration.indexOf(':');
      const property = declaration.slice(0, colon).trim();
      if (colon < 0 || !(property.startsWith('--') || property === 'color-scheme')) continue;
      tokens.set(property, declaration.slice(colon + 1).trim());
    }
    themes.set(name, tokens);
  }
  return themes;
}
