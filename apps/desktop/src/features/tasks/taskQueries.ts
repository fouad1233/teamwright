import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { Assignee, CalendarDate, Id, NewTask, Priority, Task } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { useOpenArea } from '../areas/AreaContext';
import { withMovedTask, withTask } from './taskModel';

/** The name under which the tasks of a project are cached. */
export function tasksKey(projectId: Id) {
  return ['tasks', projectId] as const;
}

/** The tasks of a project. */
export function useTasks(projectId: Id) {
  const backend = useBackend();
  return useQuery({ queryKey: tasksKey(projectId), queryFn: () => backend.listTasks(projectId) });
}

/**
 * What has happened to a task since it was opened, the oldest change first.
 * It is asked for again whenever the task has changed.
 */
export function useTaskHistory(task: Pick<Task, 'id' | 'updatedAt'>) {
  const backend = useBackend();
  return useQuery({
    queryKey: ['task-history', task.id, task.updatedAt] as const,
    queryFn: () => backend.listTaskHistory(task.id),
  });
}

/**
 * Everything the interface can do to the tasks of one project.
 *
 * Each command sends a request to the backend and brings the tasks on screen
 * up to date. If the backend refuses, the user is told why and the tasks are
 * loaded again, so the screen never shows something that was not stored.
 */
export interface TaskCommands {
  /** Opens a task. Resolves to the new task, or to nothing if it was refused. */
  create(task: Omit<NewTask, 'projectId'>): Promise<Task | undefined>;
  rename(taskId: Id, title: string): Promise<void>;
  describe(taskId: Id, description: string): Promise<void>;
  prioritize(taskId: Id, priority: Priority): Promise<void>;
  assign(taskId: Id, assignee: Assignee | null): Promise<void>;
  estimate(taskId: Id, minutes: number | null): Promise<void>;
  reschedule(taskId: Id, startOn: CalendarDate | null, dueOn: CalendarDate | null): Promise<void>;
  /** Moves a task to a status and a position there. The move shows at once. */
  move(taskId: Id, statusId: Id, index: number): Promise<void>;
  addDependency(taskId: Id, blockerId: Id): Promise<void>;
  removeDependency(taskId: Id, blockerId: Id): Promise<void>;
  /** Removes a task together with its subtasks. Resolves to whether it was removed. */
  remove(taskId: Id): Promise<boolean>;
}

/**
 * The commands for the tasks of a project. A task opened while an area is
 * open goes into that area, unless it names one or is a subtask, which goes
 * into the area of its task.
 */
export function useTaskCommands(projectId: Id): TaskCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const notifier = useNotifier();
  const openAreaId = useOpenArea()?.id;

  return useMemo(() => {
    const queryKey = tasksKey(projectId);

    /** Puts a task the backend answered with into the list on screen. */
    const show = (task: Task) => {
      queryClient.setQueryData<Task[]>(queryKey, (tasks = []) => withTask(tasks, task));
    };

    const reload = () => queryClient.invalidateQueries({ queryKey });

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

    /** Runs a request that answers with the changed task, and shows that task. */
    const change = async (request: () => Promise<Task>) => {
      const task = await attempt(request);
      if (task) {
        show(task);
      }
    };

    return {
      create: async (task) => {
        const areaId = task.areaId ?? (task.parentId === undefined ? openAreaId : undefined);
        const created = await attempt(() =>
          backend.createTask({ ...task, projectId, ...(areaId === undefined ? {} : { areaId }) }),
        );
        if (created) {
          show(created);
        }
        return created;
      },
      rename: (taskId, title) => change(() => backend.renameTask(taskId, title)),
      describe: (taskId, description) => change(() => backend.describeTask(taskId, description)),
      prioritize: (taskId, priority) => change(() => backend.prioritizeTask(taskId, priority)),
      assign: (taskId, assignee) => change(() => backend.assignTask(taskId, assignee)),
      estimate: (taskId, minutes) => change(() => backend.estimateTask(taskId, minutes)),
      reschedule: async (taskId, startOn, dueOn) => {
        // Show the new dates at once: a bar that was dragged on the chart
        // must not jump back while the backend answers. If the backend
        // refuses, the tasks are loaded again and the bar goes back.
        queryClient.setQueryData<Task[]>(queryKey, (tasks = []) =>
          tasks.map((task) => (task.id === taskId ? { ...task, startOn, dueOn } : task)),
        );
        await change(() => backend.rescheduleTask(taskId, startOn, dueOn));
      },
      move: async (taskId, statusId, index) => {
        // Show the move at once instead of waiting for the backend.
        const tasks = queryClient.getQueryData<Task[]>(queryKey) ?? [];
        const moved = withMovedTask(tasks, taskId, statusId, index);
        const expectedRank = moved.find((task) => task.id === taskId)?.rank;
        queryClient.setQueryData<Task[]>(queryKey, moved);

        const task = await attempt(() => backend.moveTask(taskId, statusId, index));
        if (!task) {
          return;
        }
        show(task);
        // When two ranks get too close, the backend spaces the whole status
        // out again. The other ranks on screen are then out of date.
        if (task.rank !== expectedRank) {
          await reload();
        }
      },
      addDependency: (taskId, blockerId) =>
        change(() => backend.addTaskDependency(taskId, blockerId)),
      removeDependency: (taskId, blockerId) =>
        change(() => backend.removeTaskDependency(taskId, blockerId)),
      remove: async (taskId) => {
        const removed = await attempt(async () => {
          await backend.deleteTask(taskId);
          return true;
        });
        // Subtasks went with it and waiting tasks were released, so the
        // whole list is loaded again.
        await reload();
        return removed ?? false;
      },
    };
  }, [backend, queryClient, notifier, projectId, openAreaId]);
}
