import { describe, expect, it } from 'vitest';

import type { Project, Task } from '@/shared/api/types';

import {
  INTRODUCTION_MAX_LENGTH,
  NEW_REPORT,
  attentionInWords,
  comingNext,
  dateLine,
  firstLines,
  moreInWords,
  parseReport,
  progressBar,
  serializeReport,
  shows,
  tasksInWork,
  togglePart,
  withIntroduction,
} from './reportModel';

// Named the way a manager might name them: the rules follow the categories.
const project: Project = {
  id: 'project',
  key: 'SSN',
  name: 'Smart sensor node',
  description: '',
  statuses: [
    { id: 'ideas', name: 'Ideas', category: 'todo', color: 'gray' },
    { id: 'bench', name: 'On the bench', category: 'active', color: 'blue' },
    { id: 'look', name: 'Second look', category: 'review', color: 'purple' },
    { id: 'out', name: 'Out of the door', category: 'done', color: 'green' },
  ],
  createdAt: '2026-11-01T09:00:00.000Z',
  archivedAt: null,
};

/** A Thursday. Its week runs from Monday 9 to Sunday 15 November. */
const TODAY = '2026-11-12';

function task(overrides: Partial<Task> & Pick<Task, 'id' | 'number'>): Task {
  return {
    projectId: 'project',
    areaId: 'area',
    title: `Task ${overrides.number.toString()}`,
    description: '',
    sprintId: null,
    points: null,
    statusId: 'ideas',
    priority: 'normal',
    assignee: null,
    parentId: null,
    blockedBy: [],
    estimateMinutes: null,
    startOn: null,
    dueOn: null,
    rank: 0,
    createdAt: '2026-11-01T09:00:00.000Z',
    updatedAt: '2026-11-01T09:00:00.000Z',
    completedAt: null,
    ...overrides,
  };
}

describe('parseReport', () => {
  it('reads nothing, an empty text or damaged content as a new report', () => {
    expect(parseReport(null)).toEqual(NEW_REPORT);
    expect(parseReport('  ')).toEqual(NEW_REPORT);
    expect(parseReport('{not json')).toEqual(NEW_REPORT);
    expect(parseReport('[1, 2]')).toEqual(NEW_REPORT);
    expect(parseReport('"text"')).toEqual(NEW_REPORT);
  });

  it('leaves out only the usage in a new report', () => {
    expect(shows(NEW_REPORT, 'usage')).toBe(false);
    expect(shows(NEW_REPORT, 'sprint')).toBe(true);
  });

  it('reads back what it wrote', () => {
    const settings = { introduction: 'For the board.\nSecond line.', hidden: ['sprint' as const] };
    expect(parseReport(serializeReport(settings))).toEqual(settings);
  });

  it('drops parts it does not know, and doubles, and keeps the order of the sheet', () => {
    expect(
      parseReport(JSON.stringify({ hidden: ['team', 'charts', 'sprint', 'team', 4] })).hidden,
    ).toEqual(['sprint', 'team']);
  });

  it('shows every part when the content leaves nothing out', () => {
    expect(parseReport(JSON.stringify({ introduction: 'Hello', hidden: [] }))).toEqual({
      introduction: 'Hello',
      hidden: [],
    });
  });

  it('takes the usual parts when the list is missing, and no text that is not text', () => {
    expect(parseReport(JSON.stringify({ introduction: 7 }))).toEqual(NEW_REPORT);
  });

  it('cuts an introduction that is too long', () => {
    const long = 'a'.repeat(INTRODUCTION_MAX_LENGTH + 10);
    expect(parseReport(JSON.stringify({ introduction: long })).introduction).toHaveLength(
      INTRODUCTION_MAX_LENGTH,
    );
  });
});

describe('the changes', () => {
  it('turn a part off and on again, in the order of the sheet', () => {
    const off = togglePart(togglePart(NEW_REPORT, 'team'), 'standing');
    expect(off.hidden).toEqual(['standing', 'team', 'usage']);
    expect(togglePart(off, 'team').hidden).toEqual(['standing', 'usage']);
  });

  it('keep the introduction to the longest that is kept', () => {
    expect(withIntroduction(NEW_REPORT, 'Short').introduction).toBe('Short');
    expect(
      withIntroduction(NEW_REPORT, 'b'.repeat(INTRODUCTION_MAX_LENGTH + 1)).introduction,
    ).toHaveLength(INTRODUCTION_MAX_LENGTH);
  });
});

describe('dateLine', () => {
  it('writes the day in full with its year', () => {
    expect(dateLine(TODAY)).toBe('Thursday 12 November 2026');
  });
});

describe('progressBar', () => {
  it('gives the shares of the finished and the begun', () => {
    expect(progressBar({ total: 20, finished: 5, inWork: 4 })).toEqual({
      finished: 0.25,
      inWork: 0.2,
    });
  });

  it('is empty for a project without tasks', () => {
    expect(progressBar({ total: 0, finished: 0, inWork: 0 })).toEqual({ finished: 0, inWork: 0 });
  });
});

describe('the lists', () => {
  const tasks = [
    task({ id: 'c', number: 3, statusId: 'look' }),
    task({ id: 'a', number: 1, statusId: 'bench', dueOn: '2026-11-10' }),
    task({ id: 'b', number: 2, statusId: 'out', dueOn: '2026-11-13' }),
    task({ id: 'd', number: 4, dueOn: '2026-11-15' }),
    task({ id: 'e', number: 5, dueOn: '2026-11-12' }),
    task({ id: 'f', number: 6, dueOn: '2026-11-16' }),
  ];

  it('name the tasks in work by the category of their status, in the order of their keys', () => {
    expect(tasksInWork(project, tasks).map((each) => each.id)).toEqual(['a', 'c']);
  });

  it('put next what is due by Sunday and not late, the first due first', () => {
    expect(comingNext(project, tasks, TODAY).map((each) => each.id)).toEqual(['e', 'd']);
  });

  it('cut a long list and say how much more there is', () => {
    expect(firstLines([1, 2, 3, 4], 3)).toEqual({ lines: [1, 2, 3], more: 1 });
    expect(firstLines([1, 2], 3)).toEqual({ lines: [1, 2], more: 0 });
    expect(moreInWords(4)).toBe('And 4 more.');
  });

  it('say what is late and what waits, for a reader who is not the manager', () => {
    expect(attentionInWords(0, 0)).toBe('Nothing is late. Nothing waits for the manager.');
    expect(attentionInWords(2, 1)).toBe('1 thing waits for the manager.');
    expect(attentionInWords(0, 3)).toBe('Nothing is late. 3 things wait for the manager.');
  });
});
