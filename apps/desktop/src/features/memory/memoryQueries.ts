import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { Id, MemoryEntry } from '@/shared/api/types';

/**
 * The name under which the memory of a project is cached. It names the
 * project second, so that syncing asks for it again when another computer
 * changed the project. A search is cached under it too, so that a change to
 * memory asks for both again.
 */
export function memoryKey(projectId: Id) {
  return ['memory', projectId] as const;
}

/**
 * Every entry of the memory of a project, current and replaced, the newest
 * first. Specialists remember while they work, and the manager's answer to
 * a question becomes a decision, and both change a conversation of the
 * project, so the memory is asked for again whenever one changes.
 */
export function useProjectMemory(projectId: Id) {
  const backend = useBackend();
  const queryClient = useQueryClient();

  useEffect(
    () =>
      backend.onConversationChanged((conversation) => {
        if (conversation.projectId === projectId) {
          void queryClient.invalidateQueries({ queryKey: memoryKey(projectId) });
        }
      }),
    [backend, queryClient, projectId],
  );

  return useQuery({
    queryKey: memoryKey(projectId),
    queryFn: () => backend.projectMemory(projectId),
  });
}

/**
 * The current entries that fit the words of a search, by the rule the
 * specialists search by. Nothing is asked while the words are blank. While
 * the next words are asked for, the entries of the last ones stay.
 */
export function useMemorySearch(projectId: Id, query: string) {
  const backend = useBackend();
  const words = query.trim();
  return useQuery({
    queryKey: [...memoryKey(projectId), 'search', words] as const,
    queryFn: () => backend.searchMemory(projectId, words),
    enabled: words !== '',
    placeholderData: keepPreviousData,
  });
}

/** What the manager does to the memory of a project. Each throws what the backend refused. */
export interface MemoryCommands {
  addFact(title: string, text: string): Promise<MemoryEntry>;
  change(entryId: Id, title: string, text: string): Promise<MemoryEntry>;
  remove(entryId: Id): Promise<void>;
}

export function useMemoryCommands(projectId: Id): MemoryCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();

  return useMemo(() => {
    /** Runs a request and asks for the memory again, whatever came of it. */
    const thenReload = async <Result>(request: () => Promise<Result>): Promise<Result> => {
      try {
        return await request();
      } finally {
        await queryClient.invalidateQueries({ queryKey: memoryKey(projectId) });
      }
    };
    return {
      addFact: (title, text) =>
        thenReload(() => backend.addMemoryFact({ projectId, title, text, taskId: null })),
      change: (entryId, title, text) =>
        thenReload(() => backend.changeMemoryEntry(entryId, title, text)),
      remove: (entryId) => thenReload(() => backend.removeMemoryEntry(entryId)),
    };
  }, [backend, queryClient, projectId]);
}
