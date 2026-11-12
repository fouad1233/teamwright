import { OUTFITS, type Look } from './characterModel';
import {
  BODIES,
  CHAIR,
  CUP_TO_MOUTH,
  DESK_ROW,
  DESK_TOP,
  FIGURE_HEIGHT,
  FIGURE_WIDTH,
  HAIR,
  HEADS,
  HEADWEAR,
  PROPS,
  RAISED_ARM,
  RAISED_LEFT_ARM,
  SCENE_WIDTH,
  type BodyName,
  type Facing,
  type Part,
} from './sprites';

/*
 * Puts the drawings of sprites.ts together into the pictures that are
 * painted: a figure in a pose, a seated figure with its bench, the outline
 * round them. Everything here is pure: grids of letters in, grids out.
 */

/** A picture as rows of letters of the same length. A dot is empty. */
export interface Sprite {
  width: number;
  height: number;
  rows: readonly string[];
}

/** The letter that marks the outline that spriteModel adds. */
export const OUTLINE = 'K';
const EMPTY = '.';

/** An empty picture. */
function blank(width: number, height: number): string[][] {
  return Array.from({ length: height }, () => Array.from({ length: width }, () => EMPTY));
}

/** Draws a part over a picture. What lies outside the picture is left out. */
function stamp(canvas: string[][], part: Part, dx = 0, dy = 0): void {
  part.rows.forEach((row, rowIndex) => {
    const y = part.y + rowIndex + dy;
    const line = canvas[y];
    if (!line) {
      return;
    }
    for (let column = 0; column < row.length; column += 1) {
      const letter = row[column];
      const x = part.x + column + dx;
      if (letter !== undefined && letter !== EMPTY && x >= 0 && x < line.length) {
        line[x] = letter;
      }
    }
  });
}

function toSprite(canvas: string[][]): Sprite {
  return {
    width: canvas[0]?.length ?? 0,
    height: canvas.length,
    rows: canvas.map((line) => line.join('')),
  };
}

/**
 * Draws the outline round everything that is not empty: every empty pixel
 * that touches a drawn one, side by side or above and below, becomes line.
 * Pixels marked to stay open, such as light or steam, get no line.
 */
export function outlined(sprite: Sprite, open: ReadonlySet<string> = OPEN_LETTERS): Sprite {
  const at = (x: number, y: number) => sprite.rows[y]?.[x] ?? EMPTY;
  const closed = (x: number, y: number) => {
    const letter = at(x, y);
    return letter !== EMPTY && letter !== OUTLINE && !open.has(letter);
  };
  const rows = sprite.rows.map((row, y) =>
    Array.from(row, (letter, x) =>
      letter === EMPTY &&
      (closed(x - 1, y) || closed(x + 1, y) || closed(x, y - 1) || closed(x, y + 1))
        ? OUTLINE
        : letter,
    ).join(''),
  );
  return { ...sprite, rows };
}

/** Letters that are drawn without a line round them: light, steam, smoke. */
export const OPEN_LETTERS: ReadonlySet<string> = new Set(['y', 'Y', 'f']);

/** The picture turned left for right, for a figure that faces left. */
export function mirrored(sprite: Sprite): Sprite {
  return { ...sprite, rows: sprite.rows.map((row) => Array.from(row).reverse().join('')) };
}

/** A pose of a whole figure, as it is drawn standing, walking or seated. */
export type Pose =
  | 'stand'
  | 'standDown'
  | 'standBack'
  | 'standSide'
  | 'walkFront'
  | 'walkBack'
  | 'walkSide'
  | 'seatRest'
  | 'seatGlance'
  | 'seatWork'
  | 'seatWait'
  | 'seatSip'
  | 'seatStretch'
  | 'seatPage';

