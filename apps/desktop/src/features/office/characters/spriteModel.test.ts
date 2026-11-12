import { describe, expect, it } from 'vitest';

import { lookOf, OUTFITS, type Look, type Role } from './characterModel';
import { colorsOf } from './paint';
import type { ThemeColors } from './palette';
import {
  benchScene,
  FRAMES,
  mirrored,
  outlined,
  OUTLINE,
  portrait,
  standingFigure,
  type Pose,
  type Sprite,
} from './spriteModel';
import {
  BODIES,
  DESK_ROW,
  FIGURE_HEIGHT,
  FIGURE_WIDTH,
  FURNITURE,
  HAIR,
  HEADS,
  HEADWEAR,
  PROPS,
  SCENE_WIDTH,
  type Part,
  type Prop,
} from './sprites';

const ROLES = Object.keys(OUTFITS) as Role[];
const POSES = Object.keys(FRAMES) as Pose[];
const STANDING_POSES = POSES.filter((pose) => !pose.startsWith('seat'));
const SEATED_POSES = POSES.filter((pose) => pose.startsWith('seat'));

const THEME: ThemeColors = {
  theme: 'night',
  brass: '#c9a96e',
  pigments: {
    gray: '#8a857a',
    blue: '#5b8fd9',
    purple: '#9a7bd6',
    amber: '#e0913a',
    green: '#4fae82',
    red: '#dd5f4a',
    pink: '#d4699a',
    teal: '#3fa8a0',
  },
};

function lookFor(role: Role, hairStyle: Look['hairStyle'] = 'long'): Look {
  return { role, skin: 0, hair: 0, hairStyle, shirt: 'blue', seed: 1 };
}

function allParts(): Part[] {
  return [
    ...Object.values(HEADS),
    ...Object.values(HAIR).flatMap((facings) => Object.values(facings)),
    ...Object.values(BODIES),
    ...Object.values(HEADWEAR).flatMap((facings: Readonly<Record<string, Part>>) =>
      Object.values(facings),
    ),
    ...Object.values(PROPS).flatMap((prop: Prop) => [
      ...prop.still,
      ...(prop.working?.flat() ?? []),
    ]),
    ...Object.values(FURNITURE),
  ];
}

function letters(sprite: Sprite): Set<string> {
  return new Set(sprite.rows.join('').replace(/\./g, ''));
}

describe('the drawings', () => {
  it('have rows of one length each', () => {
    for (const part of allParts()) {
      const widths = new Set(part.rows.map((row) => row.length));
      expect(widths.size).toBeLessThanOrEqual(1);
    }
  });
});

describe('figures', () => {
  it('are all of one size, in every pose and frame of every role', () => {
    for (const role of ROLES) {
      for (const pose of STANDING_POSES) {
        for (let frame = 0; frame < FRAMES[pose]; frame += 1) {
          const figure = standingFigure(lookFor(role), pose, frame, false);
          expect(figure.width).toBe(FIGURE_WIDTH);
          expect(figure.height).toBe(FIGURE_HEIGHT);
          expect(figure.rows.every((row) => row.length === FIGURE_WIDTH)).toBe(true);
        }
      }
    }
  });

  it('leave room for their outline on every side', () => {
    for (const role of ROLES) {
      for (const pose of POSES) {
        for (let frame = 0; frame < FRAMES[pose]; frame += 1) {
          const figure = standingFigure(lookFor(role, 'long'), pose, frame, false);
          expect(figure.rows[0]?.replace(new RegExp(OUTLINE, 'g'), '')).toMatch(/^\.*$/);
          expect(figure.rows[FIGURE_HEIGHT - 1]).toMatch(/^\.*$/);
          for (const row of figure.rows) {
            expect(row[0]).toMatch(/[.K]/);
            expect(row[FIGURE_WIDTH - 1]).toMatch(/[.K]/);
          }
        }
      }
    }
  });

  it('have a color for every letter they use, in both themes', () => {
    for (const role of ROLES) {
      for (const theme of ['night', 'paper'] as const) {
        const colors = colorsOf(lookFor(role), { ...THEME, theme });
        for (const pose of POSES) {
          for (let frame = 0; frame < FRAMES[pose]; frame += 1) {
            const drawn = pose.startsWith('seat')
              ? benchScene(lookFor(role), pose, frame, true)
              : standingFigure(lookFor(role), pose, frame, false);
            for (const letter of letters(drawn)) {
              expect(colors.has(letter), `${role} ${pose} ${letter}`).toBe(true);
            }
          }
        }
      }
    }
  });

  it('look different from the front and from behind', () => {
    const look = lookOf({ kind: 'advisor' });
    expect(standingFigure(look, 'stand', 0, false).rows).not.toEqual(
      standingFigure(look, 'standBack', 0, false).rows,
    );
  });

  it('move their legs as they walk', () => {
    const look = lookFor('software');
    const frames = [0, 1, 2, 3].map((frame) => standingFigure(look, 'walkSide', frame, false).rows);
    expect(frames[0]).not.toEqual(frames[1]);
    expect(frames[0]).not.toEqual(frames[2]);
  });

  it('raise a hand while they wait', () => {
    const look = lookFor('default', 'short');
    const resting = benchScene(look, 'seatRest', 0, false);
    const waiting = benchScene(look, 'seatWait', 0, false);
    // The raised hand stands above the shoulders, beside the head.
    const aboveShoulders = (sprite: Sprite) => sprite.rows.slice(0, 8).join('').split('s').length;
    expect(aboveShoulders(waiting)).toBeGreaterThan(aboveShoulders(resting));
  });
});

describe('outlined', () => {
  it('draws a line round what is drawn, and leaves light open', () => {
    const sprite = outlined({ width: 3, height: 3, rows: ['...', '.s.', '...'] });
    expect(sprite.rows).toEqual(['.K.', 'KsK', '.K.']);
    const light = outlined({ width: 3, height: 3, rows: ['...', '.y.', '...'] });
    expect(light.rows).toEqual(['...', '.y.', '...']);
  });
});

describe('mirrored', () => {
  it('turns a picture left for right', () => {
    expect(mirrored({ width: 3, height: 1, rows: ['ab.'] }).rows).toEqual(['.ba']);
  });
});

describe('benchScene', () => {
  it('hides the body behind the bench and keeps the hands on it', () => {
    for (const role of ROLES) {
      for (const pose of SEATED_POSES) {
        const scene = benchScene(lookFor(role), pose, 0, false);
        expect(scene.width).toBe(SCENE_WIDTH);
        const person = scene.rows.slice(DESK_ROW).map((row) => row.slice(0, 16));
        // Below the edge of the bench only hands, outline and things are drawn.
        expect(person.join('')).not.toMatch(/[cdlLbBnpqk]/);
      }
    }
  });

  it('keeps the chair and the things of somebody who is away', () => {
    const away = benchScene(lookFor('software'), null, 0, false);
    expect(away.rows.join('')).not.toMatch(/[sh]/);
    expect(away.rows.join('')).toMatch(/m/);
  });

  it('moves the things of a role at work, and only then', () => {
    const look = lookFor('hardware');
    const working = [0, 1].map((frame) => benchScene(look, 'seatWork', frame, true).rows);
    expect(working[0]).not.toEqual(working[1]);
    const idle = benchScene(look, 'seatRest', 0, false).rows.join('');
    expect(idle).not.toMatch(/f/);
  });
});

describe('portrait', () => {
  it('is the head and shoulders, square', () => {
    const picture = portrait(lookOf({ kind: 'manager' }));
    expect(picture.width).toBe(picture.height);
    expect(picture.rows.join('')).toMatch(/e/);
  });
});
