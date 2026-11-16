import { describe, expect, it } from 'vitest';

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
  scaleWithin,
  withinReach,
  zoomedAbout,
  type Fit,
  type Placement,
} from './groundModel';

const FRAME = { width: 1000, height: 600 };

const FIT: Fit = { margin: 20, limits: SCALE_LIMITS, largestFit: 1 };

describe('scaleWithin', () => {
  it('leaves a scale inside the limits as it is', () => {
    expect(scaleWithin(0.8)).toBe(0.8);
  });

  it('stops at the smallest and at the largest scale', () => {
    expect(scaleWithin(0.01)).toBe(SCALE_LIMITS.smallest);
    expect(scaleWithin(40)).toBe(SCALE_LIMITS.largest);
    expect(scaleWithin(40, { smallest: 1, largest: 8 })).toBe(8);
  });
});

describe('zoomedAbout', () => {
  it('keeps what lies under the point where it is', () => {
    const before: Placement = { x: 120, y: -40, scale: 0.8 };
    const point = { x: 400, y: 250 };
    const after = zoomedAbout(before, point, 1.5);

    expect(after.scale).toBe(1.5);
    const under = pointOnGround(before, point);
    const stillUnder = pointOnGround(after, point);
    expect(stillUnder.x).toBeCloseTo(under.x);
    expect(stillUnder.y).toBeCloseTo(under.y);
  });

  it('moves everything else away from the point when it zooms in', () => {
    const after = zoomedAbout(UNMOVED, { x: 100, y: 100 }, 2);
    // The origin of the ground was 100 pixels up and left of the point. Now it is 200.
    expect(after).toEqual({ x: -100, y: -100, scale: 2 });
  });

  it('does not go past the limits', () => {
    expect(zoomedAbout(UNMOVED, { x: 0, y: 0 }, 99).scale).toBe(SCALE_LIMITS.largest);
    expect(zoomedAbout(UNMOVED, { x: 0, y: 0 }, 0).scale).toBe(SCALE_LIMITS.smallest);
  });

  it('changes nothing when the scale stays the same', () => {
    const at = { x: 5, y: 6, scale: SCALE_LIMITS.largest };
    expect(zoomedAbout(at, { x: 300, y: 300 }, 5)).toBe(at);
  });
});

describe('scaleAfterWheel', () => {
  it('grows when the wheel turns away and shrinks when it turns back', () => {
    expect(scaleAfterWheel(1, -10)).toBeGreaterThan(1);
    expect(scaleAfterWheel(1, 10)).toBeLessThan(1);
  });

  it('undoes a turn with the same turn back', () => {
    expect(scaleAfterWheel(scaleAfterWheel(0.7, -30), 30)).toBeCloseTo(0.7);
  });

  it('changes by the same factor at every scale', () => {
    expect(scaleAfterWheel(2, -20) / 2).toBeCloseTo(scaleAfterWheel(0.5, -20) / 0.5);
  });

  it('counts a notch of a mouse wheel as no more than a moderate step', () => {
    expect(scaleAfterWheel(1, -100)).toBe(scaleAfterWheel(1, -400));
    expect(scaleAfterWheel(1, -100)).toBeLessThan(1.25);
  });
});

