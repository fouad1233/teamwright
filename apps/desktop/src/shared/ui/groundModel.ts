/**
 * The arithmetic of a ground: a surface larger than the frame it is seen
 * through, which the manager moves and zooms. A mind map, a map and a
 * whiteboard each lie on one.
 *
 * Everything here is a pure function. `Ground.tsx` holds the state and
 * listens to the pointer, the wheel and the keys.
 */

/** A place, in pixels: on the ground, or in the frame. */
export interface Point {
  x: number;
  y: number;
}

export interface Size {
  width: number;
  height: number;
}

/** A rectangle: its top left corner and its size. */
export interface Box extends Point, Size {}

/**
 * How the ground lies in its frame: where its origin is, in pixels from the
 * top left corner of the frame, and how large it is drawn. At a scale of 1
 * one unit of the ground is one pixel.
 */
export interface Placement {
  x: number;
  y: number;
  scale: number;
}

/** The smallest and the largest scale a ground can be drawn at. */
export interface ScaleLimits {
  smallest: number;
  largest: number;
}

/** The ground as it lies before anything has moved it. */
export const UNMOVED: Placement = { x: 0, y: 0, scale: 1 };

/** The limits of a ground that names none: a quarter of its size to twice its size. */
export const SCALE_LIMITS: ScaleLimits = { smallest: 0.25, largest: 2 };

/**
 * The scales the zoom buttons stop at. A press goes to the next one in its
 * direction, so that the scale reads as a round figure.
 */
export const SCALE_STOPS: readonly number[] = [
  0.1, 0.25, 0.33, 0.5, 0.67, 0.8, 1, 1.25, 1.5, 2, 3, 4, 6, 8,
];

/** How much of the scale one pixel of the wheel's travel changes. */
const WHEEL_ZOOM_RATE = 0.003;

/**
 * The most one event of the wheel counts for, in pixels. A notch of a mouse
 * wheel is a hundred pixels or more and would jump; two fingers on a touch
 * pad send many small events, which are left as they are.
 */
const WHEEL_LARGEST_TURN = 50;

/** How close two scales must be to count as the same stop. */
const SAME_STOP = 0.005;

/** A scale brought inside the limits. */
export function scaleWithin(scale: number, limits: ScaleLimits = SCALE_LIMITS): number {
  return Math.min(Math.max(scale, limits.smallest), limits.largest);
}

/**
 * The ground drawn at another scale, zoomed about a point of the frame:
 * what lies under that point stays under it. The scale is kept inside the
 * limits.
 */
export function zoomedAbout(
  placement: Placement,
  point: Point,
  scale: number,
  limits: ScaleLimits = SCALE_LIMITS,
): Placement {
  const next = scaleWithin(scale, limits);
  if (next === placement.scale) {
    return placement;
  }
  const ratio = next / placement.scale;
  return {
    x: point.x - (point.x - placement.x) * ratio,
    y: point.y - (point.y - placement.y) * ratio,
    scale: next,
  };
}

/**
 * The scale after the wheel has turned by so many pixels: larger when it
 * turns away from the manager, smaller when it turns towards them, and by
 * the same factor for the same turn at every scale.
 */
export function scaleAfterWheel(scale: number, turned: number): number {
  const turn = Math.min(Math.max(turned, -WHEEL_LARGEST_TURN), WHEEL_LARGEST_TURN);
  return scale * Math.exp(-turn * WHEEL_ZOOM_RATE);
}

/**
 * The scale a zoom button goes to: the next stop above the scale, or below
 * it, and the limit when there is none.
 */
export function nextScaleStop(
  scale: number,
  direction: 'in' | 'out',
  limits: ScaleLimits = SCALE_LIMITS,
): number {
  const stops = SCALE_STOPS.filter((stop) => stop > limits.smallest && stop < limits.largest);
  if (direction === 'in') {
    return stops.find((stop) => stop > scale * (1 + SAME_STOP)) ?? limits.largest;
  }
  return stops.reverse().find((stop) => stop < scale * (1 - SAME_STOP)) ?? limits.smallest;
}

