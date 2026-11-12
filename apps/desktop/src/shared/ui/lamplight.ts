import type { PointerEvent } from 'react';

/**
 * Lets a lamp follow the pointer over an element: a card on the board, or
 * the primary button (decision 0015).
 *
 * It sets `--pointer-x` and `--pointer-y`, the place of the pointer inside
 * the element, which the style sheet of the element uses as the center of
 * its light. It writes to the element directly rather than through React,
 * so that moving the pointer redraws the one element under it and renders
 * nothing.
 *
 * Give it as both `onPointerEnter` and `onPointerMove`, so the light starts
 * where the pointer comes in.
 */
export function followPointer(event: PointerEvent<HTMLElement>): void {
  const element = event.currentTarget;
  const box = element.getBoundingClientRect();
  element.style.setProperty('--pointer-x', `${(event.clientX - box.left).toString()}px`);
  element.style.setProperty('--pointer-y', `${(event.clientY - box.top).toString()}px`);
}
