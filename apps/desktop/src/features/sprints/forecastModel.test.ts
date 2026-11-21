import { describe, expect, it } from 'vitest';

import type { SprintForecast } from '@/shared/api/types';

import { aheadInWords, loadInWords, paceInWords, withoutPointsInWords } from './forecastModel';

const KNOWN: SprintForecast = {
  velocity: { state: 'known', finished: [], low: 5, high: 11, sprints: 3 },
  unit: 'points',
  planned: [{ sprintId: 'five', work: 16, load: 'above', withoutPoints: [] }],
  ahead: { work: 42, tasks: 12, fewestSprints: 4, mostSprints: 9, withoutPoints: [] },
};

const project = { key: 'SSN' };

describe('the forecast in words', () => {
  it('tells the pace as a range over the sprints it was read from', () => {
    expect(paceInWords(KNOWN)).toBe(
      'The team finished 5–11 points a sprint over its last 3 sprints.',
    );
  });

  it('says that it is too early with fewer than three sprints, and nothing without any', () => {
    expect(
      paceInWords({
        ...KNOWN,
        velocity: { state: 'too_early', finished: [8, 5], low: null, high: null, sprints: null },
      }),
    ).toBe('Too early to tell a pace: 2 sprints completed, 3 are needed.');
    expect(
      paceInWords({
        ...KNOWN,
        velocity: { state: 'no_sprints', finished: [], low: null, high: null, sprints: null },
      }),
    ).toBeNull();
  });

  it('compares a planned sprint with the range', () => {
    const load = KNOWN.planned[0];
    expect(load && loadInWords(load, KNOWN)).toBe(
      'more than the team has finished in a sprint (5–11 points)',
    );
  });

  it('tells how many sprints the rest would take, and leaves the end open after a sprint of nothing', () => {
    expect(aheadInWords(KNOWN)).toBe(
      'The work not finished, 42 points in 12 tasks, would take 4 to 9 sprints at this pace.',
    );
    expect(aheadInWords({ ...KNOWN, ahead: { ...KNOWN.ahead, mostSprints: null } })).toMatch(
      /at least 4 sprints/,
    );
  });

  it('names the tasks without points, the first four of them', () => {
    const tasks = [12, 21, 22, 23, 24].map((number) => ({
      taskId: `t${number.toString()}`,
      number,
      title: '',
    }));

    expect(withoutPointsInWords(project, tasks.slice(0, 1))).toBe(
      '1 task has no points and is not counted: SSN-12.',
    );
    expect(withoutPointsInWords(project, tasks)).toBe(
      '5 tasks have no points and are not counted: SSN-12, SSN-21, SSN-22, SSN-23 and 1 more.',
    );
    expect(withoutPointsInWords(project, [])).toBeNull();
  });
});
