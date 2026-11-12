import {
  DESK_ROW,
  FEET_ROW,
  FURNITURE,
  SCENE_WIDTH,
  type FurnitureName,
} from './characters/sprites';
import type { Sitter } from './characters/characterModel';
import type { PlaceFigure, PlaceState } from './officeModel';

/*
 * Where the people of the office are, and what they do (decision 0019).
 * Everybody sits at a bench. A free member now and then gets up, walks the
 * aisles to one of the shared places of the room, stays a while, and walks
 * back. Everything here is pure: the room is measured by the component and
 * handed in, and time and chance are handed in too.
 *
 * Lengths are in pixels of the page. `unit` is how many of them one pixel
 * of a character takes.
 */

/** A point of the room, from its top left corner. */
export interface Point {
  x: number;
  y: number;
}

/** A box of the room. */
export interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** What a person at a bench is doing, for the way it is drawn. */
export type Activity = 'working' | 'waiting' | 'idle';

/** The activity a place's state calls for. */
export function activityOf(state: PlaceState): Activity {
  switch (state) {
    case 'working':
      return 'working';
    case 'waiting':
      return 'waiting';
    case 'free':
    case 'person':
    case 'manager':
      return 'idle';
  }
}

/** Who sits at a place, as the characters know them. */
export function sitterOf(figure: PlaceFigure): Sitter {
  return figure.kind === 'member' ? { kind: 'member', member: figure.member } : figure;
}

/** Somebody in the room. Only an idle person ever leaves the bench. */
export interface Person {
  key: string;
  activity: Activity;
  /** A number of the person's own, so that the room does not move in step. */
  seed: number;
}

/** Where a person sits: the bench seen from above, with the figure behind its far edge. */
export interface Seat {
  key: string;
  /** The middle of the figure. */
  x: number;
  /** The far edge of the bench: the top of its card. */
  deskY: number;
  /** Where the feet are while walking the aisle in front of this row of benches. */
  laneY: number;
  /** Where the seated figure and its things are drawn: the top left corner of the scene. */
  scene: Point;
}

/** The way a figure stands at a shared place. */
export type Stance = 'window' | 'table' | 'coffee';

/** A place in the room where one person may stand. */
export interface Spot {
  id: string;
  stance: Stance;
  /** The middle of the figure, and the line its feet stand on. */
  x: number;
  feetY: number;
}

/** A piece of furniture, by its top left corner. */
export interface Fixture {
  name: FurnitureName;
  left: number;
  top: number;
}

/** The room, worked out from what was measured. */
export interface RoomPlan {
  unit: number;
  seats: readonly Seat[];
  /** The line down the side of the room that joins the aisles of the rows to the shared places. */
  aisleX: number;
  /** Where the feet are while walking along the shared places. */
  commonsLaneY: number;
  spots: readonly Spot[];
  fixtures: readonly Fixture[];
}

/** A bench, as the component measured it. */
export interface MeasuredBench {
  key: string;
  box: Box;
}

/** What the component measures in the room. */
export interface RoomMeasures {
  unit: number;
  /** Every bench, front and floor. */
  benches: readonly MeasuredBench[];
  /** The inside face of the right wall. */
  wallRight: number;
  /** The band at the foot of the room that holds the shared places. */
  commons: Box;
}

/**
 * How far right of the middle of a bench its scene begins, so that the lamp
 * that hangs over the middle stays clear of the figure.
 */
export const SCENE_OFFSET = 14;

/** A little room between things that must not touch. */
const CLEARANCE = 2;

/** The seat at a bench: its figure behind the far edge, to the right of the lamp. */
export function seatAt(bench: MeasuredBench, unit: number): Seat {
  const { box } = bench;
  const sceneLeft = box.left + box.width / 2 + SCENE_OFFSET;
  const sceneTop = box.top - DESK_ROW * unit;
  return {
    key: bench.key,
    x: sceneLeft + 8 * unit,
    deskY: box.top,
    // The walker's feet stand just above the heads of the seated row.
    laneY: sceneTop - CLEARANCE,
    scene: { x: sceneLeft, y: sceneTop },
  };
}

