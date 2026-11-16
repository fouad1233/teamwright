import type { Box, Point } from '@/shared/ui/groundModel';

/**
 * The model of a whiteboard: what lies on it, and every change the manager
 * can make to it, as pure functions. The component holds the state and
 * listens to the pointer and the keys; nothing here reads or writes
 * anything.
 *
 * A board holds shapes (notes, texts, boxes and ellipses) and joins (a line
 * or an arrow from one shape to another, which follows them when they
 * move). The shapes are kept in the order they are drawn: a later one lies
 * over an earlier one.
 */

/** The kinds of shape a board holds. */
export type ShapeKind = 'note' | 'text' | 'box' | 'ellipse';

export const SHAPE_KINDS: readonly ShapeKind[] = ['note', 'text', 'box', 'ellipse'];

/**
 * The tints a note can have: the paper of the program, or the wash of one
 * of its pigments. No bright yellow: a note is a tinted card, not a toy.
 */
export type Tint = 'plain' | 'amber' | 'green' | 'blue' | 'purple' | 'pink' | 'teal';

export const TINTS: readonly Tint[] = ['plain', 'amber', 'green', 'blue', 'purple', 'pink', 'teal'];

/** A thing on the board with a place and words. */
export interface Shape {
  id: string;
  kind: ShapeKind;
  x: number;
  y: number;
  width: number;
  height: number;
  text: string;
  /** The tint of a note. Other shapes have none. */
  tint?: Tint;
  /** The task a note became, by its id. */
  taskId?: string;
}

/** A line between two shapes, with an arrowhead where it meets the second one if it is an arrow. */
export interface Join {
  id: string;
  from: string;
  to: string;
  isArrow: boolean;
}

export interface Board {
  shapes: readonly Shape[];
  joins: readonly Join[];
}

export const EMPTY_BOARD: Board = { shapes: [], joins: [] };

/** The ids of what is selected: shapes and joins alike. */
export type Selection = ReadonlySet<string>;

export const NOTHING_SELECTED: Selection = new Set();

/** The size a shape gets when it is made with a press rather than a drag, in pixels of the ground. */
export const DEFAULT_SIZES: Readonly<Record<ShapeKind, { width: number; height: number }>> = {
  note: { width: 176, height: 128 },
  text: { width: 240, height: 36 },
  box: { width: 240, height: 160 },
  ellipse: { width: 168, height: 112 },
};

/** The smallest a shape can be drawn or resized to, in pixels of the ground. */
export const SMALLEST_SIDE = 24;

/** Below this distance a drag that makes a shape counts as a press. */
const DRAG_THRESHOLD = 8;

/** How far a duplicate lies from its original, across and down. */
export const DUPLICATE_OFFSET = 24;

/** How many steps undo remembers. */
const HISTORY_LENGTH = 100;

/** The most bytes the content of a view may have, counted in UTF-8: 2 MiB. The backend refuses more. */
export const MAX_CONTENT_BYTES = 2 * 1024 * 1024;

/** Below this much room the board says how much is left. */
const LITTLE_ROOM_BYTES = 256 * 1024;

/** The version of the content written today. */
const CONTENT_VERSION = 1;

// ---------------------------------------------------------------------------
// Reading and writing the content
// ---------------------------------------------------------------------------

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}

function isId(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && value.length <= 64;
}

/** A shape as it was read, or nothing if it cannot be one. */
function shapeOf(value: unknown): Shape | null {
  if (!isRecord(value)) {
    return null;
  }
  const { id, kind, x, y, width, height, text, tint, taskId } = value;
  if (
    !isId(id) ||
    !SHAPE_KINDS.includes(kind as ShapeKind) ||
    !isNumber(x) ||
    !isNumber(y) ||
    !isNumber(width) ||
    !isNumber(height)
  ) {
    return null;
  }
  const shape: Shape = {
    id,
    kind: kind as ShapeKind,
    x,
    y,
    width: Math.max(width, SMALLEST_SIDE),
    height: Math.max(height, SMALLEST_SIDE),
    text: typeof text === 'string' ? text : '',
  };
  if (shape.kind === 'note') {
    shape.tint = TINTS.includes(tint as Tint) ? (tint as Tint) : 'plain';
    if (isId(taskId)) {
      shape.taskId = taskId;
    }
  }
  return shape;
}