describe('nextScaleStop', () => {
  it('goes to the next round figure in its direction', () => {
    expect(nextScaleStop(1, 'in')).toBe(1.25);
    expect(nextScaleStop(1, 'out')).toBe(0.8);
  });

  it('goes to the nearest stop from a scale between two', () => {
    expect(nextScaleStop(0.93, 'in')).toBe(1);
    expect(nextScaleStop(0.93, 'out')).toBe(0.8);
  });

  it('does not stop twice at a scale it has all but reached', () => {
    expect(nextScaleStop(0.9999, 'in')).toBe(1.25);
    expect(nextScaleStop(1.0001, 'out')).toBe(0.8);
  });

  it('ends at the limits', () => {
    expect(nextScaleStop(1.5, 'in')).toBe(SCALE_LIMITS.largest);
    expect(nextScaleStop(SCALE_LIMITS.largest, 'in')).toBe(SCALE_LIMITS.largest);
    expect(nextScaleStop(0.33, 'out')).toBe(SCALE_LIMITS.smallest);
    expect(nextScaleStop(SCALE_LIMITS.smallest, 'out')).toBe(SCALE_LIMITS.smallest);
  });

  it('uses the stops between the limits it is given', () => {
    const wide = { smallest: 0.5, largest: 8 };
    expect(nextScaleStop(2, 'in', wide)).toBe(3);
    expect(nextScaleStop(6, 'in', wide)).toBe(8);
    expect(nextScaleStop(0.67, 'out', wide)).toBe(0.5);
  });
});

describe('fittedTo', () => {
  it('shrinks a box that is too wide until it fits between the margins', () => {
    const fitted = fittedTo({ x: 0, y: 0, width: 1920, height: 200 }, FRAME, FIT);
    expect(fitted.scale).toBeCloseTo(0.5);
    expect(fitted.x).toBeCloseTo(20);
    // In the middle of the height.
    expect(fitted.y).toBeCloseTo((600 - 100) / 2);
  });

  it('shrinks a box that is too high until it fits', () => {
    const fitted = fittedTo({ x: 0, y: 0, width: 100, height: 1120 }, FRAME, FIT);
    expect(fitted.scale).toBeCloseTo(0.5);
    expect(fitted.y).toBeCloseTo(20);
  });

  it('shows a box smaller than the frame as it is, in the middle', () => {
    const fitted = fittedTo({ x: 0, y: 0, width: 400, height: 200 }, FRAME, FIT);
    expect(fitted).toEqual({ x: 300, y: 200, scale: 1 });
  });

  it('enlarges a small box when it is allowed to', () => {
    const fitted = fittedTo({ x: 0, y: 0, width: 480, height: 100 }, FRAME, {
      ...FIT,
      largestFit: 8,
    });
    expect(fitted.scale).toBe(SCALE_LIMITS.largest);
  });

  it('puts a box that does not begin at the origin in the middle all the same', () => {
    const fitted = fittedTo({ x: -200, y: 50, width: 400, height: 200 }, FRAME, FIT);
    expect(fitted).toEqual({ x: 500, y: 150, scale: 1 });
  });

  it('shows a box too large for the smallest scale about its middle', () => {
    const fitted = fittedTo({ x: 0, y: 0, width: 40000, height: 100 }, FRAME, FIT);
    expect(fitted.scale).toBe(SCALE_LIMITS.smallest);
    expect(fitted.x + (40000 * fitted.scale) / 2).toBeCloseTo(FRAME.width / 2);
  });

  it('gives a box without a size the largest scale it may choose', () => {
    expect(fittedTo({ x: 10, y: 10, width: 0, height: 0 }, FRAME, FIT)).toEqual({
      x: 490,
      y: 290,
      scale: 1,
    });
  });
});

describe('movedBy', () => {
  it('moves the ground and keeps its scale', () => {
    expect(movedBy({ x: 10, y: 20, scale: 0.5 }, 5, -5)).toEqual({ x: 15, y: 15, scale: 0.5 });
  });

  it('changes nothing for no movement', () => {
    const at = { x: 10, y: 20, scale: 0.5 };
    expect(movedBy(at, 0, 0)).toBe(at);
  });
});

