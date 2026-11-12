import { type Look, lookOf, type Sitter } from './characters/characterModel';
import {
  colorsOf,
  currentTheme,
  deviceUnit,
  floorShadow,
  paintSprite,
  readThemeColors,
  roomColors,
  SpriteCache,
  type Colors,
} from './characters/paint';
import type { ThemeColors } from './characters/palette';
import { benchScene, outlined, standingFigure, type Pose } from './characters/spriteModel';
import { FEET_ROW, FURNITURE, STEAM } from './characters/sprites';
import {
  advanceRoom,
  chance,
  figuresOf,
  planRoom,
  startRoom,
  type Figure,
  type Person,
  type RoomPlan,
  type RoomState,
} from './roomModel';

/*
 * Draws the people of the office on one canvas laid over the room, in one
 * loop for the whole room (decision 0019). The loop runs a few times a
 * second, does no layout, and stops while the window is hidden or motion is
 * turned down. The room is measured only when its size changes.
 */

/** The size of a pixel of a character, in pixels of the page, as it was drawn to be seen. */
export const CHARACTER_SCALE = 2;
/** How many times a second the room is drawn while people move. Pixel figures need few. */
const FRAMES_PER_SECOND = 15;
/** The longest step of time the room takes at once, so that a hidden window does not jump. */
const LONGEST_STEP = 0.25;

/** One person of the room and the character that stands for them. */
export interface RoomSitter {
  person: Person;
  sitter: Sitter;
}

function lookKey(look: Look): string {
  return `${look.role}.${look.skin.toString()}.${look.hair.toString()}.${look.hairStyle}.${look.shirt}`;
}

/** The pose of a standing figure. */
function standingPose(figure: Extract<Figure, { kind: 'standing' }>): {
  pose: Pose;
  facesLeft: boolean;
} {
  if (!figure.walking) {
    switch (figure.stance) {
      case 'window':
        return { pose: 'standSide', facesLeft: true };
      case 'table':
        return { pose: 'standDown', facesLeft: false };
      default:
        return { pose: 'stand', facesLeft: false };
    }
  }
  switch (figure.heading) {
    case 'down':
      return { pose: 'walkFront', facesLeft: false };
    case 'up':
      return { pose: 'walkBack', facesLeft: false };
    case 'left':
      return { pose: 'walkSide', facesLeft: true };
    case 'right':
      return { pose: 'walkSide', facesLeft: false };
  }
}

export class RoomPainter {
  private readonly canvas: HTMLCanvasElement;
  private readonly room: HTMLElement;
  private readonly cache = new SpriteCache();
  private readonly random = chance(Date.now());
  private sitters: readonly RoomSitter[] = [];
  private signature = '';
  private looks = new Map<string, { look: Look; colors: Colors | null }>();
  private plan: RoomPlan | null = null;
  private state: RoomState = { time: 0, phases: {} };
  private theme: ThemeColors | null = null;
  private furnitureColors: Colors | null = null;
  private pixelRatio = 1;
  private device = 1;
  private isStill = false;
  private isHidden = false;
  private frame = 0;
  private lastTick = 0;
  /** What the canvas shows now, to know when it need not be painted again. */
  private painted = '';

  constructor(canvas: HTMLCanvasElement, room: HTMLElement) {
    this.canvas = canvas;
    this.room = room;
  }

  /** Who is in the room. A new person starts at the bench; somebody gone is forgotten. */
  setSitters(sitters: readonly RoomSitter[]): void {
    // The places are described again every second while somebody works;
    // only a change of who is there, or of what they do, needs anything.
    const signature = sitters
      .map(({ person, sitter }) => `${person.key}:${person.activity}:${JSON.stringify(sitter)}`)
      .join('|');
    if (signature === this.signature) {
      return;
    }
    this.signature = signature;
    this.sitters = sitters;
    const looks = new Map<string, { look: Look; colors: Colors | null }>();
    for (const { person, sitter } of sitters) {
      const look = lookOf(sitter);
      const known = this.looks.get(person.key);
      looks.set(person.key, {
        look,
        colors: known && lookKey(known.look) === lookKey(look) ? known.colors : null,
      });
    }
    this.looks = looks;
    this.measure();
  }

  /** Motion turned down: nobody walks and nothing loops. */
  setStill(isStill: boolean): void {
    this.isStill = isStill;
    this.resume();
  }

  /** The window was hidden or shown again. */
  setHidden(isHidden: boolean): void {
    this.isHidden = isHidden;
    this.resume();
  }

  /** The theme changed: the colors are read again and everything is painted anew. */
  themeChanged(): void {
    this.theme = null;
    this.cache.clear();
    this.draw(true);
  }