function joinOf(value: unknown): Join | null {
  if (!isRecord(value)) {
    return null;
  }
  const { id, from, to, isArrow } = value;
  if (!isId(id) || !isId(from) || !isId(to) || from === to) {
    return null;
  }
  return { id, from, to, isArrow: isArrow === true };
}

/**
 * Reads what a whiteboard keeps. Content that is missing, damaged or written
 * by another version gives what can be read of it and never throws: a
 * shape that cannot be read is left out, a join whose ends are gone is left
 * out, and an id met twice is kept once.
 */
export function parseBoard(content: string | null): Board {
  if (content === null || content.trim() === '') {
    return EMPTY_BOARD;
  }
  let read: unknown;
  try {
    read = JSON.parse(content);
  } catch {
    return EMPTY_BOARD;
  }
  if (!isRecord(read)) {
    return EMPTY_BOARD;
  }
  const ids = new Set<string>();
  const shapes: Shape[] = [];
  for (const value of Array.isArray(read.shapes) ? read.shapes : []) {
    const shape = shapeOf(value);
    if (shape && !ids.has(shape.id)) {
      ids.add(shape.id);
      shapes.push(shape);
    }
  }
  const joins: Join[] = [];
  for (const value of Array.isArray(read.joins) ? read.joins : []) {
    const join = joinOf(value);
    if (join && !ids.has(join.id) && ids.has(join.from) && ids.has(join.to)) {
      ids.add(join.id);
      joins.push(join);
    }
  }
  return shapes.length === 0 ? EMPTY_BOARD : { shapes, joins };
}

/** The content a board is kept as: nothing for an empty board. */
export function serializeBoard(board: Board): string | null {
  if (board.shapes.length === 0) {
    return null;
  }
  return JSON.stringify({ version: CONTENT_VERSION, shapes: board.shapes, joins: board.joins });
}

/** How many bytes the content of a board weighs as the backend counts them. */
export function boardBytes(board: Board): number {
  const content = serializeBoard(board);
  return content === null ? 0 : new TextEncoder().encode(content).length;
}

