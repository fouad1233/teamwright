/**
 * Draws the Teamwright key visual, "Plate 01", and writes it as SVG and PNG.
 *
 * The plate is an expression of the design philosophy in
 * docs/design/philosophy.md: an engineering sheet with two figures, an
 * assembly of five parts and the order of the work.
 *
 * Usage:
 *   node tools/design/key-visual.mjs <fonts-directory> <output-directory>
 *
 * The fonts directory must contain Instrument Sans, Instrument Serif and IBM
 * Plex Mono as .ttf files (all three are licensed under the SIL Open Font
 * License). Rendering needs the package @resvg/resvg-js.
 */

import { readdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { Resvg } from '@resvg/resvg-js';

const [fontsDirectory, outputDirectory] = process.argv.slice(2);
if (!fontsDirectory || !outputDirectory) {
  console.error('usage: node key-visual.mjs <fonts-directory> <output-directory>');
  process.exit(1);
}

// ---------------------------------------------------------------- the sheet

const SHEET = { width: 3000, height: 2000 };

/** The left and right edges that everything on the sheet lines up with. */
const MARGIN = { left: 240, right: SHEET.width - 240 };

/** The colors of the plate: paper, ink in three strengths, and brass. */
const COLOR = {
  paper: '#f3efe6',
  ink: '#211f1b',
  inkSoft: '#6d685e',
  inkFaint: '#b9b3a6',
  brass: '#bf9130',
};

/** The three weights of line. */
const LINE = { heavy: 4, medium: 2.5, hair: 1.5 };

const FONT = {
  sans: 'Instrument Sans',
  serif: 'Instrument Serif',
  mono: 'IBM Plex Mono',
};

/** The module everything is measured in. */
const UNIT = 20;

// ------------------------------------------------------------ small helpers

const round = (value) => Math.round(value * 100) / 100;

function line(x1, y1, x2, y2, { stroke = COLOR.ink, width = LINE.hair, dash } = {}) {
  const dashes = dash ? ` stroke-dasharray="${dash}"` : '';
  return `<line x1="${round(x1)}" y1="${round(y1)}" x2="${round(x2)}" y2="${round(y2)}" stroke="${stroke}" stroke-width="${width}"${dashes}/>`;
}

function rect(x, y, width, height, { fill = 'none', stroke = 'none', strokeWidth = 0 } = {}) {
  return `<rect x="${round(x)}" y="${round(y)}" width="${round(width)}" height="${round(height)}" fill="${fill}" stroke="${stroke}" stroke-width="${strokeWidth}"/>`;
}

function circle(x, y, radius, { fill = 'none', stroke = 'none', strokeWidth = 0 } = {}) {
  return `<circle cx="${round(x)}" cy="${round(y)}" r="${radius}" fill="${fill}" stroke="${stroke}" stroke-width="${strokeWidth}"/>`;
}

function text(x, y, content, options = {}) {
  const {
    font = FONT.mono,
    size = 20,
    fill = COLOR.inkSoft,
    anchor = 'start',
    weight = 400,
    style = 'normal',
    spacing = 0,
    rotate,
  } = options;
  const transform =
    rotate === undefined ? '' : ` transform="rotate(${rotate} ${round(x)} ${round(y)})"`;
  return `<text x="${round(x)}" y="${round(y)}" font-family="${font}" font-size="${size}" font-weight="${weight}" font-style="${style}" letter-spacing="${spacing}" fill="${fill}" text-anchor="${anchor}"${transform}>${content}</text>`;
}

/** Capital labels, set small and widely spaced in the monospace. */
function label(x, y, content, options = {}) {
  return text(x, y, content, { size: 19, spacing: 3.2, ...options });
}

// ------------------------------------------------------------------ hatches

/**
 * The hatches that tell the parts apart. They are defined in the coordinates
 * of the sheet, not of each part, so the strokes of a hatch line up across
 * every part that carries it.
 */
const HATCH = {
  diagonal: 'hatch-diagonal',
  dots: 'hatch-dots',
  horizontal: 'hatch-horizontal',
};

function hatchDefinitions() {
  const pitch = 16;
  return `
    <pattern id="${HATCH.diagonal}" width="${pitch}" height="${pitch}" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
      <line x1="0" y1="0" x2="0" y2="${pitch}" stroke="${COLOR.ink}" stroke-width="2.2"/>
    </pattern>
    <pattern id="${HATCH.dots}" width="${pitch}" height="${pitch}" patternUnits="userSpaceOnUse">
      <circle cx="${pitch / 4}" cy="${pitch / 4}" r="2.3" fill="${COLOR.ink}"/>
      <circle cx="${(pitch * 3) / 4}" cy="${(pitch * 3) / 4}" r="2.3" fill="${COLOR.ink}"/>
    </pattern>
    <pattern id="${HATCH.horizontal}" width="${pitch}" height="${pitch * 0.75}" patternUnits="userSpaceOnUse">
      <line x1="0" y1="${pitch * 0.375}" x2="${pitch}" y2="${pitch * 0.375}" stroke="${COLOR.ink}" stroke-width="2.2"/>
    </pattern>`;
}

/** The fill of a part: a hatch, solid ink, or brass. */
function partFill(kind) {
  if (kind === 'ink') return COLOR.ink;
  if (kind === 'brass') return COLOR.brass;
  return `url(#${HATCH[kind]})`;
}

// --------------------------------------------------------------- the border

/**
 * The border of an engineering sheet: two frames with zone marks between
 * them, numbers along the top and bottom and letters down the sides.
 */
function sheetBorder() {
  const outer = 80;
  const inner = 124;
  const parts = [
    rect(outer, outer, SHEET.width - 2 * outer, SHEET.height - 2 * outer, {
      stroke: COLOR.inkFaint,
      strokeWidth: LINE.hair,
    }),
    rect(inner, inner, SHEET.width - 2 * inner, SHEET.height - 2 * inner, {
      stroke: COLOR.ink,
      strokeWidth: LINE.medium,
    }),
  ];

  const columns = 12;
  const rows = 8;
  const columnWidth = (SHEET.width - 2 * inner) / columns;
  const rowHeight = (SHEET.height - 2 * inner) / rows;
  const middle = (outer + inner) / 2;
  const zone = { size: 17, anchor: 'middle', fill: COLOR.inkSoft };

  for (let column = 0; column <= columns; column += 1) {
    const x = inner + column * columnWidth;
    parts.push(line(x, outer, x, inner, { stroke: COLOR.inkFaint }));
    parts.push(line(x, SHEET.height - inner, x, SHEET.height - outer, { stroke: COLOR.inkFaint }));
    if (column < columns) {
      const center = x + columnWidth / 2;
      const number = String(column + 1).padStart(2, '0');
      parts.push(text(center, middle + 6, number, zone));
      parts.push(text(center, SHEET.height - middle + 6, number, zone));
    }
  }

  for (let row = 0; row <= rows; row += 1) {
    const y = inner + row * rowHeight;
    parts.push(line(outer, y, inner, y, { stroke: COLOR.inkFaint }));
    parts.push(line(SHEET.width - inner, y, SHEET.width - outer, y, { stroke: COLOR.inkFaint }));
    if (row < rows) {
      const center = y + rowHeight / 2 + 6;
      const letter = String.fromCharCode('A'.charCodeAt(0) + row);
      parts.push(text(middle, center, letter, zone));
      parts.push(text(SHEET.width - middle, center, letter, zone));
    }
  }
  return parts.join('\n');
}

/** The running head of the plate. */
function header() {
  const y = 262;
  return [
    label(MARGIN.left, y, 'ON THE FITTING OF PARTS'),
    label(MARGIN.right, y, 'PLATE 01', { anchor: 'end' }),
    line(MARGIN.left, y + 26, MARGIN.right, y + 26, { stroke: COLOR.inkFaint }),
  ].join('\n');
}

// ------------------------------------------------- figure 1: the assembly

const ASSEMBLY = { left: 420, top: 450, block: 270, kerf: UNIT };

/** A dimension line with a tick at each end and its measure set in a gap. */
function dimension(x1, y1, x2, y2, measure) {
  const isHorizontal = y1 === y2;
  const tick = 9;
  const gap = 46;
  const midX = (x1 + x2) / 2;
  const midY = (y1 + y2) / 2;
  const style = { stroke: COLOR.inkSoft };
  const measureStyle = { anchor: 'middle', size: 19, spacing: 2 };
  if (isHorizontal) {
    return [
      line(x1, y1, midX - gap, y1, style),
      line(midX + gap, y1, x2, y1, style),
      line(x1 - tick, y1 + tick, x1 + tick, y1 - tick, style),
      line(x2 - tick, y1 + tick, x2 + tick, y1 - tick, style),
      text(midX, y1 + 7, measure, measureStyle),
    ].join('\n');
  }
  return [
    line(x1, y1, x1, midY - gap, style),
    line(x1, midY + gap, x1, y2, style),
    line(x1 - tick, y1 + tick, x1 + tick, y1 - tick, style),
    line(x1 - tick, y2 + tick, x1 + tick, y2 - tick, style),
    text(x1 + 7, midY, measure, { ...measureStyle, rotate: -90 }),
  ].join('\n');
}

/**
 * A numbered balloon with a leader to the part it names. The leader ends in
 * a dot, which takes the color of the paper on a part that is itself ink.
 */
function balloon(x, y, mark, toX, toY, dotColor = COLOR.ink) {
  const radius = 23;
  const angle = Math.atan2(toY - y, toX - x);
  const fromX = x + Math.cos(angle) * radius;
  const fromY = y + Math.sin(angle) * radius;
  return [
    line(fromX, fromY, toX, toY, { stroke: COLOR.inkSoft }),
    circle(toX, toY, 4, { fill: dotColor }),
    circle(x, y, radius, { stroke: COLOR.ink, strokeWidth: LINE.hair }),
    text(x, y + 7, mark, { anchor: 'middle', size: 20, fill: COLOR.ink }),
  ].join('\n');
}

function assemblyFigure() {
  const { left, top, block, kerf } = ASSEMBLY;
  const step = block + kerf;
  const size = 3 * block + 2 * kerf;
  const right = left + size;
  const bottom = top + size;
  const stemLeft = left + step;
  const stemRight = stemLeft + block;
  const centerX = stemLeft + block / 2;
  const rowCenter = (row) => top + row * step + block / 2;

  /** The five parts of the T: a row of three, and two below the middle one. */
  const parts = [
    { column: 0, row: 0, kind: 'diagonal' },
    { column: 1, row: 0, kind: 'brass' },
    { column: 2, row: 0, kind: 'dots' },
    { column: 1, row: 1, kind: 'ink' },
    { column: 1, row: 2, kind: 'horizontal' },
  ];

  const drawing = parts.map(({ column, row, kind }) =>
    rect(left + column * step, top + row * step, block, block, {
      fill: partFill(kind),
      stroke: kind === 'brass' ? COLOR.brass : COLOR.ink,
      strokeWidth: LINE.heavy,
    }),
  );

  // The keystone carries the one register mark of the sheet.
  const keyY = rowCenter(0);
  const mark = { stroke: COLOR.paper, width: LINE.medium };
  drawing.push(
    circle(centerX, keyY, 30, { stroke: COLOR.paper, strokeWidth: LINE.medium }),
    line(centerX - 46, keyY, centerX + 46, keyY, mark),
    line(centerX, keyY - 46, centerX, keyY + 46, mark),
  );

  // The centre line, drawn the way a draftsman draws it: long dash, short dash.
  const centerLine = { stroke: COLOR.inkSoft, dash: '34 9 5 9' };
  drawing.push(
    line(centerX, top - 62, centerX, top - 10, centerLine),
    line(centerX, bottom + 10, centerX, bottom + 40, centerLine),
  );

  // Dimensions: the whole is three units wide and three units high, and
  // every part is one unit.
  const overall = 92;
  const single = 68;
  const extension = { stroke: COLOR.inkFaint };
  drawing.push(
    line(left, top - 14, left, top - overall - 16, extension),
    line(right, top - 14, right, top - overall - 16, extension),
    dimension(left, top - overall, right, top - overall, '3 U'),
    line(left - 14, top, left - overall - 16, top, extension),
    line(stemLeft - 14, bottom, left - overall - 16, bottom, extension),
    dimension(left - overall, top, left - overall, bottom, '3 U'),
    line(stemLeft, bottom + 14, stemLeft, bottom + single + 16, extension),
    line(stemRight, bottom + 14, stemRight, bottom + single + 16, extension),
    dimension(stemLeft, bottom + single, stemRight, bottom + single, '1 U'),
  );

  // Balloons name the parts, each with a leader that crosses nothing. The
  // keystone needs none: it is the only brass on the sheet.
  const balloonX = right + 150;
  const inset = 30;
  drawing.push(
    balloon(left + block / 2, top + block + 150, '1', left + block / 2, top + block - inset),
    balloon(balloonX, rowCenter(0), '2', right - inset, rowCenter(0)),
    balloon(balloonX, rowCenter(1), '3', stemRight - inset, rowCenter(1), COLOR.paper),
    balloon(balloonX, rowCenter(2), '4', stemRight - inset, rowCenter(2)),
  );

  return drawing.join('\n');
}

// ------------------------------------------- figure 2: the order of work

/** The scale of the figure is 52 units long and ends on the right margin. */
const SEQUENCE_UNITS = 52;
const SEQUENCE = {
  left: MARGIN.right - SEQUENCE_UNITS * UNIT,
  top: ASSEMBLY.top,
  rowPitch: 62,
  barHeight: 28,
};

/**
 * The work, as bars on a scale. Each bar begins where the bars it waits for
 * end.
 */
const WORK = [
  { from: 0, to: 7, kind: 'diagonal' },
  { from: 7, to: 15, kind: 'diagonal' },
  { from: 7, to: 12, kind: 'dots' },
  { from: 15, to: 24, kind: 'diagonal' },
  { from: 15, to: 22, kind: 'dots' },
  { from: 15, to: 19, kind: 'horizontal' },
  { from: 22, to: 31, kind: 'dots' },
  { from: 24, to: 30, kind: 'diagonal' },
  { from: 31, to: 40, kind: 'horizontal' },
  { from: 31, to: 37, kind: 'dots' },
  { from: 37, to: 42, kind: 'diagonal' },
  { from: 40, to: 46, kind: 'horizontal' },
  { from: 46, to: 52, kind: 'ink' },
];

/** The moments at which work is handed on: [unit, first row, last row]. */
const HANDOVERS = [
  [7, 0, 2],
  [15, 1, 5],
  [22, 4, 6],
  [24, 3, 7],
  [31, 6, 9],
  [37, 9, 10],
  [40, 8, 11],
  [46, 11, 12],
];

/** The handovers at which the decision is the manager's. */
const DECISIONS = [15, 31, 46];

function sequenceFigure() {
  const { left, top, rowPitch, barHeight } = SEQUENCE;
  const right = left + SEQUENCE_UNITS * UNIT;
  const firstRow = top + 78;
  const rowTop = (row) => firstRow + row * rowPitch;
  const drawing = [];

  // The scale: a tick for every unit, a longer one for every fifth, and a
  // numeral for every tenth.
  drawing.push(line(left, top, right, top, { width: LINE.medium }));
  for (let unit = 0; unit <= SEQUENCE_UNITS; unit += 1) {
    const x = left + unit * UNIT;
    const isTenth = unit % 10 === 0;
    const length = isTenth ? 26 : unit % 5 === 0 ? 17 : 9;
    drawing.push(line(x, top, x, top + length));
    if (isTenth) {
      const numeral = String(unit).padStart(2, '0');
      drawing.push(text(x, top - 16, numeral, { anchor: 'middle', size: 18 }));
    }
  }

  // Handovers: a dotted line from the bar that ends to the last bar that waits.
  for (const [unit, fromRow, toRow] of HANDOVERS) {
    const x = left + unit * UNIT;
    drawing.push(
      line(x, rowTop(fromRow) + barHeight, x, rowTop(toRow), {
        stroke: COLOR.inkSoft,
        dash: '3 6',
      }),
    );
  }

  for (const [row, bar] of WORK.entries()) {
    drawing.push(
      rect(left + bar.from * UNIT, rowTop(row), (bar.to - bar.from) * UNIT, barHeight, {
        fill: partFill(bar.kind),
        stroke: COLOR.ink,
        strokeWidth: LINE.medium,
      }),
    );
  }

  // Decisions sit just under the scale, in brass.
  for (const unit of DECISIONS) {
    const x = left + unit * UNIT;
    const y = top + 50;
    const half = 13;
    drawing.push(
      `<path d="M ${x} ${y - half} L ${x + half} ${y} L ${x} ${y + half} L ${x - half} ${y} Z" fill="${COLOR.brass}"/>`,
    );
  }

  return drawing.join('\n');
}

// ------------------------------------------------------ captions and legend

function captions() {
  const y = 1462;
  const figure = (x, number, title) =>
    [
      label(x, y, `FIG. ${number}`),
      text(x, y + 62, title, { font: FONT.serif, style: 'italic', size: 50, fill: COLOR.ink }),
    ].join('\n');
  return [
    figure(ASSEMBLY.left, 1, 'An assembly of five parts'),
    figure(SEQUENCE.left, 2, 'The order of the work'),
  ].join('\n');
}

const TITLE_BLOCK = { bottom: 1760, height: 168 };

/** The legend: what each hatch, the ink and the brass stand for. */
function legend() {
  const entries = [
    { mark: '1', kind: 'diagonal', name: 'HARDWARE' },
    { mark: '2', kind: 'dots', name: 'FIRMWARE' },
    { mark: '3', kind: 'ink', name: 'ADVISOR' },
    { mark: '4', kind: 'horizontal', name: 'ENCLOSURE' },
    { mark: 'K', kind: 'brass', name: 'MANAGER' },
  ];
  const swatch = 44;
  const pitch = 214;
  // The legend stands on the same centre line as the title block.
  const top = TITLE_BLOCK.bottom - TITLE_BLOCK.height / 2 - swatch / 2;

  return entries
    .map((entry, index) => {
      const x = ASSEMBLY.left + index * pitch;
      return [
        rect(x, top, swatch, swatch, {
          fill: partFill(entry.kind),
          stroke: entry.kind === 'brass' ? COLOR.brass : COLOR.ink,
          strokeWidth: LINE.medium,
        }),
        text(x + swatch + 18, top + 17, entry.mark, { size: 19, fill: COLOR.ink }),
        label(x + swatch + 18, top + 42, entry.name, { size: 16, spacing: 2.6 }),
      ].join('\n');
    })
    .join('\n');
}

/** The title block in the corner of the sheet, as wide as the figure above it. */
function titleBlock() {
  const { bottom, height } = TITLE_BLOCK;
  const left = SEQUENCE.left;
  const right = MARGIN.right;
  const top = bottom - height;
  const middle = top + height / 2;
  const cellWidth = 174;
  const secondDivider = right - cellWidth;
  const firstDivider = secondDivider - cellWidth;

  const cell = (x, y, name, value) =>
    [
      label(x + 22, y + 30, name, { size: 14, spacing: 2.6 }),
      text(x + 22, y + 64, value, { size: 24, fill: COLOR.ink, spacing: 1 }),
    ].join('\n');

  return [
    rect(left, top, right - left, height, { stroke: COLOR.ink, strokeWidth: LINE.medium }),
    line(left, middle, right, middle),
    line(firstDivider, top, firstDivider, bottom),
    line(secondDivider, top, secondDivider, bottom),
    text(left + 28, top + 56, 'TEAMWRIGHT', {
      font: FONT.sans,
      weight: 700,
      size: 40,
      fill: COLOR.ink,
      spacing: 9,
    }),
    text(left + 28, middle + 54, 'The right team, built for the job.', {
      font: FONT.serif,
      style: 'italic',
      size: 34,
      fill: COLOR.ink,
    }),
    cell(firstDivider, top, 'PLATE', '01'),
    cell(secondDivider, top, 'SHEET', '1 / 1'),
    cell(firstDivider, middle, 'SCALE', '1 : 1'),
    cell(secondDivider, middle, 'YEAR', '2026'),
  ].join('\n');
}

// ------------------------------------------------------------------ output

const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${SHEET.width}" height="${SHEET.height}" viewBox="0 0 ${SHEET.width} ${SHEET.height}">
  <defs>${hatchDefinitions()}
  </defs>
  ${rect(0, 0, SHEET.width, SHEET.height, { fill: COLOR.paper })}
  ${sheetBorder()}
  ${header()}
  ${assemblyFigure()}
  ${sequenceFigure()}
  ${captions()}
  ${legend()}
  ${titleBlock()}
</svg>
`;

const fontFiles = readdirSync(fontsDirectory)
  .filter((name) => /^(InstrumentSans|InstrumentSerif|IBMPlexMono).*\.ttf$/.test(name))
  .map((name) => join(fontsDirectory, name));

const image = new Resvg(svg, {
  font: { fontFiles, loadSystemFonts: false, defaultFontFamily: FONT.sans },
});

writeFileSync(join(outputDirectory, 'key-visual.svg'), svg);
writeFileSync(join(outputDirectory, 'key-visual.png'), image.render().asPng());
console.log(`wrote key-visual.svg and key-visual.png to ${outputDirectory}`);
