import { describe, expect, it } from 'vitest';

import { DESK_ROW, FEET_ROW, SCENE_WIDTH } from './characters/sprites';
import {
  activityOf,
  advanceRoom,
  chance,
  commonsOf,
  figuresOf,
  gestureAt,
  mostAway,
  planRoom,
  replanRoom,
  speedIn,
  pointAlong,
  SCENE_OFFSET,
  seatAt,
  startRoom,
  walkTo,
  type Person,
  type RoomMeasures,
  type RoomState,
} from './roomModel';

const UNIT = 2;
/** A bench as wide as the token says, and the aisle as deep. */
const BENCH = 248;
const AISLE = 80;

/** A room of two rows of three benches, as the component would measure it. */
function measures(): RoomMeasures {
  const rowTop = (row: number) => AISLE + row * (200 + AISLE);
  const benches = [0, 1].flatMap((row) =>
    [0, 1, 2].map((column) => ({
      key: `member-${row.toString()}-${column.toString()}`,
      box: { left: 40 + column * (BENCH + 16), top: rowTop(row), width: BENCH, height: 200 },
    })),
  );
  return {
    unit: UNIT,
    benches,
    wallRight: 40 + 3 * BENCH + 2 * 16 + 48 - 5,
    commons: { left: 40, top: rowTop(2), width: 3 * BENCH + 32, height: 112 },
  };
}

const people = (activity: Person['activity'] = 'idle'): Person[] =>
  measures().benches.map((bench, index) => ({ key: bench.key, activity, seed: index }));

/** Runs the room for a number of seconds, in the steps the loop takes. */
function run(
  state: RoomState,
  who: readonly Person[],
  seconds: number,
  random: () => number,
  check?: (state: RoomState) => void,
): RoomState {
  const plan = planRoom(measures());
  let current = state;
  for (let step = 0; step < seconds * 15; step += 1) {
    current = advanceRoom(current, who, plan, 1 / 15, random);
    check?.(current);
  }
  return current;
}

describe('activityOf', () => {
  it('follows what the bench says', () => {
    expect(activityOf('working')).toBe('working');
    expect(activityOf('waiting')).toBe('waiting');
    expect(activityOf('free')).toBe('idle');
    expect(activityOf('person')).toBe('idle');
    expect(activityOf('manager')).toBe('idle');
  });
});

describe('seatAt', () => {
  const bench = { key: 'b', box: { left: 100, top: 300, width: BENCH, height: 200 } };
  const seat = seatAt(bench, UNIT);

  it('seats the figure right of the lamp in the middle of the bench', () => {
    expect(seat.scene.x).toBe(100 + BENCH / 2 + SCENE_OFFSET);
    expect(seat.scene.x + SCENE_WIDTH * UNIT).toBeLessThanOrEqual(100 + BENCH);
  });

  it('cuts the figure at the far edge of the bench', () => {
    expect(seat.scene.y + DESK_ROW * UNIT).toBe(300);
  });

  it('keeps the aisle clear of the seated heads and of the row in front', () => {
    // Somebody walking the aisle stands wholly above the seated row...
    expect(seat.laneY).toBeLessThanOrEqual(seat.scene.y);
    // ...and below the benches of the row before, an aisle away.
    expect(seat.laneY - FEET_ROW * UNIT).toBeGreaterThanOrEqual(300 - AISLE);
  });
});

describe('commonsOf', () => {
  it('puts every shared place apart from the others, so two people never overlap', () => {
    const { spots, laneY } = commonsOf({ left: 0, top: 0, width: 760, height: 112 }, UNIT);
    expect(spots.map((spot) => spot.stance).sort()).toEqual(['coffee', 'table', 'table', 'window']);
    const xs = spots.map((spot) => spot.x).sort((a, b) => a - b);
    for (let index = 1; index < xs.length; index += 1) {
      expect((xs[index] ?? 0) - (xs[index - 1] ?? 0)).toBeGreaterThanOrEqual(16 * UNIT);
    }
    // Somebody at a place stands below the lane, so that a walker passes behind.
    for (const spot of spots) {
      expect(spot.feetY - FEET_ROW * UNIT).toBeGreaterThanOrEqual(laneY);
    }
  });

  it('keeps the furniture inside the band', () => {
    const band = { left: 0, top: 0, width: 760, height: 112 };
    const { fixtures } = commonsOf(band, UNIT);
    for (const fixture of fixtures) {
      expect(fixture.left).toBeGreaterThanOrEqual(band.left);
      expect(fixture.top).toBeGreaterThanOrEqual(band.top);
    }
  });

  it('leaves out what does not fit in a narrow room, the table last', () => {
    const ids = (width: number) =>
      commonsOf({ left: 0, top: 0, width, height: 112 }, UNIT).spots.map((spot) => spot.id);
    expect(ids(150)).toEqual(['window', 'table-left', 'table-right']);
    expect(ids(60)).toEqual(['window']);
  });
});

