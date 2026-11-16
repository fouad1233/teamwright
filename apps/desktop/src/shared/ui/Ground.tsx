import { Maximize, Minus, Plus } from 'lucide-react';
import {
  useEffect,
  useEffectEvent,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ComponentProps,
  type CSSProperties,
  type KeyboardEvent,
  type MouseEvent,
  type PointerEvent,
  type ReactNode,
  type UIEvent,
} from 'react';

import { classNames } from '@/shared/lib/classNames';

import styles from './Ground.module.css';
import {
  SCALE_LIMITS,
  UNMOVED,
  fittedTo,
  movedBy,
  nextScaleStop,
  percentOf,
  pointOnGround,
  revealing,
  scaleAfterWheel,
  withinReach,
  zoomedAbout,
  type Box,
  type Placement,
  type Point,
  type ScaleLimits,
  type Size,
} from './groundModel';
import { Tooltip } from './Tooltip';

/*
 * A ground is a surface larger than the frame it is seen through: the sheet
 * of a mind map, a map, a whiteboard. The manager drags it to move it, turns
 * the wheel to move it and, with Ctrl, to zoom about the pointer, and has a
 * small cluster of buttons to zoom in, to zoom out and to fit everything.
 *
 * A view uses it in three parts:
 *
 *   const ground = useGround({ content: boxOfEverything });
 *   <GroundFrame ground={ground} className={styles.frame} aria-label="…">
 *     …what lies on the ground, placed in the ground's own pixels…
 *   </GroundFrame>
 *   <GroundControls ground={ground} className={styles.controls} />
 *
 * The arithmetic is in `groundModel.ts`.
 */

/** How far the pointer must travel before a press on the ground becomes a move, in pixels. */
const MOVE_START_DISTANCE = 4;

/** How far an arrow key moves the ground, in pixels. With Shift it is four times as far. */
const KEY_STEP = 48;

/** The room "fit" leaves between the content and the edges of the frame, in pixels. */
const FIT_MARGIN = 24;

/**
 * How much of the frame the content's box always covers, across and down,
 * unless the view says otherwise: a half. An edge of the content can then
 * be dragged to the middle of the frame and no further.
 */
const REACH = 0.5;

/** How far inside the frame a thing is brought when it is revealed, in pixels. */
const REVEAL_MARGIN = 24;

/** What a line of the wheel counts for, where the wheel reports lines, in pixels. */
const PIXELS_PER_LINE = 16;

/** The buttons of a mouse a press can move the ground with: the main one and the middle one. */
const MAIN_BUTTON = 0;
const MIDDLE_BUTTON = 1;

/** What `useGround` is given. */
export interface GroundOptions {
  /**
   * The box of everything that lies on the ground, in the ground's own
   * pixels: what "fit" brings into view, and what is kept within reach.
   * Nothing for a ground that is empty or not laid out yet. Give the same
   * object as long as the box is the same.
   */
  content: Box | null;
  /** The smallest and the largest scale. Left out, a quarter to twice the size. */
  limits?: ScaleLimits;
  /**
   * The largest scale "fit" chooses. Left out, 1: content smaller than the
   * frame is shown as it is. A map that should fill its frame gives more.
   */
  largestFit?: number;
  /**
   * How much of the frame the box of the content always covers, as a share
   * of the frame's width and of its height, so that the content is not
   * dragged out of sight. Left out, a half. With 0 the content can be moved
   * until it only touches the frame. With `null` nothing holds the ground:
   * it goes wherever it is moved, as a whiteboard without edges does, and
   * "fit" brings the content back.
   */
  reach?: number | null;
  /**
   * Whether the ground fits its content only once, when it first has both
   * a content and a frame, and then stays where it lies while the content
   * changes, until "fit" is pressed. Left out, no: the ground follows its
   * content until it is moved or zoomed. A ground on which the manager
   * draws must not jump every time a thing is added.
   */
  fitsOnce?: boolean;
  /**
   * Says whether a press moves the ground. It is asked for every press of
   * the main or the middle button that reaches the frame. Left out, every
   * such press moves it. A whiteboard on which a press draws answers no,
   * except for the middle button or its hand tool. A thing on the ground
   * that is dragged itself needs none of this: it stops its own press
   * (`event.stopPropagation()`), and the ground never hears of it.
   */
  takesPress?: (event: PointerEvent<HTMLDivElement>) => boolean;
}

