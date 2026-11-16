import {
  Circle,
  Ellipsis,
  MousePointer2,
  MoveUpRight,
  Slash,
  Square,
  StickyNote,
  Type,
} from 'lucide-react';
import {
  memo,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type CSSProperties,
  type FocusEvent,
  type KeyboardEvent,
  type MouseEvent,
  type PointerEvent,
  type RefObject,
} from 'react';

import { useNavigation } from '@/app/navigation';
import type { Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { taskReference } from '@/shared/lib/format';
import { GroundControls, GroundFrame, useGround } from '@/shared/ui/Ground';
import type { Box, Point } from '@/shared/ui/groundModel';
import { ActionMenu, type Action } from '@/shared/ui/Menu';
import { Tooltip } from '@/shared/ui/Tooltip';

import { StatusMark } from '../tasks/TaskBadges';
import { statusOf } from '../tasks/taskModel';
import { useTaskCommands } from '../tasks/taskQueries';
import type { KindProps } from '../views/viewKinds';
import { useKeptBoard } from './useKeptBoard';
import styles from './WhiteboardView.module.css';
import {
  CORNERS,
  NOTHING_SELECTED,
  TINTS,
  arrowhead,
  boardBox,
  boxAround,
  boxBetween,
  boxOf,
  boxOfNewShape,
  caughtBy,
  duplicated,
  everything,
  joinEnds,
  joined,
  moved,
  newShape,
  removed,
  resized,
  resizedBox,
  roomNote,
  shapeAt,
  stillThere,
  tinted,
  toggled,
  withShape,
  withTask,
  withText,
  edgePoint,
  type Board,
  type Corner,
  type Selection,
  type Shape,
  type ShapeKind,
  type Tint,
} from './whiteboardModel';

/** What a press on the ground does: select and move, make a shape, or join two. */
type Tool = 'select' | ShapeKind | 'line' | 'arrow';

/** A drag under way. */
type Gesture =
  | { kind: 'make'; shape: ShapeKind; start: Point; now: Point }
  | { kind: 'frame'; start: Point; now: Point; base: Selection }
  | { kind: 'move'; id: string; start: Point; now: Point; ids: Selection }
  | { kind: 'resize'; id: string; corner: Corner; start: Point; now: Point }
  | { kind: 'join'; from: string; now: Point; isArrow: boolean };

/** Words being written into a shape, and the height they need so far. */
interface Writing {
  id: string;
  text: string;
  height: number;
}

interface ToolInfo {
  tool: Tool;
  label: string;
  key: string;
  icon: typeof Square;
}

const TOOLS: readonly ToolInfo[] = [
  { tool: 'select', label: 'Select and move', key: 'V', icon: MousePointer2 },
  { tool: 'note', label: 'Note', key: 'N', icon: StickyNote },
  { tool: 'text', label: 'Text', key: 'T', icon: Type },
  { tool: 'box', label: 'Box', key: 'R', icon: Square },
  { tool: 'ellipse', label: 'Ellipse', key: 'O', icon: Circle },
  { tool: 'line', label: 'Line between two things', key: 'L', icon: Slash },
  { tool: 'arrow', label: 'Arrow from one thing to another', key: 'A', icon: MoveUpRight },
];

const TINT_NAMES: Readonly<Record<Tint, string>> = {
  plain: 'Plain',
  amber: 'Amber',
  green: 'Green',
  blue: 'Blue',
  purple: 'Purple',
  pink: 'Pink',
  teal: 'Teal',
};

const KIND_NAMES: Readonly<Record<ShapeKind, string>> = {
  note: 'Note',
  text: 'Text',
  box: 'Box',
  ellipse: 'Ellipse',
};

/** The distance between the dots of the ground, in pixels of the ground. */
const GRID = 24;

/** How long the strokes of an arrowhead are on screen, in pixels. */
const ARROWHEAD = 9;

/** How far an arrow key nudges what is selected, in pixels of the ground; with Shift, ten times. */
const NUDGE = 1;

/** The box the ground is given while the board is empty: its origin, in the middle of the frame. */
const NOTHING_DRAWN: Box = { x: 0, y: 0, width: 0, height: 0 };

/** A fresh id for a thing on the board: twelve letters of a random UUID. */
function freshId(): string {
  return crypto.randomUUID().replaceAll('-', '').slice(0, 12);
}

/** Whether something is a field the manager types in. */
function isField(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || target.matches('input, textarea, select'))
  );
}