describe('walkTo', () => {
  const plan = planRoom(measures());
  const seat = plan.seats[4];
  const spot = plan.spots[0];

  it('goes from the bench to the place in right angles, along the aisles', () => {
    if (!seat || !spot) {
      throw new Error('the room has no seat or no place');
    }
    const walk = walkTo(seat, spot, plan);
    expect(walk.path[0]).toEqual({ x: seat.x, y: seat.deskY - 2 });
    expect(walk.path[walk.path.length - 1]).toEqual({ x: spot.x, y: spot.feetY });
    for (let index = 1; index < walk.path.length; index += 1) {
      const from = walk.path[index - 1];
      const to = walk.path[index];
      expect(from?.x === to?.x || from?.y === to?.y).toBe(true);
    }
    // Down the side of the room, right of every bench.
    expect(walk.path.some((point) => point.x === plan.aisleX)).toBe(true);
    const rightmost = Math.max(...measures().benches.map(({ box }) => box.left + box.width));
    expect(plan.aisleX - 8 * UNIT).toBeGreaterThanOrEqual(rightmost);
  });
});

describe('pointAlong', () => {
  const path = [
    { x: 0, y: 100 },
    { x: 0, y: 0 },
    { x: 50, y: 0 },
  ];

  it('finds the point a distance along, and the way the figure faces', () => {
    expect(pointAlong(path, 40)).toEqual({ x: 0, y: 60, heading: 'up' });
    expect(pointAlong(path, 120)).toEqual({ x: 20, y: 0, heading: 'right' });
    expect(pointAlong(path, 500)).toEqual({ x: 50, y: 0, heading: 'right' });
  });
});

describe('advanceRoom', () => {
  it('sends a free person to a shared place now and then, and brings them back', () => {
    const random = chance(1);
    const who = people();
    const kinds = new Set<string>();
    run(startRoom(who, random), who, 400, random, (state) => {
      for (const phase of Object.values(state.phases)) {
        kinds.add(phase.kind);
      }
    });
    expect([...kinds].sort()).toEqual(['back', 'out', 'seated', 'visiting']);
  });

  it('lets one person walk at a time and never two at one place', () => {
    const random = chance(2);
    const who = people();
    run(startRoom(who, random), who, 600, random, (state) => {
      const phases = Object.values(state.phases);
      const walking = phases.filter((phase) => phase.kind === 'out' || phase.kind === 'back');
      expect(walking.length).toBeLessThanOrEqual(1);
      const spots = phases.flatMap((phase) => (phase.kind === 'seated' ? [] : [phase.walk.spot]));
      expect(new Set(spots).size).toBe(spots.length);
    });
  });

  it('keeps somebody at work or waiting at the bench', () => {
    const random = chance(3);
    const who = people('working');
    run(startRoom(who, random), who, 300, random, (state) => {
      expect(Object.values(state.phases).every((phase) => phase.kind === 'seated')).toBe(true);
    });
  });

  it('turns somebody back at once when their work starts', () => {
    const random = chance(4);
    const idle = people();
    let state = startRoom(idle, random);
    // Walk until somebody is on the way out.
    for (let step = 0; step < 15 * 120; step += 1) {
      state = run(state, idle, 1 / 15, random);
      if (Object.values(state.phases).some((phase) => phase.kind === 'out')) {
        break;
      }
    }
    const [key] = Object.entries(state.phases).find(([, phase]) => phase.kind === 'out') ?? [];
    expect(key).toBeDefined();
    const working = idle.map((person) =>
      person.key === key ? { ...person, activity: 'working' as const } : person,
    );
    state = run(state, working, 1 / 15, random);
    expect(state.phases[key ?? '']?.kind).toBe('back');
  });
});

