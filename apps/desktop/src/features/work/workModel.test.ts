import { describe, expect, it } from 'vitest';

import type { AgentProgram, Conversation, DecisionRequest, Turn } from '@/shared/api/types';

import {
  allTokens,
  askedAt,
  chatsWith,
  conversationOfTask,
  decisionInWords,
  durationInWords,
  formatTokens,
  hasSession,
  hasTurnStartedOrEnded,
  openDecision,
  runningTurn,
  signInInWords,
  tasksBeingWorkedOn,
  tasksWaitingForManager,
  tokensInWords,
  tokensOfTurns,
  whatMemberIsWorkingOn,
  whatMemberWaitsFor,
  whyProgramIsNotReady,
  withConversation,
} from './workModel';

function turn(overrides: Partial<Turn> & Pick<Turn, 'id'>): Turn {
  return {
    request: 'Do it',
    origin: 'brief',
    state: 'finished',
    startedAt: '2026-11-11T09:00:00.000Z',
    endedAt: '2026-11-11T09:00:38.000Z',
    steps: [],
    answer: 'Done.',
    failure: null,
    model: 'simulated',
    tokens: { input: 10, output: 100, cacheRead: 1000, cacheWritten: 5 },
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
    memberId: 'member',
    taskId: null,
    startedAt: '2026-11-11T09:00:00.000Z',
    turns: [],
    ...overrides,
  };
}

const running = turn({ id: 'running', state: 'running', endedAt: null, tokens: null, answer: '' });

describe('conversationOfTask', () => {
  it('finds the conversation about a task', () => {
    const conversations = [
      conversation({ id: 'chat' }),
      conversation({ id: 'work', taskId: 'task' }),
    ];

    expect(conversationOfTask(conversations, 'task')?.id).toBe('work');
    expect(conversationOfTask(conversations, 'other')).toBeUndefined();
  });
});

describe('runningTurn and tasksBeingWorkedOn', () => {
  it('find the turn that is running', () => {
    expect(runningTurn(conversation({ id: 'c', turns: [turn({ id: 'done' }), running] }))?.id).toBe(
      'running',
    );
    expect(runningTurn(conversation({ id: 'c', turns: [turn({ id: 'done' })] }))).toBeUndefined();
  });

  it('name the tasks somebody is working on, and no chat', () => {
    const conversations = [
      conversation({ id: 'busy', taskId: 'task one', turns: [running] }),
      conversation({ id: 'idle', taskId: 'task two', turns: [turn({ id: 'done' })] }),
      conversation({ id: 'chat', turns: [running] }),
    ];

    expect([...tasksBeingWorkedOn(conversations)]).toEqual(['task one']);
  });
});

describe('hasSession', () => {
  it('is true once a turn knows the model it runs on', () => {
    const waiting = turn({ id: 'first', state: 'running', model: null });
    expect(hasSession(conversation({ id: 'new', turns: [] }))).toBe(false);
    expect(hasSession(conversation({ id: 'starting', turns: [waiting] }))).toBe(false);
    expect(
      hasSession(conversation({ id: 'started', turns: [waiting, turn({ id: 'next' })] })),
    ).toBe(true);
  });
});

describe('whatMemberIsWorkingOn and chatsWith', () => {
  const conversations = [
    conversation({ id: 'old chat', turns: [turn({ id: 'answered' })] }),
    conversation({ id: 'work', taskId: 'task', turns: [running] }),
    conversation({ id: 'other', memberId: 'somebody else', turns: [running] }),
    conversation({ id: 'new chat' }),
  ];

  it('find what a member is working on at this moment', () => {
    expect(whatMemberIsWorkingOn(conversations, 'member')?.id).toBe('work');
    expect(whatMemberIsWorkingOn(conversations, 'nobody')).toBeUndefined();
  });

  it('list the conversations with a member that are not about a task, the oldest first', () => {
    expect(chatsWith(conversations, 'member').map((chat) => chat.id)).toEqual([
      'old chat',
      'new chat',
    ]);
  });
});

