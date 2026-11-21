import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { Conversation, Id, NewPermissionAnswer } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { tasksKey } from '../tasks/taskQueries';
import { withConversation } from '../work/workModel';
import { conversationsKey } from '../work/workQueries';

/**
 * The name under which the permission requests of a project are cached. It
 * names the project second, so that syncing asks for them again when
 * another computer changed the project.
 */
export function permissionsKey(projectId: Id) {
  return ['permissions', projectId] as const;
}

/** The name under which the requests that wait for the manager are cached. */
const OPEN_KEY = ['permissions-open'] as const;

/**
 * Asks for the requests again whenever a conversation changes: a request
 * is made in a turn, and an answer starts one.
 */
function useAskedAgainWhenConversationsChange(queryKey: readonly unknown[]) {
  const backend = useBackend();
  const queryClient = useQueryClient();
  useEffect(
    () =>
      backend.onConversationChanged(() => {
        void queryClient.invalidateQueries({ queryKey });
      }),
    [backend, queryClient, queryKey],
  );
}

/** Every permission request of a project, answered or not, the oldest first. */
export function usePermissionRequests(projectId: Id) {
  const backend = useBackend();
  const queryKey = useMemo(() => permissionsKey(projectId), [projectId]);
  useAskedAgainWhenConversationsChange(queryKey);
  return useQuery({ queryKey, queryFn: () => backend.permissionRequests(projectId) });
}

/** The requests that wait for the manager, in every project, the oldest first. */
export function useOpenPermissionRequests() {
  const backend = useBackend();
  useAskedAgainWhenConversationsChange(OPEN_KEY);
  return useQuery({ queryKey: OPEN_KEY, queryFn: () => backend.openPermissionRequests() });
}

export interface PermissionCommands {
  /** Answers a request. Resolves to whether the answer was taken. */
  answer(requestId: Id, answer: NewPermissionAnswer): Promise<boolean>;
  /** Takes back a standing permission. */
  withdraw(requestId: Id): Promise<void>;
}

/** The commands for the permission requests of a project. */
export function usePermissionCommands(projectId: Id): PermissionCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const notifier = useNotifier();

  return useMemo(() => {
    const askAgain = () =>
      Promise.all([
        queryClient.invalidateQueries({ queryKey: permissionsKey(projectId) }),
        queryClient.invalidateQueries({ queryKey: OPEN_KEY }),
      ]);

    return {
      answer: async (requestId, answer) => {
        try {
          const conversation = await backend.answerPermission(requestId, answer);
          queryClient.setQueryData<Conversation[]>(
            conversationsKey(projectId),
            (conversations = []) => withConversation(conversations, conversation),
          );
          await Promise.all([
            askAgain(),
            queryClient.invalidateQueries({ queryKey: tasksKey(projectId) }),
          ]);
          return true;
        } catch (failure) {
          notifier.showError(describeFailure(failure));
          return false;
        }
      },
      withdraw: async (requestId) => {
        try {
          await backend.withdrawPermission(requestId);
          await askAgain();
        } catch (failure) {
          notifier.showError(describeFailure(failure));
        }
      },
    };
  }, [backend, queryClient, notifier, projectId]);
}