/** Whether the things on a bench fit on it, without running past its right edge. */
export function sceneFits(bench: MeasuredBench, unit: number): boolean {
  const sceneLeft = bench.box.left + bench.box.width / 2 + SCENE_OFFSET;
  return sceneLeft + SCENE_WIDTH * unit <= bench.box.left + bench.box.width;
}

/** The width of a piece of furniture, in pixels of the page. */
function widthOf(name: FurnitureName, unit: number): number {
  return (FURNITURE[name].rows[0]?.length ?? 0) * unit;
}

/** The height of a piece of furniture, in pixels of the page. */
function heightOf(name: FurnitureName, unit: number): number {
  return FURNITURE[name].rows.length * unit;
}

/**
 * The shared places along the band at the foot of the room: a window in
 * the left wall with a plant beside it, the table with the plan, and the
 * coffee corner. People walk along the top of the band and step down to a
 * place; the furniture stands in front of them. What does not fit in a
 * narrow room is left out, from the right.
 */
export function commonsOf(
  commons: Box,
  unit: number,
): {
  laneY: number;
  spots: Spot[];
  fixtures: Fixture[];
} {
  const figureHeight = FEET_ROW * unit;
  const laneY = commons.top + figureHeight + CLEARANCE;
  // A figure at a place stands below the lane, so that somebody walking
  // along the lane passes behind it without touching.
  const feetY = laneY + figureHeight + CLEARANCE;
  const spots: Spot[] = [];
  const fixtures: Fixture[] = [];
  const right = commons.left + commons.width;

  // The window: one place, close to the wall, and a plant beside it.
  const windowX = commons.left + 10 * unit;
  spots.push({ id: 'window', stance: 'window', x: windowX, feetY });
  let cursor = windowX + 8 * unit;
  const plantWidth = widthOf('plant', unit);
  if (cursor + plantWidth <= right) {
    fixtures.push({ name: 'plant', left: cursor, top: feetY + 2 * unit - heightOf('plant', unit) });
    cursor += plantWidth;
  }
  const gap = 8 * unit;
  const tableWidth = widthOf('planTable', unit);
  const counterWidth = widthOf('coffee', unit);
  const room = right - cursor;
  // The table with the plan comes first; the coffee corner only if both fit.
  const hasCoffee = room >= tableWidth + counterWidth + 2 * gap || room < tableWidth + gap;
  const hasTable = room >= tableWidth + gap;

  // The coffee corner stands at the right end, by the door: one place
  // behind the counter, by the cup.
  const counterLeft = right - counterWidth;
  if (hasCoffee && counterLeft >= cursor + gap) {
    fixtures.push({ name: 'coffee', left: counterLeft, top: feetY - 6 * unit });
    spots.push({ id: 'coffee', stance: 'coffee', x: counterLeft + 13 * unit, feetY });
  }

  // The table with the plan stands in the middle of what is left: two
  // places behind it. Its top hides the legs of whoever stands there.
  if (hasTable) {
    const end = hasCoffee ? counterLeft - gap : right;
    const middle = Math.round((commons.left + right - tableWidth) / 2);
    const tableLeft = Math.min(Math.max(cursor + gap, middle), end - tableWidth);
    fixtures.push({ name: 'planTable', left: tableLeft, top: feetY - 5 * unit });
    spots.push({ id: 'table-left', stance: 'table', x: tableLeft + 9 * unit, feetY });
    spots.push({ id: 'table-right', stance: 'table', x: tableLeft + 25 * unit, feetY });
  }

  return { laneY, spots, fixtures };
}

