import { describe, expect, it } from 'vitest';

import type { GitHubOverview } from '@/shared/api/types';

import {
  agoInWords,
  gitHubInWords,
  linkedInWords,
  markOf,
  numberInWords,
  stateInWords,
} from './gitHubModel';

const overview: GitHubOverview = {
  signIn: { state: 'signed_in', program: 'gh', account: 'you', problem: null },
  repository: 'octo-org/sensor-node',
  isFromWorkFolder: true,
  readAt: null,
  linkedPullRequests: 1,
  linkedIssues: 2,
  unlinked: [],
  unlinkedCount: 0,
};

describe('the words of GitHub', () => {
  it('says the state of an item and marks it as a status', () => {
    expect(stateInWords({ state: 'merged' })).toBe('Merged');
    expect(markOf({ state: 'draft' })).toEqual({ category: 'todo', color: 'gray' });
    expect(markOf({ state: 'merged' }).category).toBe('done');
    expect(numberInWords({ kind: 'pull_request', number: 7 })).toBe('Pull request #7');
    expect(numberInWords({ kind: 'issue', number: 3 })).toBe('Issue #3');
  });

  it('says how long ago something was read', () => {
    const now = new Date('2026-11-16T10:00:00.000Z');
    expect(agoInWords('2026-11-16T09:59:40.000Z', now)).toBe('just now');
    expect(agoInWords('2026-11-16T09:59:00.000Z', now)).toBe('1 minute ago');
    expect(agoInWords('2026-11-16T09:15:00.000Z', now)).toBe('45 minutes ago');
    expect(agoInWords('2026-11-16T07:00:00.000Z', now)).toBe('3 hours ago');
    expect(agoInWords('2026-11-14T10:00:00.000Z', now)).toBe('2 days ago');
  });

  it('says whether GitHub is on, and what is about tasks', () => {
    expect(gitHubInWords(overview)).toBe('On: octo-org/sensor-node');
    expect(gitHubInWords({ ...overview, repository: null })).toBe('Off: no repository');
    expect(
      gitHubInWords({
        ...overview,
        signIn: { state: 'not_installed', program: null, account: null, problem: 'x' },
      }),
    ).toBe('Off: gh is not installed');
    expect(linkedInWords(overview)).toBe('1 pull request and 2 issues are about tasks.');
  });
});
