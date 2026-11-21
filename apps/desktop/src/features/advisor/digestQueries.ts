import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { DigestDraft, Id } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

/**
 * The name under which the digests of a project are cached. Its second part
 * is the project, so that a change from another computer loads them again.
 */
function digestsKey(projectId: Id) {
  return ['digests', projectId] as const;
}

/** The name under which the earlier conversations found for a project are cached. */
function earlierKey(projectId: Id) {
  return ['earlier-conversations', projectId] as const;
}

/** The digests kept with a project, the one kept last first. */
export function useDigests(projectId: Id) {
  const backend = useBackend();
  return useQuery({
    queryKey: digestsKey(projectId),
    queryFn: () => backend.listDigests(projectId),
  });
}

/**
 * The earlier conversations on this computer, read while their list is
 * shown. They are read again each time it opens: Claude Code may have
 * written a new one in the meantime.
 */
export function useEarlierConversations(projectId: Id) {
  const backend = useBackend();
  return useQuery({
    queryKey: earlierKey(projectId),
    queryFn: () => backend.findEarlierConversations(projectId),
    staleTime: 0,
  });
}

/**
 * The digest of an earlier conversation, made by the rules, for the manager
 * to read. It is made once while the editor is open and forgotten after.
 */
export function useDigestDraft(projectId: Id, source: string) {
  const backend = useBackend();
  return useQuery({
    queryKey: ['digest-draft', projectId, source] as const,
    queryFn: () => backend.draftDigest(projectId, source),
    staleTime: Infinity,
    gcTime: 0,
    retry: false,
  });
}

/** What the manager can do with digests. A refusal is said where it happened. */
export interface DigestCommands {
  /** Keeps a digest. Resolves to the reason it was refused, or `null`. */
  keep(digest: DigestDraft): Promise<string | null>;
  /** Removes a digest. A refusal is shown in a notice. */
  remove(digestId: Id): Promise<void>;
}

export function useDigestCommands(projectId: Id): DigestCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const notifier = useNotifier();

  return useMemo(() => {
    const refresh = () =>
      Promise.all([
        queryClient.invalidateQueries({ queryKey: digestsKey(projectId) }),
        queryClient.invalidateQueries({ queryKey: earlierKey(projectId) }),
      ]);
    return {
      keep: async (digest) => {
        try {
          await backend.keepDigest(projectId, digest);
          await refresh();
          return null;
        } catch (failure) {
          return describeFailure(failure);
        }
      },
      remove: async (digestId) => {
        try {
          await backend.removeDigest(digestId);
          await refresh();
        } catch (failure) {
          notifier.showError(describeFailure(failure));
        }
      },
    };
  }, [backend, queryClient, notifier, projectId]);
}
