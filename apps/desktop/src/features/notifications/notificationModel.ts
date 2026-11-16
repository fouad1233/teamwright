import type { Id, NotificationChoice, NotificationPressed } from '@/shared/api/types';

/** Where pressing a notification leads. */
export type PlaceOfPress =
  | { kind: 'inbox' }
  | { kind: 'task'; projectId: Id; taskId: Id }
  | { kind: 'advisor'; projectId: Id }
  | { kind: 'chat'; projectId: Id; memberId: Id }
  | { kind: 'nothing' };

/**
 * Where a pressed notification leads: what waits for the manager to the
 * inbox, where it can be answered; anything else to its task, or to the
 * conversation with the Advisor or the specialist. A notification the
 * platform did not describe only brings the window to the front.
 */
export function placeOfPress(pressed: NotificationPressed): PlaceOfPress {
  if (pressed.kind === 'waiting') {
    return { kind: 'inbox' };
  }
  const { projectId } = pressed;
  if (projectId === null) {
    return { kind: 'nothing' };
  }
  if (pressed.taskId !== null) {
    return { kind: 'task', projectId, taskId: pressed.taskId };
  }
  if (pressed.isWithAdvisor) {
    return { kind: 'advisor', projectId };
  }
  if (pressed.memberId !== null) {
    return { kind: 'chat', projectId, memberId: pressed.memberId };
  }
  return { kind: 'nothing' };
}

/** Whether to ask the manager to allow notifications: only while never asked. */
export function shouldAsk(choice: NotificationChoice | undefined): boolean {
  return choice?.permission === 'not_asked';
}
