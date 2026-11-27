import { describe, expect, it } from 'vitest';

import type {
  ApprovedProposal,
  Conversation,
  InterviewQuestion,
  Member,
  ProposedTask,
  Turn,
} from '@/shared/api/types';

import {
  advisorConversation,
  advisorWaitsFor,
  answerInWords,
  answersOf,
  approvalInWords,
  countWithoutOwner,
  emptyAnswers,
  isAnswered,
  openInterview,
  openProposal,
  ownerChoices,
  ownerInWords,
  planInWords,
  proposalInWords,
  valueOfOwner,
  waitChoices,
  waitedFor,
  waitsForInWords,
} from './advisorModel';

function turn(overrides: Partial<Turn> = {}): Turn {
  return {
    id: 'turn',
    request: 'A small air sensor.',
    origin: 'message',
    state: 'finished',
    startedAt: '2026-11-12T09:00:00.000Z',
    endedAt: '2026-11-12T09:00:20.000Z',
    steps: [],
    answer: '',
    failure: null,
    model: 'simulated',
    tokens: null,
    decision: null,
    interview: null,
    proposal: null,
    planLimitReached: false,
    context: null,
    ...overrides,
  };
}

function conversation(overrides: Partial<Conversation> = {}): Conversation {
  return {
    id: 'conversation',
    projectId: 'project',
    isWithAdvisor: true,
    memberId: null,
    taskId: null,
    startedAt: '2026-11-12T09:00:00.000Z',
    homeComputer: 'desk',
    isHere: true,
    turns: [],
    context: null,
    ...overrides,
  };
}

function task(title: string, overrides: Partial<ProposedTask> = {}): ProposedTask {
  return {
    title,
    description: '',
    owner: null,
    estimateMinutes: null,
    priority: 'normal',
    waitsFor: [],
    partOf: null,
    ...overrides,
  };
}

const QUESTION: InterviewQuestion = {
  question: 'What matters most?',
  reason: 'It decides what comes first.',
  options: [
    { label: 'A prototype soon', consequence: 'The shortest way.' },
    { label: 'The lowest cost', consequence: 'Longer to find parts.' },
  ],
  recommended: 0,
};

const DRAFT: ApprovedProposal = {
  members: [
    { name: 'Hardware engineer', responsibility: '', reason: '', model: null, effort: 'high' },
    { name: 'Firmware engineer', responsibility: '', reason: '', model: 'sonnet', effort: null },
  ],
  tasks: [
    task('Agree on the goal', { owner: { kind: 'manager' }, estimateMinutes: 120 }),
    task('Draw the schematic', {
      owner: { kind: 'member', name: 'Hardware engineer' },
      estimateMinutes: 480,
      waitsFor: [0],
    }),
    task('Write the drivers', {
      owner: { kind: 'member', name: 'Firmware engineer' },
      estimateMinutes: 45,
      waitsFor: [1],
    }),
    task('Test the device', { owner: { kind: 'manager' }, waitsFor: [2, 1] }),
  ],
};

function member(name: string): Member {
  return {
    id: name,
    projectId: 'project',
    name,
    kind: 'agent',
    responsibility: '',
    model: null,
    effort: null,
    instructions: '',
    color: 'teal',
    joinedAt: '2026-11-12T09:00:00.000Z',
    updatedAt: '2026-11-12T09:00:00.000Z',
  };
}

describe('advisorConversation', () => {
  it('is the conversation with the Advisor that was started last', () => {
    const first = conversation({ id: 'first', startedAt: '2026-11-12T09:00:00.000Z' });
    const second = conversation({ id: 'second', startedAt: '2026-11-12T10:00:00.000Z' });
    const other = conversation({
      id: 'other',
      isWithAdvisor: false,
      memberId: 'member',
      startedAt: '2026-11-12T11:00:00.000Z',
    });

    expect(advisorConversation([second, other, first])?.id).toBe('second');
  });

  it('is nothing in a project that was never planned', () => {
    expect(advisorConversation([conversation({ isWithAdvisor: false })])).toBeUndefined();
  });
});

describe('what is open in a conversation', () => {
  const interview = { questions: [QUESTION], answers: null, answeredAt: null };
  const proposal = { summary: 'A device.', ...DRAFT, approvedAt: null };

  it('is the interview of the latest turn until it is answered', () => {
    expect(openInterview(conversation({ turns: [turn({ interview })] }))).toBe(interview);
    expect(
      openInterview(
        conversation({
          turns: [
            turn({
              interview: {
                ...interview,
                answers: [{ option: 0, note: '' }],
                answeredAt: '2026-11-12T09:05:00.000Z',
              },
            }),
          ],
        }),
      ),
    ).toBeUndefined();
  });

  it('is the proposal of the latest turn until it is approved', () => {
    expect(openProposal(conversation({ turns: [turn({ proposal })] }))).toBe(proposal);
    expect(
      openProposal(
        conversation({
          turns: [turn({ proposal: { ...proposal, approvedAt: '2026-11-12T09:05:00.000Z' } })],
        }),
      ),
    ).toBeUndefined();
  });

  it('is nothing while a turn runs, and nothing of an earlier turn', () => {
    const running = turn({ id: 'running', state: 'running', endedAt: null });

    expect(openInterview(conversation({ turns: [turn({ interview }), running] }))).toBeUndefined();
    expect(openProposal(conversation({ turns: [turn({ proposal }), turn()] }))).toBeUndefined();
  });

  it('says in a few words what the Advisor waits for', () => {
    expect(advisorWaitsFor(conversation({ turns: [turn({ interview })] }))).toBe('asks 1 question');
    expect(advisorWaitsFor(conversation({ turns: [turn({ proposal })] }))).toBe(
      'proposes 2 specialists and 4 tasks',
    );
    expect(advisorWaitsFor(conversation({ turns: [turn()] }))).toBeNull();
  });
});

