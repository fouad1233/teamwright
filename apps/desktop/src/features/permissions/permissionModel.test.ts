import { describe, expect, it } from 'vitest';

import type { PermissionRequest } from '@/shared/api/types';
import {
  WHAT_A_COMMAND_MEANS,
  choicesFor,
  permissionInWords,
  standingPermissionsOf,
} from './permissionModel';

function request(change: Partial<PermissionRequest> = {}): PermissionRequest {
  return {
    id: 'request-1',
    projectId: 'project-1',
    memberId: 'member-1',
    conversationId: 'conversation-1',
    turnId: 'turn-1',
    taskId: 'task-1',
    kind: 'command',
    what: 'npm run test',
    reason: 'To run the tests.',
    askedAt: '2026-11-21T22:00:00.000Z',
    answer: null,
    ...change,
  };
}

function allowed(scope: 'once' | 'task' | 'always', withdrawnAt: string | null = null) {
  return {
    decision: 'allowed' as const,
    scope,
    kind: 'command' as const,
    what: 'npm run test',
    note: '',
    answeredAt: '2026-11-21T22:05:00.000Z',
    onThisComputer: true,
    withdrawnAt,
    noLongerValid: false,
  };
}

describe('permissions in words', () => {
  it('says what each kind allows', () => {
    expect(permissionInWords('command', 'npm test')).toBe('Run the command npm test');
    expect(permissionInWords('web_domain', 'docs.rs')).toBe('Fetch web pages from docs.rs');
  });

  it('says plainly what allowing a command means, and what holds it in', () => {
    expect(WHAT_A_COMMAND_MEANS[0]).toBe(
      'This specialist can change the files this command runs (scripts, build files, tests). Allowing it lets the specialist run any program as you.',
    );
    expect(WHAT_A_COMMAND_MEANS[1]).toMatch(/^It runs in a sandbox/);
    expect(WHAT_A_COMMAND_MEANS[1]).toMatch(/has not been proven/);
    expect(WHAT_A_COMMAND_MEANS[2]).toBe(
      'With any allowed command the specialist can also read, list and search files inside the folders of the project with the shell.',
    );
  });
});

describe('the answers to a request', () => {
  it('never offers to allow a command always', () => {
    expect(choicesFor(request(), 'Tester').map(({ choice }) => choice)).toEqual([
      'once',
      'task',
      'refuse',
    ]);
    expect(choicesFor(request({ taskId: null }), 'Tester').map(({ choice }) => choice)).toEqual([
      'once',
      'refuse',
    ]);
  });

  it('offers allowing a web domain always, and for the task only when there is one', () => {
    const domain = { kind: 'web_domain' as const, what: 'docs.rs' };
    expect(choicesFor(request(domain), 'Tester').map(({ choice }) => choice)).toEqual([
      'once',
      'task',
      'always',
      'refuse',
    ]);
    expect(
      choicesFor(request({ ...domain, taskId: null }), 'Tester').map(({ choice }) => choice),
    ).toEqual(['once', 'always', 'refuse']);
  });

  it('offers only to refuse a command while command permissions are switched off', () => {
    expect(choicesFor(request(), 'Tester', false).map(({ choice }) => choice)).toEqual(['refuse']);
    const domain = { kind: 'web_domain' as const, what: 'docs.rs' };
    expect(choicesFor(request(domain), 'Tester', false).map(({ choice }) => choice)).toEqual([
      'once',
      'task',
      'always',
      'refuse',
    ]);
  });

  it('says for a command allowed for a task that only turns the manager starts get it', () => {
    const task = choicesFor(request(), 'Tester').find(({ choice }) => choice === 'task');
    expect(task?.option.consequence).toMatch(/that you start yourself/);
    expect(task?.option.consequence).toMatch(/Never in a turn the coordinator starts/);
  });
});

describe('the standing permissions of a specialist', () => {
  it('are those allowed for a task or always, and not taken back', () => {
    const requests = [
      request({ id: 'open' }),
      request({ id: 'once', answer: allowed('once') }),
      request({ id: 'task', answer: allowed('task') }),
      request({ id: 'always', answer: allowed('always') }),
      request({ id: 'taken back', answer: allowed('always', '2026-11-21T23:00:00.000Z') }),
      request({ id: 'other', memberId: 'member-2', answer: allowed('always') }),
    ];
    expect(standingPermissionsOf(requests, 'member-1').map((standing) => standing.id)).toEqual([
      'task',
      'always',
    ]);
  });
});
