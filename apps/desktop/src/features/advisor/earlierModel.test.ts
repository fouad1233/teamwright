import { describe, expect, it } from 'vitest';

import type { ConversationsOfFolder, EarlierConversation } from '@/shared/api/types';

import { countOf, durationInWords, filterFolders, nameOf, sizeInWords } from './earlierModel';

function conversation(title: string | null, firstRequest: string | null): EarlierConversation {
  return {
    source: `folder/${title ?? firstRequest ?? 'none'}`,
    heldIn: '/home/ada/air',
    title,
    firstRequest,
    startedAt: null,
    endedAt: null,
    sizeBytes: 100,
    isKept: false,
  };
}

const FOLDERS: ConversationsOfFolder[] = [
  {
    folder: '/home/ada/air',
    isOfProject: true,
    conversations: [
      conversation('Sensor choice', 'Help me choose the sensors'),
      conversation(null, 'Can it run on a battery?'),
    ],
  },
  {
    folder: '/home/ada/website',
    isOfProject: false,
    conversations: [conversation('A page', 'Make a website')],
  },
];

describe('filterFolders', () => {
  it('keeps everything for an empty filter', () => {
    expect(filterFolders(FOLDERS, '  ')).toEqual(FOLDERS);
  });

  it('keeps the conversations that hold every word, in title, request or folder', () => {
    const found = filterFolders(FOLDERS, 'SENSORS help');

    expect(found).toHaveLength(1);
    expect(found[0]?.conversations.map(nameOf)).toEqual(['Sensor choice']);
  });

  it('matches the folder too', () => {
    expect(countOf(filterFolders(FOLDERS, 'website'))).toBe(1);
  });

  it('leaves out folders with nothing that matches', () => {
    expect(filterFolders(FOLDERS, 'nothing like this')).toEqual([]);
  });
});

describe('nameOf', () => {
  it('is the title, else the first request', () => {
    expect(nameOf(conversation('A', 'B'))).toBe('A');
    expect(nameOf(conversation(null, 'B'))).toBe('B');
    expect(nameOf(conversation(null, null))).toBe('A conversation without a request');
  });
});

describe('durationInWords', () => {
  it('says how long a conversation went', () => {
    expect(durationInWords('2026-11-01T09:00:00Z', '2026-11-01T09:30:00Z')).toBe('under an hour');
    expect(durationInWords('2026-11-01T09:00:00Z', '2026-11-01T10:10:00Z')).toBe('1 hour');
    expect(durationInWords('2026-11-01T09:00:00Z', '2026-11-01T14:00:00Z')).toBe('5 hours');
    expect(durationInWords('2026-11-01T09:00:00Z', '2026-11-02T10:00:00Z')).toBe('1 day');
    expect(durationInWords('2026-11-01T09:00:00Z', '2026-11-04T09:00:00Z')).toBe('3 days');
  });

  it('says nothing when it is not known', () => {
    expect(durationInWords(null, '2026-11-01T09:00:00Z')).toBeNull();
    expect(durationInWords('2026-11-02T09:00:00Z', '2026-11-01T09:00:00Z')).toBeNull();
  });
});

describe('sizeInWords', () => {
  it('writes bytes, kilobytes and megabytes', () => {
    expect(sizeInWords(512)).toBe('512 bytes');
    expect(sizeInWords(12_300)).toBe('12 KB');
    expect(sizeInWords(3_565_158)).toBe('3.4 MB');
    expect(sizeInWords(129_644_729)).toBe('124 MB');
  });
});