/** The whole plan of the room, from what was measured. */
export function planRoom(measures: RoomMeasures): RoomPlan {
  const { unit, benches, wallRight, commons } = measures;
  const seats = benches.map((bench) => seatAt(bench, unit));
  const rightmost = Math.max(
    commons.left + commons.width,
    ...benches.map(({ box }) => box.left + box.width),
  );
  const { laneY, spots, fixtures } = commonsOf(commons, unit);
  return {
    unit,
    seats,
    aisleX: rightmost + (wallRight - rightmost) / 2,
    commonsLaneY: laneY,
    spots,
    fixtures,
  };
}

/** A walk from a bench to a shared place, along the aisles. */
export interface Walk {
  spot: string;
  path: readonly Point[];
  length: number;
}

function distance(from: Point, to: Point): number {
  return Math.abs(to.x - from.x) + Math.abs(to.y - from.y);
}

/**
 * The way from a seat to a place: up out of the bench into the aisle in
 * front of its row, along it to the side of the room, down the side to the
 * band of shared places, along it, and down into the place. Every turn is a
 * right angle, and no part of the way crosses a bench, a wall or a word.
 */
export function walkTo(seat: Seat, spot: Spot, plan: RoomPlan): Walk {
  const points: Point[] = [
    { x: seat.x, y: seat.deskY - CLEARANCE },
    { x: seat.x, y: seat.laneY },
    { x: plan.aisleX, y: seat.laneY },
    { x: plan.aisleX, y: plan.commonsLaneY },
    { x: spot.x, y: plan.commonsLaneY },
    { x: spot.x, y: spot.feetY },
  ];
  // Leave out the steps that go nowhere.
  const path = points.filter((point, index) => {
    const before = points[index - 1];
    return !before || distance(before, point) > 0;
  });
  const length = path.reduce((sum, point, index) => {
    const before = path[index - 1];
    return before ? sum + distance(before, point) : sum;
  }, 0);
  return { spot: spot.id, path, length };
}

/** How a walking figure faces: to the room, away from it, or to one side. */
export type Heading = 'down' | 'up' | 'left' | 'right';

/** Where a walk has got to after a distance, and which way the figure faces. */
export function pointAlong(
  path: readonly Point[],
  travelled: number,
): Point & { heading: Heading } {
  let left = Math.max(0, travelled);
  for (let index = 1; index < path.length; index += 1) {
    const from = path[index - 1];
    const to = path[index];
    if (!from || !to) {
      continue;
    }
    const stretch = distance(from, to);
    const heading: Heading =
      to.x > from.x ? 'right' : to.x < from.x ? 'left' : to.y < from.y ? 'up' : 'down';
    if (left <= stretch || index === path.length - 1) {
      const share = stretch === 0 ? 1 : Math.min(1, left / stretch);
      return {
        x: from.x + (to.x - from.x) * share,
        y: from.y + (to.y - from.y) * share,
        heading,
      };
    }
    left -= stretch;
  }
  const last = path[path.length - 1] ?? { x: 0, y: 0 };
  return { ...last, heading: 'down' };
}

/** The other way round. */
export function reversed(heading: Heading): Heading {
  switch (heading) {
    case 'down':
      return 'up';
    case 'up':
      return 'down';
    case 'left':
      return 'right';
    case 'right':
      return 'left';
  }
}

/** Where one person is in their day. */
export type Phase =
  | { kind: 'seated'; restUntil: number }
  | { kind: 'out'; walk: Walk; startedAt: number }
  | { kind: 'visiting'; walk: Walk; until: number }
  | { kind: 'back'; walk: Walk; startedAt: number };

/** The people of the room at a moment. */
export interface RoomState {
  /** Seconds since the room began to move. */
  time: number;
  phases: Readonly<Record<string, Phase>>;
}

/**
 * How fast people walk, in pixels of a character each second: an easy
 * office pace, a little more than a body's height a second, so that most
 * walks to a shared place take less than twenty seconds.
 */
export const WALKING_SPEED = 28;

/** How fast people walk in a plan, in pixels of the page each second. */
export function speedIn(plan: RoomPlan): number {
  return WALKING_SPEED * plan.unit;
}

