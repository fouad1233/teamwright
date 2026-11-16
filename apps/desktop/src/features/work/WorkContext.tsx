import { createContext, useContext, useMemo, type ReactNode } from 'react';

import type { Conversation, Id, ProjectUsage } from '@/shared/api/types';

import { conversationOfTask, tasksBeingWorkedOn, tasksWaitingForManager } from './workModel';

/** What the views of a project know about the work of its specialists. */
interface Work {
  conversations: readonly Conversation[];
  /** The tasks a specialist is working on at this moment. */
  busyTasks: ReadonlySet<Id>;
  /** The tasks whose specialist waits for a decision of the manager. */
  waitingTasks: ReadonlySet<Id>;
  /** What the agents of the project have spent, once it is known. */
  usage: ProjectUsage | undefined;
}

const NO_WORK: Work = {
  conversations: [],
  busyTasks: new Set(),
  waitingTasks: new Set(),
  usage: undefined,
};

const WorkContext = createContext<Work>(NO_WORK);

interface WorkProviderProps {
  /** The conversations of the open project. */
  conversations: readonly Conversation[];
  /** What the agents of the open project have spent, once it is known. */
  usage?: ProjectUsage;
  children: ReactNode;
}

/**
 * Makes the conversations of the open project known to everything inside,
 * so that a card can show that somebody is working on its task without the
 * conversations being handed down to it.
 */
export function WorkProvider({ conversations, usage, children }: WorkProviderProps) {
  const work = useMemo(
    () => ({
      conversations,
      busyTasks: tasksBeingWorkedOn(conversations),
      waitingTasks: tasksWaitingForManager(conversations),
      usage,
    }),
    [conversations, usage],
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

/** What the agents of the open project have spent, and how full their contexts are, once it is known. */
export function useUsageOfProject(): ProjectUsage | undefined {
  return useContext(WorkContext).usage;
}
