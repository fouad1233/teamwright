import { describe, expect, it } from 'vitest';

import type { PermissionRequest } from '@/shared/api/types';
import { choicesFor, permissionInWords, prefixOf, standingPermissionsOf } from './permissionModel';

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
  };
}

describe('permissions in words', () => {
  it('says what each kind allows', () => {
    expect(permissionInWords('command', 'npm test')).toBe('Run the command npm test');
    expect(permissionInWords('command_prefix', 'npm run')).toBe(
      'Run commands that start with npm run',
    );
    expect(permissionInWords('web_domain', 'docs.rs')).toBe('Fetch web pages from docs.rs');
  });

  it('offers the program and its subcommand as the narrowest prefix', () => {
    expect(prefixOf('npm run test -- --watch')).toBe('npm run');
    expect(prefixOf('make')).toBeNull();
  });
});

describe('the answers to a request', () => {
  it('offers allowing for the task only when the conversation is about one', () => {
    expect(choicesFor(request(), 'Tester').map(({ choice }) => choice)).toEqual([
      'once',
      'task',
      'always',
      'refuse',
    ]);
    expect(choicesFor(request({ taskId: null }), 'Tester').map(({ choice }) => choice)).toEqual([
      'once',
      'always',
      'refuse',
    ]);
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