describe('withConversation', () => {
  it('replaces a conversation that is in the list and adds one that is not', () => {
    const before = [conversation({ id: 'a' })];

    const replaced = withConversation(before, conversation({ id: 'a', taskId: 'task' }));
    const added = withConversation(before, conversation({ id: 'b' }));

    expect(replaced.map((listed) => listed.taskId)).toEqual(['task']);
    expect(added.map((listed) => listed.id)).toEqual(['a', 'b']);
  });
});

describe('hasTurnStartedOrEnded', () => {
  const working = conversation({ id: 'c', turns: [running] });

  it('is true when a conversation is new', () => {
    expect(hasTurnStartedOrEnded(undefined, working)).toBe(true);
  });

  it('is false for a step within a running turn', () => {
    const aStepLater = conversation({
      id: 'c',
      turns: [
        { ...running, steps: [{ kind: 'text', at: '2026-11-11T09:00:01.000Z', text: 'Hi' }] },
      ],
    });

    expect(hasTurnStartedOrEnded(working, aStepLater)).toBe(false);
  });

  it('is true when a turn ends and when another one starts', () => {
    const ended = conversation({ id: 'c', turns: [turn({ id: 'running' })] });
    const again = conversation({ id: 'c', turns: [turn({ id: 'running' }), running] });

    expect(hasTurnStartedOrEnded(working, ended)).toBe(true);
    expect(hasTurnStartedOrEnded(ended, again)).toBe(true);
  });
});

describe('tokensOfTurns and allTokens', () => {
  it('add up the tokens of the turns that have ended', () => {
    const sum = tokensOfTurns([turn({ id: 'a' }), turn({ id: 'b' }), running]);

    expect(sum).toEqual({ input: 20, output: 200, cacheRead: 2000, cacheWritten: 10 });
    expect(allTokens(sum)).toBe(2230);
  });
});

describe('formatTokens', () => {
  it('writes small numbers as they are and large ones short', () => {
    expect(formatTokens(0)).toBe('0');
    expect(formatTokens(880)).toBe('880');
    expect(formatTokens(1232)).toBe('1.2k');
    expect(formatTokens(21_500)).toBe('21.5k');
    expect(formatTokens(162_654)).toBe('163k');
    expect(formatTokens(1_300_000)).toBe('1.3M');
  });
});

describe('tokensInWords', () => {
  it('says what was written, what was read, and what came from the cache', () => {
    expect(tokensInWords({ input: 42, output: 880, cacheRead: 21_500, cacheWritten: 6400 })).toBe(
      '880 tokens written · 6.4k read · 21.5k read again from the cache',
    );
  });

  it('leaves out the kinds that count nothing', () => {
    expect(tokensInWords({ input: 0, output: 12, cacheRead: 0, cacheWritten: 0 })).toBe(
      '12 tokens written',
    );
    expect(tokensInWords({ input: 0, output: 0, cacheRead: 0, cacheWritten: 0 })).toBe('no tokens');
  });
});

describe('durationInWords', () => {
  it('uses the units that fit the length', () => {
    const start = '2026-11-11T09:00:00.000Z';

    expect(durationInWords(start, '2026-11-11T09:00:38.400Z')).toBe('38 s');
    expect(durationInWords(start, '2026-11-11T09:02:05.000Z')).toBe('2 min 5 s');
    expect(durationInWords(start, '2026-11-11T09:03:00.000Z')).toBe('3 min');
    expect(durationInWords(start, '2026-11-11T10:12:30.000Z')).toBe('1 h 12 min');
    expect(durationInWords(start, '2026-11-11T11:00:00.000Z')).toBe('2 h');
  });
});

