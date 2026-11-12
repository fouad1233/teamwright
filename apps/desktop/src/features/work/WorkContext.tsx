import { createContext, useContext, useMemo, type ReactNode } from 'react';

import type { Conversation, Id } from '@/shared/api/types';

import { conversationOfTask, tasksBeingWorkedOn, tasksWaitingForManager } from './workModel';

/** What the views of a project know about the work of its specialists. */
interface Work {
  conversations: readonly Conversation[];
  /** The tasks a specialist is working on at this moment. */
  busyTasks: ReadonlySet<Id>;
  /** The tasks whose specialist waits for a decision of the manager. */
  waitingTasks: ReadonlySet<Id>;
}

const NO_WORK: Work = { conversations: [], busyTasks: new Set(), waitingTasks: new Set() };

const WorkContext = createContext<Work>(NO_WORK);

interface WorkProviderProps {
  /** The conversations of the open project. */
  conversations: readonly Conversation[];
  children: ReactNode;
}

/**
 * Makes the conversations of the open project known to everything inside,
 * so that a card can show that somebody is working on its task without the
 * conversations being handed down to it.
 */
export function WorkProvider({ conversations, children }: WorkProviderProps) {
  const work = useMemo(
    () => ({
      conversations,
      busyTasks: tasksBeingWorkedOn(conversations),
      waitingTasks: tasksWaitingForManager(conversations),
    }),
    [conversations],
  );
  return <WorkContext value={work}>{children}</WorkContext>;
}

/** The conversation about a task, if work on it has started. */
export function useConversationOfTask(taskId: Id): Conversation | undefined {
  return conversationOfTask(useContext(WorkContext).conversations, taskId);
}

/** Whether a specialist is working on a task at this moment. */
export function useIsBeingWorkedOn(taskId: Id): boolean {
  return useContext(WorkContext).busyTasks.has(taskId);
}

/** Whether the specialist of a task waits for a decision of the manager. */
export function useWaitsForManager(taskId: Id): boolean {
  return useContext(WorkContext).waitingTasks.has(taskId);
}

/** Every conversation of the open project. */
export function useConversationsOfProject(): readonly Conversation[] {
  return useContext(WorkContext).conversations;
}