/** What `GroundFrame` needs of a ground. Nothing else reads it. */
interface FrameBinding {
  /** Tells the ground which element is its frame, and when that element is gone. */
  attach: (element: HTMLDivElement | null) => void;
  onPointerDown: (event: PointerEvent<HTMLDivElement>) => void;
  onPointerMove: (event: PointerEvent<HTMLDivElement>) => void;
  onPointerUp: (event: PointerEvent<HTMLDivElement>) => void;
  onPointerCancel: (event: PointerEvent<HTMLDivElement>) => void;
  onLostPointerCapture: (event: PointerEvent<HTMLDivElement>) => void;
  onMouseDown: (event: MouseEvent<HTMLDivElement>) => void;
  onClickCapture: (event: MouseEvent<HTMLDivElement>) => void;
  onKeyDown: (event: KeyboardEvent<HTMLDivElement>) => void;
  onScroll: (event: UIEvent<HTMLDivElement>) => void;
}

/** A ground: how it lies in its frame, and the ways to move it. */
export interface Ground {
  /**
   * How the ground lies in its frame: where its origin is, in pixels from
   * the top left corner of the frame, and its scale.
   */
  placement: Placement;
  /** Whether the manager is dragging the ground at this moment. */
  isMoving: boolean;
  /** Whether the ground slides to its place: after a button or a key, not under the pointer. */
  isGliding: boolean;
  canZoomIn: boolean;
  canZoomOut: boolean;
  /** Zooms to the next round scale, about the middle of the frame. */
  zoomIn: () => void;
  zoomOut: () => void;
  /** Zooms to a scale, about the middle of the frame. `zoomTo(1)` shows the ground as it is. */
  zoomTo: (scale: number) => void;
  /**
   * Brings all the content into view. The ground then follows its content
   * and its frame, as it does from the start, until it is moved or zoomed;
   * a ground that `fitsOnce` stays where the fit put it.
   */
  fit: () => void;
  /** Moves the ground by so many pixels of the frame. */
  moveBy: (dx: number, dy: number) => void;
  /** Brings a box of the ground into the frame, moving as little as needed. */
  reveal: (box: Box) => void;
  /**
   * The point of the ground under a point of the window (`clientX`,
   * `clientY` of an event). Nothing before the frame is on the page.
   */
  pointAt: (clientX: number, clientY: number) => Point | null;
  /** For `GroundFrame` alone. */
  frame: FrameBinding;
}

/** A press that may become a move of the ground. */
interface Press {
  pointerId: number;
  /** The bit of `buttons` that says the button is still down. */
  button: number;
  /**
   * Where the pointer was when the ground last followed it, in pixels of
   * the window: where it was pressed, while the press is not a move yet.
   */
  x: number;
  y: number;
  isMove: boolean;
}

/** Whether something is a field the manager types in, whose keys and presses are its own. */
function isField(target: EventTarget): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || target.matches('input, textarea, select'))
  );
}

/**
 * The state of a ground, and what moves it.
 *
 * Until the manager moves or zooms it, the ground is fitted: all its content
 * in view, in the middle of the frame, never enlarged past `largestFit`. It
 * stays fitted while the content or the frame changes. The first move or
 * zoom puts it where the manager wants it, and "fit" lets it follow again.
 *
 * The pointer: a press of the main or the middle button that reaches the
 * frame and travels four pixels drags the ground; a shorter press stays a
 * click for what was pressed. The wheel moves the ground (with Shift,
 * sideways) and zooms about the pointer with Ctrl, which is also how two
 * fingers on a touch pad arrive. The keys, while the focus is in the frame
 * and not in a field: + and - zoom, 0 fits, 1 shows the ground at 100%;
 * the arrows move the ground when the frame itself has the focus.
 */
