import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { NewProject, Project } from '@/shared/api/types';

/** The name under which the list of projects is cached. */
const PROJECTS_KEY = ['projects'] as const;

/** The projects that are not archived, oldest first. */
export function useProjects() {
  const backend = useBackend();
  return useQuery({ queryKey: PROJECTS_KEY, queryFn: () => backend.listProjects() });
}

/**
 * Returns a function that starts a project and adds it to the list on
 * screen. A refusal by the backend is thrown to the caller, which shows it
 * next to the form.
 */
export function useCreateProject(): (project: NewProject) => Promise<Project> {
  const backend = useBackend();
  const queryClient = useQueryClient();

  return useCallback(
    async (project) => {
      const created = await backend.createProject(project);
      queryClient.setQueryData<Project[]>(PROJECTS_KEY, (projects = []) => [...projects, created]);
      return created;
    },
    [backend, queryClient],
  );
}