  /** Measures the room and works out its plan again. Called when its size changes. */
  measure(): void {
    const box = this.room.getBoundingClientRect();
    const originX = box.left + this.room.clientLeft;
    const originY = box.top + this.room.clientTop;
    const relative = (element: Element) => {
      const rect = element.getBoundingClientRect();
      return {
        left: rect.left - originX,
        top: rect.top - originY,
        width: rect.width,
        height: rect.height,
      };
    };
    const commons = this.room.querySelector('[data-commons]');
    const pixelRatio = window.devicePixelRatio || 1;
    const device = deviceUnit(CHARACTER_SCALE, pixelRatio);
    if (device !== this.device || pixelRatio !== this.pixelRatio) {
      this.cache.clear();
    }
    this.pixelRatio = pixelRatio;
    this.device = device;

    const width = this.room.clientWidth;
    const height = this.room.clientHeight;
    this.canvas.style.width = `${width.toString()}px`;
    this.canvas.style.height = `${height.toString()}px`;
    this.canvas.width = Math.round(width * pixelRatio);
    this.canvas.height = Math.round(height * pixelRatio);

    const benches = [...this.room.querySelectorAll<HTMLElement>('[data-place-key]')].map(
      (element) => ({ key: element.dataset.placeKey ?? '', box: relative(element) }),
    );
    const wallInset =
      1 + Number.parseFloat(getComputedStyle(this.room).getPropertyValue('--space-1'));
    const previous = this.plan;
    this.plan = commons
      ? planRoom({
          unit: device / pixelRatio,
          benches,
          wallRight: width - (Number.isNaN(wallInset) ? 5 : wallInset),
          commons: relative(commons),
        })
      : null;
    // Somebody halfway down an aisle that has moved would be left standing
    // in a wall: when the room changes, everybody is back at the bench.
    if (!previous || !this.plan || !samePlan(previous, this.plan)) {
      this.state = startRoom(
        this.sitters.map(({ person }) => person),
        this.random,
      );
    }
    // A canvas whose size is set again is empty.
    this.draw(true);
    this.resume();
  }

  /** Starts the loop if people may move, or stops it if they may not. */
  private resume(): void {
    const shouldRun = !this.isStill && !this.isHidden && this.plan !== null;
    if (shouldRun && this.frame === 0) {
      this.lastTick = performance.now();
      this.frame = requestAnimationFrame(this.tick);
    } else if (!shouldRun && this.frame !== 0) {
      cancelAnimationFrame(this.frame);
      this.frame = 0;
      this.draw(true);
    }
  }

  /** Stops for good. */
  dispose(): void {
    if (this.frame !== 0) {
      cancelAnimationFrame(this.frame);
      this.frame = 0;
    }
  }

  private readonly tick = (now: number) => {
    this.frame = requestAnimationFrame(this.tick);
    const elapsed = (now - this.lastTick) / 1000;
    if (elapsed < 1 / FRAMES_PER_SECOND) {
      return;
    }
    this.lastTick = now;
    if (this.plan) {
      this.state = advanceRoom(
        this.state,
        this.sitters.map(({ person }) => person),
        this.plan,
        Math.min(elapsed, LONGEST_STEP),
        this.random,
      );
    }
    this.draw();
  };

  private colorsFor(key: string): Colors | null {
    const entry = this.looks.get(key);
    const theme = this.theme;
    if (!entry || !theme) {
      return null;
    }
    entry.colors ??= colorsOf(entry.look, theme);
    return entry.colors;
  }