export function useGround({
  content,
  limits = SCALE_LIMITS,
  largestFit = 1,
  reach = REACH,
  fitsOnce = false,
  takesPress,
}: GroundOptions): Ground {
  const [frame, setFrame] = useState<HTMLDivElement | null>(null);
  const [frameSize, setFrameSize] = useState<Size | null>(null);
  // Where the manager put the ground, or nothing while it is fitted.
  const [chosen, setChosen] = useState<Placement | null>(null);
  const [isGliding, setIsGliding] = useState(false);
  const [isMoving, setIsMoving] = useState(false);
  const press = useRef<Press | null>(null);
  // Whether the press that just ended moved the ground, so that the click
  // that follows it is not taken for a press on what lies under the pointer.
  const endedInMove = useRef(false);

  const fitted = useMemo(
    () =>
      content && frameSize
        ? fittedTo(content, frameSize, { margin: FIT_MARGIN, limits, largestFit })
        : UNMOVED,
    [content, frameSize, limits, largestFit],
  );
  const placement = chosen ?? fitted;

  if (fitsOnce && chosen === null && content && frameSize) {
    // The ground has fitted its content once, and now stays where it lies.
    setChosen(fitted);
  }

  useLayoutEffect(() => {
    if (!frame) {
      return;
    }
    const read = () => {
      const width = frame.clientWidth;
      const height = frame.clientHeight;
      setFrameSize((size) =>
        size?.width === width && size.height === height ? size : { width, height },
      );
    };
    read();
    const observer = new ResizeObserver(read);
    observer.observe(frame);
    return () => {
      observer.disconnect();
    };
  }, [frame]);

  /**
   * Lays the ground somewhere else. `to` gets where it lies now. With
   * `glides` it slides there; without, it is there at once, as it must be
   * under a pointer.
   */
  function change(to: (from: Placement) => Placement, glides: boolean) {
    setChosen((current) => {
      const from = current ?? fitted;
      const next = to(from);
      if (next === from) {
        return current;
      }
      return content && frameSize && reach !== null
        ? withinReach(next, content, frameSize, reach)
        : next;
    });
    setIsGliding(glides);
  }

  /** The middle of the frame, which the buttons and the keys zoom about. */
  function middle(): Point {
    return { x: (frameSize?.width ?? 0) / 2, y: (frameSize?.height ?? 0) / 2 };
  }

  function zoomTo(scale: number) {
    change((from) => zoomedAbout(from, middle(), scale, limits), true);
  }

  function zoomIn() {
    change(
      (from) => zoomedAbout(from, middle(), nextScaleStop(from.scale, 'in', limits), limits),
      true,
    );
  }

  function zoomOut() {
    change(
      (from) => zoomedAbout(from, middle(), nextScaleStop(from.scale, 'out', limits), limits),
      true,
    );
  }

  function fit() {
    setChosen(null);
    setIsGliding(true);
  }

  function moveBy(dx: number, dy: number) {
    change((from) => movedBy(from, dx, dy), false);
  }

  function reveal(box: Box) {
    change((from) => (frameSize ? revealing(from, box, frameSize, REVEAL_MARGIN) : from), true);
  }

  function pointAt(clientX: number, clientY: number): Point | null {
    if (!frame) {
      return null;
    }
    const box = frame.getBoundingClientRect();
    return pointOnGround(placement, { x: clientX - box.left, y: clientY - box.top });
  }

  // The wheel is listened to on the element itself: only there can it be
  // told not to scroll or zoom the page as well.
  const turnWheel = useEffectEvent((event: WheelEvent) => {
    if (!frame) {
      return;
    }
    event.preventDefault();
    const unit =
      event.deltaMode === WheelEvent.DOM_DELTA_LINE
        ? PIXELS_PER_LINE
        : event.deltaMode === WheelEvent.DOM_DELTA_PAGE
          ? frame.clientHeight
          : 1;
    const across = event.deltaX * unit;
    const down = event.deltaY * unit;
    if (event.ctrlKey || event.metaKey) {
      const box = frame.getBoundingClientRect();
      const point = { x: event.clientX - box.left, y: event.clientY - box.top };
      change((from) => zoomedAbout(from, point, scaleAfterWheel(from.scale, down), limits), false);
    } else if (event.shiftKey && across === 0) {
      // Some systems turn the wheel sideways themselves when Shift is held.
      change((from) => movedBy(from, -down, 0), false);
    } else {
      change((from) => movedBy(from, -across, -down), false);
    }
  });

  useEffect(() => {
    if (!frame) {
      return;
    }
    const listener = (event: WheelEvent) => {
      turnWheel(event);
    };
    frame.addEventListener('wheel', listener, { passive: false });
    return () => {
      frame.removeEventListener('wheel', listener);
    };
  }, [frame]);

  function handlePointerDown(event: PointerEvent<HTMLDivElement>) {
    endedInMove.current = false;
    if (event.button !== MAIN_BUTTON && event.button !== MIDDLE_BUTTON) {
      return;
    }
    if (isField(event.target) || !(takesPress?.(event) ?? true)) {
      return;
    }
    press.current = {
      pointerId: event.pointerId,
      // In `buttons` the main button is 1 and the middle button 4.
      button: event.button === MAIN_BUTTON ? 1 : 4,
      x: event.clientX,
      y: event.clientY,
      isMove: false,
    };
  }

  function handlePointerMove(event: PointerEvent<HTMLDivElement>) {
    const held = press.current;
    if (held?.pointerId !== event.pointerId) {
      return;
    }
    if ((event.buttons & held.button) === 0) {
      // The button was let go where the frame could not see it.
      endPress();
      return;
    }
    const dx = event.clientX - held.x;
    const dy = event.clientY - held.y;
    if (!held.isMove) {
      if (Math.hypot(dx, dy) < MOVE_START_DISTANCE) {
        return;
      }
      held.isMove = true;
      endedInMove.current = true;
      // From here on the pointer belongs to the ground, wherever it goes.
      event.currentTarget.setPointerCapture(event.pointerId);
      setIsMoving(true);
    }
    held.x = event.clientX;
    held.y = event.clientY;
    moveBy(dx, dy);
  }

  function endPress() {
    press.current = null;
    setIsMoving(false);
  }

  function handlePointerEnd(event: PointerEvent<HTMLDivElement>) {
    if (press.current?.pointerId === event.pointerId) {
      endPress();
    }
  }

  function handleMouseDown(event: MouseEvent<HTMLDivElement>) {
    // The middle button would otherwise start the browser's own scrolling.
    if (event.button === MIDDLE_BUTTON) {
      event.preventDefault();
    }
  }

  function handleClickCapture(event: MouseEvent<HTMLDivElement>) {
    if (endedInMove.current) {
      // The ground was dragged: what lay under the pointer was not pressed.
      endedInMove.current = false;
      event.preventDefault();
      event.stopPropagation();
    }
  }

  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.defaultPrevented || event.altKey || isField(event.target)) {
      return;
    }
    const step = event.shiftKey ? 4 * KEY_STEP : KEY_STEP;
    // The arrows move the ground only from the frame itself: on a thing of
    // the ground they belong to that thing.
    const isOnFrame = event.target === event.currentTarget;
    if (event.key === '+' || event.key === '=') {
      zoomIn();
    } else if (event.key === '-' || event.key === '_') {
      zoomOut();
    } else if (event.key === '0') {
      fit();
    } else if (event.key === '1') {
      zoomTo(1);
    } else if (isOnFrame && event.key === 'ArrowLeft') {
      change((from) => movedBy(from, step, 0), true);
    } else if (isOnFrame && event.key === 'ArrowRight') {
      change((from) => movedBy(from, -step, 0), true);
    } else if (isOnFrame && event.key === 'ArrowUp') {
      change((from) => movedBy(from, 0, step), true);
    } else if (isOnFrame && event.key === 'ArrowDown') {
      change((from) => movedBy(from, 0, -step), true);
    } else {
      return;
    }
    event.preventDefault();
  }

  function handleScroll(event: UIEvent<HTMLDivElement>) {
    // Where the frame can still be scrolled (an old web view), a browser
    // that brings a focused thing into view scrolls it. The ground is moved
    // by its placement alone, so the scrolling is taken back.
    const element = event.currentTarget;
    if (element.scrollLeft !== 0 || element.scrollTop !== 0) {
      element.scrollTo(0, 0);
    }
  }

  return {
    placement,
    isMoving,
    isGliding,
    canZoomIn: placement.scale < limits.largest,
    canZoomOut: placement.scale > limits.smallest,
    zoomIn,
    zoomOut,
    zoomTo,
    fit,
    moveBy,
    reveal,
    pointAt,
    frame: {
      attach: setFrame,
      onPointerDown: handlePointerDown,
      onPointerMove: handlePointerMove,
      onPointerUp: handlePointerEnd,
      onPointerCancel: handlePointerEnd,
      onLostPointerCapture: handlePointerEnd,
      onMouseDown: handleMouseDown,
      onClickCapture: handleClickCapture,
      onKeyDown: handleKeyDown,
      onScroll: handleScroll,
    },
  };
}

