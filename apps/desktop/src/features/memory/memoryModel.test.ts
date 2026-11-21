import { describe, expect, it } from 'vitest';

import type { Member, MemoryEntry } from '@/shared/api/types';

import { listingOf, rememberedInTurn, sourceInWords, toolStepInWords } from './memoryModel';

function entry(id: string, changes: Partial<MemoryEntry> = {}): MemoryEntry {
  return {
    id,
    kind: 'fact',
    title: id,
    text: 'Something true.',
    source: 'manager',
    memberId: null,
    conversationId: null,
    turnId: null,
    taskId: null,
    recordedAt: '2026-11-21T09:00:00.000Z',
    replacedBy: null,
    ...changes,
  };
}

const hardware = { id: 'hardware', name: 'Hardware engineer' } as Member;

describe('where an entry came from', () => {
  it('is said with the name of the specialist, or of a former one', () => {
    expect(sourceInWords(entry('a'), [hardware])).toBe('Written by you');
    expect(
      sourceInWords(entry('b', { source: 'specialist', memberId: 'hardware' }), [hardware]),
    ).toBe('Remembered by Hardware engineer');
    expect(
      sourceInWords(entry('c', { source: 'decision_request', memberId: 'hardware' }), [hardware]),
    ).toBe('Decided by you when Hardware engineer asked');
    expect(sourceInWords(entry('d', { source: 'handoff', memberId: 'gone' }), [hardware])).toBe(
      'Handed over by a former teammate',
    );
  });
});

describe('the listing of memory', () => {
  const memory = [
    entry('old fact', { recordedAt: '2026-11-20T09:00:00.000Z', replacedBy: 'new fact' }),
    entry('new fact', { recordedAt: '2026-11-21T10:00:00.000Z' }),
    entry('decision', { kind: 'decision', recordedAt: '2026-11-21T08:00:00.000Z' }),
    entry('contract', { kind: 'contract', recordedAt: '2026-11-19T08:00:00.000Z' }),
  ];

  it('shows the current entries of the kind chosen, the newest first, with counts of each kind', () => {
    const listing = listingOf(memory, undefined, 'all', false);
    expect(listing.shown.map((shown) => shown.id)).toEqual(['new fact', 'decision', 'contract']);
    expect(listing.counts).toEqual({ all: 3, decision: 1, fact: 1, contract: 1 });
    expect(listing.replaced).toBe(1);
    expect(listingOf(memory, undefined, 'decision', false).shown.map((shown) => shown.id)).toEqual([
      'decision',
    ]);
  });

  it('shows the replaced entries when asked, but not while searching', () => {
    expect(listingOf(memory, undefined, 'fact', true).shown.map((shown) => shown.id)).toEqual([
      'new fact',
      'old fact',
    ]);
    const found = [memory[2] as MemoryEntry];
    const searched = listingOf(memory, found, 'all', true);
    expect(searched.shown.map((shown) => shown.id)).toEqual(['decision']);
    expect(searched.replaced).toBe(0);
    expect(searched.counts.all).toBe(1);
  });
});

describe('memory in a conversation', () => {
  it('finds the facts a turn remembered, the oldest first', () => {
    const memory = [
      entry('second', {
        source: 'specialist',
        turnId: 't',
        recordedAt: '2026-11-21T10:00:00.000Z',
      }),
      entry('first', { source: 'specialist', turnId: 't', recordedAt: '2026-11-21T09:00:00.000Z' }),
      entry('other turn', { source: 'specialist', turnId: 'u' }),
      entry('decision', { source: 'decision_request', turnId: 't' }),
    ];
    expect(rememberedInTurn(memory, 't').map((fact) => fact.id)).toEqual(['first', 'second']);
  });

  it('names the tools of memory in words', () => {
    expect(toolStepInWords('mcp__teamwright__search_memory', 'display bus')).toEqual({
      name: 'Memory',
      summary: 'searched for “display bus”',
    });
    expect(toolStepInWords('Read', 'README.md')).toEqual({ name: 'Read', summary: 'README.md' });
  });
});