/** How long somebody stays at a shared place, in seconds. */
const VISIT = { shortest: 8, longest: 16 };
/** How long somebody stays at the bench between two walks, in seconds. */
const REST = { shortest: 25, longest: 70 };

/** How many people may be away from their bench at once: two in a team of six or more. */
export function mostAway(people: number): number {
  return people >= 6 ? 2 : 1;
}
/** How long before the first walk of the day, so that a room that opens is soon alive. */
const FIRST_REST = { shortest: 3, longest: 30 };

function between(range: { shortest: number; longest: number }, random: () => number): number {
  return range.shortest + (range.longest - range.shortest) * random();
}

/** A room where everybody sits at the bench. */
export function startRoom(people: readonly Person[], random: () => number): RoomState {
  return {
    time: 0,
    phases: Object.fromEntries(
      people.map((person) => [
        person.key,
        { kind: 'seated', restUntil: between(FIRST_REST, random) },
      ]),
    ),
  };
}

function isWalking(phase: Phase | undefined): boolean {
  return phase?.kind === 'out' || phase?.kind === 'back';
}

function spotOf(phase: Phase): string | null {
  return phase.kind === 'seated' ? null : phase.walk.spot;
}

/**
 * The free place a person goes to: the nearest, now and then the second
 * nearest, so that a short walk is the rule and the room still varies.
 */
function chooseSpot(
  seat: Seat,
  free: readonly Spot[],
  plan: RoomPlan,
  random: () => number,
): Walk | null {
  const walks = free
    .map((spot) => walkTo(seat, spot, plan))
    .sort((first, second) => first.length - second.length);
  const pick = walks.length > 1 && random() < 0.3 ? 1 : 0;
  return walks[pick] ?? null;
}

/**
 * The room a moment later. One person walks at a time for the pleasure of
 * it; one may be away from the bench, or two in a team of six or more; and
 * nobody goes to a place another person holds, so two people never stand
 * in each other. Somebody whose work starts while away turns back at once.
 */
export function advanceRoom(
  state: RoomState,
  people: readonly Person[],
  plan: RoomPlan,
  seconds: number,
  random: () => number,
): RoomState {
  const time = state.time + Math.max(0, seconds);
  const speed = speedIn(plan);
  const phases: Record<string, Phase> = {};
  for (const person of people) {
    phases[person.key] = state.phases[person.key] ?? {
      kind: 'seated',
      restUntil: time + between(FIRST_REST, random),
    };
  }
  const seats = new Map(plan.seats.map((seat) => [seat.key, seat]));
  const anybodyWalking = () => Object.values(phases).some(isWalking);
  const away = () => Object.values(phases).filter((phase) => phase.kind !== 'seated').length;
  // Somebody whose visit is over and who waits to come back goes first.
  const anybodyDueBack = () =>
    Object.values(phases).some((phase) => phase.kind === 'visiting' && time >= phase.until);
  const takenSpots = () =>
    new Set(
      Object.values(phases)
        .map(spotOf)
        .filter((spot) => spot !== null),
    );

  for (const person of people) {
    const phase = phases[person.key];
    const seat = seats.get(person.key);
    if (!phase || !seat) {
      continue;
    }
    const mayStayAway = person.activity === 'idle';
    switch (phase.kind) {
      case 'seated': {
        if (!mayStayAway || time < phase.restUntil) {
          break;
        }
        const taken = takenSpots();
        const free = plan.spots.filter((spot) => !taken.has(spot.id));
        const walk =
          anybodyWalking() || anybodyDueBack() || away() >= mostAway(people.length)
            ? null
            : chooseSpot(seat, free, plan, random);
        if (!walk) {
          // Try again in a little while.
          phases[person.key] = { kind: 'seated', restUntil: time + 2 + 4 * random() };
          break;
        }
        phases[person.key] = { kind: 'out', walk, startedAt: time };
        break;
      }
      case 'out': {
        const travelled = (time - phase.startedAt) * speed;
        if (!mayStayAway) {
          // Turn round where one stands: the way back is what is left of the way out.
          const left = Math.min(travelled, phase.walk.length);
          phases[person.key] = {
            kind: 'back',
            walk: phase.walk,
            startedAt: time - (phase.walk.length - left) / speed,
          };
        } else if (travelled >= phase.walk.length) {
          phases[person.key] = {
            kind: 'visiting',
            walk: phase.walk,
            until: time + between(VISIT, random),
          };
        }
        break;
      }
      case 'visiting': {
        if (!mayStayAway || (time >= phase.until && !anybodyWalking())) {
          phases[person.key] = { kind: 'back', walk: phase.walk, startedAt: time };
        }
        break;
      }
      case 'back': {
        if ((time - phase.startedAt) * speed >= phase.walk.length) {
          phases[person.key] = { kind: 'seated', restUntil: time + between(REST, random) };
        }
        break;
      }
    }
  }
  return { time, phases };
}

