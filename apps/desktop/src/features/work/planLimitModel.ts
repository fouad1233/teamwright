import type { PlanLimit, PlanWindow, PlanWindowKind } from '@/shared/api/types';

/**
 * The words of what is known about the limit of the Claude plan
 * (decision 0040). Whether the limit is reached, and when it resets, is
 * decided by the backend; this only says it.
 */

const MONTHS = [
  'January',
  'February',
  'March',
  'April',
  'May',
  'June',
  'July',
  'August',
  'September',
  'October',
  'November',
  'December',
] as const;

function twoDigits(value: number): string {
  return value.toString().padStart(2, '0');
}

function sameDay(one: Date, other: Date): boolean {
  return (
    one.getFullYear() === other.getFullYear() &&
    one.getMonth() === other.getMonth() &&
    one.getDate() === other.getDate()
  );
}

/**
 * When a limit resets, by the clocks of this computer: "at 22:40" on the
 * same day, "tomorrow at 07:00", or "on 25 November at 10:00".
 */
export function resetInWords(resetsAt: string, now: Date): string {
  const then = new Date(resetsAt);
  const time = `${twoDigits(then.getHours())}:${twoDigits(then.getMinutes())}`;
  if (sameDay(then, now)) {
    return `at ${time}`;
  }
  const tomorrow = new Date(now);
  tomorrow.setDate(now.getDate() + 1);
  if (sameDay(then, tomorrow)) {
    return `tomorrow at ${time}`;
  }
  return `on ${then.getDate().toString()} ${MONTHS[then.getMonth()] ?? ''} at ${time}`;
}

/**
 * The one plain line that says the limit is reached, or null while it is
 * not: "The limit of your Claude plan is reached. It resets at 22:40."
 */
export function planLimitLine(planLimit: PlanLimit | undefined, now: Date): string | null {
  const reached = planLimit?.reached;
  if (!reached) {
    return null;
  }
  const opening = 'The limit of your Claude plan is reached.';
  return reached.resetsAt === null
    ? `${opening} When it resets is not known.`
    : `${opening} It resets ${resetInWords(reached.resetsAt, now)}.`;
}

/**
 * What a manager who starts work while the limit is reached is told: it is
 * allowed, because the manager may know better, but it will most likely
 * stop at once. Null while the limit is not reached.
 */
export function startWarning(planLimit: PlanLimit | undefined, now: Date): string | null {
  const reached = planLimit?.reached;
  if (!reached) {
    return null;
  }
  return reached.resetsAt === null
    ? 'The limit of your Claude plan is reached. Work started now may stop at once; if it goes through, the limit has reset.'
    : `The limit of your Claude plan is reached until ${resetInWords(reached.resetsAt, now).replace(/^(at|on) /, '')}. Work started before then will most likely stop at once.`;
}

/**
 * What stands beside "Try again" while the limit is reached, under the line
 * that already says when it resets. Null while it is not reached.
 */
export function tryAgainHint(planLimit: PlanLimit | undefined): string | null {
  const reached = planLimit?.reached;
  if (!reached) {
    return null;
  }
  return reached.resetsAt === null
    ? 'It may stop again at once; if it goes through, the limit has reset.'
    : 'Before then it will most likely stop again at once.';
}

/** The name of a window of the plan, for people. */
export function windowName(kind: PlanWindowKind): string {
  switch (kind) {
    case 'five_hour':
      return 'Five hours';
    case 'seven_day':
      return 'Week';
    default: {
      // Claude Code's own names, such as `seven_day_opus`, in words.
      const words = kind.replace(/_/g, ' ').replace(/\bseven day\b/, 'week');
      return words.charAt(0).toUpperCase() + words.slice(1);
    }
  }
}

/** How much of a window is used, in words: "42 %", or "unknown". */
export function usedInWords(window: PlanWindow): string {
  if (window.used === null) {
    return 'unknown';
  }
  return `${Math.round(window.used * 100).toString()} %`;
}

/** The share of a window used, from 0 to 1, for a bar; null when not known. */
export function usedShare(window: PlanWindow): number | null {
  return window.used === null ? null : Math.min(1, Math.max(0, window.used));
}

/**
 * The two windows the manager knows, the five hours and the week, in that
 * order, as reported or as unknown; then any other window Claude Code
 * reported.
 */
export function windowsToShow(planLimit: PlanLimit | undefined): PlanWindow[] {
  const reported = planLimit?.windows ?? [];
  const unknown = (kind: PlanWindowKind): PlanWindow => ({
    kind,
    used: null,
    resetsAt: null,
    status: null,
  });
  const known = (kind: PlanWindowKind) =>
    reported.find((window) => window.kind === kind) ?? unknown(kind);
  return [
    known('five_hour'),
    known('seven_day'),
    ...reported.filter((window) => window.kind !== 'five_hour' && window.kind !== 'seven_day'),
  ];
}
