import { describe, expect, it } from 'vitest';

import type { NotificationPressed } from '@/shared/api/types';

import { placeOfPress, shouldAsk } from './notificationModel';

const pressed = (change: Partial<NotificationPressed>): NotificationPressed => ({
  kind: 'finished',
  projectId: 'p',
  taskId: null,
  memberId: null,
  isWithAdvisor: false,
  ...change,
});

describe('placeOfPress', () => {
  it('leads what waits for the manager to the inbox', () => {
    expect(placeOfPress(pressed({ kind: 'waiting', taskId: 't' }))).toEqual({ kind: 'inbox' });
  });

  it('leads finished work to its task', () => {
    expect(placeOfPress(pressed({ taskId: 't' }))).toEqual({
      kind: 'task',
      projectId: 'p',
      taskId: 't',
    });
  });

  it('leads a stop without a task to the Advisor or to the specialist', () => {
    expect(placeOfPress(pressed({ kind: 'stopped', isWithAdvisor: true }))).toEqual({
      kind: 'advisor',
      projectId: 'p',
    });
    expect(placeOfPress(pressed({ kind: 'stopped', memberId: 'm' }))).toEqual({
      kind: 'chat',
      projectId: 'p',
      memberId: 'm',
    });
  });

  it('leads nowhere when the platform did not describe the notification', () => {
    expect(placeOfPress(pressed({ kind: null, projectId: null }))).toEqual({ kind: 'nothing' });
  });
});

describe('shouldAsk', () => {
  it('asks only while the manager was never asked', () => {
    const choice = { waiting: true, finished: true, stopped: true };
    expect(shouldAsk({ ...choice, permission: 'not_asked' })).toBe(true);
    expect(shouldAsk({ ...choice, permission: 'declined' })).toBe(false);
    expect(shouldAsk({ ...choice, permission: 'allowed' })).toBe(false);
    expect(shouldAsk(undefined)).toBe(false);
  });
});
