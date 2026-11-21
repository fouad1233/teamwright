import { describe, expect, it } from 'vitest';

import type { Handoff, Member, TeamMessage, TeamTalk } from '@/shared/api/types';

import {
  NO_TALK,
  hasTalk,
  kindInWords,
  latestHandoffOf,
  messagesLatestFirst,
  nameOf,
  talkOfTurn,
  waitingCount,
} from './teamworkModel';

function handoff(id: string, taskId: string, turnId: string, publishedAt: string): Handoff {
  return {
    id,
    taskId,
    authorId: 'hardware',
    conversationId: 'conversation',
    turnId,
    summary: `Summary ${id}`,
    contract: '',
    files: [],
    publishedAt,
  };
}

function message(
  id: string,
  turnId: string,
  sentAt: string,
  deliveredTurnId: string | null = null,
): TeamMessage {
  return {
    id,
    fromId: 'hardware',
    toId: 'firmware',
    conversationId: 'conversation',
    turnId,
    kind: 'question',
    text: `Text ${id}`,
    sentAt,
    deliveredTurnId,
    deliveredAt: deliveredTurnId === null ? null : sentAt,
  };
}

const TALK: TeamTalk = {
  handoffs: [
    handoff('first', 'pins', 'turn-1', '2026-11-21T09:00:00.000Z'),
    handoff('second', 'pins', 'turn-2', '2026-11-21T10:00:00.000Z'),
  ],
  messages: [
    message('asked', 'turn-1', '2026-11-21T09:05:00.000Z', 'turn-9'),
    message('waiting', 'turn-2', '2026-11-21T10:05:00.000Z'),
  ],
};

describe('the talk of a turn', () => {
  it('holds what the turn published and sent, and what came with it', () => {
    const first = talkOfTurn(TALK, 'turn-1');
    expect(first.handoffs.map((found) => found.id)).toEqual(['first']);
    expect(first.sent.map((found) => found.id)).toEqual(['asked']);
    expect(first.arrived).toEqual([]);
    expect(talkOfTurn(TALK, 'turn-9').arrived.map((found) => found.id)).toEqual(['asked']);
    expect(hasTalk(first)).toBe(true);
  });

  it('is nothing for a turn without any, or while nothing is loaded', () => {
    expect(talkOfTurn(TALK, 'turn-5')).toBe(NO_TALK);
    expect(talkOfTurn(undefined, 'turn-1')).toBe(NO_TALK);
    expect(hasTalk(NO_TALK)).toBe(false);
  });
});

describe('the handoff of a task', () => {
  it('is the latest it published, with how many came before it', () => {
    const found = latestHandoffOf(TALK.handoffs, 'pins');
    expect(found?.latest.id).toBe('second');
    expect(found?.earlier).toBe(1);
    expect(latestHandoffOf(TALK.handoffs, 'other')).toBeUndefined();
  });
});

describe('the messages between teammates', () => {
  it('are read the latest first, and count those that wait', () => {
    expect(messagesLatestFirst(TALK.messages).map((found) => found.id)).toEqual([
      'waiting',
      'asked',
    ]);
    expect(waitingCount(TALK.messages)).toBe(1);
    expect(kindInWords('question')).toBe('Question');
    expect(kindInWords('note')).toBe('Note');
  });

  it('name a member who has left the team as a former teammate', () => {
    const members = [{ id: 'hardware', name: 'Hardware engineer' } as Member];
    expect(nameOf(members, 'hardware')).toBe('Hardware engineer');
    expect(nameOf(members, 'gone')).toBe('A former teammate');
  });
});