/** How "fit" lays a box into a frame. */
export interface Fit {
  /** The room left free between the box and the edges of the frame, in pixels. */
  margin: number;
  limits: ScaleLimits;
  /**
   * The largest scale to choose. With 1, a box that is smaller than the
   * frame is shown as it is and not enlarged.
   */
  largestFit: number;
}

/**
 * The ground laid so that a box of it lies whole in the frame, as large as
 * the margin allows, and in the middle. A box too large for the smallest
 * scale is shown at that scale, about its middle.
 */
export function fittedTo(content: Box, frame: Size, fit: Fit): Placement {
  const room = {
    width: Math.max(frame.width - 2 * fit.margin, 1),
    height: Math.max(frame.height - 2 * fit.margin, 1),
  };
  const largest = Math.min(fit.limits.largest, Math.max(fit.largestFit, fit.limits.smallest));
  const wanted = Math.min(
    content.width > 0 ? room.width / content.width : largest,
    content.height > 0 ? room.height / content.height : largest,
  );
  const scale = Math.min(Math.max(wanted, fit.limits.smallest), largest);
  return {
    x: (frame.width - content.width * scale) / 2 - content.x * scale,
    y: (frame.height - content.height * scale) / 2 - content.y * scale,
    scale,
  };
}

/** The ground moved by so many pixels of the frame. */
export function movedBy(placement: Placement, dx: number, dy: number): Placement {
  if (dx === 0 && dy === 0) {
    return placement;
  }
  return { ...placement, x: placement.x + dx, y: placement.y + dy };
}

/**
 * Where one side of a box must go so that the box lies between two edges:
 * as it is when it does, to the nearer edge when it does not, and to the
 * first edge when it is longer than the room between them.
 */
function broughtBetween(start: number, length: number, first: number, last: number): number {
  if (length > last - first || start < first) {
    return first;
  }
  return start + length > last ? last - length : start;
}

/**
 * The ground moved as little as needed for a box of it to lie in the
 * frame, a margin away from its edges. A box that is in view already moves
 * nothing. A box larger than the frame shows its top left corner.
 */
export function revealing(placement: Placement, box: Box, frame: Size, margin: number): Placement {
  const left = placement.x + box.x * placement.scale;
  const top = placement.y + box.y * placement.scale;
  const width = box.width * placement.scale;
  const height = box.height * placement.scale;
  return movedBy(
    placement,
    broughtBetween(left, width, margin, frame.width - margin) - left,
    broughtBetween(top, height, margin, frame.height - margin) - top,
  );
}

/**
 * The ground held within reach: moved back just far enough that the box of
 * its content still covers a share of the frame, across and down, so that
 * what lies on it is not dragged out of sight. With a share of a half, an
 * edge of the content can be brought to the middle of the frame and no
 * further. Content smaller than that share of the frame stays in whole. A
 * share of nothing lets the content go until it only touches the frame.
 */
export function withinReach(
  placement: Placement,
  content: Box,
  frame: Size,
  share: number,
): Placement {
  const held = (origin: number, start: number, length: number, room: number): number => {
    const kept = Math.min(share * room, length * placement.scale);
    const least = kept - (start + length) * placement.scale;
    const most = room - kept - start * placement.scale;
    return Math.min(Math.max(origin, least), most);
  };
  const x = held(placement.x, content.x, content.width, frame.width);
  const y = held(placement.y, content.y, content.height, frame.height);
  return x === placement.x && y === placement.y ? placement : { ...placement, x, y };
}

/** The point of the ground that lies under a point of the frame. */
export function pointOnGround(placement: Placement, inFrame: Point): Point {
  return {
    x: (inFrame.x - placement.x) / placement.scale,
    y: (inFrame.y - placement.y) / placement.scale,
  };
}

/** A scale as it is written for the manager: "80%". */
export function percentOf(scale: number): string {
  return `${Math.round(scale * 100).toString()}%`;
}