/** The id of the thing an event happened on, if it happened on one. */
function itemOf(target: EventTarget): string | null {
  if (!(target instanceof Element)) {
    return null;
  }
  return target.closest<HTMLElement | SVGElement>('[data-item]')?.dataset.item ?? null;
}

/** Gives an element the focus without the browser moving anything to show it. */
function focusQuietly(element: HTMLElement | null) {
  element?.focus({ preventScroll: true });
}

function sameBox(one: Box, other: Box): boolean {
  return (
    one.x === other.x &&
    one.y === other.y &&
    one.width === other.width &&
    one.height === other.height
  );
}

function isShapeTool(tool: Tool): tool is ShapeKind {
  return tool === 'note' || tool === 'text' || tool === 'box' || tool === 'ellipse';
}

/** The board as it looks during a drag: moved or resized, not yet kept. */
function underGesture(board: Board, gesture: Gesture | null): Board {
  if (gesture?.kind === 'move') {
    return moved(
      board,
      gesture.ids,
      gesture.now.x - gesture.start.x,
      gesture.now.y - gesture.start.y,
    );
  }
  if (gesture?.kind === 'resize') {
    const shape = board.shapes.find((each) => each.id === gesture.id);
    if (shape) {
      return resized(
        board,
        shape.id,
        resizedBox(
          boxOf(shape),
          gesture.corner,
          gesture.now.x - gesture.start.x,
          gesture.now.y - gesture.start.y,
        ),
      );
    }
  }
  return board;
}

/**
 * A free ground for notes and sketches: notes in quiet tints, words, boxes
 * and ellipses, joined by lines and arrows that follow them. It lies on a
 * ground that is moved and zoomed like the mind map's, without edges.
 *
 * A tool in the bar at the top makes a thing with a press or a drag, and
 * then goes back to selecting. A double press, or Enter, writes in a thing.
 * A note can become a task. Everything is kept in the content of the view,
 * a second after the last change; undo and redo work within the session.
 */