/** How many frames each pose has. A pose of one frame stands still. */
export const FRAMES: Readonly<Record<Pose, number>> = {
  stand: 1,
  standDown: 1,
  standBack: 1,
  standSide: 1,
  walkFront: 4,
  walkBack: 4,
  walkSide: 4,
  seatRest: 1,
  seatGlance: 1,
  seatWork: 4,
  seatWait: 2,
  seatSip: 2,
  seatStretch: 2,
  seatPage: 2,
};

/** Whether a pose is seated behind a bench. */
export function isSeated(pose: Pose): boolean {
  return pose.startsWith('seat');
}

interface Arrangement {
  body: BodyName;
  head: keyof typeof HEADS;
  facing: Facing;
  /** How far the head alone is lowered: a nod over the work, the dip of a step. */
  nod: number;
  /** Arms and what they hold, drawn last, over the head. */
  over: readonly Part[];
}

/** A part moved down by some pixels. */
function lowered(part: Part, by: number): Part {
  return { ...part, y: part.y + by };
}

/** The parts of a pose at one of its frames. */
function arrangementOf(pose: Pose, frame: number): Arrangement {
  const still = { nod: 0, over: [] };
  const step = frame % 4;
  // A walk is a step, a pass, the other step, a pass. The head dips a
  // pixel on each step and rises as it passes over the foot it stands on.
  const passing = step % 2 === 1;
  switch (pose) {
    case 'stand':
      return { ...still, body: 'standFront', head: 'front', facing: 'front' };
    case 'standDown':
      // Standing, looking down at something: the plan on the table.
      return { ...still, body: 'standFront', head: 'down', facing: 'front' };
    case 'standBack':
      return { ...still, body: 'standFront', head: 'back', facing: 'back' };
    case 'standSide':
      return { ...still, body: 'standSide', head: 'side', facing: 'side' };
    case 'walkFront':
    case 'walkBack': {
      const body: BodyName = passing
        ? 'standFront'
        : step === 0
          ? 'walkFrontLeft'
          : 'walkFrontRight';
      return {
        ...still,
        body,
        head: pose === 'walkFront' ? 'front' : 'back',
        facing: pose === 'walkFront' ? 'front' : 'back',
        nod: passing ? 0 : 1,
      };
    }
    case 'walkSide': {
      const body: BodyName = passing ? 'standSide' : step === 0 ? 'walkSideNear' : 'walkSideFar';
      return { ...still, body, head: 'side', facing: 'side', nod: passing ? 0 : 1 };
    }
    case 'seatRest':
      return { ...still, body: 'seatRest', head: 'front', facing: 'front' };
    case 'seatGlance':
      return { ...still, body: 'seatRest', head: 'glance', facing: 'front' };
    case 'seatWork': {
      // Both hands on the work, the right lifts, both again, the left lifts.
      const bodies: readonly BodyName[] = ['seatWork', 'seatWorkRight', 'seatWork', 'seatWorkLeft'];
      return {
        ...still,
        body: bodies[step] ?? 'seatWork',
        head: 'down',
        facing: 'front',
        // One small nod in every round of the hands.
        nod: step === 2 ? 1 : 0,
      };
    }
    case 'seatWait':
      // The raised hand dips a pixel every other beat, a small wave.
      return {
        ...still,
        body: 'seatWait',
        head: 'front',
        facing: 'front',
        over: [lowered(RAISED_ARM, frame % 2)],
      };
    case 'seatSip':
      // The cup at the mouth, then a pixel lower as it is put down a little.
      return {
        ...still,
        body: 'seatSip',
        head: 'down',
        facing: 'front',
        over: [lowered(CUP_TO_MOUTH, frame % 2)],
      };
    case 'seatStretch':
      // Both arms up, then a pixel lower: a slow stretch.
      return {
        ...still,
        body: 'seatStretch',
        head: 'glance',
        facing: 'front',
        over: [lowered(RAISED_ARM, frame % 2), lowered(RAISED_LEFT_ARM, frame % 2)],
      };
    case 'seatPage':
      // A hand turns a page on the bench, first the right, then the left.
      return {
        ...still,
        body: frame % 2 === 0 ? 'seatWorkRight' : 'seatWorkLeft',
        head: 'down',
        facing: 'front',
      };
  }
}

