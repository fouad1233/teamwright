import { describe, expect, it } from 'vitest';

import type { Conversation, DecisionRequest, Interview, Member, Turn } from '@/shared/api/types';

import {
  ADVISOR,
  addressees,
  canSend,
  firstRecipient,
  howToSend,
  latestTalkWith,
  runsOfTalk,
  sendingInWords,
} from './chatModel';

function turn(overrides: Partial<Turn> & Pick<Turn, 'id' | 'startedAt'>): Turn {
  return {
    request: 'What do you see?',
    origin: 'message',
    state: 'finished',
    endedAt: overrides.startedAt,
    steps: [],
    answer: 'This.',
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

function conversation(overrides: Partial<Conversation> & Pick<Conversation, 'id'>): Conversation {
  return {
    projectId: 'project',
    isWithAdvisor: false,
    memberId: 'hardware',
    taskId: null,
    startedAt: '2026-11-16T09:00:00.000Z',
    homeComputer: 'desk',
    isHere: true,
    turns: [],
    context: null,
    ...overrides,
  };
}

function member(id: string, kind: Member['kind'] = 'agent'): Member {
  return {
    id,
    projectId: 'project',
    name: `${id} engineer`,
    kind,
    responsibility: '',
    model: null,
    effort: null,
    instructions: '',
    color: 'teal',
    joinedAt: '2026-11-01T09:00:00.000Z',
    updatedAt: '2026-11-01T09:00:00.000Z',
  };
}

const at = (minute: number) => `2026-11-16T09:${minute.toString().padStart(2, '0')}:00.000Z`;

const running = (id: string, minute: number) =>
  turn({ id, startedAt: at(minute), state: 'running', endedAt: null, answer: '' });

const decision: DecisionRequest = {
  question: 'Which sensor?',
  reason: '',
  options: [
    { label: 'The cheap one', consequence: '' },
    { label: 'The exact one', consequence: '' },
  ],
  recommended: null,
  answer: null,
};

describe('runsOfTalk', () => {
  it('lays the turns of several conversations out in the order of time', () => {
    const hardware = conversation({
      id: 'hardware chat',
      turns: [turn({ id: 'h1', startedAt: at(1) }), turn({ id: 'h2', startedAt: at(5) })],
    });
    const firmware = conversation({
      id: 'firmware chat',
      memberId: 'firmware',
      startedAt: at(2),
      turns: [turn({ id: 'f1', startedAt: at(2) }), turn({ id: 'f2', startedAt: at(3) })],
    });

    const runs = runsOfTalk([hardware, firmware]);

    expect(runs.map((run) => run.turns.map((each) => each.id))).toEqual([
      ['h1'],
      ['f1', 'f2'],
      ['h2'],
    ]);
    expect(runs.map((run) => run.startsConversation)).toEqual([true, true, false]);
    expect(runs.map((run) => run.isLastOfConversation)).toEqual([false, true, true]);
  });

  it('leaves out the work on tasks', () => {
    const work = conversation({
      id: 'work',
      taskId: 'task',
      turns: [turn({ id: 'w1', startedAt: at(1), origin: 'brief' })],
    });
    const chat = conversation({ id: 'chat', turns: [turn({ id: 'c1', startedAt: at(2) })] });

    expect(runsOfTalk([work, chat]).map((run) => run.key)).toEqual(['c1']);
  });

  it('keeps the talk with the Advisor', () => {
    const advisor = conversation({
      id: 'advisor',
      isWithAdvisor: true,
      memberId: null,
      turns: [turn({ id: 'a1', startedAt: at(1), origin: 'brief' })],
    });

    expect(runsOfTalk([advisor])).toHaveLength(1);
  });

  it('keeps two turns that started in the same moment in their order', () => {
    const chat = conversation({
      id: 'chat',
      turns: [turn({ id: 'first', startedAt: at(1) }), turn({ id: 'second', startedAt: at(1) })],
    });

    expect(runsOfTalk([chat])[0]?.turns.map((each) => each.id)).toEqual(['first', 'second']);
  });

  it('is empty without a conversation', () => {
    expect(runsOfTalk([])).toEqual([]);
  });
});

describe('addressees', () => {
  const team = [member('hardware'), member('maker', 'human'), member('firmware')];

  it('offers the AI specialists, in the order of the team', () => {
    expect(addressees(team, []).map((each) => each.recipient)).toEqual(['hardware', 'firmware']);
  });

  it('offers the Advisor once a plan has been started with it', () => {
    const advisor = conversation({ id: 'advisor', isWithAdvisor: true, memberId: null });

    expect(addressees(team, [advisor]).map((each) => each.recipient)).toEqual([
      'hardware',
      'firmware',
      ADVISOR,
    ]);
  });
});

describe('firstRecipient', () => {
  const offered = addressees([member('hardware'), member('firmware')], []);

  it('is whoever the talk ended with', () => {
    const chat = conversation({
      id: 'chat',
      memberId: 'firmware',
      turns: [turn({ id: 'f1', startedAt: at(1) })],
    });

    expect(firstRecipient(runsOfTalk([chat]), offered)).toBe('firmware');
  });

  it('is the first specialist when the talk ended with somebody who has left', () => {
    const chat = conversation({
      id: 'chat',
      memberId: null,
      turns: [turn({ id: 'x1', startedAt: at(1) })],
    });

    expect(firstRecipient(runsOfTalk([chat]), offered)).toBe('hardware');
  });

  it('is nobody when there is nobody to write to', () => {
    expect(firstRecipient([], [])).toBeNull();
  });
});

describe('latestTalkWith', () => {
  it('is the conversation with a specialist that started last, not one about a task', () => {
    const conversations = [
      conversation({ id: 'later', startedAt: at(5) }),
      conversation({ id: 'earlier', startedAt: at(1) }),
      conversation({ id: 'work', startedAt: at(9), taskId: 'task' }),
    ];

    expect(latestTalkWith(conversations, 'hardware')?.id).toBe('later');
    expect(latestTalkWith(conversations, 'firmware')).toBeUndefined();
  });
});

describe('howToSend', () => {
  it('starts a conversation with a specialist that has none', () => {
    expect(howToSend([], 'hardware')).toEqual({ kind: 'ask' });
  });

  it('continues the open conversation', () => {
    const chat = conversation({ id: 'chat', turns: [turn({ id: 'h1', startedAt: at(1) })] });

    expect(howToSend([chat], 'hardware')).toEqual({ kind: 'reply', conversation: chat });
  });

  it('starts a new conversation where the last one runs on another computer', () => {
    const chat = conversation({ id: 'chat', isHere: false });

    expect(howToSend([chat], 'hardware')).toEqual({ kind: 'ask' });
  });

  it('holds a message while the specialist answers', () => {
    const chat = conversation({ id: 'chat', turns: [running('h1', 1)] });

    const sending = howToSend([chat], 'hardware');

    expect(sending).toEqual({ kind: 'busy', conversation: chat });
    expect(canSend(sending)).toBe(false);
  });

  it('holds a message while the specialist works on a task', () => {
    const work = conversation({ id: 'work', taskId: 'task', turns: [running('w1', 1)] });
    const chat = conversation({ id: 'chat', turns: [turn({ id: 'h1', startedAt: at(0) })] });

    expect(howToSend([work, chat], 'hardware')).toEqual({ kind: 'busy', conversation: work });
  });

  it('waits for the decision a specialist asked for', () => {
    const chat = conversation({
      id: 'chat',
      turns: [turn({ id: 'h1', startedAt: at(1), decision })],
    });

    expect(howToSend([chat], 'hardware').kind).toBe('decision');
  });

  it('continues the conversation with the Advisor', () => {
    const advisor = conversation({
      id: 'advisor',
      isWithAdvisor: true,
      memberId: null,
      turns: [turn({ id: 'a1', startedAt: at(1), origin: 'brief' })],
    });

    expect(howToSend([advisor], ADVISOR)).toEqual({ kind: 'reply', conversation: advisor });
  });

  it('sends the manager to the Advisor while its questions are open', () => {
    const interview: Interview = { questions: [], answers: null, answeredAt: null };
    const advisor = conversation({
      id: 'advisor',
      isWithAdvisor: true,
      memberId: null,
      turns: [turn({ id: 'a1', startedAt: at(1), origin: 'brief', interview })],
    });

    expect(howToSend([advisor], ADVISOR).kind).toBe('interview');
  });

  it('leaves a conversation with the Advisor on another computer to that computer', () => {
    const advisor = conversation({
      id: 'advisor',
      isWithAdvisor: true,
      memberId: null,
      isHere: false,
    });

    expect(howToSend([advisor], ADVISOR).kind).toBe('elsewhere');
  });

  it('holds a message while the Advisor works', () => {
    const advisor = conversation({
      id: 'advisor',
      isWithAdvisor: true,
      memberId: null,
      turns: [running('a1', 1)],
    });

    expect(howToSend([advisor], ADVISOR).kind).toBe('busy');
  });

  it('has nobody to send to without a recipient', () => {
    expect(howToSend([], null)).toEqual({ kind: 'nobody' });
    expect(howToSend([], ADVISOR)).toEqual({ kind: 'nobody' });
  });
});

describe('sendingInWords', () => {
  const taskName = (taskId: string) => `SH-${taskId}`;

  it('says that a specialist is answering', () => {
    const chat = conversation({ id: 'chat', turns: [running('h1', 1)] });

    expect(sendingInWords({ kind: 'busy', conversation: chat }, 'Ada', taskName)).toBe(
      'Ada is answering. You can write again when it is done.',
    );
  });

  it('names the task a specialist works on', () => {
    const work = conversation({ id: 'work', taskId: '4', turns: [running('w1', 1)] });

    expect(sendingInWords({ kind: 'busy', conversation: work }, 'Ada', taskName)).toBe(
      'Ada is working on SH-4. You can write when that is done.',
    );
  });
});