export function WhiteboardView({ view, project, everyTask, assigneeOfNewTasks }: KindProps) {
  const navigation = useNavigation();
  const commands = useTaskCommands(project.id);
  const kept = useKeptBoard(view);
  const { board } = kept;
  const viewRef = useRef<HTMLDivElement>(null);
  const sheet = useRef<HTMLDivElement>(null);
  const writingField = useRef<HTMLTextAreaElement>(null);
  const isSpaceHeld = useRef(false);
  const [isPanning, setIsPanning] = useState(false);
  const [tool, setTool] = useState<Tool>('select');
  const [chosen, setSelection] = useState<Selection>(NOTHING_SELECTED);
  const [gesture, setGesture] = useState<Gesture | null>(null);
  const [writing, setWriting] = useState<Writing | null>(null);
  const selection = stillThere(board, chosen);

  const taskById = useMemo(() => new Map(everyTask.map((task) => [task.id, task])), [everyTask]);

  // The ground is given the same box as long as its sides are the same.
  const drawn = boardBox(board) ?? NOTHING_DRAWN;
  const [content, setContent] = useState(drawn);
  if (!sameBox(content, drawn)) {
    setContent(drawn);
  }
  const ground = useGround({
    content,
    reach: null,
    fitsOnce: true,
    takesPress: (event) => event.button === 1 || isSpaceHeld.current,
  });
  const { placement } = ground;

  const shown = underGesture(board, gesture);
  const shownSelection: Selection =
    gesture?.kind === 'frame'
      ? new Set([...gesture.base, ...caughtBy(board, boxBetween(gesture.start, gesture.now))])
      : selection;
  const selectedShapes = shown.shapes.filter((shape) => shownSelection.has(shape.id));
  const single =
    selectedShapes.length === 1 && shownSelection.size === 1 ? selectedShapes[0] : undefined;

  // Space held down moves the ground under the pointer, as in a drawing program.
  useEffect(() => {
    const down = (event: globalThis.KeyboardEvent) => {
      if (event.key === ' ' && !event.repeat && !isField(event.target)) {
        isSpaceHeld.current = true;
        setIsPanning(true);
      }
    };
    const up = (event: globalThis.KeyboardEvent) => {
      if (event.key === ' ') {
        isSpaceHeld.current = false;
        setIsPanning(false);
      }
    };
    const leave = () => {
      isSpaceHeld.current = false;
      setIsPanning(false);
    };
    window.addEventListener('keydown', down);
    window.addEventListener('keyup', up);
    window.addEventListener('blur', leave);
    return () => {
      window.removeEventListener('keydown', down);
      window.removeEventListener('keyup', up);
      window.removeEventListener('blur', leave);
    };
  }, []);

  // The field to write in takes the focus when it opens, with the caret at the end.
  const writingId = writing?.id;
  useEffect(() => {
    const field = writingField.current;
    if (writingId !== undefined && field) {
      focusQuietly(field);
      field.setSelectionRange(field.value.length, field.value.length);
    }
  }, [writingId]);

  function focusItem(id: string) {
    focusQuietly(
      sheet.current?.querySelector<HTMLElement>(`[data-item="${CSS.escape(id)}"]`) ?? null,
    );
  }

  function startWriting(id: string) {
    const shape = board.shapes.find((each) => each.id === id);
    if (shape) {
      setSelection(new Set([id]));
      setWriting({ id, text: shape.text, height: shape.height });
    }
  }

  function stopWriting(returnsFocus: boolean) {
    if (!writing) {
      return;
    }
    const { id, text, height } = writing;
    setWriting(null);
    kept.change((current) => {
      const shape = current.shapes.find((each) => each.id === id);
      // A text without words would be nothing to see: it goes.
      if (shape?.kind === 'text' && text.trim() === '') {
        return removed(current, new Set([id]));
      }
      return withText(current, id, text, height);
    });
    if (returnsFocus) {
      focusItem(id);
    }
  }

  /** Makes a shape in a box, selects it, and opens a note or a text for writing. */
  function make(kind: ShapeKind, box: Box) {
    const id = freshId();
    const shape = newShape(id, kind, box);
    kept.change((current) => withShape(current, shape));
    setSelection(new Set([id]));
    setTool('select');
    if (kind === 'note' || kind === 'text') {
      setWriting({ id, text: '', height: shape.height });
    }
  }

  /** The point of the ground in the middle of the frame, where the keyboard makes things. */
  function middleOfFrame(): Point {
    const box = viewRef.current?.getBoundingClientRect();
    return (
      (box && ground.pointAt(box.left + box.width / 2, box.top + box.height / 2)) ?? { x: 0, y: 0 }
    );
  }

  /** A tool chosen from the bar: with the pointer it waits for a press; with a key it acts at once. */
  function chooseTool(next: Tool, byKeyboard: boolean) {
    if (!byKeyboard || next === 'select') {
      setTool(next);
      return;
    }
    if (isShapeTool(next)) {
      const middle = middleOfFrame();
      const box = boxOfNewShape(next, middle, middle);
      make(next, {
        ...box,
        x: box.x - Math.round(box.width / 2),
        y: box.y - Math.round(box.height / 2),
      });
      return;
    }
    // A line from the keyboard joins the two shapes selected, in the order they were chosen.
    const ids = [...selection].filter((id) => board.shapes.some((shape) => shape.id === id));
    const [from, to] = ids;
    if (ids.length === 2 && from !== undefined && to !== undefined) {
      const id = freshId();
      kept.change((current) => joined(current, { id, from, to, isArrow: next === 'arrow' }));
      setSelection(new Set([id]));
    } else {
      setTool(next);
    }
  }

  function remove(ids: Selection) {
    kept.change((current) => removed(current, ids));
    setSelection(NOTHING_SELECTED);
    focusQuietly(viewRef.current?.querySelector<HTMLElement>('[data-frame]') ?? null);
  }

  function duplicate(ids: Selection) {
    let copies: Set<string> = new Set();
    kept.change((current) => {
      const result = duplicated(current, ids, freshId);
      copies = result.copies;
      return result.board;
    });
    if (copies.size > 0) {
      setSelection(copies);
    }
  }

  async function makeTask(shape: Shape) {
    const [first = '', ...rest] = shape.text.trim().split('\n');
    const description = rest.join('\n').trim();
    const created = await commands.create({
      title: first.trim() || 'A note from the whiteboard',
      assignee: assigneeOfNewTasks,
      ...(description === '' ? {} : { description }),
    });
    if (created) {
      kept.change((current) => withTask(current, shape.id, created.id));
    }
  }

  function handlePointerDown(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0 || isSpaceHeld.current || gesture) {
      return;
    }
    const target = event.target;
    // A press in the menu of a shape reaches the frame through React, from
    // elsewhere on the page: it is the menu's.
    if (
      !(target instanceof Element) ||
      !event.currentTarget.contains(target) ||
      target.closest('button, textarea')
    ) {
      return;
    }
    const point = ground.pointAt(event.clientX, event.clientY);
    if (!point) {
      return;
    }
    const at = { start: point, now: point };
    const corner = target.closest<HTMLElement>('[data-corner]')?.dataset.corner as
      Corner | undefined;
    const id = itemOf(target);
    const shape = id === null ? undefined : board.shapes.find((each) => each.id === id);

    if (corner && single) {
      setGesture({ kind: 'resize', id: single.id, corner, ...at });
    } else if (isShapeTool(tool)) {
      setGesture({ kind: 'make', shape: tool, ...at });
    } else if (tool === 'line' || tool === 'arrow') {
      if (shape) {
        setGesture({ kind: 'join', from: shape.id, now: point, isArrow: tool === 'arrow' });
      } else {
        return;
      }
    } else if (id !== null) {
      let next = selection;
      if (event.shiftKey) {
        next = toggled(selection, id);
      } else if (!selection.has(id)) {
        next = new Set([id]);
      }
      setSelection(next);
      if (shape && next.has(id)) {
        setGesture({ kind: 'move', id, ...at, ids: next });
      }
    } else {
      const base = event.shiftKey ? selection : NOTHING_SELECTED;
      setSelection(base);
      setGesture({ kind: 'frame', ...at, base });
    }
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function handlePointerMove(event: PointerEvent<HTMLDivElement>) {
    if (!gesture) {
      return;
    }
    const point = ground.pointAt(event.clientX, event.clientY);
    if (point) {
      setGesture({ ...gesture, now: point });
    }
  }

  function handlePointerUp(event: PointerEvent<HTMLDivElement>) {
    if (!gesture) {
      return;
    }
    setGesture(null);
    if (event.type === 'pointercancel') {
      return;
    }
    if (gesture.kind === 'make') {
      make(gesture.shape, boxOfNewShape(gesture.shape, gesture.start, gesture.now));
    } else if (gesture.kind === 'frame') {
      setSelection(shownSelection);
    } else if (gesture.kind === 'move' || gesture.kind === 'resize') {
      const after = kept.change(() => shown);
      if (after === board && gesture.kind === 'move' && !event.shiftKey && selection.size > 1) {
        // A press without a move on one of several selected things selects that one.
        setSelection(new Set([gesture.id]));
      }
    } else {
      const target = shapeAt(board, gesture.now);
      if (target && target.id !== gesture.from) {
        const id = freshId();
        kept.change((current) =>
          joined(current, { id, from: gesture.from, to: target.id, isArrow: gesture.isArrow }),
        );
        setSelection(new Set([id]));
        setTool('select');
      }
    }
  }

  function handleDoubleClick(event: MouseEvent<HTMLDivElement>) {
    // The pointer was held by the frame during the presses, so the double
    // press is found by where it was, not by what it reached.
    const point = ground.pointAt(event.clientX, event.clientY);
    const shape = point && shapeAt(board, point);
    if (
      shape &&
      tool === 'select' &&
      !writing &&
      event.currentTarget.contains(event.target as Node)
    ) {
      startWriting(shape.id);
    }
  }

  function handleFocus(event: FocusEvent<HTMLDivElement>) {
    // A thing the Tab key stops at is brought into view and selected.
    const element: EventTarget = event.target;
    if (!(element instanceof HTMLElement) || !element.matches('[data-item]:focus-visible')) {
      return;
    }
    const shape = board.shapes.find((each) => each.id === element.dataset.item);
    if (shape && !selection.has(shape.id)) {
      ground.reveal(boxOf(shape));
      setSelection(new Set([shape.id]));
    }
  }

  function handleKeyDownCapture(event: KeyboardEvent<HTMLDivElement>) {
    const target = event.target;
    // A key in a menu, which lies elsewhere on the page, or in a field, is theirs.
    if (
      !(target instanceof Node) ||
      !event.currentTarget.contains(target) ||
      isField(target) ||
      event.altKey
    ) {
      return;
    }
    const isButton = target instanceof Element && target.closest('button') !== null;
    const withCommand = event.ctrlKey || event.metaKey;
    const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
    const shapes = new Set(selectedShapes.map((shape) => shape.id));
    const step = event.shiftKey ? NUDGE * 10 : NUDGE;
    const nudges: Partial<Record<string, [number, number]>> = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -step],
      ArrowDown: [0, step],
    };
    const nudge = nudges[event.key];

    if (withCommand && key === 'z') {
      setSelection(NOTHING_SELECTED);
      if (event.shiftKey) {
        kept.redo();
      } else {
        kept.undo();
      }
    } else if (withCommand && key === 'y') {
      kept.redo();
    } else if (withCommand && key === 'd') {
      duplicate(selection);
    } else if (withCommand && key === 'a') {
      setSelection(everything(board));
    } else if (withCommand) {
      return;
    } else if (event.key === 'Escape') {
      if (gesture) {
        setGesture(null);
      } else if (tool !== 'select') {
        setTool('select');
      } else if (selection.size > 0) {
        setSelection(NOTHING_SELECTED);
      } else {
        return;
      }
    } else if ((event.key === 'Delete' || event.key === 'Backspace') && selection.size > 0) {
      remove(selection);
    } else if (event.key === 'Enter' && !isButton && single) {
      startWriting(single.id);
    } else if (nudge && shapes.size > 0) {
      kept.change((current) => moved(current, shapes, nudge[0], nudge[1]));
    } else if (event.key === ' ' && !isButton) {
      // Space is held to move the ground; it must not scroll anything.
    } else {
      const chosenTool = TOOLS.find((each) => each.key.toLowerCase() === key && !event.shiftKey);
      if (!chosenTool) {
        return;
      }
      setTool(chosenTool.tool);
    }
    event.preventDefault();
  }

  function changeWriting(event: ChangeEvent<HTMLTextAreaElement>) {
    const field = event.target;
    setWriting((current) =>
      current
        ? {
            ...current,
            text: field.value,
            height: Math.max(current.height, field.scrollHeight),
          }
        : current,
    );
  }

  const room = roomNote(kept.bytes);
  const isEmpty = board.shapes.length === 0 && !gesture;
  const scale = placement.scale;
  // The dots of the ground: every 24 pixels, every 48 when they would crowd.
  const gridStep = GRID * scale * (scale < 0.5 ? 2 : 1);
  const selectedBox = selectedShapes.length > 1 ? boxAround(selectedShapes.map(boxOf)) : null;
  const makingBox =
    gesture?.kind === 'make' ? boxOfNewShape(gesture.shape, gesture.start, gesture.now) : null;
  const framing = gesture?.kind === 'frame' ? boxBetween(gesture.start, gesture.now) : null;

  return (
    <div ref={viewRef} className={styles.view} onKeyDownCapture={handleKeyDownCapture}>
      <GroundFrame
        ground={ground}
        className={styles.frame}
        tabIndex={-1}
        data-frame
        data-tool={tool}
        data-panning={isPanning || undefined}
        aria-label={`Whiteboard ${view.name}`}
        style={
          {
            '--grid-step': `${gridStep.toString()}px`,
            '--grid-x': `${Math.round(placement.x).toString()}px`,
            '--grid-y': `${Math.round(placement.y).toString()}px`,
          } as CSSProperties
        }
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerUp}
        onDoubleClick={handleDoubleClick}
        onFocus={handleFocus}
      >
        <div ref={sheet} className={styles.sheet}>
          <Joins board={shown} selection={shownSelection} gesture={gesture} scale={scale} />

          {shown.shapes.map((shape) => (
            <ShapeView
              key={shape.id}
              shape={shape}
              project={project}
              task={shape.taskId === undefined ? undefined : taskById.get(shape.taskId)}
              isSelected={shownSelection.has(shape.id)}
              isTarget={
                gesture?.kind === 'join' &&
                gesture.from !== shape.id &&
                shapeAt(board, gesture.now)?.id === shape.id
              }
              writing={writing?.id === shape.id ? writing : null}
              writingField={writingField}
              onWrite={changeWriting}
              onStopWriting={stopWriting}
              onOpenTask={navigation.openTask}
            />
          ))}

          {single && !writing && !gesture && (
            <>
              {CORNERS.map((corner) => (
                <span
                  key={corner}
                  className={styles.grip}
                  data-corner={corner}
                  style={{
                    left: corner.endsWith('left') ? single.x : single.x + single.width,
                    top: corner.startsWith('top') ? single.y : single.y + single.height,
                  }}
                />
              ))}
              <ShapeMenu
                shape={single}
                project={project}
                task={single.taskId === undefined ? undefined : taskById.get(single.taskId)}
                onOpenTask={navigation.openTask}
                onMakeTask={makeTask}
                onWrite={startWriting}
                onDuplicate={(id) => {
                  duplicate(new Set([id]));
                }}
                onDelete={(id) => {
                  remove(new Set([id]));
                }}
                onTint={(id, tint) => {
                  kept.change((current) => tinted(current, new Set([id]), tint));
                }}
              />
            </>
          )}

          {selectedBox && !gesture && (
            <span
              className={styles.around}
              style={{
                left: selectedBox.x,
                top: selectedBox.y,
                width: selectedBox.width,
                height: selectedBox.height,
              }}
            />
          )}

          {makingBox && (
            <span
              className={classNames(
                styles.making,
                gesture?.kind === 'make' && styles[gesture.shape],
              )}
              style={{
                left: makingBox.x,
                top: makingBox.y,
                width: makingBox.width,
                height: makingBox.height,
              }}
            />
          )}

          {framing && (
            <span
              className={styles.framing}
              style={{
                left: framing.x,
                top: framing.y,
                width: framing.width,
                height: framing.height,
              }}
            />
          )}
        </div>
      </GroundFrame>

      <div className={styles.tools} role="toolbar" aria-label="Tools">
        {TOOLS.map((each, index) => (
          <Tooltip key={each.tool} label={`${each.label} (${each.key})`} side="bottom">
            <button
              type="button"
              className={classNames(styles.tool, index === 1 && styles.afterSelect)}
              aria-label={each.label}
              aria-pressed={tool === each.tool}
              aria-keyshortcuts={each.key}
              onClick={(event) => {
                chooseTool(each.tool, event.detail === 0);
              }}
            >
              <each.icon aria-hidden />
            </button>
          </Tooltip>
        ))}
      </div>

      {isEmpty && (
        <div className={styles.empty}>
          <p className={styles.emptyLead}>A free ground for notes and sketches.</p>
          <p className={styles.emptyNote}>
            Choose Note in the bar above and press anywhere to pin the first one. Draw a box around
            a group, join two things with an arrow, and turn a note into a task when it is ready.
          </p>
        </div>
      )}

      <div className={styles.foot}>
        <GroundControls ground={ground} />
        {room !== null && (
          <p className={styles.room} role="status">
            {room}
          </p>
        )}
      </div>
    </div>
  );
}

