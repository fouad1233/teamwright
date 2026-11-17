// Draws the patterns of the four special palettes (decision 0028) and
// prints their tokens, the drawings as SVG in data URIs, for
// apps/desktop/src/shared/styles/themes.css.
//
//   node tools/design/theme-patterns.mjs [file]
//
// Each drawing is a tile that repeats. A motif that crosses an edge of the
// tile is drawn again on the far side, so the repeat shows no seam, and the
// motifs are placed by eye, not on a grid, so the repeat is hard to see.
import { writeFileSync } from 'node:fs';

const f = (n) => Number(n.toFixed(1));
const rot = ([x, y], a) => [x * Math.cos(a) - y * Math.sin(a), x * Math.sin(a) + y * Math.cos(a)];
const pt = (p) => `${f(p[0])} ${f(p[1])}`;

/** Draws a motif, and again on the far side of every edge it crosses, so the tile has no seam. */
function wrapped(W, H, x, y, reach, draw) {
  let out = '';
  for (const dx of [-W, 0, W]) {
    for (const dy of [-H, 0, H]) {
      const cx = x + dx;
      const cy = y + dy;
      if (cx + reach < 0 || cx - reach > W || cy + reach < 0 || cy - reach > H) continue;
      out += draw(cx, cy);
    }
  }
  return out;
}

/* Blossom: plum blossoms with five round petals, a ring at the heart and
   a crown of stamens, with single petals falling between them. */
function blossom() {
  const W = 300;
  const H = 300;
  const flower = (r, turn) => (cx, cy) => {
    let d = '';
    for (let i = 0; i < 5; i += 1) {
      const a = turn + (i * 2 * Math.PI) / 5;
      const P = (x, y) => {
        const [u, v] = rot([x, y], a);
        return [cx + u, cy + v];
      };
      const w = r * 0.62;
      d += `M${pt(P(-r * 0.12, -r * 0.2))}`;
      d += `C${pt(P(-w * 1.05, -r * 0.42))} ${pt(P(-w * 0.86, -r * 1.06))} ${pt(P(0, -r * 1.02))}`;
      d += `C${pt(P(w * 0.86, -r * 1.06))} ${pt(P(w * 1.05, -r * 0.42))} ${pt(P(r * 0.12, -r * 0.2))}`;
    }
    d += `M${pt([cx + r * 0.16, cy])}a${f(r * 0.16)} ${f(r * 0.16)} 0 1 0 ${f(-r * 0.32)} 0a${f(r * 0.16)} ${f(r * 0.16)} 0 1 0 ${f(r * 0.32)} 0`;
    let s = '';
    for (let i = 0; i < 9; i += 1) {
      const a = turn + 0.35 + (i * 2 * Math.PI) / 9;
      const inner = rot([0, -r * 0.2], a);
      const len = r * (0.44 + (i % 2) * 0.08);
      const outer = rot([0, -len], a);
      s += `M${pt([cx + inner[0], cy + inner[1]])}L${pt([cx + outer[0], cy + outer[1]])}`;
      const dot = rot([0, -len - r * 0.05], a);
      s += `M${pt([cx + dot[0], cy + dot[1]])}h.1`;
    }
    return `<path d="${d}"/><path d="${s}" stroke-linecap="round" stroke-width="1.4"/>`;
  };
  const petal = (r, a) => (cx, cy) => {
    const P = (x, y) => {
      const [u, v] = rot([x, y], a);
      return [cx + u, cy + v];
    };
    return `<path d="M${pt(P(0, r))}C${pt(P(-r * 0.9, r * 0.3))} ${pt(P(-r * 0.7, -r))} ${pt(P(0, -r * 0.9))}C${pt(P(r * 0.7, -r))} ${pt(P(r * 0.9, r * 0.3))} ${pt(P(0, r))}"/>`;
  };
  const motifs = [
    [62, 70, 19, 0.2, 'flower'],
    [208, 46, 12, 0.9, 'flower'],
    [176, 168, 22, 1.3, 'flower'],
    [48, 226, 11, 0.5, 'flower'],
    [272, 262, 9, 1.0, 'flower'],
    [126, 252, 14, 0.1, 'flower'],
    [140, 104, 5, 0.6, 'petal'],
    [258, 128, 4.5, 2.1, 'petal'],
    [22, 150, 4, -0.8, 'petal'],
    [96, 18, 3.6, 1.4, 'petal'],
    [222, 222, 4, 2.8, 'petal'],
  ];
  let body = '';
  for (const [x, y, r, a, kind] of motifs) {
    body += wrapped(W, H, x, y, r * 1.2, kind === 'flower' ? flower(r, a) : petal(r, a));
  }
  return { W, H, body };
}

