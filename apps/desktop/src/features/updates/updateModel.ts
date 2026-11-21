import type { UpdateStatus } from '@/shared/api/types';

import { sinceInWords } from '../sync/syncModel';

/**
 * What the last look came to, in a line: when it was, or that it could not
 * be made, or that none was. A failed look is only said, never shown as an
 * error: the program may simply be offline.
 */
export function lastLookInWords(status: UpdateStatus, now: Date = new Date()): string {
  if (status.lastLook === null) {
    return 'Not looked yet.';
  }
  const when = sinceInWords(status.lastLook, now);
  if (status.lastLookFailed) {
    return `Could not look ${when}. Nothing was lost; try again later.`;
  }
  return status.newer === null ? `Looked ${when}. This is the newest version.` : `Looked ${when}.`;
}

/**
 * The sentence about a newer version. A pre-release is called one, so that
 * the manager is not led to think it is the settled one.
 */
export function newerInWords(status: UpdateStatus): string | null {
  const { newer } = status;
  if (newer === null) {
    return null;
  }
  const what = newer.isPreRelease ? 'a newer pre-release' : 'a newer version';
  return `Version ${newer.version} is out, ${what} than the ${status.thisVersion} that runs here. Teamwright does not install it by itself: the release page tells how to get it.`;
}

/** The few words at the foot of the navigation. */
export function markInWords(status: UpdateStatus): string | null {
  return status.newer === null ? null : `${status.newer.version} is out`;
}
