import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { GitHubOverview, Id, TaskOnGitHub } from '@/shared/api/types';

/** The name under which GitHub of a project is cached. */
function overviewKey(projectId: Id) {
  return ['github-overview', projectId] as const;
}

/** The name under which GitHub of a task is cached. */
function taskKey(taskId: Id) {
  return ['github-task', taskId] as const;
}

/** GitHub for a project, from what was last read. Asking does not read GitHub. */
export function useGitHubOverview(projectId: Id) {
  const backend = useBackend();
  return useQuery({
    queryKey: overviewKey(projectId),
    queryFn: () => backend.gitHubOverview(projectId),
  });
}

/**
 * The pull requests and issues about a task. The first time, the
 * repository is read; a failure is shown where the part is, not retried.
 */
export function useTaskOnGitHub(taskId: Id) {
  const backend = useBackend();
  return useQuery({
    queryKey: taskKey(taskId),
    queryFn: () => backend.taskOnGitHub(taskId, false),
    retry: false,
    staleTime: Infinity,
  });
}

/** What the manager can do about GitHub. A refusal is thrown to the caller, which says why. */
export function useGitHubCommands() {
  const backend = useBackend();
  const queryClient = useQueryClient();

  return useMemo(
    () => ({
      /** Reads a project's repository again. Every task shows what was read. */
      refresh: async (projectId: Id) => {
        queryClient.setQueryData<GitHubOverview>(
          overviewKey(projectId),
          await backend.refreshGitHub(projectId),
        );
        await queryClient.invalidateQueries({ queryKey: ['github-task'] });
      },
      /** Reads the repository of a task's project again, and shows the task's part. */
      refreshTask: async (taskId: Id) => {
        queryClient.setQueryData<TaskOnGitHub>(
          taskKey(taskId),
          await backend.taskOnGitHub(taskId, true),
        );
        await queryClient.invalidateQueries({ queryKey: ['github-overview'] });
      },
      /** Names the `gh` of this computer, or looks for it again with null. */
      setProgram: async (program: string | null) => {
        await backend.setGitHubProgram(program);
        await queryClient.invalidateQueries({ queryKey: ['github-overview'] });
      },
      issueDraft: (taskId: Id) => backend.issueDraft(taskId),
      /** Opens the issue the manager confirmed, and shows it with the task. */
      openIssue: async (taskId: Id) => {
        const issue = await backend.openIssue(taskId);
        await queryClient.invalidateQueries({ queryKey: taskKey(taskId) });
        await queryClient.invalidateQueries({ queryKey: ['github-overview'] });
        return issue;
      },
      open: (address: string) => backend.openWebAddress(address),
    }),
    [backend, queryClient],
  );
}
