import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type {
  Id,
  NewSprint,
  Sprint,
  SprintChanges,
  SprintCompletion,
  SprintStart,
  Task,
} from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { tasksKey } from '../tasks/taskQueries';
import { withTask } from '../tasks/taskModel';
import { withPlannedTask } from './sprintModel';

/** The name under which the sprints of a project are cached. */
export function sprintsKey(projectId: Id) {
  return ['sprints', projectId] as const;
}

/** The name under which the reports of sprints are cached. */
const REPORTS_KEY = 'sprint-report';

/** The sprints of a project, in the order the backend lists them. */
export function useSprints(projectId: Id) {
  const backend = useBackend();
  return useQuery({
    queryKey: sprintsKey(projectId),
    queryFn: () => backend.listSprints(projectId),
  });
}

/**
 * The report of a sprint for its burndown chart. `version` changes whenever
 * the tasks of the sprint change, so the report is asked for again then.
 */
export function useSprintReport(sprintId: Id, version: string) {
  const backend = useBackend();
  return useQuery({
    queryKey: [REPORTS_KEY, sprintId, version] as const,
    queryFn: () => backend.sprintReport(sprintId),
  });
}

/**
 * Everything the interface can do with the sprints of one project.
 *
 * What is done in a dialog throws the refusal of the backend to the dialog,
 * which shows it next to its fields. What is done in place shows a refusal
 * as a message and loads the tasks again.
 */
export interface SprintCommands {
  create(sprint: Omit<NewSprint, 'projectId'>): Promise<Sprint>;
  change(sprintId: Id, changes: SprintChanges): Promise<Sprint>;
  start(sprintId: Id, start: SprintStart): Promise<Sprint>;
  complete(sprintId: Id, completion: SprintCompletion): Promise<Sprint>;
  /** Removes a planned sprint. Resolves to whether it was removed. */
  remove(sprintId: Id): Promise<boolean>;
  /** Puts a task into a sprint, or into the backlog for nothing. The move shows at once. */
  plan(taskId: Id, sprintId: Id | null): Promise<void>;
  setPoints(taskId: Id, points: number | null): Promise<void>;
}

export function useSprintCommands(projectId: Id): SprintCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const notifier = useNotifier();

  return useMemo(() => {
    /**
     * Brings everything a sprint touches up to date: a sprint that is
     * deleted or completed moves tasks, and every change can change a report.
     */
    const reload = () =>
      Promise.all([
        queryClient.invalidateQueries({ queryKey: sprintsKey(projectId) }),
        queryClient.invalidateQueries({ queryKey: tasksKey(projectId) }),
        queryClient.invalidateQueries({ queryKey: [REPORTS_KEY] }),
      ]);

    /** Runs a request of a dialog, and brings the screen up to date if it succeeded. */
    const inDialog = async <Result>(request: () => Promise<Result>): Promise<Result> => {
      const result = await request();
      await reload();
      return result;
    };

    /** Runs a request in place. A refusal is shown and answered with nothing. */
    const inPlace = async <Result>(request: () => Promise<Result>): Promise<Result | undefined> => {
      try {
        return await request();
      } catch (failure) {
        notifier.showError(describeFailure(failure));
        return undefined;
      } finally {
        await reload();
      }
    };

    return {
      create: (sprint) => inDialog(() => backend.createSprint({ ...sprint, projectId })),
      change: (sprintId, changes) => inDialog(() => backend.changeSprint(sprintId, changes)),
      start: (sprintId, start) => inDialog(() => backend.startSprint(sprintId, start)),
      complete: (sprintId, completion) =>
        inDialog(() => backend.completeSprint(sprintId, completion)),
      remove: async (sprintId) => {
        const removed = await inPlace(async () => {
          await backend.deleteSprint(sprintId);
          return true;
        });
        return removed ?? false;
      },
      plan: async (taskId, sprintId) => {
        // Show the move at once: a row that was dragged must not jump back
        // while the backend answers. The subtasks go with their task.
        queryClient.setQueryData<Task[]>(tasksKey(projectId), (tasks = []) =>
          withPlannedTask(tasks, taskId, sprintId),
        );
        await inPlace(() => backend.planTask(taskId, sprintId));
      },
      setPoints: async (taskId, points) => {
        const task = await inPlace(() => backend.setTaskPoints(taskId, points));
        if (task) {
          queryClient.setQueryData<Task[]>(tasksKey(projectId), (tasks = []) =>
            withTask(tasks, task),
          );
        }
      },
    };
  }, [backend, queryClient, notifier, projectId]);
}