describe('revealing', () => {
  it('moves nothing for a box that is in view', () => {
    const at = { x: 0, y: 0, scale: 1 };
    expect(revealing(at, { x: 100, y: 100, width: 200, height: 40 }, FRAME, 24)).toBe(at);
  });

  it('brings a box beyond the right and the bottom just inside the margin', () => {
    const shown = revealing(UNMOVED, { x: 1100, y: 700, width: 200, height: 40 }, FRAME, 24);
    expect(shown.x + 1300).toBe(FRAME.width - 24);
    expect(shown.y + 740).toBe(FRAME.height - 24);
  });

  it('brings a box beyond the left and the top just inside the margin', () => {
    const shown = revealing(UNMOVED, { x: -300, y: -90, width: 200, height: 40 }, FRAME, 24);
    expect(shown).toEqual({ x: 324, y: 114, scale: 1 });
  });

  it('counts in the pixels of the frame at any scale', () => {
    const at = { x: 0, y: 0, scale: 2 };
    const shown = revealing(at, { x: 600, y: 10, width: 100, height: 20 }, FRAME, 24);
    // The box is 200 pixels wide on screen and ended at 1400.
    expect(shown.x + 1400).toBe(FRAME.width - 24);
    expect(shown.y).toBe(24 - 20);
  });

  it('shows the top left corner of a box that is larger than the frame', () => {
    const shown = revealing(UNMOVED, { x: 500, y: 300, width: 5000, height: 4000 }, FRAME, 24);
    expect(shown).toEqual({ x: -476, y: -276, scale: 1 });
  });
});

describe('withinReach', () => {
  const content = { x: 0, y: 0, width: 800, height: 400 };

  it('leaves a ground whose content is in view where it is', () => {
    const at = { x: 50, y: 50, scale: 1 };
    expect(withinReach(at, content, FRAME, 0.5)).toBe(at);
  });

  it('lets an edge of the content come to the middle of the frame and no further', () => {
    // The right edge of the content, then its left edge, at the middle across.
    expect(withinReach({ x: -5000, y: 0, scale: 1 }, content, FRAME, 0.5).x + 800).toBe(500);
    expect(withinReach({ x: 5000, y: 0, scale: 1 }, content, FRAME, 0.5).x).toBe(500);
    // Its bottom edge, then its top edge, at the middle down.
    expect(withinReach({ x: 0, y: -5000, scale: 1 }, content, FRAME, 0.5).y + 400).toBe(300);
    expect(withinReach({ x: 0, y: 5000, scale: 1 }, content, FRAME, 0.5).y).toBe(300);
  });

  it('holds a smaller share of the frame when it is asked to', () => {
    expect(withinReach({ x: 5000, y: 0, scale: 1 }, content, FRAME, 0.1).x).toBe(900);
  });

  it('keeps content that is smaller than its share of the frame whole', () => {
    const small = { x: 0, y: 0, width: 40, height: 40 };
    const held = withinReach({ x: -900, y: 900, scale: 1 }, small, FRAME, 0.5);
    expect(held).toEqual({ x: 0, y: FRAME.height - 40, scale: 1 });
  });

  it('counts the content at the scale it is drawn at', () => {
    // At half its size the content is 400 pixels wide: less than half the frame.
    const held = withinReach({ x: -5000, y: 0, scale: 0.5 }, content, FRAME, 0.5);
    expect(held.x).toBe(0);
  });

  it('lets the content go until it only touches the frame, with a share of nothing', () => {
    expect(withinReach({ x: -5000, y: 0, scale: 1 }, content, FRAME, 0).x).toBe(-800);
    expect(withinReach({ x: 5000, y: 0, scale: 1 }, content, FRAME, 0).x).toBe(FRAME.width);
  });
});

describe('pointOnGround', () => {
  it('reads a point of the frame as a place on the ground', () => {
    expect(pointOnGround({ x: 100, y: 50, scale: 2 }, { x: 300, y: 250 })).toEqual({
      x: 100,
      y: 100,
    });
  });
});

describe('percentOf', () => {
  it('writes a scale as a whole percentage', () => {
    expect(percentOf(1)).toBe('100%');
    expect(percentOf(0.666)).toBe('67%');
    expect(percentOf(1.25)).toBe('125%');
  });
});