interface JoinsProps {
  board: Board;
  selection: Selection;
  gesture: Gesture | null;
  scale: number;
}

/**
 * The lines and arrows of the board, under its shapes. Each leaves its
 * first shape at the edge and meets the second at the edge, so it follows
 * them wherever they go. A wide clear stroke under each makes it easy to
 * press.
 */
function Joins({ board, selection, gesture, scale }: JoinsProps) {
  const byId = new Map(board.shapes.map((shape) => [shape.id, shape]));
  const head = ARROWHEAD / scale;
  const lines = board.joins.flatMap((join) => {
    const from = byId.get(join.from);
    const to = byId.get(join.to);
    return from && to ? [{ join, ...joinEnds(from, to) }] : [];
  });
  let drawing: { start: Point; end: Point; isArrow: boolean } | null = null;
  if (gesture?.kind === 'join') {
    const from = byId.get(gesture.from);
    const over = shapeAt(board, gesture.now);
    if (from) {
      drawing =
        over && over.id !== from.id
          ? { ...joinEnds(from, over), isArrow: gesture.isArrow }
          : { start: edgePoint(from, gesture.now), end: gesture.now, isArrow: gesture.isArrow };
    }
  }
  const points = [
    ...board.shapes.map(boxOf),
    ...(drawing ? [{ ...drawing.end, width: 0, height: 0 }] : []),
  ];
  const around = boxAround(points);
  if (!around || (lines.length === 0 && !drawing)) {
    return null;
  }
  const margin = 40;
  const box = {
    x: around.x - margin,
    y: around.y - margin,
    width: around.width + 2 * margin,
    height: around.height + 2 * margin,
  };

  function arrowPath(start: Point, end: Point): string {
    const strokes = arrowhead(start, end, head);
    if (!strokes) {
      return '';
    }
    const [one, other] = strokes;
    return `M ${one.x.toString()} ${one.y.toString()} L ${end.x.toString()} ${end.y.toString()} L ${other.x.toString()} ${other.y.toString()}`;
  }

  function linePath(start: Point, end: Point): string {
    return `M ${start.x.toString()} ${start.y.toString()} L ${end.x.toString()} ${end.y.toString()}`;
  }

  return (
    <svg
      className={styles.joins}
      style={{ left: box.x, top: box.y }}
      width={box.width}
      height={box.height}
      viewBox={`${box.x.toString()} ${box.y.toString()} ${box.width.toString()} ${box.height.toString()}`}
    >
      {lines.map(({ join, start, end }) => (
        <g
          key={join.id}
          className={classNames(styles.join, selection.has(join.id) && styles.joinSelected)}
        >
          <path className={styles.hit} d={linePath(start, end)} data-item={join.id} />
          <path d={linePath(start, end)} />
          {join.isArrow && <path d={arrowPath(start, end)} />}
        </g>
      ))}
      {drawing && (
        <g className={classNames(styles.join, styles.joinDrawing)}>
          <path d={linePath(drawing.start, drawing.end)} />
          {drawing.isArrow && <path d={arrowPath(drawing.start, drawing.end)} />}
        </g>
      )}
    </svg>
  );
}