/** The figure of a character in a pose, without its outline. */
export function figure(look: Look, pose: Pose, frame: number): Sprite {
  const { body, head, facing, nod, over } = arrangementOf(pose, frame);
  const canvas = blank(FIGURE_WIDTH, FIGURE_HEIGHT);
  const bodyPart = BODIES[body];
  // From behind there is no strip down the front and no apron: the back is
  // all one cloth.
  stamp(
    canvas,
    facing === 'back'
      ? {
          ...bodyPart,
          rows: bodyPart.rows.map((row) => row.replace(/[nb]/g, 'c').replace(/B/g, 'd')),
        }
      : bodyPart,
  );
  stamp(canvas, HEADS[head], 0, nod);
  stamp(canvas, HAIR[look.hairStyle][facing], 0, nod);
  const headwear = OUTFITS[look.role].headwear;
  if (headwear) {
    stamp(canvas, HEADWEAR[headwear][facing], 0, nod);
  }
  for (const part of over) {
    stamp(canvas, part);
  }
  return toSprite(canvas);
}

/** One outlined picture laid over another: where the upper one is drawn, it wins. */
function laid(under: Sprite, over: Sprite): Sprite {
  return {
    ...under,
    rows: under.rows.map((row, y) =>
      Array.from(row, (letter, x) => {
        const upper = over.rows[y]?.[x] ?? EMPTY;
        return upper === EMPTY ? letter : upper;
      }).join(''),
    ),
  };
}

/**
 * A seated character at its bench: the figure cut where the bench begins,
 * the top of its desk, and on it what its role works with. While it
 * works, the moving parts of its things are drawn over them. Without a
 * pose, the bench of somebody who is away: the empty chair and the things.
 */
export function benchScene(
  look: Look,
  pose: Pose | null,
  frame: number,
  isWorking: boolean,
  hasDesk = true,
): Sprite {
  const canvas = blank(SCENE_WIDTH, FIGURE_HEIGHT);
  stamp(canvas, CHAIR);
  if (pose !== null) {
    const person = figure(look, pose, frame);
    // The body below the edge of the bench is hidden by the bench; only the
    // hands, which rest on it, are drawn there.
    const visible = person.rows.map((row, y) => (y < DESK_ROW ? row : row.replace(/[^s]/g, EMPTY)));
    stamp(canvas, { x: 0, y: 0, rows: visible });
  }
  const prop = PROPS[OUTFITS[look.role].prop];
  const things = blank(SCENE_WIDTH, FIGURE_HEIGHT);
  for (const part of prop.still) {
    stamp(things, part);
  }
  if (isWorking && 'working' in prop) {
    const moving = prop.working[frame % prop.working.length] ?? [];
    for (const part of moving) {
      stamp(things, part);
    }
  }
  // The desk, the figure and the things are outlined each on its own, so
  // that a line runs between a thing and what stands behind it. The hands
  // rest on the desk, and the things stand on it.
  const person = outlined(toSprite(canvas));
  const desk = blank(SCENE_WIDTH, FIGURE_HEIGHT);
  if (hasDesk) {
    stamp(desk, DESK_TOP);
  }
  return laid(laid(outlined(toSprite(desk)), person), outlined(toSprite(things)));
}

/** A figure in a pose, outlined and turned the way it faces. */
export function standingFigure(look: Look, pose: Pose, frame: number, facesLeft: boolean): Sprite {
  const drawn = outlined(figure(look, pose, frame));
  return facesLeft ? mirrored(drawn) : drawn;
}

/**
 * The head and shoulders of a character, for its portrait: the top of the
 * figure seen from the front, cut square.
 */
export function portrait(look: Look): Sprite {
  const drawn = outlined(figure(look, 'stand', 0));
  const size = FIGURE_WIDTH;
  return { width: size, height: size, rows: drawn.rows.slice(0, size) };
}