/** How a person is drawn at a moment: seated at the bench, or standing somewhere in the room. */
/** A small thing somebody idle at the bench does now and then. */
export type Gesture = 'glance' | 'sip' | 'stretch' | 'page';

export type Figure =
  | {
      kind: 'seated';
      seat: Seat;
      activity: Activity;
      frame: number;
      /** What an idle person is doing for a moment, if anything. */
      gesture: Gesture | null;
    }
  | {
      kind: 'standing';
      /** Where the feet are. */
      at: Point;
      heading: Heading;
      /** Whether the figure is walking, and how far, for the step it is on. */
      walking: boolean;
      frame: number;
      stance: Stance | null;
    };

/** How far a figure walks for each frame of its step, in pixels of a character. */
const STRIDE = 3;
/** How long one frame of work at the bench lasts, in seconds. */
const WORK_BEAT = 0.42;
/** How long the raised hand of somebody waiting takes to dip and rise, in seconds. */
const WAVE_BEAT = 1.1;
/** How often an idle person does a small thing, in seconds, and for how long. */
const GESTURE_EVERY = 9;
const GESTURE_FOR = 2.4;
/** How long each of the two frames of a gesture lasts, in seconds: slow. */
const GESTURE_BEAT = 0.6;
/** The small things, in the order a person goes through them, glancing most. */
const GESTURES: readonly Gesture[] = [
  'glance',
  'sip',
  'glance',
  'page',
  'stretch',
  'glance',
  'sip',
];

/** What an idle person does at a moment: mostly nothing, now and then a small thing. */
export function gestureAt(seed: number, time: number): { gesture: Gesture | null; frame: number } {
  const shifted = time + (seed % 997) * 0.37;
  const into = shifted % GESTURE_EVERY;
  if (into >= GESTURE_FOR) {
    return { gesture: null, frame: 0 };
  }
  const round = Math.floor(shifted / GESTURE_EVERY) + (seed % GESTURES.length);
  return {
    gesture: GESTURES[round % GESTURES.length] ?? null,
    frame: Math.floor(into / GESTURE_BEAT) % 2,
  };
}

/** The figure of a seated person: its pose follows what the bench says. */
function seatedFigure(person: Person, seat: Seat, time: number, isStill: boolean): Figure {
  const offset = (person.seed % 1000) / 1000;
  if (isStill) {
    return { kind: 'seated', seat, activity: person.activity, frame: 0, gesture: null };
  }
  switch (person.activity) {
    case 'working':
      return {
        kind: 'seated',
        seat,
        activity: 'working',
        frame: Math.floor(time / WORK_BEAT + offset * 4),
        gesture: null,
      };
    case 'waiting':
      return {
        kind: 'seated',
        seat,
        activity: 'waiting',
        frame: Math.floor(time / WAVE_BEAT + offset * 2),
        gesture: null,
      };
    case 'idle': {
      const { gesture, frame } = gestureAt(person.seed, time);
      return { kind: 'seated', seat, activity: 'idle', frame, gesture };
    }
  }
}