/* Fern: fronds that arc, each a stem with leaflets that shorten towards
   the tip, the way a block print cuts them. */
function fern() {
  const W = 320;
  const H = 320;
  const frond = (length, angle, bend) => (x0, y0) => {
    const end = [x0 + Math.cos(angle) * length, y0 + Math.sin(angle) * length];
    const mid = [x0 + Math.cos(angle) * length * 0.5, y0 + Math.sin(angle) * length * 0.5];
    const ctrl = [mid[0] - Math.sin(angle) * bend, mid[1] + Math.cos(angle) * bend];
    const at = (t) => [
      (1 - t) ** 2 * x0 + 2 * (1 - t) * t * ctrl[0] + t * t * end[0],
      (1 - t) ** 2 * y0 + 2 * (1 - t) * t * ctrl[1] + t * t * end[1],
    ];
    const tangent = (t) =>
      Math.atan2(
        2 * (1 - t) * (ctrl[1] - y0) + 2 * t * (end[1] - ctrl[1]),
        2 * (1 - t) * (ctrl[0] - x0) + 2 * t * (end[0] - ctrl[0]),
      );
    let d = `M${pt([x0, y0])}Q${pt(ctrl)} ${pt(end)}`;
    const count = 10;
    for (let i = 1; i <= count; i += 1) {
      const t = 0.08 + (i / (count + 1)) * 0.9;
      const a = tangent(t);
      const size = length * 0.19 * (1 - t * 0.8);
      for (const side of [-1, 1]) {
        // Leaflets alternate a little, as a real frond's do.
        const p = at(Math.min(0.99, t + (side > 0 ? 0.018 : 0)));
        const lean = a + side * (Math.PI / 2 - 0.6);
        const tip = [p[0] + Math.cos(lean) * size, p[1] + Math.sin(lean) * size];
        const w = size * 0.3;
        const n = [-Math.sin(lean) * w, Math.cos(lean) * w];
        const half = [p[0] + Math.cos(lean) * size * 0.45, p[1] + Math.sin(lean) * size * 0.45];
        d += `M${pt(p)}Q${pt([half[0] + n[0], half[1] + n[1]])} ${pt(tip)}Q${pt([half[0] - n[0], half[1] - n[1]])} ${pt(p)}`;
      }
    }
    return `<path d="${d}" stroke-linejoin="round" stroke-linecap="round"/>`;
  };
  const motifs = [
    [30, 300, 170, -1.1, 30],
    [190, 250, 104, -1.85, -18],
    [228, 120, 76, -0.5, 12],
    [96, 112, 58, -2.5, -10],
    [292, 40, 50, -2.0, 8],
  ];
  let body = '';
  for (const [x, y, length, angle, bend] of motifs) {
    // The reach is measured from the root, which is where the frond starts.
    body += wrapped(W, H, x, y, length * 1.1, frond(length, angle, bend));
  }
  return { W, H, body };
}

/* Wing: swallows in flight, each two tapered wings and a forked tail,
   filled as a cut block leaves them, in a loose and uneven flock. */
function wing() {
  const W = 340;
  const H = 260;
  const bird = (s, a) => (cx, cy) => {
    const P = (x, y) => {
      const [u, v] = rot([x * s, y * s], a);
      return [cx + u, cy + v];
    };
    const d = [
      `M${pt(P(0, 0))}`,
      `C${pt(P(-4, -3.4))} ${pt(P(-10, -5.6))} ${pt(P(-18, -2.6))}`,
      `C${pt(P(-11.5, -3.4))} ${pt(P(-5.5, -1.4))} ${pt(P(-1.3, 1.6))}`,
      `L${pt(P(-2.8, 7.4))}L${pt(P(-0.5, 3.8))}L${pt(P(0, 4.4))}L${pt(P(0.5, 3.8))}L${pt(P(2.8, 7.4))}`,
      `L${pt(P(1.3, 1.6))}`,
      `C${pt(P(5.5, -1.4))} ${pt(P(11.5, -3.4))} ${pt(P(18, -2.6))}`,
      `C${pt(P(10, -5.6))} ${pt(P(4, -3.4))} ${pt(P(0, 0))}Z`,
    ].join('');
    return `<path d="${d}"/>`;
  };
  const places = [
    [48, 44, 1.25, -0.2],
    [112, 70, 0.85, 0.14],
    [150, 36, 0.64, -0.06],
    [262, 108, 1.05, 0.24],
    [214, 66, 0.5, 0.1],
    [70, 168, 0.72, -0.32],
    [196, 204, 1.45, 0.06],
    [300, 222, 0.58, -0.14],
    [26, 236, 0.5, 0.22],
    [326, 30, 0.7, 0.3],
  ];
  let body = '';
  for (const [x, y, s, a] of places) body += wrapped(W, H, x, y, 20 * s, bird(s, a));
  return { W, H, body, filled: true };
}

