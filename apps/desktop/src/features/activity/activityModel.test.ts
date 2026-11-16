import { describe, expect, it } from 'vitest';

import type { Moment, Project, ProjectChange, TaskChange } from '@/shared/api/types';
import { calendarDateOf } from '@/shared/lib/format';

import {
  changesToShow,
  dayHeading,
  isReadFarEnough,
  isShownBy,
  logDays,
  logEntries,
  longestKey,
  nothingInWords,
  oldestMomentOf,
  type LoggedTask,
  type Narrowing,
} from './activityModel';

const project: Project = {
  id: 'project',
  key: 'SSN',
  name: 'Smart sensor node',
  description: '',
  statuses: [
    { id: 'todo', name: 'To do', category: 'todo', color: 'gray' },
    { id: 'doing', name: 'In progress', category: 'active', color: 'blue' },
    { id: 'done', name: 'Done', category: 'done', color: 'green' },
    { id: 'shipped', name: 'Shipped', category: 'done', color: 'teal' },
  ],
  createdAt: '2026-11-11T09:00:00.000Z',
  archivedAt: null,
};

const schematic: LoggedTask = {
  id: 'schematic',
  number: 4,
  title: 'Draw the schematic',
  gone: false,
};
const board: LoggedTask = { id: 'board', number: 6, title: 'Route the board', gone: false };
const enclosure: LoggedTask = { id: 'print', number: 23, title: 'Print an enclosure', gone: true };

/**
 * A moment of November 2026 where the tests run. The log is told by the
 * clock of the user, so the tests name their moments by that clock and
 * hold in every time zone.
 */
function at(day: number, hours: number, minutes: number, seconds = 0): Moment {
  return new Date(2026, 10, day, hours, minutes, seconds).toISOString();
}

/** A change without its moment and its task. Written this way, it holds for each kind in turn. */
type WhatChanged<Change> = Change extends unknown ? Omit<Change, 'at' | 'computer'> : never;

const OPENED: WhatChanged<TaskChange> = { kind: 'created' };

function change(
  task: LoggedTask,
  moment: Moment,
  what: WhatChanged<TaskChange> = OPENED,
  computer?: string,
): ProjectChange {
  return { ...what, at: moment, task, ...(computer === undefined ? {} : { computer }) };
}

const moved = (fromStatusId: string, toStatusId: string): WhatChanged<TaskChange> => ({
  kind: 'moved',
  fromStatusId,
  toStatusId,
});

const noDates = { startOn: null, dueOn: null };

describe('isShownBy', () => {
  const moment = at(12, 9, 0);
  const everyKind: ProjectChange[] = [
    change(schematic, moment, { kind: 'created' }),
    change(schematic, moment, { kind: 'renamed', from: 'Draw it', to: 'Draw the schematic' }),
    change(schematic, moment, { kind: 'described' }),
    change(schematic, moment, moved('todo', 'doing')),
    change(schematic, moment, { kind: 'assigned', from: null, to: { kind: 'manager' } }),
    change(schematic, moment, { kind: 'prioritized', from: 'normal', to: 'high' }),
    change(schematic, moment, { kind: 'estimated', from: null, to: 120 }),
    change(schematic, moment, {
      kind: 'rescheduled',
      from: noDates,
      to: { startOn: null, dueOn: '2026-11-14' },
    }),
    change(schematic, moment, { kind: 'now_waits_for', taskId: 'board' }),
    change(schematic, moment, { kind: 'no_longer_waits_for', taskId: 'board' }),
  ];

  function kindsShownBy(narrowing: Narrowing): string[] {
    return everyKind.filter((each) => isShownBy(narrowing, each, project)).map((each) => each.kind);
  }

  it('shows every change when the log is not narrowed', () => {
    expect(kindsShownBy('everything')).toHaveLength(everyKind.length);
  });

  it('narrows to the tasks that were opened, or that moved', () => {
    expect(kindsShownBy('opened')).toEqual(['created']);
    expect(kindsShownBy('moved')).toEqual(['moved']);
  });

  it('counts as planned what says when and in what order the work is done', () => {
    expect(kindsShownBy('planned')).toEqual([
      'estimated',
      'rescheduled',
      'now_waits_for',
      'no_longer_waits_for',
    ]);
  });

  it('counts a task as finished when it goes into a status that means done, whatever its name', () => {
    const finished = (from: string, to: string) =>
      isShownBy('finished', change(schematic, moment, moved(from, to)), project);

    expect(finished('doing', 'done')).toBe(true);
    expect(finished('todo', 'shipped')).toBe(true);
    expect(finished('todo', 'doing')).toBe(false);
    // Taken up again, it is not finished.
    expect(finished('done', 'doing')).toBe(false);
    // From one status that means done to another, it was finished before.
    expect(finished('done', 'shipped')).toBe(false);
  });

  it('cannot tell of a status that was removed', () => {
    const finished = (from: string, to: string) =>
      isShownBy('finished', change(schematic, moment, moved(from, to)), project);

    // What the task left does not matter as long as it is done now.
    expect(finished('removed', 'done')).toBe(true);
    expect(finished('doing', 'removed')).toBe(false);
  });
});