/** A number of bytes in words: "740 bytes", "96 kB", "1.4 MB". Kilo is 1024, as the limit counts. */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) {
    return `${Math.round(bytes).toString()} bytes`;
  }
  if (bytes < 1024 * 1024) {
    return `${Math.round(bytes / 1024).toString()} kB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** What the board says about its room, or nothing while there is plenty. */
export function roomNote(bytes: number): string | null {
  const left = Math.max(0, MAX_CONTENT_BYTES - bytes);
  if (left >= LITTLE_ROOM_BYTES) {
    return null;
  }
  return `The board is nearly full: ${formatBytes(left)} left of ${formatBytes(MAX_CONTENT_BYTES)}. Delete what is no longer needed to make room.`;
}

/** Whether a board fits in what a view may hold. */
export function fits(board: Board): boolean {
  return boardBytes(board) <= MAX_CONTENT_BYTES;
}

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

/** The box of a shape. */
export function boxOf({ x, y, width, height }: Shape): Box {
  return { x, y, width, height };
}

/** The middle of a box. */
export function middleOf(box: Box): Point {
  return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
}

/** The smallest box that holds every one of several boxes, or nothing for none. */
export function boxAround(boxes: readonly Box[]): Box | null {
  if (boxes.length === 0) {
    return null;
  }
  let left = Infinity;
  let top = Infinity;
  let right = -Infinity;
  let bottom = -Infinity;
  for (const box of boxes) {
    left = Math.min(left, box.x);
    top = Math.min(top, box.y);
    right = Math.max(right, box.x + box.width);
    bottom = Math.max(bottom, box.y + box.height);
  }
  return { x: left, y: top, width: right - left, height: bottom - top };
}

/** The box of everything on the board, or nothing for an empty board. */
export function boardBox(board: Board): Box | null {
  return boxAround(board.shapes.map(boxOf));
}

/** The box between two corners, whichever way it was dragged. */
export function boxBetween(one: Point, other: Point): Box {
  return {
    x: Math.min(one.x, other.x),
    y: Math.min(one.y, other.y),
    width: Math.abs(other.x - one.x),
    height: Math.abs(other.y - one.y),
  };
}

/**
 * The box a new shape takes when it is made from a press at one point and
 * a release at another, in whole pixels. A press, or a drag too short to
 * mean a size, gives the shape its usual size with its top left corner
 * where it was pressed; a drag gives the box dragged, never smaller than
 * the smallest side.
 */
export function boxOfNewShape(kind: ShapeKind, start: Point, end: Point): Box {
  if (Math.hypot(end.x - start.x, end.y - start.y) < DRAG_THRESHOLD) {
    const size = DEFAULT_SIZES[kind];
    return { x: Math.round(start.x), y: Math.round(start.y), ...size };
  }
  const box = boxBetween(start, end);
  return {
    x: Math.round(box.x),
    y: Math.round(box.y),
    width: Math.max(SMALLEST_SIDE, Math.round(box.width)),
    height: Math.max(SMALLEST_SIDE, Math.round(box.height)),
  };
}

/** A corner of a shape that is dragged to resize it. */
export type Corner = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';

export const CORNERS: readonly Corner[] = ['top-left', 'top-right', 'bottom-left', 'bottom-right'];

/**
 * A box with one corner dragged by so much, in whole pixels. The opposite
 * corner stays where it is, and no side becomes smaller than the smallest.
 */
export function resizedBox(box: Box, corner: Corner, dx: number, dy: number): Box {
  const movesLeft = corner === 'top-left' || corner === 'bottom-left';
  const movesTop = corner === 'top-left' || corner === 'top-right';
  const right = box.x + box.width;
  const bottom = box.y + box.height;
  const width = Math.max(SMALLEST_SIDE, Math.round(box.width + (movesLeft ? -dx : dx)));
  const height = Math.max(SMALLEST_SIDE, Math.round(box.height + (movesTop ? -dy : dy)));
  return {
    x: movesLeft ? right - width : box.x,
    y: movesTop ? bottom - height : box.y,
    width,
    height,
  };
}

/**
 * Where a line from the middle of a shape towards a point leaves the
 * shape: on the edge of its box, or of its ellipse. A point in the very
 * middle gives the middle.
 */
export function edgePoint(shape: Shape, toward: Point): Point {
  const middle = middleOf(boxOf(shape));
  const dx = toward.x - middle.x;
  const dy = toward.y - middle.y;
  if (dx === 0 && dy === 0) {
    return middle;
  }
  const halfWidth = shape.width / 2;
  const halfHeight = shape.height / 2;
  let reach: number;
  if (shape.kind === 'ellipse') {
    // The point (dx, dy) * t lies on the ellipse where
    // (dx t / a)^2 + (dy t / b)^2 = 1.
    reach = 1 / Math.sqrt((dx / halfWidth) ** 2 + (dy / halfHeight) ** 2);
  } else {
    reach = Math.min(
      dx === 0 ? Infinity : halfWidth / Math.abs(dx),
      dy === 0 ? Infinity : halfHeight / Math.abs(dy),
    );
  }
  return { x: middle.x + dx * reach, y: middle.y + dy * reach };
}

/** The two ends of a line between two shapes: where it leaves the first and meets the second. */
export function joinEnds(from: Shape, to: Shape): { start: Point; end: Point } {
  return {
    start: edgePoint(from, middleOf(boxOf(to))),
    end: edgePoint(to, middleOf(boxOf(from))),
  };
}

/**
 * The two strokes of an arrowhead at the end of a line from `start` to
 * `end`, each `length` long and opening at about thirty degrees on either
 * side. Nothing for a line without length.
 */
export function arrowhead(start: Point, end: Point, length: number): [Point, Point] | null {
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const distance = Math.hypot(dx, dy);
  if (distance === 0) {
    return null;
  }
  const back = Math.atan2(-dy, -dx);
  const spread = Math.PI / 6;
  return [
    { x: end.x + length * Math.cos(back + spread), y: end.y + length * Math.sin(back + spread) },
    { x: end.x + length * Math.cos(back - spread), y: end.y + length * Math.sin(back - spread) },
  ];
}

/** Whether one box lies wholly inside another. */
function isInside(inner: Box, outer: Box): boolean {
  return (
    inner.x >= outer.x &&
    inner.y >= outer.y &&
    inner.x + inner.width <= outer.x + outer.width &&
    inner.y + inner.height <= outer.y + outer.height
  );
}

/**
 * What a dragged frame catches: every shape that lies wholly inside it, and
 * every join between two shapes it catches.
 */
export function caughtBy(board: Board, frame: Box): Set<string> {
  const caught = new Set<string>();
  for (const shape of board.shapes) {
    if (isInside(boxOf(shape), frame)) {
      caught.add(shape.id);
    }
  }
  for (const join of board.joins) {
    if (caught.has(join.from) && caught.has(join.to)) {
      caught.add(join.id);
    }
  }
  return caught;
}

/** The shape that lies on top at a point of the ground, if any. */
export function shapeAt(board: Board, point: Point): Shape | null {
  for (let index = board.shapes.length - 1; index >= 0; index -= 1) {
    const shape = board.shapes[index];
    if (
      shape &&
      point.x >= shape.x &&
      point.y >= shape.y &&
      point.x <= shape.x + shape.width &&
      point.y <= shape.y + shape.height
    ) {
      return shape;
    }
  }
  return null;
}

// ---------------------------------------------------------------------------
// Changes
// ---------------------------------------------------------------------------

/** A board with a new shape on top of the others. */
export function withShape(board: Board, shape: Shape): Board {
  return { ...board, shapes: [...board.shapes, shape] };
}

/** A new shape of a kind in a box, with no words yet. A note starts plain. */
export function newShape(id: string, kind: ShapeKind, box: Box): Shape {
  return { id, kind, ...box, text: '', ...(kind === 'note' ? { tint: 'plain' } : {}) };
}

/** Changes the shapes whose ids are given; the others stay, and so does the board if none changes. */
function changingShapes(board: Board, ids: Selection, change: (shape: Shape) => Shape): Board {
  let changed = false;
  const shapes = board.shapes.map((shape) => {
    if (!ids.has(shape.id)) {
      return shape;
    }
    const next = change(shape);
    changed ||= next !== shape;
    return next;
  });
  return changed ? { ...board, shapes } : board;
}

/** The selected shapes moved by so much, in whole pixels. */
export function moved(board: Board, ids: Selection, dx: number, dy: number): Board {
  const across = Math.round(dx);
  const down = Math.round(dy);
  if (across === 0 && down === 0) {
    return board;
  }
  return changingShapes(board, ids, (shape) => ({
    ...shape,
    x: shape.x + across,
    y: shape.y + down,
  }));
}

/** A shape laid in another box. */
export function resized(board: Board, id: string, box: Box): Board {
  return changingShapes(board, new Set([id]), (shape) =>
    shape.x === box.x &&
    shape.y === box.y &&
    shape.width === box.width &&
    shape.height === box.height
      ? shape
      : { ...shape, ...box },
  );
}

/** A shape with other words, and at least as high as they need. */
export function withText(board: Board, id: string, text: string, height?: number): Board {
  return changingShapes(board, new Set([id]), (shape) => {
    const tall = Math.max(shape.height, Math.round(height ?? 0));
    return shape.text === text && shape.height === tall ? shape : { ...shape, text, height: tall };
  });
}

/** The selected notes in a tint. Other shapes have none and stay as they are. */
export function tinted(board: Board, ids: Selection, tint: Tint): Board {
  return changingShapes(board, ids, (shape) =>
    shape.kind !== 'note' || shape.tint === tint ? shape : { ...shape, tint },
  );
}

/** A note that became a task. */
export function withTask(board: Board, id: string, taskId: string): Board {
  return changingShapes(board, new Set([id]), (shape) =>
    shape.kind === 'note' ? { ...shape, taskId } : shape,
  );
}

/**
 * The board without what is selected. A join goes with either of its
 * shapes.
 */
export function removed(board: Board, ids: Selection): Board {
  if (ids.size === 0) {
    return board;
  }
  const shapes = board.shapes.filter((shape) => !ids.has(shape.id));
  const joins = board.joins.filter(
    (join) => !ids.has(join.id) && !ids.has(join.from) && !ids.has(join.to),
  );
  if (shapes.length === board.shapes.length && joins.length === board.joins.length) {
    return board;
  }
  return shapes.length === 0 ? EMPTY_BOARD : { shapes, joins };
}

/**
 * The board with a copy of every selected shape, a little below and to the
 * right of it and on top of the others, and a copy of every join between
 * two of them. A copy of a note that became a task is a plain note: the
 * task stays with the one note. `newId` gives a fresh id each time it is
 * called. Gives the board and the ids of the copies, which are then the
 * selection.
 */
export function duplicated(
  board: Board,
  ids: Selection,
  newId: () => string,
): { board: Board; copies: Set<string> } {
  const copyOf = new Map<string, string>();
  const shapes: Shape[] = [];
  for (const shape of board.shapes) {
    if (ids.has(shape.id)) {
      const id = newId();
      copyOf.set(shape.id, id);
      const copy: Shape = {
        ...shape,
        id,
        x: shape.x + DUPLICATE_OFFSET,
        y: shape.y + DUPLICATE_OFFSET,
      };
      delete copy.taskId;
      shapes.push(copy);
    }
  }
  if (shapes.length === 0) {
    return { board, copies: new Set() };
  }
  const joins: Join[] = [];
  for (const join of board.joins) {
    const from = copyOf.get(join.from);
    const to = copyOf.get(join.to);
    if (from !== undefined && to !== undefined) {
      joins.push({ id: newId(), from, to, isArrow: join.isArrow });
    }
  }
  return {
    board: { shapes: [...board.shapes, ...shapes], joins: [...board.joins, ...joins] },
    copies: new Set([...shapes.map((shape) => shape.id), ...joins.map((join) => join.id)]),
  };
}

/**
 * The board with a line or an arrow between two shapes. Nothing changes
 * for a shape joined to itself, a shape that is not there, or two shapes
 * that are joined already, either way round: the arrow then turns into a
 * line or a line into an arrow as asked.
 */
export function joined(board: Board, join: Join): Board {
  if (join.from === join.to) {
    return board;
  }
  const ids = new Set(board.shapes.map((shape) => shape.id));
  if (!ids.has(join.from) || !ids.has(join.to)) {
    return board;
  }
  const existing = board.joins.find(
    (other) =>
      (other.from === join.from && other.to === join.to) ||
      (other.from === join.to && other.to === join.from),
  );
  if (existing) {
    if (existing.isArrow === join.isArrow && existing.from === join.from) {
      return board;
    }
    return {
      ...board,
      joins: board.joins.map((other) =>
        other === existing
          ? { ...other, from: join.from, to: join.to, isArrow: join.isArrow }
          : other,
      ),
    };
  }
  return { ...board, joins: [...board.joins, join] };
}

/** Everything on the board, for "select all". */
export function everything(board: Board): Set<string> {
  return new Set([...board.shapes.map((shape) => shape.id), ...board.joins.map((join) => join.id)]);
}

/** A selection without what is no longer on the board. */
export function stillThere(board: Board, selection: Selection): Selection {
  const ids = everything(board);
  const kept = [...selection].filter((id) => ids.has(id));
  return kept.length === selection.size ? selection : new Set(kept);
}

/** A selection with one thing added, or taken away if it was in it already. */
export function toggled(selection: Selection, id: string): Set<string> {
  const next = new Set(selection);
  if (next.has(id)) {
    next.delete(id);
  } else {
    next.add(id);
  }
  return next;
}

// ---------------------------------------------------------------------------
// Undo and redo
// ---------------------------------------------------------------------------

/** The boards before and after the one on screen, for undo and redo within the session. */
export interface History {
  past: readonly Board[];
  present: Board;
  future: readonly Board[];
}

export function historyOf(board: Board): History {
  return { past: [], present: board, future: [] };
}

/** The history after a change. A change that changes nothing is no step. */
export function recorded(history: History, next: Board): History {
  if (next === history.present) {
    return history;
  }
  return {
    past: [...history.past, history.present].slice(-HISTORY_LENGTH),
    present: next,
    future: [],
  };
}

export function undone(history: History): History {
  const previous = history.past[history.past.length - 1];
  if (previous === undefined) {
    return history;
  }
  return {
    past: history.past.slice(0, -1),
    present: previous,
    future: [history.present, ...history.future],
  };
}

export function redone(history: History): History {
  const [next, ...rest] = history.future;
  if (next === undefined) {
    return history;
  }
  return { past: [...history.past, history.present], present: next, future: rest };
}
