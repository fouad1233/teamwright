import { describe, expect, it } from 'vitest';

import { PretendedGitHub, mentions } from './gitHub';

const tasks = [
  { number: 1, title: 'Draw the schematic', description: '' },
  { number: 2, title: 'Route the board', description: 'Two layers.' },
  { number: 3, title: 'Write the firmware', description: '' },
  { number: 41, title: 'Order parts', description: '' },
];

describe('the GitHub of the demo', () => {
  it('finds a reference whole only, as the real application does', () => {
    expect(mentions('sh-4-schematic', 'SH-4')).toBe(true);
    expect(mentions('SH-41 route', 'SH-4')).toBe(false);
    expect(mentions('XSH-4', 'SH-4')).toBe(false);
  });

  it('reads nothing until asked, then counts what is about tasks', () => {
    const gitHub = new PretendedGitHub();
    const before = gitHub.overview('p', 'SH', 'octo-org/node', true, tasks);
    expect(before.readAt).toBeNull();
    expect(before.linkedPullRequests).toBe(0);

    gitHub.read('p', '2026-11-16T09:00:00.000Z');
    const after = gitHub.overview('p', 'SH', 'octo-org/node', true, tasks);
    expect(after.linkedPullRequests).toBe(3);
    expect(after.linkedIssues).toBe(2);
    expect(after.unlinked.map((item) => item.number)).toEqual([38, 37]);
    expect(gitHub.ofTask('p', 'SH', 'octo-org/node', tasks, tasks[2]!).items).toHaveLength(2);
  });

  it('opens an issue for a task with its reference first', () => {
    const gitHub = new PretendedGitHub();
    const draft = gitHub.draftOf('SH', tasks[1]!, 'octo-org/node');
    expect(draft.title).toBe('SH-2 Route the board');
    expect(draft.body).toBe('Two layers.\n\nOpened from Teamwright for SH-2.');
    gitHub.open('p', draft);
    expect(gitHub.ofTask('p', 'SH', 'octo-org/node', tasks, tasks[1]!).items).toHaveLength(2);
  });
});
