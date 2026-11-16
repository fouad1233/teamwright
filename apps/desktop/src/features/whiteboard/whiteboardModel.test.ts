import { describe, expect, it } from 'vitest';

import {
  DEFAULT_SIZES,
  DUPLICATE_OFFSET,
  EMPTY_BOARD,
  MAX_CONTENT_BYTES,
  SMALLEST_SIDE,
  arrowhead,
  boardBox,
  boardBytes,
  boxOfNewShape,
  caughtBy,
  duplicated,
  edgePoint,
  everything,
  historyOf,
  joinEnds,
  joined,
  moved,
  newShape,
  parseBoard,
  recorded,
  redone,
  removed,
  resized,
  resizedBox,
  roomNote,
  serializeBoard,
  shapeAt,
  stillThere,
  tinted,
  toggled,
  undone,
  withShape,
  withTask,
  withText,
  type Board,
  type Shape,
} from './whiteboardModel';

function note(id: string, x: number, y: number, extra: Partial<Shape> = {}): Shape {
  return { id, kind: 'note', x, y, width: 100, height: 80, text: id, tint: 'plain', ...extra };
}

const a = note('a', 0, 0);
const b = note('b', 300, 0);
const c: Shape = { id: 'c', kind: 'ellipse', x: 0, y: 200, width: 200, height: 100, text: '' };
const board: Board = {
  shapes: [a, b, c],
  joins: [{ id: 'j', from: 'a', to: 'b', isArrow: true }],
};

function counter(): () => string {
  let next = 0;
  return () => {
    next += 1;
    return `new${next.toString()}`;
  };
}

describe('reading the content', () => {
  it('gives an empty board for nothing, for damage and for what is not a board', () => {
    expect(parseBoard(null)).toBe(EMPTY_BOARD);
    expect(parseBoard('')).toBe(EMPTY_BOARD);
    expect(parseBoard('{not json')).toBe(EMPTY_BOARD);
    expect(parseBoard('[1, 2]')).toBe(EMPTY_BOARD);
    expect(parseBoard('"text"')).toBe(EMPTY_BOARD);
    expect(parseBoard('{"shapes": "many"}')).toBe(EMPTY_BOARD);
  });

  it('reads back what it wrote', () => {
    const content = serializeBoard(board);
    expect(parseBoard(content)).toEqual(board);
  });

  it('writes nothing for an empty board', () => {
    expect(serializeBoard(EMPTY_BOARD)).toBeNull();
    expect(boardBytes(EMPTY_BOARD)).toBe(0);
  });

  it('leaves out what it cannot read, and joins whose ends are gone', () => {
    const content = JSON.stringify({
      shapes: [
        a,
        { id: 'x', kind: 'star', x: 0, y: 0, width: 10, height: 10 },
        { id: 'y', kind: 'box', x: 'left', y: 0, width: 10, height: 10 },
        { ...b, id: 'a' },
        42,
      ],
      joins: [
        { id: 'j1', from: 'a', to: 'gone' },
        { id: 'j2', from: 'a', to: 'a' },
        { id: 'a', from: 'a', to: 'a' },
      ],
    });
    expect(parseBoard(content)).toEqual({ shapes: [a], joins: [] });
  });

  it('mends what can be mended: a tint it does not know, words that are missing, a size too small', () => {
    const content = JSON.stringify({
      shapes: [
        { id: 'n', kind: 'note', x: 1, y: 2, width: 3, height: 400, tint: 'yellow' },
        { id: 't', kind: 'text', x: 0, y: 0, width: 100, height: 30, text: 'Hi', tint: 'pink' },
      ],
    });
    const read = parseBoard(content);
    expect(read.shapes[0]).toEqual({
      id: 'n',
      kind: 'note',
      x: 1,
      y: 2,
      width: SMALLEST_SIDE,
      height: 400,
      text: '',
      tint: 'plain',
    });
    // Only a note has a tint.
    expect(read.shapes[1]?.tint).toBeUndefined();
  });

  it('keeps the task of a note', () => {
    const content = serializeBoard({ shapes: [note('n', 0, 0, { taskId: 't1' })], joins: [] });
    expect(parseBoard(content).shapes[0]?.taskId).toBe('t1');
  });
});

describe('the room a board takes', () => {
  it('says nothing while there is plenty', () => {
    expect(roomNote(boardBytes(board))).toBeNull();
  });

  it('says plainly when the board is nearly full', () => {
    expect(roomNote(MAX_CONTENT_BYTES - 100 * 1024)).toBe(
      'The board is nearly full: 100 kB left of 2.0 MB. Delete what is no longer needed to make room.',
    );
  });
});