/** Calls the ground's handler of an event, then the view's own, if it has one. */
function both<Happening>(
  ours: (event: Happening) => void,
  theirs: ((event: Happening) => void) | undefined,
): (event: Happening) => void {
  return (event) => {
    ours(event);
    theirs?.(event);
  };
}

type GroundFrameProps = Omit<ComponentProps<'div'>, 'ref'> & {
  ground: Ground;
  /** What lies on the ground, placed in the ground's own pixels from its origin. */
  children: ReactNode;
};

/**
 * The frame a ground is seen through. It takes the size its class gives it
 * and cuts off what lies outside. Its children lie on the ground: they are
 * placed absolutely, in the ground's own pixels, and move and grow with it.
 * Inside it, `--ground-scale` is the scale, for a line or a mark that is to
 * keep its size on screen: `calc(1px / var(--ground-scale))`.
 *
 * Give it `tabIndex={0}` to put the ground itself among the things the Tab
 * key reaches, where the arrow keys then move it, or `tabIndex={-1}` to let
 * a press on the ground take the focus, so that the keys work after it.
 * Every other property goes to its element; a handler of the pointer, of a
 * click or of a key runs after the ground's own.
 */
export function GroundFrame({
  ground,
  className,
  children,
  onPointerDown,
  onPointerMove,
  onPointerUp,
  onPointerCancel,
  onLostPointerCapture,
  onMouseDown,
  onClickCapture,
  onKeyDown,
  onScroll,
  ...rest
}: GroundFrameProps) {
  const { placement, frame } = ground;
  // Taken out by name: the rule about refs reads `ref={frame.attach}` as
  // reading a ref while rendering, and then distrusts all of `frame`.
  const { attach } = frame;
  return (
    <div
      {...rest}
      ref={attach}
      className={classNames(styles.frame, className)}
      data-moving={ground.isMoving || undefined}
      onPointerDown={both(frame.onPointerDown, onPointerDown)}
      onPointerMove={both(frame.onPointerMove, onPointerMove)}
      onPointerUp={both(frame.onPointerUp, onPointerUp)}
      onPointerCancel={both(frame.onPointerCancel, onPointerCancel)}
      onLostPointerCapture={both(frame.onLostPointerCapture, onLostPointerCapture)}
      onMouseDown={both(frame.onMouseDown, onMouseDown)}
      onClickCapture={both(frame.onClickCapture, onClickCapture)}
      onKeyDown={both(frame.onKeyDown, onKeyDown)}
      onScroll={both(frame.onScroll, onScroll)}
    >
      <div
        className={styles.ground}
        data-gliding={ground.isGliding || undefined}
        style={
          {
            // Whole pixels, so that hairlines stay sharp at a scale of 1.
            transform: `translate(${Math.round(placement.x).toString()}px, ${Math.round(placement.y).toString()}px) scale(${placement.scale.toString()})`,
            '--ground-scale': placement.scale,
          } as CSSProperties
        }
      >
        {children}
      </div>
    </div>
  );
}