/* Tide: overlapping scales of waves (seigaiha), four rings each, the
   lower row laid over the one above it. */
function tide() {
  const R = 34;
  const W = 2 * R;
  const H = R;
  // Each row is seen only where the rows below it do not cover it: a mask
  // of its own, so that the tile needs no ground colour.
  const rows = [
    [0, R],
    [R / 2, 0],
    [R, R],
    [(3 * R) / 2, 0],
    [2 * R, R],
  ];
  const xsOf = (offset) => (offset === 0 ? [0, 2 * R] : [R]);
  let defs = '';
  let body = '';
  rows.forEach(([y, offset], i) => {
    let hide = '';
    for (const [y2, offset2] of rows.slice(i + 1)) {
      for (const x2 of xsOf(offset2)) hide += `<circle cx="${x2}" cy="${y2}" r="${R}"/>`;
    }
    if (hide) defs += `<mask id="m${i}"><rect x="-9" y="-9" width="99" height="99" fill="white"/><g fill="black">${hide}</g></mask>`;
    let rings = '';
    for (const x of xsOf(offset)) {
      for (const k of [1, 0.75, 0.5, 0.25]) rings += `<circle cx="${x}" cy="${y}" r="${f(R * k - 0.5)}"/>`;
    }
    body += hide ? `<g mask="url(#m${i})">${rings}</g>` : rings;
  });
  return { W, H, body: `<defs>${defs}</defs>${body}` };
}

function uri({ W, H, body, filled }, color, opacity) {
  const paint = filled
    ? `fill='${color}' fill-opacity='${opacity}'`
    : `fill='none' stroke='${color}' stroke-opacity='${opacity}'`;
  const svg = `<svg xmlns='http://www.w3.org/2000/svg' width='${W}' height='${H}'><g ${paint}>${body.replaceAll('"', "'")}</g></svg>`;
  return `url("data:image/svg+xml,${svg.replaceAll('#', '%23').replaceAll('<', '%3C').replaceAll('>', '%3E')}")`;
}

/* Each pattern in the ink of its palette, at three strengths: faint on the
   ground of the window, stronger above the foot of the sidebar and under
   an empty state, strongest in the small swatch of the chooser. */
const PATTERNS = {
  blossom: { draw: blossom(), color: '#9b4a55', soft: 0.075, strong: 0.2, swatch: 0.45 },
  wing: { draw: wing(), color: '#d8cdb8', soft: 0.065, strong: 0.15, swatch: 0.4 },
  fern: { draw: fern(), color: '#3f5a3a', soft: 0.075, strong: 0.2, swatch: 0.45 },
  tide: { draw: tide(), color: '#cfe3df', soft: 0.05, strong: 0.12, swatch: 0.3 },
};

let css = '';
for (const [name, pattern] of Object.entries(PATTERNS)) {
  const { draw, color } = pattern;
  css += `/* ${name} */\n`;
  css += `--pattern: ${uri(draw, color, pattern.soft)};\n`;
  css += `--pattern-strong: ${uri(draw, color, pattern.strong)};\n`;
  css += `--pattern-swatch: ${uri(draw, color, pattern.swatch)};\n`;
  css += `--pattern-size: ${draw.W}px ${draw.H}px;\n`;
  css += `--pattern-swatch-size: ${Math.round(draw.W * 0.6)}px ${Math.round(draw.H * 0.6)}px;\n\n`;
}
if (process.argv[2]) {
  writeFileSync(process.argv[2], css);
} else {
  process.stdout.write(css);
}
