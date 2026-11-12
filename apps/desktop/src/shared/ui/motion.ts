import { useSyncExternalStore } from 'react';

/*
 * Motion that is played from code rather than from a style sheet: the curve
 * and the duration still come from the tokens in tokens.css, so that every
 * movement of the interface is drawn from the same small set
 * (decision 0015).
 */

const REDUCED_MOTION = '(prefers-reduced-motion: reduce)';

function reducedMotionQuery(): MediaQueryList | undefined {
  return typeof window.matchMedia === 'function' ? window.matchMedia(REDUCED_MOTION) : undefined;
}

function subscribeToReducedMotion(onChange: () => void): () => void {
  const query = reducedMotionQuery();
  query?.addEventListener('change', onChange);
  return () => {
    query?.removeEventListener('change', onChange);
  };
}

function isMotionReduced(): boolean {
  return reducedMotionQuery()?.matches ?? false;
}

/** Whether the user asked the system for less motion. Follows the setting as it changes. */
export function usePrefersReducedMotion(): boolean {
  return useSyncExternalStore(subscribeToReducedMotion, isMotionReduced, () => false);
}

function tokenValue(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

/** A duration token, such as `--duration-settle`, in milliseconds. Nothing known is no time. */
export function durationToken(name: string): number {
  const value = tokenValue(name);
  const amount = Number.parseFloat(value);
  if (Number.isNaN(amount)) {
    return 0;
  }
  return value.endsWith('ms') ? amount : amount * 1000;
}

/** A curve token, such as `--ease-arrive`, as a timing function. */
export function easingToken(name: string): string {
  return tokenValue(name) || 'linear';
}
