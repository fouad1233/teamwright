import { describe, expect, it } from 'vitest';

import type { Computer, SyncNotice, SyncState, Turn } from '@/shared/api/types';

import {
  computersInOrder,
  homeComputerName,
  isIdentityInUse,
  isStaleAfterChanges,
  lastNewsOf,
  noticeTarget,
  problemInWords,
  sinceInWords,
  stateLineOf,
  waitsForRestart,
} from './syncModel';

const NOW = new Date(2026, 10, 13, 15, 0, 0);

function minutesBefore(minutes: number): string {
  return new Date(NOW.getTime() - minutes * 60_000).toISOString();
}

function computer(overrides: Partial<Computer> & Pick<Computer, 'id'>): Computer {
  return {
    name: 'Laptop',
    version: '0.1.0',
    acceptance: 'accepted',
    lastHeard: minutesBefore(2),
    clockAhead: false,
    newerFormat: false,
    ...overrides,
  };
}

function state(overrides: Partial<SyncState> = {}): SyncState {
  return {
    folder: 'D:\\Cloud Drive\\Teamwright',
    thisComputer: 'desk',
    computerName: 'Desk',
    suggestedName: 'Desk',
    phase: 'in_step',
    lastRound: minutesBefore(2),
    problem: null,
    problemCode: null,
    computers: [],
    ...overrides,
  };
}

const IDENTITY_IN_USE = {
  problem:
    'another computer uses the identity of this computer; give this computer a new identity and start Teamwright again',
  problemCode: 'identity_in_use',
} as const;
const NEW_IDENTITY = {
  problem: 'This computer has a new identity. Start Teamwright again to go on.',
  problemCode: 'restart_needed',
} as const;

describe('sinceInWords', () => {
  it('says "just now" within a minute, also for a moment a little ahead', () => {
    expect(sinceInWords(minutesBefore(0.5), NOW)).toBe('just now');
    expect(sinceInWords(minutesBefore(-3), NOW)).toBe('just now');
  });

  it('counts minutes within the hour', () => {
    expect(sinceInWords(minutesBefore(1), NOW)).toBe('1 min ago');
    expect(sinceInWords(minutesBefore(59), NOW)).toBe('59 min ago');
  });

  it('counts hours on the same day', () => {
    expect(sinceInWords(minutesBefore(60), NOW)).toBe('1 h ago');
    expect(sinceInWords(minutesBefore(14 * 60), NOW)).toBe('14 h ago');
  });

  it('says "yesterday" for the day before, and the date before that', () => {
    expect(sinceInWords(new Date(2026, 10, 12, 23, 0).toISOString(), NOW)).toBe('yesterday');
    expect(sinceInWords(new Date(2026, 10, 9, 10, 0).toISOString(), NOW)).toBe('9 Nov');
    expect(sinceInWords(new Date(2025, 10, 9, 10, 0).toISOString(), NOW)).toBe('9 Nov 2025');
  });
});

describe('stateLineOf', () => {
  it('says nothing while syncing is off', () => {
    expect(stateLineOf(state({ phase: 'off', folder: null }), NOW)).toBeNull();
  });

  it('is quiet in step, and says since when', () => {
    expect(stateLineOf(state(), NOW)).toEqual({ words: 'In step, 2 min ago', isLit: false });
    expect(stateLineOf(state({ lastRound: null }), NOW)).toEqual({
      words: 'In step',
      isLit: false,
    });
  });

  it('is quiet while a round writes', () => {
    expect(stateLineOf(state({ phase: 'writing' }), NOW)).toEqual({
      words: 'Writing to the folder',
      isLit: false,
    });
  });

  it('lights when the folder cannot be reached', () => {
    expect(
      stateLineOf(
        state({ phase: 'unreachable', problem: 'gone', problemCode: 'sync_folder_unreachable' }),
        NOW,
      ),
    ).toEqual({
      words: 'The folder cannot be reached',
      isLit: true,
    });
  });

  it('lights when it stopped, and says why', () => {
    expect(stateLineOf(state({ phase: 'stopped', ...IDENTITY_IN_USE }), NOW)).toEqual({
      words: 'Stopped: two computers share one identity',
      isLit: true,
    });
    expect(stateLineOf(state({ phase: 'stopped', ...NEW_IDENTITY }), NOW)).toEqual({
      words: 'Stopped: start Teamwright again',
      isLit: true,
    });
    expect(stateLineOf(state({ phase: 'stopped', problem: null }), NOW)?.words).toBe('Stopped');
  });

  it('lights when a computer waits to be taken in', () => {
    const waiting = computer({ id: 'laptop', acceptance: 'undecided' });
    expect(stateLineOf(state({ computers: [waiting] }), NOW)).toEqual({
      words: 'Laptop waits to be taken in',
      isLit: true,
    });
  });

  it('lights when a computer it takes in uses a newer Teamwright', () => {
    const newer = computer({ id: 'laptop', newerFormat: true });
    expect(stateLineOf(state({ computers: [newer] }), NOW)).toEqual({
      words: 'Laptop uses a newer Teamwright',
      isLit: true,
    });
  });

  it('stays quiet for a computer that is ignored', () => {
    const ignored = computer({ id: 'laptop', acceptance: 'ignored', newerFormat: true });
    expect(stateLineOf(state({ computers: [ignored] }), NOW)?.isLit).toBe(false);
  });
});

