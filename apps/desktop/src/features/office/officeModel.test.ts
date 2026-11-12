import { describe, expect, it } from 'vitest';

import type {
  Conversation,
  DecisionRequest,
  Member,
  Project,
  Task,
  Turn,
} from '@/shared/api/types';

import {
  arrangeOffice,
  countWaitingForManager,
  describeOffice,
  idealColumns,
  isAnybodyWorking,
  lastThingDone,
  placeOfAdvisor,
  placeOfManager,
  placeOfMember,
  placeReference,
  rowLetter,
  rowsOf,
  stepInWords,
  summaryInWords,
  usageInWords,
} from './officeModel';

const project: Project = {
  id: 'project',
  key: 'SSN',
  name: 'Smart sensor node',
  description: '',
  statuses: [
    { id: 'todo', name: 'To do', category: 'todo', color: 'gray' },
    { id: 'doing', name: 'In progress', category: 'active', color: 'blue' },
    { id: 'done', name: 'Done', category: 'done', color: 'green' },
  ],
  createdAt: '2026-11-01T09:00:00.000Z',
  archivedAt: null,
};

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
    title: `Task ${overrides.number.toString()}`,
    description: '',
    sprintId: null,
    points: null,
    statusId: 'todo',
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
    tokens: { input: 1000, output: 500, cacheRead: 20000, cacheWritten: 0 },
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
    turns: [],
    ...overrides,
  };
}

function running(overrides: Partial<Turn> = {}): Turn {
  return turn({
    id: 'running',
    state: 'running',
    startedAt: '2026-11-12T10:00:00.000Z',
    endedAt: null,
    tokens: null,
    answer: '',
    ...overrides,
  });
}

const question: DecisionRequest = {
  question: 'Which microcontroller should the board use? It decides the price.',
  reason: '',
  options: [
    { label: 'ESP32-C3', consequence: '' },
    { label: 'RP2040 with a radio', consequence: '' },
  ],
  recommended: 0,
  answer: null,
};

const NOW = '2026-11-12T10:02:05.000Z';

const hardware = member({ id: 'hardware', name: 'Hardware engineer', model: 'opus' });
const routeTheBoard = task({
  id: 'route',
  number: 6,
  title: 'Route the board',
  statusId: 'doing',
  estimateMinutes: 360,
  assignee: { kind: 'member', memberId: 'hardware' },
});

describe('stepInWords and lastThingDone', () => {
  it('say what a tool did in a few words', () => {
    expect(
      stepInWords({
        at: NOW,
        kind: 'tool',
        id: '1',
        tool: 'Read',
        summary: 'README.md',
        outcome: 'done',
      }),
    ).toBe('Reads README.md');
    expect(
      stepInWords({
        at: NOW,
        kind: 'tool',
        id: '2',
        tool: 'Bash',
        summary: '',
        outcome: 'running',
      }),
    ).toBe('Runs a command');
    expect(
      stepInWords({
        at: NOW,
        kind: 'tool',
        id: '3',
        tool: 'mcp__kicad__drc',
        summary: 'board',
        outcome: 'done',
      }),
    ).toBe('Uses mcp__kicad__drc on board');
    expect(stepInWords({ at: NOW, kind: 'denied', tool: 'WebFetch' })).toBe(
      'Was not allowed to use WebFetch',
    );
  });

  it('quote the first sentence of what the specialist wrote, shortened', () => {
    expect(
      stepInWords({ at: NOW, kind: 'text', text: 'I am reading the task. Then I write.' }),
    ).toBe('“I am reading the task.”');
    const long = stepInWords({
      at: NOW,
      kind: 'text',
      text: 'The schematic has the power supply, the microcontroller and its clock, the connectors and the sensors on four sheets',
    });
    expect(long).toBe('“The schematic has the power supply, the microcontroller and its clock…”');
  });

  it('pass over what the program noted and find the last thing done', () => {
    expect(
      lastThingDone([
        { at: NOW, kind: 'tool', id: '1', tool: 'Write', summary: 'pins.md', outcome: 'done' },
        { at: NOW, kind: 'notice', message: 'Retrying.' },
      ]),
    ).toBe('Writes pins.md');
    expect(lastThingDone([])).toBe('Starting');
  });
});

describe('usageInWords', () => {
  it('writes tokens the way the rest of the program does', () => {
    expect(usageInWords(0)).toBe('nothing yet');
    expect(usageInWords(21_500)).toBe('21.5k tokens');
  });
});

