import { describe, expect, it } from 'vitest';

import type { SprintOutcome } from '@/shared/api/types';
import { formatDate } from '@/shared/lib/format';

import {
  askedInWords,
  dueInWords,
  hasOpenPart,
  isPastDue,
  sprintDatesInWords,
  sprintOutcomeInWords,
  waitingInWords,
} from './wholeReportModel';

const sprint: SprintOutcome = {
  name: 'Sprint 1',
  goal: '',
  state: 'completed',
  startOn: '2026-11-02',
  endOn: '2026-11-06',
  unit: 'points',
  committed: 10,
  completed: 5,
};

describe('the words of the sheet of the whole project', () => {
  it('says how a task met its due date', () => {
    expect(dueInWords({ kind: 'none' }, null)).toBe('');
    expect(dueInWords({ kind: 'met' }, '2026-11-10')).toBe('On time');
    expect(dueInWords({ kind: 'missed', days: 1 }, '2026-11-10')).toBe('1 day late');
    expect(dueInWords({ kind: 'late', days: 3 }, '2026-11-10')).toBe('3 days late');
    expect(dueInWords({ kind: 'coming' }, '2026-11-25')).toBe(formatDate('2026-11-25'));
    expect(isPastDue({ kind: 'late', days: 3 })).toBe(true);
    expect(isPastDue({ kind: 'met' })).toBe(false);
  });

  it('says what a sprint finished and when it ran', () => {
    expect(sprintOutcomeInWords(sprint)).toBe('Completed: 5 of 10 points finished');
    expect(sprintOutcomeInWords({ ...sprint, state: 'active', unit: 'tasks', committed: 1 })).toBe(
      'Running: 5 of 1 task finished',
    );
    expect(sprintDatesInWords(sprint)).toBe(
      `${formatDate('2026-11-02')} to ${formatDate('2026-11-06')}`,
    );
    expect(sprintDatesInWords({ ...sprint, startOn: null, endOn: null })).toBe('');
  });

  it('says who asked and what still waits', () => {
    expect(askedInWords({ askedBy: 'Ada', task: 'SSN-4', answeredOn: '2026-11-05' })).toBe(
      `Asked by Ada about SSN-4. Answered on ${formatDate('2026-11-05')}.`,
    );
    expect(waitingInWords(1)).toBe("1 question waits for the manager's answer.");
    expect(waitingInWords(2)).toBe("2 questions wait for the manager's answer.");
    expect(hasOpenPart({ open: [], waiting: 0 })).toBe(false);
    expect(hasOpenPart({ open: [], waiting: 1 })).toBe(true);
  });
});