  /**
   * Paints the whole room once, unless it would look the same as it does.
   * `force` paints it all the same, after the canvas was cleared by a
   * change of its size or of the theme.
   */
  draw(force = false): void {
    const context = this.canvas.getContext('2d');
    const plan = this.plan;
    if (!context) {
      return;
    }
    if (!plan) {
      context.setTransform(1, 0, 0, 1, 0, 0);
      context.clearRect(0, 0, this.canvas.width, this.canvas.height);
      this.painted = '';
      return;
    }
    if (!this.theme) {
      this.theme = readThemeColors(currentTheme());
      this.furnitureColors = roomColors(this.theme);
      for (const entry of this.looks.values()) {
        entry.colors = null;
      }
    }
    context.imageSmoothingEnabled = false;
    const ratio = this.pixelRatio;
    const unit = this.device;
    const toDevice = (value: number) => Math.round(value * ratio);
    const people = this.sitters.map(({ person }) => person);
    const figures = figuresOf(this.state, people, plan, this.isStill);

    // Everything is drawn from the back of the room to the front, so that
    // whoever stands nearer covers whoever stands behind.
    const layers: { depth: number; key: string; paint: () => void }[] = [];

    const occupied = new Set(
      figures.flatMap(({ figure }) =>
        figure.kind === 'standing' && !figure.walking && figure.stance === 'coffee'
          ? ['coffee']
          : [],
      ),
    );
    for (const fixture of plan.fixtures) {
      const drawing = FURNITURE[fixture.name];
      const colors = this.furnitureColors;
      if (!colors) {
        continue;
      }
      const picture = this.cache.get(`fixture.${fixture.name}`, () =>
        paintSprite(
          outlined({
            width: drawing.rows[0]?.length ?? 0,
            height: drawing.rows.length,
            rows: drawing.rows,
          }),
          colors,
          unit,
        ),
      );
      const left = toDevice(fixture.left);
      const top = toDevice(fixture.top);
      // Steam rises from the coffee machine while somebody stands at it.
      const steaming = fixture.name === 'coffee' && occupied.has('coffee') && !this.isStill;
      const puffIndex = steaming ? Math.floor(this.state.time / 0.8) % STEAM.length : -1;
      const steam = STEAM[puffIndex];
      layers.push({
        depth: fixture.top + drawing.rows.length * plan.unit,
        key: `${fixture.name}:${left.toString()},${top.toString()}:${puffIndex.toString()}`,
        paint: () => {
          context.drawImage(picture, left, top);
          if (steam) {
            const puff = this.cache.get(`steam.${steam.rows.join('/')}`, () =>
              paintSprite({ width: 2, height: 2, rows: steam.rows }, colors, unit),
            );
            context.drawImage(puff, left + steam.x * unit, top + steam.y * unit);
          }
        },
      });
    }

    for (const { person, figure } of figures) {
      const entry = this.looks.get(person.key);
      const colors = this.colorsFor(person.key);
      if (!entry || !colors) {
        continue;
      }
      const key = lookKey(entry.look);
      if (figure.kind === 'seated') {
        const pose: Pose =
          figure.activity === 'working'
            ? 'seatWork'
            : figure.activity === 'waiting'
              ? 'seatWait'
              : figure.glancing
                ? 'seatGlance'
                : 'seatRest';
        const frame = figure.frame % 4;
        const isWorking = figure.activity === 'working';
        const name = `${key}.scene.${pose}.${frame.toString()}.${String(isWorking)}`;
        const picture = this.cache.get(name, () =>
          paintSprite(benchScene(entry.look, pose, frame, isWorking), colors, unit),
        );
        const left = toDevice(figure.seat.scene.x);
        const top = toDevice(figure.seat.scene.y);
        layers.push({
          depth: figure.seat.deskY,
          key: `${person.key}:${name}:${left.toString()},${top.toString()}`,
          paint: () => {
            context.drawImage(picture, left, top);
          },
        });
        continue;
      }
      // The bench of somebody who is away keeps its chair and its things.
      const seat = plan.seats.find((candidate) => candidate.key === person.key);
      if (seat) {
        const bench = this.cache.get(`${key}.away`, () =>
          paintSprite(benchScene(entry.look, null, 0, false), colors, unit),
        );
        const benchLeft = toDevice(seat.scene.x);
        const benchTop = toDevice(seat.scene.y);
        layers.push({
          depth: seat.deskY,
          key: `${person.key}:away:${benchLeft.toString()},${benchTop.toString()}`,
          paint: () => {
            context.drawImage(bench, benchLeft, benchTop);
          },
        });
      }
      const { pose, facesLeft } = standingPose(figure);
      const frame = figure.frame % 4;
      const name = `${key}.stand.${pose}.${frame.toString()}.${String(facesLeft)}`;
      const picture = this.cache.get(name, () =>
        paintSprite(standingFigure(entry.look, pose, frame, facesLeft), colors, unit),
      );
      const left = toDevice(figure.at.x) - 8 * unit;
      const feet = toDevice(figure.at.y);
      const top = feet - FEET_ROW * unit;
      const shadow = floorShadow(this.theme.theme);
      layers.push({
        depth: figure.at.y,
        key: `${person.key}:${name}:${left.toString()},${top.toString()}`,
        paint: () => {
          // A soft shadow under the feet keeps a figure on the floor.
          context.fillStyle = shadow;
          context.fillRect(left + 4 * unit, feet - unit, 8 * unit, 2 * unit);
          context.fillRect(left + 3 * unit, feet, 10 * unit, unit);
          context.drawImage(picture, left, top);
        },
      });
    }

    // Most moments look like the one before: somebody sits at a bench and
    // nothing has moved by a pixel. Then nothing is painted.
    const signature = layers.map((layer) => layer.key).join('|');
    if (signature === this.painted && !force) {
      return;
    }
    this.painted = signature;
    context.setTransform(1, 0, 0, 1, 0, 0);
    context.clearRect(0, 0, this.canvas.width, this.canvas.height);
    layers.sort((first, second) => first.depth - second.depth);
    for (const layer of layers) {
      layer.paint();
    }
  }
}

/** Whether two plans put the seats and the shared places in the same spots. */
function samePlan(first: RoomPlan, second: RoomPlan): boolean {
  const seats = (plan: RoomPlan) =>
    plan.seats.map((seat) => `${seat.key}:${seat.x.toFixed(1)}:${seat.deskY.toFixed(1)}`).join('|');
  const spots = (plan: RoomPlan) =>
    plan.spots.map((spot) => `${spot.id}:${spot.x.toFixed(1)}:${spot.feetY.toFixed(1)}`).join('|');
  return (
    first.unit === second.unit &&
    first.aisleX.toFixed(1) === second.aisleX.toFixed(1) &&
    seats(first) === seats(second) &&
    spots(first) === spots(second)
  );
}
