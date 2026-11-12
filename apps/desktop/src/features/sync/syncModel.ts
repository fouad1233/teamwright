import type { Computer, Conversation, Id, Moment, SyncNotice, SyncState } from '@/shared/api/types';
import { calendarDateOf, formatDate } from '@/shared/lib/format';

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;

/**
 * Says how long ago something happened, in the few words a line has room
 * for: "just now", "4 min ago", "3 h ago", "yesterday", or the date.
 */
export function sinceInWords(moment: Moment, now: Date = new Date()): string {
  const then = new Date(moment);
  const passed = now.getTime() - then.getTime();
  if (passed < MINUTE) {
    // A moment a little ahead, from a clock that runs fast, is "just now" too.
    return 'just now';
  }
  if (passed < HOUR) {
    return `${Math.floor(passed / MINUTE).toString()} min ago`;
  }
  const yesterday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
  if (then.toDateString() === now.toDateString()) {
    return `${Math.floor(passed / HOUR).toString()} h ago`;
  }
  if (then.toDateString() === yesterday.toDateString()) {
    return 'yesterday';
  }
  return formatDate(calendarDateOf(then), now);
}

/** The words the application uses when another computer writes under this one's identity. */
const IDENTITY_IN_USE_WORDS = 'another computer uses the identity of this computer';

/** The words the application uses once this computer has a new identity. */
const NEW_IDENTITY_WORDS = 'This computer has a new identity';

/**
 * Whether syncing stopped because another computer writes under the
 * identity of this one: its data folder was copied there. The state says
 * it only in words, so the words are read.
 */
export function isIdentityInUse(state: SyncState): boolean {
  return state.phase === 'stopped' && (state.problem?.startsWith(IDENTITY_IN_USE_WORDS) ?? false);
}

/** Whether this computer got a new identity, which waits for Teamwright to start again. */
export function waitsForRestart(state: SyncState): boolean {
  return state.phase === 'stopped' && (state.problem?.startsWith(NEW_IDENTITY_WORDS) ?? false);
}

/** The name of a computer as the manager reads it, also before its card was read. */
export function nameOfComputer(computer: Pick<Computer, 'name'>): string {
  return computer.name ?? 'A computer without a name';
}

/**
 * The name of the computer a conversation runs on, as far as this computer
 * knows it.
 */
export function homeComputerName(
  state: SyncState | undefined,
  conversation: Pick<Conversation, 'homeComputer'>,
): string {
  const computer = state?.computers.find((known) => known.id === conversation.homeComputer);
  return computer?.name ?? 'another computer';
}

/**
 * When the latest news of a conversation arrived: its latest step, or the
 * start or the end of its latest turn, whichever came last.
 */
export function lastNewsOf(conversation: Pick<Conversation, 'startedAt' | 'turns'>): Moment {
  let latest = conversation.startedAt;
  for (const turn of conversation.turns) {
    for (const moment of [turn.startedAt, turn.endedAt, ...turn.steps.map((step) => step.at)]) {
      if (moment !== null && moment > latest) {
        latest = moment;
      }
    }
  }
  return latest;
}

/** The other computers that wait for the manager to take them in or ignore them. */
export function computersWaiting(state: SyncState): Computer[] {
  return state.computers.filter((computer) => computer.acceptance === 'undecided');
}

/** The other computers in the order they are listed: the waiting ones first, then by name. */
export function computersInOrder(state: SyncState): Computer[] {
  const rank = { undecided: 0, accepted: 1, ignored: 2 } as const;
  return [...state.computers].sort(
    (first, second) =>
      rank[first.acceptance] - rank[second.acceptance] ||
      nameOfComputer(first).localeCompare(nameOfComputer(second)),
  );
}

/** What the line at the foot of the navigation says about syncing. */
export interface StateLine {
  words: string;
  /**
   * Whether the line lights: something is wrong, or something waits for
   * the manager. Otherwise it is quiet.
   */
  isLit: boolean;
}

/**
 * The line at the foot of the navigation, in words. Nothing while no
 * folder is chosen: then there is nothing to say.
 */
export function stateLineOf(state: SyncState, now: Date = new Date()): StateLine | null {
  switch (state.phase) {
    case 'off':
      return null;
    case 'writing':
      return { words: 'Writing to the folder', isLit: false };
    case 'unreachable':
      return { words: 'The folder cannot be reached', isLit: true };
    case 'stopped':
      return {
        words: waitsForRestart(state)
          ? 'Stopped: start Teamwright again'
          : isIdentityInUse(state)
            ? 'Stopped: two computers share one identity'
            : 'Stopped',
        isLit: true,
      };
    case 'in_step': {
      const waiting = computersWaiting(state)[0];
      if (waiting) {
        return { words: `${nameOfComputer(waiting)} waits to be taken in`, isLit: true };
      }
      const newer = state.computers.find(
        (computer) => computer.acceptance === 'accepted' && computer.newerFormat,
      );
      if (newer) {
        return { words: `${nameOfComputer(newer)} uses a newer Teamwright`, isLit: true };
      }
      return {
        words:
          state.lastRound === null ? 'In step' : `In step, ${sinceInWords(state.lastRound, now)}`,
        isLit: false,
      };
    }
  }
}

/**
 * A problem of the last round as a sentence: the application writes its
 * problems as sentence fragments.
 */
export function problemInWords(problem: string): string {
  const sentence = `${problem.charAt(0).toUpperCase()}${problem.slice(1)}`;
  return /[.!?]$/.test(sentence) ? sentence : `${sentence}.`;
}

/** Where a notice leads: to its task, to its project, or nowhere. */
export type NoticeTarget =
  | { kind: 'task'; projectId: Id; taskId: Id }
  | { kind: 'project'; projectId: Id }
  | { kind: 'none' };

/** Where a notice leads, among the projects that are still there. */
export function noticeTarget(notice: SyncNotice, projectIds: readonly Id[]): NoticeTarget {
  if (notice.projectId === null || !projectIds.includes(notice.projectId)) {
    return { kind: 'none' };
  }
  return notice.taskId === null
    ? { kind: 'project', projectId: notice.projectId }
    : { kind: 'task', projectId: notice.projectId, taskId: notice.taskId };
}

/**
 * Whether something the interface has asked the backend for is to be asked
 * again after syncing took in changes to some projects. Everything of those
 * projects is, and so is everything that is not of one project: the list
 * of projects, what waits for the manager, the histories of tasks and the
 * reports of sprints.
 */
export function isStaleAfterChanges(
  queryKey: readonly unknown[],
  projectIds: readonly Id[],
): boolean {
  const [name, about] = queryKey;
  if (typeof about === 'string' && projectIds.includes(about)) {
    return true;
  }
  return (
    name === 'projects' ||
    name === 'conversations-waiting' ||
    name === 'task-history' ||
    name === 'sprint-report'
  );
}
