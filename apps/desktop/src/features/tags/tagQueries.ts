import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { Color, Id, Tag, Task } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { tasksKey } from '../tasks/taskQueries';
import { viewsKey } from '../views/viewQueries';
import { withTask } from '../tasks/taskModel';
import { withTagSet } from './tagModel';

/**
 * The name under which the tags of a project are cached. It carries the
 * project, so that the tags are asked for again when another computer
 * changed the project.
 */
export function tagsKey(projectId: Id) {
  return ['tags', projectId] as const;
}

/** The tags of a project, by name. */
export function useTags(projectId: Id) {
  const backend = useBackend();
  return useQuery({
    queryKey: tagsKey(projectId),
    queryFn: () => backend.listTags(projectId),
  });
}

/**
 * Everything the interface can do with the tags of one project and of its
 * tasks. A refusal is shown as a message; the tags, and the tasks where a
 * tag changed on them, are brought up to date either way.
 */
export interface TagCommands {
  /** Adds a tag. Resolves to the tag, or nothing if refused. */
  add(name: string, color?: Color): Promise<Tag | undefined>;
  /** Renames a tag. Resolves to whether it was renamed. */
  rename(tagId: Id, name: string): Promise<boolean>;
  recolor(tagId: Id, color: Color): Promise<void>;
  /** Removes a tag: it comes off its tasks and out of the filters of views. */
  remove(tagId: Id): Promise<void>;
  /** Puts a tag on a task, or takes it off. The change shows at once. */
  setOnTask(taskId: Id, tagId: Id, isOn: boolean): Promise<void>;
  /** Puts a tag on a task by its name: the tag the project has, or a new one. */
  putNamedOnTask(taskId: Id, name: string): Promise<void>;
}

export function useTagCommands(projectId: Id): TagCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const notifier = useNotifier();

  return useMemo(() => {
    const reload = () => queryClient.invalidateQueries({ queryKey: tagsKey(projectId) });
    const reloadTasks = () => queryClient.invalidateQueries({ queryKey: tasksKey(projectId) });
    const show = (task: Task) => {
      queryClient.setQueryData<Task[]>(tasksKey(projectId), (tasks = []) => withTask(tasks, task));
    };

    /** Runs a request. A refusal is shown and answered with nothing. */
    const attempt = async <Result>(request: () => Promise<Result>): Promise<Result | undefined> => {
      try {
        return await request();
      } catch (failure) {
        notifier.showError(describeFailure(failure));
        return undefined;
      }
    };

    return {
      add: async (name, color) => {
        const tag = await attempt(() => backend.addTag(projectId, name, color));
        await reload();
        return tag;
      },
      rename: async (tagId, name) => {
        const tag = await attempt(() => backend.renameTag(tagId, name));
        await reload();
        return tag !== undefined;
      },
      recolor: async (tagId, color) => {
        await attempt(() => backend.recolorTag(tagId, color));
        await reload();
      },
      remove: async (tagId) => {
        await attempt(() => backend.removeTag(tagId));
        // The tag came off its tasks, and out of the filters of views.
        await Promise.all([
          reload(),
          reloadTasks(),
          queryClient.invalidateQueries({ queryKey: viewsKey(projectId) }),
        ]);
      },
      setOnTask: async (taskId, tagId, isOn) => {
        // The tag shows on the task at once, and goes back if refused.
        queryClient.setQueryData<Task[]>(tasksKey(projectId), (tasks = []) =>
          tasks.map((task) => (task.id === taskId ? withTagSet(task, tagId, isOn) : task)),
        );
        const task = await attempt(() =>
          isOn ? backend.putTagOnTask(taskId, tagId) : backend.takeTagOffTask(taskId, tagId),
        );
        if (task) {
          show(task);
        } else {
          await reloadTasks();
        }
      },
      putNamedOnTask: async (taskId, name) => {
        const tagged = await attempt(() => backend.putNamedTagOnTask(taskId, name));
        if (tagged) {
          show(tagged.task);
        }
        await reload();
      },
    };
  }, [backend, queryClient, notifier, projectId]);
}
