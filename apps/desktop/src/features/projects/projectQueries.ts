import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { Id, NewProject, Project, StatusProperties } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { tasksKey } from '../tasks/taskQueries';
import { withMovedStatus } from './workflowModel';

/** The name under which the list of projects is cached. */
const PROJECTS_KEY = ['projects', 'open'] as const;

/** The name under which the list of the projects that were put away is cached. */
const ARCHIVED_PROJECTS_KEY = ['projects', 'archived'] as const;

/** The projects that are not archived, oldest first. */
export function useProjects() {
  const backend = useBackend();
  return useQuery({ queryKey: PROJECTS_KEY, queryFn: () => backend.listProjects() });
}

/** The projects that were put away, the one that was put away last first. */
export function useArchivedProjects() {
  const backend = useBackend();
  return useQuery({
    queryKey: ARCHIVED_PROJECTS_KEY,
    queryFn: () => backend.listArchivedProjects(),
  });
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

/** What can be changed about a project in its form. */
export interface ProjectChanges {
  name: string;
  description: string;
}

/**
 * Everything the interface can do to a project and to its workflow.
 *
 * Changing the name and the description happens in a form: a refusal by the
 * backend is thrown to the form, which shows it next to its fields. Every
 * other command shows a refusal to the user and loads the projects again,
 * so the screen never shows something that was not stored.
 */
export interface ProjectCommands {
  /** Gives a project another name, another description, or both. */
  change(project: Project, changes: ProjectChanges): Promise<void>;
  /** Puts a project away. Resolves to whether that was done. */
  archive(projectId: Id): Promise<boolean>;
  /** Brings a project back that was put away. Resolves to whether that was done. */
  restore(projectId: Id): Promise<boolean>;
  /** Removes a project for good. Resolves to whether that was done. */
  remove(projectId: Id): Promise<boolean>;

  /** Adds a status to the workflow of a project. Resolves to the status, or to nothing if it was refused. */
  addStatus(project: Project, status: StatusProperties): Promise<Id | undefined>;
  /** Replaces the name, the category and the color of a status. Resolves to whether that was done. */
  changeStatus(projectId: Id, statusId: Id, status: StatusProperties): Promise<boolean>;
  /** Moves a status to another place in the workflow. The move shows at once. */
  moveStatus(project: Project, statusId: Id, index: number): Promise<void>;
  /** Removes a status and sends its tasks to another one. Resolves to whether that was done. */
  removeStatus(projectId: Id, statusId: Id, moveTasksTo?: Id): Promise<boolean>;
}

/** The commands for projects and their workflows. */
export function useProjectCommands(): ProjectCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const notifier = useNotifier();

  return useMemo(() => {
    /** Puts a project the backend answered with into the list on screen. */
    const show = (project: Project) => {
      queryClient.setQueryData<Project[]>(PROJECTS_KEY, (projects = []) =>
        projects.map((other) => (other.id === project.id ? project : other)),
      );
    };

    const reload = () => queryClient.invalidateQueries({ queryKey: ['projects'] });

    /** Runs a request. A refusal is shown to the user and answered with nothing. */
    const attempt = async <Result>(request: () => Promise<Result>): Promise<Result | undefined> => {
      try {
        return await request();
      } catch (failure) {
        notifier.showError(describeFailure(failure));
        await reload();
        return undefined;
      }
    };

    /** Runs a request that answers with the changed project, and shows that project. */
    const changeWorkflow = async (
      request: () => Promise<Project>,
    ): Promise<Project | undefined> => {
      const project = await attempt(request);
      if (project) {
        show(project);
      }
      return project;
    };

    return {
      change: async (project, changes) => {
        if (changes.name.trim() !== project.name) {
          show(await backend.renameProject(project.id, changes.name));
        }
        if (changes.description.trim() !== project.description) {
          show(await backend.describeProject(project.id, changes.description));
        }
      },
      archive: async (projectId) => {
        const archived = await attempt(() => backend.archiveProject(projectId));
        await reload();
        return archived !== undefined;
      },
      restore: async (projectId) => {
        const restored = await attempt(() => backend.restoreProject(projectId));
        await reload();
        return restored !== undefined;
      },
      remove: async (projectId) => {
        const removed = await attempt(async () => {
          await backend.deleteProject(projectId);
          return true;
        });
        await reload();
        return removed ?? false;
      },

      addStatus: async (project, status) => {
        const changed = await changeWorkflow(() => backend.addStatus(project.id, status));
        // The new status is the one the project did not have before.
        const known = new Set(project.statuses.map((existing) => existing.id));
        return changed?.statuses.find((added) => !known.has(added.id))?.id;
      },
      changeStatus: async (projectId, statusId, status) => {
        const changed = await changeWorkflow(() =>
          backend.changeStatus(projectId, statusId, status),
        );
        // A status that becomes a done status, or stops being one, completes
        // or reopens its tasks.
        await queryClient.invalidateQueries({ queryKey: tasksKey(projectId) });
        return changed !== undefined;
      },
      moveStatus: async (project, statusId, index) => {
        // Show the move at once instead of waiting for the backend.
        show({ ...project, statuses: withMovedStatus(project.statuses, statusId, index) });
        await changeWorkflow(() => backend.moveStatus(project.id, statusId, index));
      },
      removeStatus: async (projectId, statusId, moveTasksTo) => {
        const changed = await changeWorkflow(() =>
          backend.removeStatus(projectId, statusId, moveTasksTo),
        );
        // The tasks of the status went to another one.
        await queryClient.invalidateQueries({ queryKey: tasksKey(projectId) });
        return changed !== undefined;
      },
    };
  }, [backend, queryClient, notifier]);
}
