import { describe, expect, it } from 'vitest';

import type {
  Conversation,
  DecisionRequest,
  Member,
  Project,
  Sprint,
  SprintReport,
  Task,
  Turn,
} from '@/shared/api/types';

import {
  atWorkInWords,
  countsOf,
  daysLate,
  isFinished,
  isLate,
  largestHolding,
  lateCountInWords,
  lateInWords,
  progressInWords,
  shareOf,
  sprintInWords,
  sprintsToShow,
  standingInWords,
  tasksAtRisk,
  thisWeek,
  usageOf,
  waitingInWords,
  weekOf,
  whoIsAtWork,
  workByHolder,
  workByStatus,
  type StandingSources,
} from './dashboardModel';

// The statuses are named the way a manager might name them. The rules must
// follow their categories, never these names.
const project: Project = {
  id: 'project',
  key: 'SSN',
  name: 'Smart sensor node',
  description: '',
  statuses: [
    { id: 'backlog', name: 'Ideas', category: 'todo', color: 'gray' },
    { id: 'doing', name: 'On the bench', category: 'active', color: 'blue' },
    { id: 'check', name: 'Second look', category: 'review', color: 'purple' },
    { id: 'shipped', name: 'Out of the door', category: 'done', color: 'green' },
  ],
  createdAt: '2026-11-01T09:00:00.000Z',
  archivedAt: null,
};

/** A Thursday. Its week runs from Monday 9 to Sunday 15 November. */
const TODAY = '2026-11-12';

function member(overrides: Partial<Member> & Pick<Member, 'id' | 'name'>): Member {
  return {
    projectId: 'project',
    kind: 'agent',
    responsibility: '',
    model: null,
    effort: null,
    color: 'teal',
    joinedAt: '2026-11-01T09:00:00.000Z',
    updatedAt: '2026-11-01T09:00:00.000Z',
    ...overrides,
  };
}

