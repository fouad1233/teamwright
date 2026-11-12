import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { Area, Id, Task } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { tasksKey } from '../tasks/taskQueries';
import { withTask } from '../tasks/taskModel';
import { withMovedArea } from './areaModel';

/**
 * The name under which the areas of a project are cached. It carries the
 * project, so that the areas are asked for again when another computer
 * changed the project.
 */
export function areasKey(projectId: Id) {
  return ['areas', projectId] as const;
}

/** The areas of a project, in the order the manager gave them. */
export function useAreas(projectId: Id) {
  const backend = useBackend();
  return useQuery({
    queryKey: areasKey(projectId),
    queryFn: () => backend.listAreas(projectId),
  });
}

/**
 * Everything the interface can do with the areas of one project. Every
 * command is done in place: a refusal is shown as a message, and the areas
 * are loaded again either way.
 */
export interface AreaCommands {
  /** Adds an area at the end. Resolves to the area, or nothing if refused. */
  add(name: string): Promise<Area | undefined>;
  /** Renames an area. Resolves to whether it was renamed. */
  rename(areaId: Id, name: string): Promise<boolean>;
  /** Moves an area to a place among the others. The move shows at once. */
  move(areaId: Id, index: number): Promise<void>;
  /**
   * Removes an area. Its tasks, if it has any, go to the area named.
   * Resolves to whether it was removed.
   */
  remove(areaId: Id, moveTasksTo?: Id): Promise<boolean>;
  /** Moves a task, with its subtasks, into another area. */
  moveTask(taskId: Id, areaId: Id): Promise<void>;
}

export function useAreaCommands(projectId: Id): AreaCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const notifier = useNotifier();

  return useMemo(() => {
    const reload = () => queryClient.invalidateQueries({ queryKey: areasKey(projectId) });
    const reloadTasks = () => queryClient.invalidateQueries({ queryKey: tasksKey(projectId) });

    /** Runs a request. A refusal is shown and answered with nothing. */
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
      add: (name) => inPlace(() => backend.addArea(projectId, name)),
      rename: async (areaId, name) =>
        (await inPlace(() => backend.renameArea(areaId, name))) !== undefined,
      move: async (areaId, index) => {
        // An area that was dragged must not jump back while the backend answers.
        queryClient.setQueryData<Area[]>(areasKey(projectId), (areas = []) =>
          withMovedArea(areas, areaId, index),
        );
        await inPlace(() => backend.moveArea(areaId, index));
      },
      remove: async (areaId, moveTasksTo) => {
        const removed = await inPlace(async () => {
          await backend.removeArea(areaId, moveTasksTo);
          return true;
        });
        // The tasks of the area went to another one.
        await reloadTasks();
        return removed ?? false;
      },
      moveTask: async (taskId, areaId) => {
        const moved = await inPlace(() => backend.moveTaskToArea(taskId, areaId));
        if (moved) {
          queryClient.setQueryData<Task[]>(tasksKey(projectId), (tasks = []) =>
            withTask(tasks, moved),
          );
        }
        // The subtasks went with it.
        await reloadTasks();
      },
    };
  }, [backend, queryClient, notifier, projectId]);
}