/**
 * How everybody is drawn at a moment. With motion turned down, `isStill`
 * keeps everybody at the bench in the still pose of what the bench says.
 */
export function figuresOf(
  state: RoomState,
  people: readonly Person[],
  plan: RoomPlan,
  isStill: boolean,
): { person: Person; figure: Figure }[] {
  const seats = new Map(plan.seats.map((seat) => [seat.key, seat]));
  const spots = new Map(plan.spots.map((spot) => [spot.id, spot]));
  const result: { person: Person; figure: Figure }[] = [];
  for (const person of people) {
    const seat = seats.get(person.key);
    if (!seat) {
      continue;
    }
    const phase = isStill ? undefined : state.phases[person.key];
    if (!phase || phase.kind === 'seated') {
      result.push({ person, figure: seatedFigure(person, seat, state.time, isStill) });
      continue;
    }
    if (phase.kind === 'visiting') {
      const spot = spots.get(phase.walk.spot);
      const end = phase.walk.path[phase.walk.path.length - 1] ?? { x: seat.x, y: seat.deskY };
      result.push({
        person,
        figure: {
          kind: 'standing',
          at: end,
          heading: spot?.stance === 'window' ? 'left' : 'down',
          walking: false,
          frame: 0,
          stance: spot?.stance ?? null,
        },
      });
      continue;
    }
    const travelled = Math.min(phase.walk.length, (state.time - phase.startedAt) * speedIn(plan));
    const along =
      phase.kind === 'out'
        ? pointAlong(phase.walk.path, travelled)
        : pointAlong(phase.walk.path, phase.walk.length - travelled);
    result.push({
      person,
      figure: {
        kind: 'standing',
        at: { x: along.x, y: along.y },
        heading: phase.kind === 'out' ? along.heading : reversed(along.heading),
        walking: true,
        frame: Math.floor(travelled / (STRIDE * plan.unit)),
        stance: null,
      },
    });
  }
  return result;
}

/**
 * The room carried over to a new plan, after the window changed size.
 * Somebody seated stays seated. Somebody away keeps their place and how
 * far along their way they are, on the way as it runs in the new room, so
 * that they walk on instead of jumping back to the bench.
 */
export function replanRoom(state: RoomState, before: RoomPlan, after: RoomPlan): RoomState {
  const seats = new Map(after.seats.map((seat) => [seat.key, seat]));
  const spots = new Map(after.spots.map((spot) => [spot.id, spot]));
  const phases: Record<string, Phase> = {};
  for (const [key, phase] of Object.entries(state.phases)) {
    if (phase.kind === 'seated') {
      phases[key] = phase;
      continue;
    }
    const seat = seats.get(key);
    const spot = spots.get(phase.walk.spot);
    if (!seat || !spot) {
      phases[key] = { kind: 'seated', restUntil: state.time };
      continue;
    }
    const walk = walkTo(seat, spot, after);
    if (phase.kind === 'visiting') {
      phases[key] = { ...phase, walk };
      continue;
    }
    const share =
      phase.walk.length === 0
        ? 1
        : Math.min(1, ((state.time - phase.startedAt) * speedIn(before)) / phase.walk.length);
    phases[key] = {
      kind: phase.kind,
      walk,
      startedAt: state.time - (share * walk.length) / speedIn(after),
    };
  }
  return { time: state.time, phases };
}

/** A small generator of chance that gives the same numbers for the same seed (mulberry32). */
export function chance(seed: number): () => number {
  let value = seed >>> 0;
  return () => {
    value = (value + 0x6d2b79f5) >>> 0;
    let mixed = value;
    mixed = Math.imul(mixed ^ (mixed >>> 15), mixed | 1);
    mixed ^= mixed + Math.imul(mixed ^ (mixed >>> 7), mixed | 61);
    return ((mixed ^ (mixed >>> 14)) >>> 0) / 4294967296;
  };
}
