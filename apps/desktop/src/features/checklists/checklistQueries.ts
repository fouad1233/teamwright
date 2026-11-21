import { useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { ChecklistItem, Id, Task } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { tasksKey } from '../tasks/taskQueries';
import { withTask } from '../tasks/taskModel';
import { withMovedItem } from './checklistModel';

/**
 * Everything the interface can do with the checklists of the tasks of one
 * project. Each command answers with the task, which takes the place of the
 * one on screen. A tick and a move show at once; a refusal is shown as a
 * message and the tasks are loaded again.
 */
export interface ChecklistCommands {
  /** Adds an item at the end. Resolves to whether it was added. */
  add(taskId: Id, text: string): Promise<boolean>;
  rename(taskId: Id, itemId: Id, text: string): Promise<void>;
  tick(taskId: Id, itemId: Id, done: boolean): Promise<void>;
  move(taskId: Id, itemId: Id, index: number): Promise<void>;
  remove(taskId: Id, itemId: Id): Promise<void>;
}

export function useChecklistCommands(projectId: Id): ChecklistCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const notifier = useNotifier();

  return useMemo(() => {
    const queryKey = tasksKey(projectId);

    /** Changes the checklist of a task on screen before the backend answers. */
    const showAtOnce = (taskId: Id, change: (checklist: ChecklistItem[]) => ChecklistItem[]) => {
      queryClient.setQueryData<Task[]>(queryKey, (tasks = []) =>
        tasks.map((task) =>
          task.id === taskId ? { ...task, checklist: change(task.checklist) } : task,
        ),
      );
    };

    /** Runs a request that answers with the task, and shows the task. */
    const send = async (request: () => Promise<Task>): Promise<boolean> => {
      try {
        const task = await request();
        queryClient.setQueryData<Task[]>(queryKey, (tasks = []) => withTask(tasks, task));
        return true;
      } catch (failure) {
        notifier.showError(describeFailure(failure));
        await queryClient.invalidateQueries({ queryKey });
        return false;
      }
    };

    return {
      add: (taskId, text) => send(() => backend.addChecklistItem(taskId, text)),
      rename: async (taskId, itemId, text) => {
        await send(() => backend.renameChecklistItem(taskId, itemId, text));
      },
      tick: async (taskId, itemId, done) => {
        showAtOnce(taskId, (checklist) =>
          checklist.map((item) => (item.id === itemId ? { ...item, done } : item)),
        );
        await send(() => backend.tickChecklistItem(taskId, itemId, done));
      },
      move: async (taskId, itemId, index) => {
        showAtOnce(taskId, (checklist) => withMovedItem(checklist, itemId, index));
        await send(() => backend.moveChecklistItem(taskId, itemId, index));
      },
      remove: async (taskId, itemId) => {
        await send(() => backend.removeChecklistItem(taskId, itemId));
      },
    };
  }, [backend, queryClient, notifier, projectId]);
}