describe('placeOfMember', () => {
  const sources = { project, tasks: [routeTheBoard], conversations: [], now: NOW };

  it('shows a specialist at work: the task, the last step, how long, and the size', () => {
    const work = conversation({
      id: 'work',
      taskId: 'route',
      turns: [
        turn({ id: 'earlier' }),
        running({
          steps: [
            {
              at: NOW,
              kind: 'tool',
              id: 'w',
              tool: 'Write',
              summary: 'board.kicad_pcb',
              outcome: 'running',
            },
          ],
        }),
      ],
    });
    const place = placeOfMember(hardware, { ...sources, conversations: [work] });

    expect(place.state).toBe('working');
    expect(place.stateInWords).toBe('At work');
    expect(place.subject).toEqual({ reference: 'SSN-6', title: 'Route the board' });
    expect(place.detail).toBe('Writes board.kicad_pcb');
    expect(place.measures).toEqual([
      { label: 'For', value: '2 min 5 s' },
      { label: 'Size', value: '6h' },
    ]);
    expect(place.usage).toEqual({ label: 'Used', value: '21.5k tokens' });
    expect(place.role).toBe('AI specialist, Opus');
    expect(place.target).toEqual({ kind: 'task', taskId: 'route' });
    expect(place.actionInWords).toBe('Open SSN-6, Route the board');
  });

  it('shows the tokens of a running turn only when they are known', () => {
    const work = conversation({
      id: 'work',
      taskId: 'route',
      turns: [running({ tokens: { input: 0, output: 1200, cacheRead: 0, cacheWritten: 0 } })],
    });
    const place = placeOfMember(hardware, { ...sources, conversations: [work] });

    expect(place.measures.at(-1)).toEqual({ label: 'This turn', value: '1.2k tokens' });
  });

  it('leads work on a question of the manager to the conversation', () => {
    const chat = conversation({ id: 'chat', turns: [running()] });
    const place = placeOfMember(hardware, { ...sources, conversations: [chat] });

    expect(place.subject).toEqual({ reference: null, title: 'A question of yours' });
    expect(place.target).toEqual({ kind: 'chat', memberId: 'hardware' });
    expect(place.actionInWords).toBe('Open the conversation with Hardware engineer');
  });

  it('shows a specialist that waits for a decision, and on which task', () => {
    const waiting = conversation({
      id: 'waiting',
      taskId: 'route',
      turns: [turn({ id: 'asked', decision: question })],
    });
    const place = placeOfMember(hardware, { ...sources, conversations: [waiting] });

    expect(place.state).toBe('waiting');
    expect(place.stateInWords).toBe('Waits for you');
    expect(place.subject?.reference).toBe('SSN-6');
    expect(place.detail).toBe('Asks you to decide: Which microcontroller should the board use?');
    expect(place.target).toEqual({ kind: 'task', taskId: 'route' });
  });

  it('shows a free specialist with the tasks given to it that have not started', () => {
    const tasks = [
      routeTheBoard,
      task({ id: 'a', number: 7, assignee: { kind: 'member', memberId: 'hardware' } }),
      task({ id: 'b', number: 8, assignee: { kind: 'member', memberId: 'hardware' } }),
      task({
        id: 'c',
        number: 9,
        statusId: 'done',
        assignee: { kind: 'member', memberId: 'hardware' },
      }),
    ];
    const place = placeOfMember(hardware, { ...sources, tasks });

    expect(place.state).toBe('free');
    expect(place.detail).toBe('2 tasks given to it not started');
    expect(place.usage.value).toBe('nothing yet');
    expect(place.target).toEqual({ kind: 'chat', memberId: 'hardware' });

    expect(placeOfMember(hardware, { ...sources, tasks: [] }).detail).toBe(
      'Nothing given to it waits to start',
    );
  });

  it('shows a person with the tasks given to them that are not finished', () => {
    const person = member({ id: 'pat', name: 'Pat', kind: 'human' });
    const tasks = [
      task({ id: 'a', number: 1, assignee: { kind: 'member', memberId: 'pat' } }),
      task({ id: 'b', number: 2, statusId: 'done', assignee: { kind: 'member', memberId: 'pat' } }),
    ];
    const place = placeOfMember(person, { ...sources, tasks });

    expect(place.state).toBe('person');
    expect(place.role).toBe('Person');
    expect(place.detail).toBe('1 task not finished');
    expect(place.target).toBeNull();
  });
});