describe('changesToShow', () => {
  const changes = [
    change(board, at(12, 11, 0), moved('doing', 'done')),
    change(enclosure, at(12, 10, 0)),
    change(schematic, at(12, 9, 0)),
    change(board, at(12, 8, 0)),
  ];
  const titles = (shown: ProjectChange[]) =>
    shown.map((each) => `${each.task.title}: ${each.kind}`);

  it('keeps the order of the history and leaves out what the log is narrowed away from', () => {
    expect(titles(changesToShow(changes, 'opened', project, null))).toEqual([
      'Print an enclosure: created',
      'Draw the schematic: created',
      'Route the board: created',
    ]);
  });

  it('shows only the tasks of the open area, which a deleted task is not in', () => {
    const area = new Set(['board', 'print-is-gone-from-here']);

    expect(titles(changesToShow(changes, 'everything', project, area))).toEqual([
      'Route the board: moved',
      'Route the board: created',
    ]);
    expect(titles(changesToShow(changes, 'finished', project, area))).toEqual([
      'Route the board: moved',
    ]);
  });
});

describe('logEntries', () => {
  it('makes an entry of every change, the newest first, with its time on the clock', () => {
    const entries = logEntries([
      change(board, at(12, 14, 2, 30), moved('todo', 'doing')),
      change(schematic, at(12, 9, 5, 10)),
    ]);

    expect(entries.map((entry) => [entry.time, entry.task.title, entry.changes.length])).toEqual([
      ['14:02', 'Route the board', 1],
      ['09:05', 'Draw the schematic', 1],
    ]);
    expect(entries[0]?.at).toBe(at(12, 14, 2, 30));
  });

  it('makes one entry of the changes to one task within one minute, in the order they happened', () => {
    const entries = logEntries([
      change(schematic, at(12, 9, 5, 55), { kind: 'estimated', from: null, to: 180 }),
      change(schematic, at(12, 9, 5, 27), { kind: 'prioritized', from: 'normal', to: 'high' }),
      change(schematic, at(12, 9, 5, 4)),
    ]);

    expect(entries).toHaveLength(1);
    expect(entries[0]?.changes.map((each) => each.kind)).toEqual([
      'created',
      'prioritized',
      'estimated',
    ]);
    // The entry stands where its latest change stands.
    expect(entries[0]?.at).toBe(at(12, 9, 5, 55));
  });

  it('gathers the changes of a task even when another task changed between them', () => {
    const entries = logEntries([
      change(schematic, at(12, 9, 5, 50), { kind: 'described' }),
      change(board, at(12, 9, 5, 30)),
      change(schematic, at(12, 9, 5, 10)),
    ]);

    expect(
      entries.map((entry) => [entry.task.title, entry.changes.map((each) => each.kind)]),
    ).toEqual([
      ['Draw the schematic', ['created', 'described']],
      ['Route the board', ['created']],
    ]);
  });

  it('keeps apart what happened in another minute, or on another computer', () => {
    const entries = logEntries([
      change(schematic, at(12, 9, 6, 0), { kind: 'described' }, 'Laptop'),
      change(schematic, at(12, 9, 6, 0), { kind: 'prioritized', from: 'normal', to: 'high' }),
      change(schematic, at(12, 9, 5, 59)),
    ]);

    expect(entries.map((entry) => [entry.time, entry.computer, entry.changes.length])).toEqual([
      ['09:06', 'Laptop', 1],
      ['09:06', undefined, 1],
      ['09:05', undefined, 1],
    ]);
    expect(new Set(entries.map((entry) => entry.key)).size).toBe(3);
  });

  it('tells the tasks of one moment in the order of their numbers', () => {
    const together = at(6, 8, 50);
    const entries = logEntries([
      change(enclosure, together),
      change(schematic, together),
      change(board, together),
    ]);

    expect(entries.map((entry) => entry.task.number)).toEqual([4, 6, 23]);
  });

  it('says which entries repeat the time of the entry above them', () => {
    const entries = logEntries([
      change(board, at(12, 9, 5, 40)),
      change(schematic, at(12, 9, 5, 10)),
      change(schematic, at(12, 9, 4, 0), { kind: 'described' }),
      // The same time on the clock, a day earlier.
      change(board, at(11, 9, 4, 0), { kind: 'described' }),
    ]);

    expect(entries.map((entry) => [entry.time, entry.repeatsTime])).toEqual([
      ['09:05', false],
      ['09:05', true],
      ['09:04', false],
      ['09:04', false],
    ]);
  });

  it('has nothing to tell of no changes', () => {
    expect(logEntries([])).toEqual([]);
  });
});