interface GroundControlsProps {
  ground: Ground;
  /** Places the cluster in the view: usually in a corner, over the frame. */
  className?: string;
}

/**
 * The buttons of a ground, set in one block like the cells of a title
 * block: zoom out, the scale (a press shows the ground at 100%), zoom in,
 * and fit everything. Each says its key when the pointer rests on it.
 */
export function GroundControls({ ground, className }: GroundControlsProps) {
  const scale = percentOf(ground.placement.scale);
  return (
    <div className={classNames(styles.controls, className)} role="group" aria-label="Zoom">
      <Tooltip label="Zoom out (-)" side="top">
        <button
          type="button"
          className={styles.control}
          aria-label="Zoom out"
          aria-keyshortcuts="-"
          disabled={!ground.canZoomOut}
          onClick={ground.zoomOut}
        >
          <Minus aria-hidden />
        </button>
      </Tooltip>
      <Tooltip label="Show at 100% (1)" side="top">
        <button
          type="button"
          className={classNames(styles.control, styles.scale)}
          aria-label={`Scale ${scale}. Show at 100%`}
          aria-keyshortcuts="1"
          onClick={() => {
            ground.zoomTo(1);
          }}
        >
          {scale}
        </button>
      </Tooltip>
      <Tooltip label="Zoom in (+)" side="top">
        <button
          type="button"
          className={styles.control}
          aria-label="Zoom in"
          aria-keyshortcuts="+"
          disabled={!ground.canZoomIn}
          onClick={ground.zoomIn}
        >
          <Plus aria-hidden />
        </button>
      </Tooltip>
      <Tooltip label="Fit everything (0)" side="top">
        <button
          type="button"
          className={styles.control}
          aria-label="Fit everything"
          aria-keyshortcuts="0"
          onClick={ground.fit}
        >
          <Maximize aria-hidden />
        </button>
      </Tooltip>
    </div>
  );
}