interface ShapeViewProps {
  shape: Shape;
  project: Project;
  /** The task the note became, while it is there. */
  task: Task | undefined;
  isSelected: boolean;
  /** Whether a line being drawn would end on this shape. */
  isTarget: boolean;
  writing: Writing | null;
  writingField: RefObject<HTMLTextAreaElement | null>;
  onWrite: (event: ChangeEvent<HTMLTextAreaElement>) => void;
  onStopWriting: (returnsFocus: boolean) => void;
  onOpenTask: (taskId: string) => void;
}

/**
 * One shape, drawn by what it is: a note as a tinted card, a text in the
 * serif, a box as a hairline frame with its name in the corner like the
 * parts of a drawing, an ellipse as a hairline ring. A note that became a
 * task carries the task's status and key above its words.
 */
const ShapeView = memo(function ShapeView({
  shape,
  project,
  task,
  isSelected,
  isTarget,
  writing,
  writingField,
  onWrite,
  onStopWriting,
  onOpenTask,
}: ShapeViewProps) {
  const status = task ? statusOf(project, task) : undefined;
  const reference = task ? taskReference(project, task) : undefined;
  const height = writing ? Math.max(shape.height, writing.height) : shape.height;
  const words = shape.text.trim() === '' ? 'empty' : shape.text.split('\n')[0];
  return (
    <div
      className={classNames(
        styles.shape,
        styles[shape.kind],
        isSelected && styles.selected,
        isTarget && styles.target,
      )}
      data-item={shape.id}
      data-tone={shape.tint === undefined || shape.tint === 'plain' ? undefined : shape.tint}
      data-progress={status?.category}
      tabIndex={0}
      role="group"
      aria-label={`${KIND_NAMES[shape.kind]}: ${words ?? ''}${reference ? `, task ${reference}` : ''}`}
      style={{ left: shape.x, top: shape.y, width: shape.width, height }}
    >
      {task && reference && (
        <span className={styles.taskLine}>
          {status && <StatusMark status={status} />}
          <button
            type="button"
            className={styles.reference}
            title={`Open ${reference}${status ? `, ${status.name}` : ''}`}
            onClick={() => {
              onOpenTask(task.id);
            }}
          >
            {reference}
          </button>
        </span>
      )}
      {writing ? (
        <textarea
          ref={writingField}
          className={styles.writing}
          value={writing.text}
          aria-label={`Words of the ${KIND_NAMES[shape.kind].toLowerCase()}`}
          spellCheck
          onChange={onWrite}
          onBlur={() => {
            onStopWriting(false);
          }}
          onKeyDown={(event) => {
            if (
              event.key === 'Escape' ||
              (event.key === 'Enter' && (event.ctrlKey || event.metaKey))
            ) {
              event.preventDefault();
              onStopWriting(true);
            }
          }}
        />
      ) : (
        <span className={styles.words}>{shape.text}</span>
      )}
    </div>
  );
});