describe('logDays', () => {
  it('puts the entries under the day they happened on, the newest day first', () => {
    const days = logDays(
      logEntries([
        change(board, at(12, 0, 10)),
        change(schematic, at(11, 23, 50)),
        change(board, at(11, 8, 0), { kind: 'described' }),
        change(schematic, at(9, 12, 0), { kind: 'described' }),
      ]),
    );

    expect(days.map((day) => [day.date, day.entries.length])).toEqual([
      ['2026-11-12', 1],
      ['2026-11-11', 2],
      ['2026-11-09', 1],
    ]);
  });

  it('reads the day off the clock of the user', () => {
    const moment = at(12, 0, 10);
    const [day] = logDays(logEntries([change(board, moment)]));

    expect(day?.date).toBe(calendarDateOf(new Date(moment)));
  });
});

describe('dayHeading', () => {
  it('calls the last two days by a word, with their date beside it', () => {
    expect(dayHeading('2026-11-16', '2026-11-16')).toEqual({
      name: 'Today',
      date: 'Monday 16 November',
    });
    expect(dayHeading('2026-11-15', '2026-11-16')).toEqual({
      name: 'Yesterday',
      date: 'Sunday 15 November',
    });
  });

  it('calls an earlier day by its date', () => {
    expect(dayHeading('2026-11-14', '2026-11-16')).toEqual({
      name: 'Saturday 14 November',
      date: null,
    });
  });

  it('knows yesterday across the turn of a month and of a year', () => {
    expect(dayHeading('2026-10-31', '2026-11-01').name).toBe('Yesterday');
    expect(dayHeading('2026-12-31', '2027-01-01')).toEqual({
      name: 'Yesterday',
      date: 'Thursday 31 December 2026',
    });
  });

  it('says the year of a day that is not of this year', () => {
    expect(dayHeading('2025-11-14', '2026-11-16').name).toBe('Friday 14 November 2025');
  });
});

describe('oldestMomentOf', () => {
  it('is the moment of the last change of the last page that has one', () => {
    const pages = [[change(board, at(12, 10, 0)), change(board, at(12, 9, 0))], []];

    expect(oldestMomentOf(pages)).toBe(at(12, 9, 0));
    expect(oldestMomentOf([[]])).toBeNull();
    expect(oldestMomentOf([])).toBeNull();
  });
});

describe('isReadFarEnough', () => {
  const FULL = 2;
  const first = [change(board, at(12, 12, 0)), change(board, at(12, 11, 0))];
  const second = [change(board, at(12, 10, 0)), change(board, at(12, 9, 0))];
  const firstReading = { to: null, andEarlier: false };

  it('stops at a full page on the first reading', () => {
    expect(isReadFarEnough([first], true, FULL, firstReading)).toBe(true);
  });

  it('stops where the history begins, however short the last page is', () => {
    expect(isReadFarEnough([first, []], false, FULL, firstReading)).toBe(true);
    expect(isReadFarEnough([[]], false, FULL, firstReading)).toBe(true);
    expect(isReadFarEnough([first], false, FULL, { to: at(12, 9, 0), andEarlier: true })).toBe(
      true,
    );
  });

  it('asks on after a page that came back short, which may have been cut at a moment', () => {
    const short = [change(board, at(12, 12, 0))];

    expect(isReadFarEnough([short], true, FULL, firstReading)).toBe(false);
    expect(isReadFarEnough([first, short], true, FULL, firstReading)).toBe(false);
  });

  it('reads back to where the reader was when the history is asked for again', () => {
    const wasReadTo = { to: at(12, 9, 0), andEarlier: false };

    expect(isReadFarEnough([first], true, FULL, wasReadTo)).toBe(false);
    expect(isReadFarEnough([first, second], true, FULL, wasReadTo)).toBe(true);
    // A change made since has pushed the pages on by one: the second page
    // now ends later than the reader had come, so a third is needed.
    const pushedOn = [change(board, at(12, 11, 0)), change(board, at(12, 10, 0))];
    expect(isReadFarEnough([first, pushedOn], true, FULL, wasReadTo)).toBe(false);
  });

  it('reads beyond where the reader was when earlier changes are asked for', () => {
    const wantsEarlier = { to: at(12, 11, 0), andEarlier: true };

    expect(isReadFarEnough([first], true, FULL, wantsEarlier)).toBe(false);
    expect(isReadFarEnough([first, second], true, FULL, wantsEarlier)).toBe(true);
  });
});

describe('longestKey', () => {
  it('counts the characters of the key of the task with the highest number', () => {
    expect(longestKey(project, [4, 21, 9])).toBe('SSN-21'.length);
    expect(longestKey(project, [4, 123])).toBe('SSN-123'.length);
    expect(longestKey({ key: 'IOT' }, [])).toBe('IOT-0'.length);
  });
});

describe('nothingInWords', () => {
  it('has a sentence of its own for every narrowing', () => {
    const narrowings: Narrowing[] = ['everything', 'opened', 'moved', 'finished', 'planned'];
    const sentences = narrowings.map((narrowing) => nothingInWords(narrowing));

    expect(new Set(sentences).size).toBe(narrowings.length);
    expect(sentences.every((sentence) => sentence.endsWith('.'))).toBe(true);
  });
});