describe('the answers to an interview', () => {
  it('start empty, one for each question', () => {
    expect(emptyAnswers([QUESTION, QUESTION])).toEqual([
      { pick: undefined, note: '' },
      { pick: undefined, note: '' },
    ]);
  });

  it('count as given when an option is picked or something is written', () => {
    expect(isAnswered({ pick: 1, note: '' })).toBe(true);
    expect(isAnswered({ pick: 'own words', note: 'Both.' })).toBe(true);
    expect(isAnswered({ pick: undefined, note: 'Both.' })).toBe(true);
    expect(isAnswered({ pick: 'own words', note: '  ' })).toBe(false);
    expect(isAnswered({ pick: undefined, note: '' })).toBe(false);
  });

  it('are handed over with the option, or with nothing where words answer', () => {
    expect(
      answersOf([
        { pick: 1, note: 'But small.' },
        { pick: 'own words', note: 'Neither.' },
      ]),
    ).toEqual([
      { option: 1, note: 'But small.' },
      { option: null, note: 'Neither.' },
    ]);
  });

  it('read as the option, what was added, or the words alone', () => {
    expect(answerInWords(QUESTION, { option: 1, note: '' })).toBe('The lowest cost');
    expect(answerInWords(QUESTION, { option: 1, note: 'But small.' })).toBe(
      'The lowest cost. But small.',
    );
    expect(answerInWords(QUESTION, { option: null, note: 'Neither.' })).toBe('Neither.');
  });
});

describe('what a proposed task can wait for', () => {
  it('offers the other tasks it does not wait for yet', () => {
    expect(waitChoices(DRAFT.tasks, 3).map((choice) => choice.place)).toEqual([0]);
    expect(waitChoices(DRAFT.tasks, 0).map((choice) => choice.title)).toEqual([
      'Draw the schematic',
      'Write the drivers',
      'Test the device',
    ]);
  });

  it('lists what a task waits for in the order of the plan', () => {
    expect(waitedFor(DRAFT.tasks, 3)).toEqual([
      { place: 1, title: 'Draw the schematic' },
      { place: 2, title: 'Write the drivers' },
    ]);
    expect(waitedFor(DRAFT.tasks, 0)).toEqual([]);
  });

  it('counts the tasks that are given to nobody', () => {
    expect(countWithoutOwner(DRAFT.tasks)).toBe(0);
    expect(countWithoutOwner([...DRAFT.tasks, task('Order the parts')])).toBe(1);
  });
});

describe('whom a proposed task can be given to', () => {
  it('lists the manager, the proposed members, the team, and nobody', () => {
    const choices = ownerChoices(DRAFT, [member('Mechanical engineer')]);

    expect(choices.map((choice) => choice.label)).toEqual([
      'You',
      'Hardware engineer',
      'Firmware engineer',
      'Mechanical engineer',
      'Nobody',
    ]);
  });

  it('lists a name once that is on the team and in the proposal', () => {
    const choices = ownerChoices(DRAFT, [member('hardware engineer')]);

    expect(choices.map((choice) => choice.label)).toEqual([
      'You',
      'Hardware engineer',
      'Firmware engineer',
      'Nobody',
    ]);
  });

  it('knows the owner of a task among the choices', () => {
    const choices = ownerChoices(DRAFT, []);

    for (const proposed of DRAFT.tasks) {
      const value = valueOfOwner(proposed.owner);
      expect(choices.find((choice) => choice.value === value)?.owner).toEqual(proposed.owner);
    }
    expect(choices.find((choice) => choice.value === valueOfOwner(null))?.label).toBe('Nobody');
  });
});

describe('a proposal in words', () => {
  it('names the owner of a task', () => {
    expect(ownerInWords({ kind: 'manager' })).toBe('You');
    expect(ownerInWords({ kind: 'member', name: 'Hardware engineer' })).toBe('Hardware engineer');
    expect(ownerInWords(null)).toBe('Nobody');
  });

  it('counts the tasks a task waits for from one, in order', () => {
    expect(waitsForInWords(task('Last', { waitsFor: [2, 0] }))).toBe('1, 3');
    expect(waitsForInWords(task('First'))).toBe('');
  });

  it('says how large the plan is, with the time if the Advisor gave sizes', () => {
    expect(planInWords(DRAFT.tasks)).toBe('4 tasks, about 10h 45m');
    expect(planInWords([task('One')])).toBe('1 task');
  });

  it('says what a proposal holds', () => {
    expect(proposalInWords(DRAFT)).toBe('2 specialists and 4 tasks');
    expect(proposalInWords({ members: [], tasks: [task('One')] })).toBe('1 task');
  });

  it('says what approving will do, without the members the team has already', () => {
    expect(approvalInWords(DRAFT, [])).toBe(
      'Adds 2 specialists and 4 tasks to the project. No work starts before you start it.',
    );
    expect(approvalInWords(DRAFT, [member('HARDWARE engineer')])).toBe(
      'Adds 1 specialist and 4 tasks to the project. No work starts before you start it.',
    );
    expect(approvalInWords(DRAFT, [member('Hardware engineer'), member('Firmware engineer')])).toBe(
      'Adds 4 tasks to the project. No work starts before you start it.',
    );
  });
});