interface ShapeMenuProps {
  shape: Shape;
  project: Project;
  task: Task | undefined;
  onOpenTask: (taskId: string) => void;
  onMakeTask: (shape: Shape) => Promise<void>;
  onWrite: (id: string) => void;
  onDuplicate: (id: string) => void;
  onDelete: (id: string) => void;
  onTint: (id: string, tint: Tint) => void;
}

/**
 * The small menu of the one selected shape, above its top right corner:
 * for a note, its task and its tint; for every shape, writing in it,
 * duplicating it and deleting it.
 */
function ShapeMenu({
  shape,
  project,
  task,
  onOpenTask,
  onMakeTask,
  onWrite,
  onDuplicate,
  onDelete,
  onTint,
}: ShapeMenuProps) {
  const { id } = shape;
  const actions: Action[] = [];
  if (shape.kind === 'note') {
    actions.push(
      task
        ? {
            label: `Open ${taskReference(project, task)}`,
            onSelect: () => {
              onOpenTask(task.id);
            },
          }
        : {
            label: 'Make a task',
            detail: 'The first line becomes its title',
            onSelect: () => {
              void onMakeTask(shape);
            },
          },
    );
  }
  actions.push(
    {
      label: 'Write',
      onSelect: () => {
        onWrite(id);
      },
    },
    {
      label: 'Duplicate',
      onSelect: () => {
        onDuplicate(id);
      },
    },
    {
      label: 'Delete',
      isDestructive: true,
      onSelect: () => {
        onDelete(id);
      },
    },
  );
  if (shape.kind === 'note') {
    for (const tint of TINTS) {
      actions.push({
        label: TINT_NAMES[tint],
        group: 'Tint',
        icon: (
          <span
            className={styles.swatch}
            data-tone={tint === 'plain' ? undefined : tint}
            aria-hidden
          />
        ),
        isDisabled: shape.tint === tint,
        onSelect: () => {
          onTint(id, tint);
        },
      });
    }
  }
  const label = `${KIND_NAMES[shape.kind]} actions`;
  return (
    <ActionMenu label={label} actions={actions}>
      <button
        type="button"
        className={styles.more}
        style={{ left: shape.x + shape.width, top: shape.y }}
        aria-label={label}
      >
        <Ellipsis aria-hidden />
      </button>
    </ActionMenu>
  );
}