function task(overrides: Partial<Task> & Pick<Task, 'id' | 'number'>): Task {
  return {
    projectId: 'project',
    areaId: 'area',
    title: `Task ${overrides.number.toString()}`,
    description: '',
    sprintId: null,
    points: null,
    statusId: 'backlog',
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

function turn(overrides: Partial<Turn> & Pick<Turn, 'id'>): Turn {
  return {
    request: 'Do it',
    origin: 'brief',
    state: 'finished',
    startedAt: '2026-11-12T09:00:00.000Z',
    endedAt: '2026-11-12T09:00:38.000Z',
    steps: [],
    answer: 'Done.',
    failure: null,
    model: 'simulated',
    tokens: { input: 1000, output: 500, cacheRead: 2000, cacheWritten: 500 },
    decision: null,
    interview: null,
    proposal: null,
    ...overrides,
  };
}

function conversation(overrides: Partial<Conversation> & Pick<Conversation, 'id'>): Conversation {
  return {
    projectId: 'project',
    isWithAdvisor: false,
    memberId: 'hardware',
    taskId: null,
    startedAt: '2026-11-12T09:00:00.000Z',
    homeComputer: 'desk',
    isHere: true,
    turns: [],
    ...overrides,
  };
}

const running = turn({ id: 'running', state: 'running', endedAt: null, tokens: null });

const question: DecisionRequest = {
  question: 'Which microcontroller?',
  reason: '',
  options: [
    { label: 'ESP32-C3', consequence: '' },
    { label: 'RP2040', consequence: '' },
  ],
  recommended: 0,
  answer: null,
};

const hardware = member({ id: 'hardware', name: 'Hardware engineer' });
const firmware = member({ id: 'firmware', name: 'Firmware engineer' });
const tester = member({ id: 'tester', name: 'Sam', kind: 'human' });

function sprint(overrides: Partial<Sprint> & Pick<Sprint, 'id' | 'name' | 'state'>): Sprint {
  return {
    projectId: 'project',
    goal: '',
    startOn: '2026-11-07',
    endOn: '2026-11-21',
    createdAt: '2026-11-01T09:00:00.000Z',
    startedAt: null,
    completedAt: null,
    summary: null,
    ...overrides,
  };
}

const sprintTwo = sprint({ id: 'two', name: 'Sprint 2', state: 'active' });

function report(remaining: (number | null)[], total = 16): SprintReport {
  return {
    sprintId: 'two',
    unit: 'points',
    total,
    days: remaining.map((left, index) => ({
      date: `2026-11-${(7 + index).toString().padStart(2, '0')}`,
      remaining: left,
    })),
  };
}

/** A small project: two finished, one in work, one in review, two late. */
const tasks: Task[] = [
  task({ id: 'a', number: 1, statusId: 'shipped', completedAt: '2026-11-10T12:00:00.000Z' }),
  task({ id: 'b', number: 2, statusId: 'shipped', completedAt: '2026-11-02T12:00:00.000Z' }),
  task({
    id: 'c',
    number: 3,
    statusId: 'doing',
    dueOn: '2026-11-10',
    assignee: { kind: 'member', memberId: 'hardware' },
  }),
  task({ id: 'd', number: 4, statusId: 'check', dueOn: '2026-11-14' }),
  task({ id: 'e', number: 5, dueOn: '2026-11-03', assignee: { kind: 'manager' } }),
  // Finished, though past its date: it is not late.
  task({ id: 'f', number: 6, statusId: 'shipped', dueOn: '2026-11-01' }),
];

describe('isFinished and isLate', () => {
  it('follow the category of the status, whatever it is called', () => {
    expect(isFinished(project, task({ id: 'x', number: 9, statusId: 'shipped' }))).toBe(true);
    expect(isFinished(project, task({ id: 'x', number: 9, statusId: 'check' }))).toBe(false);
  });

  it('call a task late only when it is not finished and due before today', () => {
    expect(isLate(project, task({ id: 'x', number: 9, dueOn: '2026-11-11' }), TODAY)).toBe(true);
    expect(isLate(project, task({ id: 'x', number: 9, dueOn: TODAY }), TODAY)).toBe(false);
    expect(isLate(project, task({ id: 'x', number: 9 }), TODAY)).toBe(false);
    expect(
      isLate(
        project,
        task({ id: 'x', number: 9, statusId: 'shipped', dueOn: '2026-11-01' }),
        TODAY,
      ),
    ).toBe(false);
  });
});

describe('countsOf', () => {
  it('counts the tasks by the categories of their statuses', () => {
    const conversations = [
      conversation({ id: 'ask', turns: [turn({ id: 't', decision: question })] }),
    ];
    expect(countsOf(project, tasks, conversations, TODAY)).toEqual({
      total: 6,
      open: 3,
      inWork: 2,
      finished: 3,
      late: 2,
      givenToNobody: 1,
      waiting: 1,
    });
  });

  it('counts nothing in a project without tasks', () => {
    expect(countsOf(project, [], [], TODAY)).toEqual({
      total: 0,
      open: 0,
      inWork: 0,
      finished: 0,
      late: 0,
      givenToNobody: 0,
      waiting: 0,
    });
  });
});

describe('tasksAtRisk', () => {
  it('lists the late tasks, the one late longest first', () => {
    expect(tasksAtRisk(project, tasks, TODAY).map((late) => late.id)).toEqual(['e', 'c']);
  });

  it('is empty when nothing is late', () => {
    expect(tasksAtRisk(project, tasks, '2026-11-01')).toEqual([]);
  });

  it('says how late a task is', () => {
    expect(daysLate({ dueOn: '2026-11-03' }, TODAY)).toBe(9);
    expect(lateInWords(1)).toBe('1 day late');
    expect(lateInWords(9)).toBe('9 days late');
  });
});

describe('workByStatus', () => {
  it('counts every status in the order of the workflow, also those without tasks', () => {
    expect(workByStatus(project, tasks).map(({ status, count }) => [status.name, count])).toEqual([
      ['Ideas', 1],
      ['On the bench', 1],
      ['Second look', 1],
      ['Out of the door', 3],
    ]);
  });
});

describe('workByHolder', () => {
  it('lists the manager, every member and nobody, with open and finished tasks', () => {
    const holders = workByHolder(project, tasks, [hardware, tester]);
    expect(holders.map(({ name, open, finished }) => [name, open, finished])).toEqual([
      ['You', 1, 0],
      ['Hardware engineer', 1, 0],
      ['Sam', 0, 0],
      ['Nobody', 1, 3],
    ]);
    expect(largestHolding(holders)).toBe(4);
  });

  it('leaves nobody out when every task is given to someone', () => {
    const given = [task({ id: 'x', number: 1, assignee: { kind: 'manager' } })];
    expect(workByHolder(project, given, []).map((holder) => holder.name)).toEqual(['You']);
  });

  it('keeps a scale of at least one, so that no bar divides by nothing', () => {
    expect(largestHolding(workByHolder(project, [], []))).toBe(1);
    expect(shareOf(0, 0)).toBe(0);
    expect(shareOf(3, 4)).toBe(0.75);
  });
});

describe('thisWeek', () => {
  it('runs from Monday to Sunday', () => {
    expect(weekOf(TODAY)).toEqual({ first: '2026-11-09', last: '2026-11-15' });
  });

  it('lists what was finished since Monday, and what is due by Sunday or late', () => {
    const week = thisWeek(project, tasks, TODAY);
    expect(week.finished.map((done) => done.id)).toEqual(['a']);
    expect(week.due.map((due) => due.id)).toEqual(['e', 'c', 'd']);
  });

  it('does not count a finished task whose status has since moved back', () => {
    const reopened = task({ id: 'x', number: 9, completedAt: '2026-11-11T12:00:00.000Z' });
    expect(thisWeek(project, [reopened], TODAY).finished).toEqual([]);
  });
});

describe('usageOf', () => {
  it('counts the Advisor, every specialist and those who left, but no person', () => {
    const conversations = [
      conversation({ id: 'h', memberId: 'hardware', turns: [turn({ id: '1' }), running] }),
      conversation({ id: 'a', isWithAdvisor: true, memberId: null, turns: [turn({ id: '2' })] }),
      conversation({ id: 'gone', memberId: 'gone', turns: [turn({ id: '3' })] }),
    ];
    const usage = usageOf([hardware, firmware, tester], conversations);
    expect(usage.total).toBe(12000);
    expect(usage.users.map(({ name, tokens }) => [name, tokens])).toEqual([
      ['Advisor', 4000],
      ['Hardware engineer', 4000],
      ['Firmware engineer', 0],
      ['Who left', 4000],
    ]);
  });

  it('lists only the Advisor in a project without a team', () => {
    expect(usageOf([], [])).toEqual({
      total: 0,
      users: [{ key: 'advisor', kind: 'advisor', member: null, name: 'Advisor', tokens: 0 }],
    });
  });
});

describe('whoIsAtWork', () => {
  it('names the specialists with a running turn, and the Advisor', () => {
    const conversations = [
      conversation({ id: 'h', memberId: 'hardware', turns: [running] }),
      conversation({ id: 'f', memberId: 'firmware', turns: [turn({ id: 'f1' })] }),
      conversation({ id: 'a', isWithAdvisor: true, memberId: null, turns: [running] }),
    ];
    expect(whoIsAtWork([hardware, firmware], conversations)).toEqual([
      'Hardware engineer',
      'The Advisor',
    ]);
  });
});

describe('the sentences of the summary', () => {
  it('says how much is finished and how much is in work', () => {
    expect(progressInWords({ total: 17, finished: 12, inWork: 3 })).toBe(
      '12 of 17 tasks are finished, and 3 are in work.',
    );
    expect(progressInWords({ total: 17, finished: 1, inWork: 1 })).toBe(
      '1 of 17 tasks is finished, and 1 is in work.',
    );
    expect(progressInWords({ total: 4, finished: 0, inWork: 0 })).toBe(
      'None of the 4 tasks is finished, and none is in work.',
    );
    expect(progressInWords({ total: 5, finished: 5, inWork: 0 })).toBe('All 5 tasks are finished.');
  });

  it('speaks of a single task as one', () => {
    expect(progressInWords({ total: 1, finished: 1, inWork: 0 })).toBe('Its one task is finished.');
    expect(progressInWords({ total: 1, finished: 0, inWork: 1 })).toBe('Its one task is in work.');
    expect(progressInWords({ total: 1, finished: 0, inWork: 0 })).toBe(
      'Its one task is not begun yet.',
    );
  });

  it('says that a project has no tasks', () => {
    expect(progressInWords({ total: 0, finished: 0, inWork: 0 })).toBe(
      'The project has no tasks yet.',
    );
  });

  it('names who is at work', () => {
    expect(atWorkInWords([])).toBeNull();
    expect(atWorkInWords(['Ada'])).toBe('Ada is at work.');
    expect(atWorkInWords(['Ada', 'Ben'])).toBe('Ada and Ben are at work.');
    expect(atWorkInWords(['Ada', 'Ben', 'The Advisor'])).toBe(
      'Ada, Ben and The Advisor are at work.',
    );
  });

  it('counts what is late and what waits', () => {
    expect(lateCountInWords(1)).toBe('1 task is late.');
    expect(lateCountInWords(2)).toBe('2 tasks are late.');
    expect(waitingInWords(1)).toBe('1 thing waits for you.');
    expect(waitingInWords(3)).toBe('3 things wait for you.');
  });
});

describe('sprintInWords', () => {
  it('says how the running sprint stands', () => {
    expect(sprintInWords([sprintTwo], report([16, 16, 16, 16, 16, 16]), TODAY)).toBe(
      'Sprint 2 has 9 days left and 16 of 16 points to go.',
    );
    expect(sprintInWords([sprintTwo], report([16, 10, 5]), TODAY)).toBe(
      'Sprint 2 has 9 days left and 5 of 16 points to go.',
    );
  });

  it('counts a sprint that has not reached its first day as all to go', () => {
    expect(sprintInWords([sprintTwo], report([null, null]), TODAY)).toBe(
      'Sprint 2 has 9 days left and 16 of 16 points to go.',
    );
  });

  it('says when all is done, nothing is planned, or the sprint counts tasks', () => {
    expect(sprintInWords([sprintTwo], report([0]), TODAY)).toBe(
      'Sprint 2 has 9 days left and all 16 points done.',
    );
    expect(sprintInWords([sprintTwo], report([0], 0), TODAY)).toBe(
      'Sprint 2 has 9 days left and nothing planned into it.',
    );
    expect(sprintInWords([sprintTwo], { ...report([1], 1), unit: 'tasks' }, TODAY)).toBe(
      'Sprint 2 has 9 days left and 1 of 1 task to go.',
    );
  });

  it('says when the sprint ends today or has run past its end', () => {
    expect(sprintInWords([sprintTwo], null, '2026-11-21')).toBe('Sprint 2 ends today.');
    expect(sprintInWords([sprintTwo], null, '2026-11-23')).toBe('Sprint 2 is 2 days past its end.');
    expect(sprintInWords([sprintTwo], null, '2026-11-20')).toBe('Sprint 2 has 1 day left.');
  });

  it('leaves the work out while the report is loading or belongs to another sprint', () => {
    expect(sprintInWords([sprintTwo], { ...report([3]), sprintId: 'one' }, TODAY)).toBe(
      'Sprint 2 has 9 days left.',
    );
  });

  it('says nothing for a project without sprints, and that none runs otherwise', () => {
    expect(sprintInWords([], null, TODAY)).toBeNull();
    expect(
      sprintInWords([sprint({ id: 'three', name: 'Sprint 3', state: 'planned' })], null, TODAY),
    ).toBe('No sprint is running.');
  });

  it('shows the running sprint and those that are over', () => {
    const one = sprint({ id: 'one', name: 'Sprint 1', state: 'completed' });
    const three = sprint({ id: 'three', name: 'Sprint 3', state: 'planned' });
    expect(sprintsToShow([one, sprintTwo, three])).toEqual({ active: sprintTwo, completed: [one] });
  });
});

describe('standingInWords', () => {
  const sources: StandingSources = {
    project,
    tasks,
    members: [hardware],
    conversations: [conversation({ id: 'ask', turns: [turn({ id: 't', decision: question })] })],
    sprints: [sprintTwo],
    report: report([16, 16, 16, 16, 16, 16]),
    today: TODAY,
  };

  it('puts the sentences together in order', () => {
    expect(standingInWords(sources)).toEqual([
      '3 of 6 tasks are finished, and 2 are in work.',
      '2 tasks are late.',
      'Sprint 2 has 9 days left and 16 of 16 points to go.',
      '1 thing waits for you.',
    ]);
  });

  it('names who is at work after what is finished', () => {
    const atWork = {
      ...sources,
      conversations: [conversation({ id: 'h', memberId: 'hardware', turns: [running] })],
    };
    expect(standingInWords(atWork)[1]).toBe('Hardware engineer is at work.');
  });

  it('leaves out what has nothing to say', () => {
    expect(
      standingInWords({
        ...sources,
        tasks: [],
        conversations: [],
        sprints: [],
        report: null,
      }),
    ).toEqual(['The project has no tasks yet.']);
  });

  it('leaves the sprint out while the sprints are loading', () => {
    expect(standingInWords({ ...sources, sprints: null })).not.toContain(
      'Sprint 2 has 9 days left and 16 of 16 points to go.',
    );
  });
});