describe('figuresOf', () => {
  it('seats everybody in the still pose of their bench when motion is turned down', () => {
    const random = chance(5);
    const who = people();
    const state = run(startRoom(who, random), who, 200, random);
    const figures = figuresOf(state, who, planRoom(measures()), true);
    expect(figures).toHaveLength(who.length);
    for (const { figure } of figures) {
      expect(figure.kind).toBe('seated');
      if (figure.kind === 'seated') {
        expect(figure.frame).toBe(0);
        expect(figure.gesture).toBeNull();
      }
    }
  });
});

/** A room laid out as the office draws eight members at three times in a wide window. */
function wideRoom(): RoomMeasures {
  const aisle = 120;
  const row = (top: number, columns: number[]) =>
    columns.map((column) => ({ left: 40 + column * 264, top, width: 248, height: 205 }));
  const boxes = [
    ...row(aisle, [0.5, 1.5]),
    ...row(aisle + 165 + 16 + aisle, [0, 1, 2]),
    ...row(aisle + 165 + 16 + aisle + 205 + aisle, [0, 1, 2]),
    ...row(aisle + 165 + 16 + aisle + 2 * (205 + aisle), [0.5, 1.5]),
  ];
  const bottom = Math.max(...boxes.map((box) => box.top + box.height));
  return {
    unit: 3,
    benches: boxes.map((box, index) => ({ key: `place-${index.toString()}`, box })),
    wallRight: 40 + 3 * 248 + 2 * 16 + 48 - 5,
    commons: { left: 40, top: bottom + 24, width: 3 * 248 + 32, height: 160 },
  };
}

describe('the pace of the room', () => {
  it('takes most people to their nearest shared place in under twenty seconds', () => {
    const plan = planRoom(wideRoom());
    const seconds = plan.seats.map((seat) =>
      Math.min(...plan.spots.map((spot) => walkTo(seat, spot, plan).length / speedIn(plan))),
    );
    const quick = seconds.filter((time) => time < 20).length;
    expect(quick / seconds.length).toBeGreaterThanOrEqual(0.6);
    expect(Math.max(...seconds)).toBeLessThan(30);
  });

  it('lets two be away in a team of six or more, and one in a smaller team', () => {
    expect(mostAway(5)).toBe(1);
    expect(mostAway(6)).toBe(2);
    const random = chance(6);
    for (const count of [4, 8]) {
      const who = people().slice(0, count);
      let most = 0;
      run(startRoom(who, random), who, 900, random, (state) => {
        const away = Object.values(state.phases).filter((phase) => phase.kind !== 'seated');
        most = Math.max(most, away.length);
      });
      expect(most).toBe(mostAway(count));
    }
  });
});

describe('gestureAt', () => {
  it('has somebody idle do a small thing now and then, and mostly nothing', () => {
    const moments = Array.from({ length: 900 }, (_, index) => gestureAt(42, index / 10));
    const busy = moments.filter((moment) => moment.gesture !== null).length;
    expect(busy).toBeGreaterThan(0);
    expect(busy / moments.length).toBeLessThan(0.4);
    const kinds = new Set(moments.map((moment) => moment.gesture));
    expect(kinds).toEqual(new Set([null, 'glance', 'sip', 'page', 'stretch']));
  });
});

describe('replanRoom', () => {
  it('keeps somebody away on their way, as far along it as before', () => {
    const random = chance(7);
    const who = people();
    const before = planRoom(measures());
    let state = startRoom(who, random);
    for (let step = 0; step < 15 * 200; step += 1) {
      state = advanceRoom(state, who, before, 1 / 15, random);
      if (Object.values(state.phases).some((phase) => phase.kind === 'out')) {
        break;
      }
    }
    const wider = measures();
    const after = planRoom({
      ...wider,
      benches: wider.benches.map((bench) => ({
        ...bench,
        box: { ...bench.box, left: bench.box.left + 30 },
      })),
    });
    const moved = replanRoom(state, before, after);
    const [key, phase] =
      Object.entries(state.phases).find(([, entry]) => entry.kind === 'out') ?? [];
    const carried = moved.phases[key ?? ''];
    expect(phase?.kind).toBe('out');
    expect(carried?.kind).toBe('out');
    if (phase?.kind === 'out' && carried?.kind === 'out') {
      const share = ((moved.time - phase.startedAt) * speedIn(before)) / phase.walk.length;
      const carriedShare =
        ((moved.time - carried.startedAt) * speedIn(after)) / carried.walk.length;
      expect(carriedShare).toBeCloseTo(share, 5);
      expect(carried.walk.path[0]?.x).toBe((phase.walk.path[0]?.x ?? 0) + 30);
    }
  });
});
