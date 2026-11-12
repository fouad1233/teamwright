import type { CSSProperties } from 'react';

import styles from './WaitsForManager.module.css';

/**
 * The frame of something that waits for the manager, shared by every card
 * of that kind (see `WaitsForManager.module.css`):
 *
 * - `frame`: the rule of gold, the sun, the grain, and the rule drawn down
 *   when it appears.
 * - `lightAtHead`: with `frame`, for a large card that is lit at its head
 *   alone.
 * - `part`: a part inside it that comes up into place after the rule.
 */
export const waitsForManager = {
  frame: styles.frame,
  lightAtHead: styles.lightAtHead,
  part: styles.part,
};

/** The place of a part in the order in which the parts come up, from 0. */
export function appearsAt(place: number): CSSProperties {
  return { '--place': place } as CSSProperties;
}
