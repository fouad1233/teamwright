import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { Conversation, Id } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { tasksKey } from '../tasks/taskQueries';
import { hasTurnStartedOrEnded, withConversation } from './workModel';

/** The name under which the state of the program for specialists is cached. */
const PROGRAM_KEY = ['agent-program'] as const;

/** How long the state of the program is taken to be the same, in milliseconds. */
const PROGRAM_IS_FRESH_FOR = 60_000;

/** The name under which the conversations of a project are cached. */
function conversationsKey(projectId: Id) {
  return ['conversations', projectId] as const;
}

/** Whether the program that runs the AI specialists is installed and signed in. */
export function useAgentProgram() {
  const backend = useBackend();
  return useQuery({
    queryKey: PROGRAM_KEY,
    queryFn: () => backend.agentProgram(),
    staleTime: PROGRAM_IS_FRESH_FOR,
  });
}

/**
 * The conversations of a project, kept up to date while specialists work.
 *
 * The backend tells about every step of a conversation. A step replaces the
 * conversation on screen. When a turn starts or ends, the task it is about
 * moves to another status, so the tasks are loaded again as well.
 */
export function useConversations(projectId: Id) {
  const backend = useBackend();
  const queryClient = useQueryClient();

  useEffect(() => {
    const queryKey = conversationsKey(projectId);
    return backend.onConversationChanged((conversation) => {
      if (conversation.projectId !== projectId) {
        return;
      }
      const before = queryClient.getQueryData<Conversation[]>(queryKey) ?? [];
      queryClient.setQueryData<Conversation[]>(queryKey, withConversation(before, conversation));
      const known = before.find((other) => other.id === conversation.id);
      if (hasTurnStartedOrEnded(known, conversation)) {
        void queryClient.invalidateQueries({ queryKey: tasksKey(projectId) });
      }
    });
  }, [backend, queryClient, projectId]);

  return useQuery({
    queryKey: conversationsKey(projectId),
    queryFn: () => backend.listConversations(projectId),
  });
}

/**
 * Everything the manager can do about the work of the specialists of one
 * project. A refusal by the backend is shown to the user.
 */
export interface WorkCommands {
  /** Lets the specialist a task is given to start on it. */
  start(taskId: Id): Promise<void>;
  /** Sends the specialist of a conversation a message. Resolves to whether it was sent. */
  reply(conversationId: Id, message: string): Promise<boolean>;
  /** Asks a specialist something that is not about a task. Resolves to the new conversation. */
  ask(memberId: Id, message: string): Promise<Conversation | undefined>;
  /** Stops the turn that is running in a conversation. */
  stop(conversationId: Id): Promise<void>;
  /** Opens a conversation in the Claude desktop app. */
  openInClaude(conversationId: Id): Promise<void>;
}

/** The commands for the work of the specialists of a project. */
export function useWorkCommands(projectId: Id): WorkCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const notifier = useNotifier();

  return useMemo(() => {
    const queryKey = conversationsKey(projectId);

    /**
     * Runs a request that answers with a conversation, and shows it. Work
     * that starts moves its task, so the tasks are loaded again.
     */
    const change = async (
      request: () => Promise<Conversation>,
    ): Promise<Conversation | undefined> => {
      try {
        const conversation = await request();
        queryClient.setQueryData<Conversation[]>(queryKey, (conversations = []) =>
          withConversation(conversations, conversation),
        );
        await queryClient.invalidateQueries({ queryKey: tasksKey(projectId) });
        return conversation;
      } catch (failure) {
        notifier.showError(describeFailure(failure));
        return undefined;
      }
    };

    return {
      start: async (taskId) => {
        await change(() => backend.startWork(taskId));
      },
      reply: async (conversationId, message) =>
        (await change(() => backend.reply(conversationId, message))) !== undefined,
      ask: (memberId, message) => change(() => backend.ask(memberId, message)),
      stop: async (conversationId) => {
        await change(() => backend.stopWork(conversationId));
      },
      openInClaude: async (conversationId) => {
        try {
          await backend.openInClaude(conversationId);
        } catch (failure) {
          notifier.showError(describeFailure(failure));
        }
      },
    };
  }, [backend, queryClient, notifier, projectId]);
}