describe('placeOfAdvisor', () => {
  it('is free and leads to its page before the project is planned', () => {
    const place = placeOfAdvisor([], NOW);

    expect(place.state).toBe('free');
    expect(place.detail).toBe('Has not planned this project yet');
    expect(place.target).toEqual({ kind: 'advisor' });
  });

  it('is at work while its latest conversation runs a turn', () => {
    const planning = conversation({
      id: 'plan',
      isWithAdvisor: true,
      memberId: null,
      turns: [
        running({
          steps: [{ at: NOW, kind: 'tool', id: 'g', tool: 'Glob', summary: '*', outcome: 'done' }],
        }),
      ],
    });
    const place = placeOfAdvisor([planning], NOW);

    expect(place.state).toBe('working');
    expect(place.detail).toBe('Looks for *');
    expect(place.measures).toEqual([{ label: 'For', value: '2 min 5 s' }]);
  });

  it('waits for answers to its questions', () => {
    const asking = conversation({
      id: 'plan',
      isWithAdvisor: true,
      memberId: null,
      turns: [
        turn({
          id: 'asked',
          interview: {
            questions: [
              { question: 'One?', reason: '', options: [], recommended: null },
              { question: 'Two?', reason: '', options: [], recommended: null },
            ],
            answers: null,
            answeredAt: null,
          },
        }),
      ],
    });
    const place = placeOfAdvisor([asking], NOW);

    expect(place.state).toBe('waiting');
    expect(place.detail).toBe('Asks 2 questions');
    expect(place.usage.value).toBe('21.5k tokens');
  });
});

describe('placeOfManager and countWaitingForManager', () => {
  it('count the decisions and what the Advisor waits for', () => {
    const conversations = [
      conversation({ id: 'a', taskId: 'route', turns: [turn({ id: 't', decision: question })] }),
      conversation({ id: 'b', memberId: 'other', turns: [turn({ id: 't', decision: question })] }),
      conversation({ id: 'c', turns: [turn({ id: 't' })] }),
    ];
    expect(countWaitingForManager(conversations)).toBe(2);

    const place = placeOfManager({
      project,
      tasks: [task({ id: 'm', number: 1, assignee: { kind: 'manager' } })],
      conversations,
    });
    expect(place.stateInWords).toBe('2 things wait for you');
    expect(place.detail).toBe('1 task of your own open');
    expect(place.usage).toEqual({ label: 'The project', value: '64.5k tokens' });
    expect(place.target).toEqual({ kind: 'inbox' });
  });

  it('says when nothing waits', () => {
    const place = placeOfManager({ project, tasks: [], conversations: [] });
    expect(place.stateInWords).toBe('Nothing waits for you');
    expect(place.detail).toBe('None of your own tasks is open');
  });
});

describe('describeOffice', () => {
  it('puts the manager and the Advisor at the front and counts the room', () => {
    const firmware = member({ id: 'firmware', name: 'Firmware engineer', color: 'blue' });
    const office = describeOffice({
      project,
      members: [hardware, firmware],
      tasks: [routeTheBoard],
      conversations: [conversation({ id: 'work', taskId: 'route', turns: [running()] })],
      now: NOW,
    });

    expect(office.front.map((place) => place.key)).toEqual(['manager', 'advisor']);
    expect(office.floor.map((place) => place.key)).toEqual(['hardware', 'firmware']);
    expect(office.summary).toEqual({
      working: 1,
      waiting: 0,
      free: 2,
      people: 0,
      used: 'nothing yet',
    });
    expect(isAnybodyWorking(office)).toBe(true);
    expect(summaryInWords(office.summary)).toBe('1 at work');
    expect(summaryInWords({ ...office.summary, working: 0 })).toBe('Nobody is at work');
    expect(summaryInWords({ ...office.summary, waiting: 2 })).toBe('1 at work, 2 waiting for you');
  });
});

describe('the arrangement of the room', () => {
  const measures = { placeWidth: 240, asideWidth: 256, gap: 16, roomInset: 64 };

  it('gives every size of team a composed number of benches in a row', () => {
    expect([1, 2, 3, 4, 5, 6, 7, 8, 12].map(idealColumns)).toEqual([1, 2, 3, 2, 3, 3, 4, 4, 4]);
    expect(idealColumns(0)).toBe(1);
  });

  it('keeps the title block beside the room while the front fits in one row', () => {
    expect(arrangeOffice(3, { ...measures, width: 1140 })).toEqual({
      columns: 3,
      isAsideBeside: true,
    });
    expect(arrangeOffice(8, { ...measures, width: 1140 })).toEqual({
      columns: 3,
      isAsideBeside: true,
    });
  });

  it('puts the title block under the room when the window is narrow', () => {
    expect(arrangeOffice(3, { ...measures, width: 700 })).toEqual({
      columns: 2,
      isAsideBeside: false,
    });
    expect(arrangeOffice(1, { ...measures, width: 300 })).toEqual({
      columns: 1,
      isAsideBeside: false,
    });
  });

  it('fills rows and leaves the rest to the last', () => {
    expect(rowsOf([1, 2, 3, 4, 5], 3)).toEqual([
      [1, 2, 3],
      [4, 5],
    ]);
    expect(rowsOf([1], 0)).toEqual([[1]]);
    expect(rowsOf([], 2)).toEqual([]);
  });

  it('names rows and places as the margin of a plate does', () => {
    expect([0, 1, 25, 26, 27].map(rowLetter)).toEqual(['A', 'B', 'Z', 'AA', 'AB']);
    expect(placeReference(1, 2)).toBe('B3');
  });
});