describe('geometry', () => {
  it('makes a shape of its usual size from a press, and of the dragged size from a drag', () => {
    expect(boxOfNewShape('note', { x: 10.4, y: 20.6 }, { x: 12, y: 21 })).toEqual({
      x: 10,
      y: 21,
      ...DEFAULT_SIZES.note,
    });
    expect(boxOfNewShape('box', { x: 100, y: 100 }, { x: 40, y: 300 })).toEqual({
      x: 40,
      y: 100,
      width: 60,
      height: 200,
    });
    expect(boxOfNewShape('box', { x: 0, y: 0 }, { x: 100, y: 3 }).height).toBe(SMALLEST_SIDE);
  });

  it('resizes from a corner, holding the opposite one, never below the smallest side', () => {
    const box = { x: 0, y: 0, width: 100, height: 80 };
    expect(resizedBox(box, 'bottom-right', 20, 10)).toEqual({ x: 0, y: 0, width: 120, height: 90 });
    expect(resizedBox(box, 'top-left', 20, 10)).toEqual({ x: 20, y: 10, width: 80, height: 70 });
    expect(resizedBox(box, 'top-left', 500, 0)).toEqual({
      x: 100 - SMALLEST_SIDE,
      y: 0,
      width: SMALLEST_SIDE,
      height: 80,
    });
  });

  it('leaves a box at its edge', () => {
    // a: middle (50, 40); towards the right it leaves at x = 100.
    expect(edgePoint(a, { x: 500, y: 40 })).toEqual({ x: 100, y: 40 });
    // Straight down it leaves at the bottom.
    expect(edgePoint(a, { x: 50, y: 500 })).toEqual({ x: 50, y: 80 });
    // Towards a far corner it leaves where the line crosses the nearer side.
    expect(edgePoint(a, { x: 150, y: 140 })).toEqual({ x: 90, y: 80 });
    expect(edgePoint(a, { x: 50, y: 40 })).toEqual({ x: 50, y: 40 });
  });

  it('leaves an ellipse at its curve', () => {
    // c: middle (100, 250), half axes 100 and 50.
    expect(edgePoint(c, { x: 400, y: 250 })).toEqual({ x: 200, y: 250 });
    const slanted = edgePoint(c, { x: 200, y: 350 });
    const onCurve = ((slanted.x - 100) / 100) ** 2 + ((slanted.y - 250) / 50) ** 2;
    expect(onCurve).toBeCloseTo(1);
  });

  it('joins two shapes from edge to edge', () => {
    expect(joinEnds(a, b)).toEqual({ start: { x: 100, y: 40 }, end: { x: 300, y: 40 } });
  });

  it('draws an arrowhead back along the line', () => {
    const head = arrowhead({ x: 0, y: 0 }, { x: 10, y: 0 }, 10);
    expect(head).not.toBeNull();
    const [one, other] = head ?? [];
    expect(one?.x).toBeCloseTo(10 - 10 * Math.cos(Math.PI / 6));
    expect(one?.y).toBeCloseTo(-other!.y);
    expect(arrowhead({ x: 1, y: 1 }, { x: 1, y: 1 }, 10)).toBeNull();
  });

  it('catches with a frame what lies wholly inside it, and the joins between what it caught', () => {
    expect(caughtBy(board, { x: -10, y: -10, width: 120, height: 100 })).toEqual(new Set(['a']));
    expect(caughtBy(board, { x: -10, y: -10, width: 500, height: 100 })).toEqual(
      new Set(['a', 'b', 'j']),
    );
    expect(caughtBy(board, { x: 50, y: 0, width: 500, height: 500 })).toEqual(new Set(['b']));
  });

  it('finds the shape on top at a point', () => {
    const over = { ...note('over', 50, 0) };
    const stacked = withShape(board, over);
    expect(shapeAt(stacked, { x: 60, y: 10 })?.id).toBe('over');
    expect(shapeAt(stacked, { x: 10, y: 10 })?.id).toBe('a');
    expect(shapeAt(stacked, { x: 250, y: 150 })).toBeNull();
  });

  it('knows the box of everything', () => {
    expect(boardBox(board)).toEqual({ x: 0, y: 0, width: 400, height: 300 });
    expect(boardBox(EMPTY_BOARD)).toBeNull();
  });
});