describe('whyProgramIsNotReady and signInInWords', () => {
  const ready: AgentProgram = {
    state: 'ready',
    version: '2.1.322',
    signInMethod: 'claude.ai',
    subscription: 'team',
    problem: null,
  };

  it('say nothing is wrong with a program that is ready, and name the plan', () => {
    expect(whyProgramIsNotReady(ready)).toBeNull();
    expect(signInInWords(ready)).toBe('the Team plan');
  });

  it('say what to do when the program is missing or signed out', () => {
    expect(whyProgramIsNotReady({ ...ready, state: 'not_installed' })).toContain('not installed');
    expect(whyProgramIsNotReady({ ...ready, state: 'signed_out' })).toContain('claude auth login');
  });

  it('pass on what went wrong when the program could not be asked', () => {
    expect(
      whyProgramIsNotReady({ ...ready, state: 'unavailable', problem: 'it did not answer' }),
    ).toBe('Claude Code could not be asked whether it is ready: it did not answer');
  });

  it('name the kind of sign-in when the plan is not known', () => {
    expect(signInInWords({ ...ready, subscription: null })).toBe('a Claude plan');
    expect(signInInWords({ ...ready, subscription: null, signInMethod: 'api_key' })).toBe(
      'an API key',
    );
  });
});

describe('the questions that wait for the manager', () => {
  const question: DecisionRequest = {
    question: 'Battery or USB power only?',
    reason: 'A battery adds a charger and about 4 mm of height.',
    options: [
      { label: 'Battery and USB', consequence: 'Works anywhere. Taller and dearer.' },
      { label: 'USB only', consequence: 'Flat and cheap. Needs a socket.' },
    ],
    recommended: 1,
    answer: null,
  };
  const asking = turn({ id: 'asking', decision: question, endedAt: '2026-11-11T10:00:00.000Z' });

  it('find the question a conversation ended with, while it is not answered', () => {
    expect(openDecision(conversation({ id: 'waiting', turns: [asking] }))).toBe(question);
    expect(openDecision(conversation({ id: 'reported', turns: [turn({ id: 'done' })] }))).toBe(
      undefined,
    );
  });

  it('take a question for closed once it is answered, and while the next turn runs', () => {
    const answered = turn({
      id: 'answered',
      decision: { ...question, answer: { option: 0, note: '', at: '2026-11-11T11:00:00.000Z' } },
    });
    const next = turn({ id: 'next', origin: 'decision', state: 'running', endedAt: null });

    expect(openDecision(conversation({ id: 'answered', turns: [answered] }))).toBe(undefined);
    expect(openDecision(conversation({ id: 'going on', turns: [answered, next] }))).toBe(undefined);
    expect(openDecision(conversation({ id: 'empty', turns: [] }))).toBe(undefined);
  });

  it('say when a question was asked', () => {
    expect(askedAt(conversation({ id: 'waiting', turns: [asking] }))).toBe(
      '2026-11-11T10:00:00.000Z',
    );
  });

  it('name the tasks that wait for the manager, and no question that is not about a task', () => {
    const waiting = tasksWaitingForManager([
      conversation({ id: 'about a task', taskId: 'schematic', turns: [asking] }),
      conversation({ id: 'a chat', taskId: null, turns: [asking] }),
      conversation({ id: 'reported', taskId: 'board', turns: [turn({ id: 'done' })] }),
    ]);

    expect([...waiting]).toEqual(['schematic']);
  });

  it('find what a member waits for, the longest wait first', () => {
    const earlier = turn({
      id: 'earlier',
      decision: question,
      endedAt: '2026-11-11T08:00:00.000Z',
    });
    const conversations = [
      conversation({ id: 'later', memberId: 'engineer', turns: [asking] }),
      conversation({ id: 'first', memberId: 'engineer', turns: [earlier] }),
      conversation({ id: 'of another', memberId: 'writer', turns: [asking] }),
    ];

    expect(whatMemberWaitsFor(conversations, 'engineer')?.id).toBe('first');
    expect(whatMemberWaitsFor(conversations, 'tester')).toBe(undefined);
  });

  it('put a decision into words: the option, the words of the manager, or both', () => {
    const decided = (answer: DecisionRequest['answer']) => decisionInWords({ ...question, answer });
    const at = '2026-11-11T11:00:00.000Z';

    expect(decided(null)).toBeNull();
    expect(decided({ option: 1, note: '', at })).toBe('USB only');
    expect(decided({ option: 0, note: 'Keep it under 12 mm.', at })).toBe(
      'Battery and USB. Keep it under 12 mm.',
    );
    expect(decided({ option: null, note: 'Neither. Use a coin cell.', at })).toBe(
      'Neither. Use a coin cell.',
    );
  });
});
