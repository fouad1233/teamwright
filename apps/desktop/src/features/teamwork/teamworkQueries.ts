import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { Id } from '@/shared/api/types';

/**
 * The name under which what the specialists of a project handed each other
 * is cached. It names the project second, so that syncing asks for it
 * again when another computer changed the project.
 */
export function teamTalkKey(projectId: Id) {
  return ['team-talk', projectId] as const;
}

/**
 * The handoffs and the messages of a project. A specialist hands over and
 * writes while it works, and its conversation changes then, so they are
 * asked for again whenever a conversation of the project changes.
 */
export function useTeamTalk(projectId: Id) {
  const backend = useBackend();
  const queryClient = useQueryClient();

  useEffect(
    () =>
      backend.onConversationChanged((conversation) => {
        if (conversation.projectId === projectId) {
          void queryClient.invalidateQueries({ queryKey: teamTalkKey(projectId) });
        }
      }),
    [backend, queryClient, projectId],
  );

  return useQuery({
    queryKey: teamTalkKey(projectId),
    queryFn: () => backend.teamTalk(projectId),
  });
}