describe('changes', () => {
  it('adds a shape on top, a note in the plain tint', () => {
    const shape = newShape('n', 'note', { x: 1, y: 2, width: 3, height: 4 });
    expect(shape.tint).toBe('plain');
    expect(newShape('t', 'text', { x: 0, y: 0, width: 1, height: 1 }).tint).toBeUndefined();
    expect(withShape(board, shape).shapes.at(-1)).toBe(shape);
  });

  it('moves the selected shapes in whole pixels, and nothing for no move', () => {
    const next = moved(board, new Set(['a', 'c']), 10.4, -5.6);
    expect(next.shapes[0]).toMatchObject({ x: 10, y: -6 });
    expect(next.shapes[1]).toBe(b);
    expect(next.shapes[2]).toMatchObject({ x: 10, y: 194 });
    expect(moved(board, new Set(['a']), 0.2, 0)).toBe(board);
  });

  it('resizes a shape, and nothing when the box is the same', () => {
    expect(resized(board, 'a', { x: 0, y: 0, width: 150, height: 80 }).shapes[0]?.width).toBe(150);
    expect(resized(board, 'a', { x: 0, y: 0, width: 100, height: 80 })).toBe(board);
  });

  it('changes words, and grows a shape to the height its words need', () => {
    const next = withText(board, 'a', 'Buy sensors', 120);
    expect(next.shapes[0]).toMatchObject({ text: 'Buy sensors', height: 120 });
    expect(withText(board, 'a', 'Short', 10).shapes[0]?.height).toBe(80);
    expect(withText(board, 'a', 'a')).toBe(board);
  });

  it('tints notes and leaves the other shapes', () => {
    const next = tinted(board, new Set(['a', 'c']), 'green');
    expect(next.shapes[0]?.tint).toBe('green');
    expect(next.shapes[2]).toBe(c);
  });

  it('makes a note a task, and nothing else', () => {
    expect(withTask(board, 'a', 't1').shapes[0]?.taskId).toBe('t1');
    expect(withTask(board, 'c', 't1')).toBe(board);
  });

  it('removes shapes with their joins, and joins alone', () => {
    expect(removed(board, new Set(['b']))).toEqual({ shapes: [a, c], joins: [] });
    expect(removed(board, new Set(['j']))).toEqual({ shapes: [a, b, c], joins: [] });
    expect(removed(board, new Set(['a', 'b', 'c']))).toBe(EMPTY_BOARD);
    expect(removed(board, new Set(['unknown']))).toBe(board);
  });

  it('duplicates shapes and the joins between them, a little away, without the task', () => {
    const withLink = withTask(board, 'a', 't1');
    const { board: next, copies } = duplicated(withLink, new Set(['a', 'b', 'c', 'j']), counter());
    expect(next.shapes).toHaveLength(6);
    const copyOfA = next.shapes[3];
    expect(copyOfA).toMatchObject({ id: 'new1', x: DUPLICATE_OFFSET, y: DUPLICATE_OFFSET });
    expect(copyOfA?.taskId).toBeUndefined();
    expect(next.joins.at(-1)).toEqual({ id: 'new4', from: 'new1', to: 'new2', isArrow: true });
    expect(copies).toEqual(new Set(['new1', 'new2', 'new3', 'new4']));
  });

  it('duplicates no join that leaves the selection', () => {
    const { board: next } = duplicated(board, new Set(['a']), counter());
    expect(next.joins).toHaveLength(1);
    expect(duplicated(board, new Set(), counter()).board).toBe(board);
  });

  it('joins two shapes once, either way round', () => {
    const line = { id: 'k', from: 'a', to: 'c', isArrow: false };
    expect(joined(board, line).joins).toHaveLength(2);
    expect(joined(board, { ...line, to: 'a' })).toBe(board);
    expect(joined(board, { ...line, to: 'gone' })).toBe(board);
    // The arrow a→b exists: the same again changes nothing.
    expect(joined(board, { id: 'k', from: 'a', to: 'b', isArrow: true })).toBe(board);
    // b→a as a line turns the arrow round and into a line, keeping its id.
    expect(joined(board, { id: 'k', from: 'b', to: 'a', isArrow: false }).joins).toEqual([
      { id: 'j', from: 'b', to: 'a', isArrow: false },
    ]);
  });
});

describe('the selection', () => {
  it('selects everything, and forgets what is gone', () => {
    expect(everything(board)).toEqual(new Set(['a', 'b', 'c', 'j']));
    const selection = new Set(['a', 'gone']);
    expect(stillThere(board, selection)).toEqual(new Set(['a']));
    const kept = new Set(['a']);
    expect(stillThere(board, kept)).toBe(kept);
  });

  it('toggles one thing', () => {
    expect(toggled(new Set(['a']), 'b')).toEqual(new Set(['a', 'b']));
    expect(toggled(new Set(['a']), 'a')).toEqual(new Set());
  });
});

describe('undo and redo', () => {
  it('steps back and forward, and a new change forgets what was undone', () => {
    const first = withText(board, 'a', 'one');
    const second = withText(first, 'a', 'two');
    let history = recorded(recorded(historyOf(board), first), second);
    expect(history.present).toBe(second);
    history = undone(history);
    expect(history.present).toBe(first);
    history = undone(history);
    expect(history.present).toBe(board);
    expect(undone(history)).toBe(history);
    history = redone(history);
    expect(history.present).toBe(first);
    const other = withText(first, 'a', 'other');
    history = recorded(history, other);
    expect(history.future).toEqual([]);
    expect(redone(history)).toBe(history);
  });

  it('makes no step of a change that changes nothing', () => {
    const history = historyOf(board);
    expect(recorded(history, board)).toBe(history);
  });

  it('remembers a hundred steps', () => {
    let history = historyOf(board);
    for (let step = 0; step < 150; step += 1) {
      history = recorded(history, withText(history.present, 'a', step.toString()));
    }
    expect(history.past).toHaveLength(100);
  });
});