describe('the identity of this computer', () => {
  it('is in use elsewhere only when it stopped for that', () => {
    expect(isIdentityInUse(state({ phase: 'stopped', ...IDENTITY_IN_USE }))).toBe(true);
    expect(isIdentityInUse(state({ phase: 'unreachable', ...IDENTITY_IN_USE }))).toBe(false);
    expect(isIdentityInUse(state({ phase: 'stopped', ...NEW_IDENTITY }))).toBe(false);
  });

  it('decides by the code, not by the words', () => {
    const reworded = {
      problem: 'Two computers share one identity.',
      problemCode: 'identity_in_use',
    } as const;
    expect(isIdentityInUse(state({ phase: 'stopped', ...reworded }))).toBe(true);
    expect(
      isIdentityInUse(
        state({ phase: 'stopped', problem: IDENTITY_IN_USE.problem, problemCode: null }),
      ),
    ).toBe(false);
  });

  it('waits for a restart once it is new', () => {
    expect(waitsForRestart(state({ phase: 'stopped', ...NEW_IDENTITY }))).toBe(true);
    expect(waitsForRestart(state({ phase: 'stopped', ...IDENTITY_IN_USE }))).toBe(false);
  });
});

describe('the other computers', () => {
  it('lists the waiting ones first, then by name, the ignored last', () => {
    const listed = computersInOrder(
      state({
        computers: [
          computer({ id: 'w', name: 'Workshop', acceptance: 'ignored' }),
          computer({ id: 'l', name: 'Laptop' }),
          computer({ id: 'a', name: 'Attic' }),
          computer({ id: 'n', name: null, acceptance: 'undecided' }),
        ],
      }),
    );
    expect(listed.map((each) => each.id)).toEqual(['n', 'a', 'l', 'w']);
  });

  it('names the computer a conversation runs on, or says it is another one', () => {
    const known = state({ computers: [computer({ id: 'laptop' })] });
    expect(homeComputerName(known, { homeComputer: 'laptop' })).toBe('Laptop');
    expect(homeComputerName(known, { homeComputer: 'attic' })).toBe('another computer');
    expect(homeComputerName(undefined, { homeComputer: 'laptop' })).toBe('another computer');
  });
});

describe('lastNewsOf', () => {
  const turn: Turn = {
    id: 't',
    request: 'Draw it.',
    origin: 'brief',
    state: 'running',
    startedAt: '2026-11-13T09:00:00.000Z',
    endedAt: null,
    steps: [
      { kind: 'text', at: '2026-11-13T09:00:05.000Z', text: 'Reading.' },
      { kind: 'notice', at: '2026-11-13T09:00:09.000Z', message: 'Retried.' },
    ],
    answer: '',
    failure: null,
    model: null,
    tokens: null,
    decision: null,
    interview: null,
    proposal: null,
  };

  it('is the latest step of a running turn', () => {
    expect(lastNewsOf({ startedAt: '2026-11-13T08:00:00.000Z', turns: [turn] })).toBe(
      '2026-11-13T09:00:09.000Z',
    );
  });

  it('is the end of a turn that ended after its steps', () => {
    const ended = { ...turn, endedAt: '2026-11-13T09:01:00.000Z' };
    expect(lastNewsOf({ startedAt: '2026-11-13T08:00:00.000Z', turns: [ended] })).toBe(
      '2026-11-13T09:01:00.000Z',
    );
  });

  it('is the start of a conversation without turns', () => {
    expect(lastNewsOf({ startedAt: '2026-11-13T08:00:00.000Z', turns: [] })).toBe(
      '2026-11-13T08:00:00.000Z',
    );
  });
});

describe('problemInWords', () => {
  it('makes a sentence of a fragment', () => {
    expect(problemInWords('the sync folder D:\\Sync cannot be reached')).toBe(
      'The sync folder D:\\Sync cannot be reached.',
    );
    expect(problemInWords(NEW_IDENTITY.problem)).toBe(NEW_IDENTITY.problem);
  });
});

describe('noticeTarget', () => {
  const notice: SyncNotice = {
    id: 1,
    at: '2026-11-13T09:00:00.000Z',
    kind: 'task_renumbered',
    projectId: 'p',
    taskId: 't',
    message: 'IOT-9 is now IOT-24.',
  };

  it('leads to the task, to the project, or nowhere', () => {
    expect(noticeTarget(notice, ['p'])).toEqual({ kind: 'task', projectId: 'p', taskId: 't' });
    expect(noticeTarget({ ...notice, taskId: null }, ['p'])).toEqual({
      kind: 'project',
      projectId: 'p',
    });
    expect(noticeTarget({ ...notice, projectId: null, taskId: null }, ['p'])).toEqual({
      kind: 'none',
    });
  });

  it('leads nowhere when the project is gone or put away', () => {
    expect(noticeTarget(notice, ['other'])).toEqual({ kind: 'none' });
  });
});

describe('isStaleAfterChanges', () => {
  it('asks again for everything of a changed project', () => {
    expect(isStaleAfterChanges(['tasks', 'p'], ['p'])).toBe(true);
    expect(isStaleAfterChanges(['conversations', 'p'], ['p'])).toBe(true);
    expect(isStaleAfterChanges(['tasks', 'q'], ['p'])).toBe(false);
  });

  it('asks again for what is not of one project', () => {
    expect(isStaleAfterChanges(['projects', 'open'], ['p'])).toBe(true);
    expect(isStaleAfterChanges(['conversations-waiting'], ['p'])).toBe(true);
    expect(isStaleAfterChanges(['task-history', 't', 'when'], ['p'])).toBe(true);
    expect(isStaleAfterChanges(['sprint-report', 's', 1], ['p'])).toBe(true);
  });

  it('leaves the program and the state of syncing alone', () => {
    expect(isStaleAfterChanges(['agent-program'], ['p'])).toBe(false);
    expect(isStaleAfterChanges(['sync', 'state'], ['p'])).toBe(false);
  });
});
