import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { Color, CustomField, FieldKind, FieldValue, Id, Task } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { tasksKey } from '../tasks/taskQueries';
import { withTask } from '../tasks/taskModel';
import { viewsKey } from '../views/viewQueries';

/** The name under which the custom fields of a project are cached. */
export function fieldsKey(projectId: Id) {
  return ['fields', projectId] as const;
}

/** The custom fields of a project, in their order. */
export function useFields(projectId: Id) {
  const backend = useBackend();
  return useQuery({
    queryKey: fieldsKey(projectId),
    queryFn: () => backend.listFields(projectId),
  });
}

/**
 * Everything the interface can do with the custom fields of one project and
 * the values of its tasks. A refusal is shown as a message; the fields, and
 * the tasks whose values changed, are brought up to date either way.
 */
export interface FieldCommands {
  /** Adds a field. Resolves to the field, or nothing if refused. */
  add(name: string, kind: FieldKind, unit: string | null): Promise<CustomField | undefined>;
  /** Renames a field. Resolves to whether it was renamed. */
  rename(fieldId: Id, name: string): Promise<boolean>;
  setUnit(fieldId: Id, unit: string | null): Promise<boolean>;
  move(fieldId: Id, index: number): Promise<void>;
  /** Removes a field: its values are cleared, and filters by it forgotten. */
  remove(fieldId: Id): Promise<void>;
  addChoice(fieldId: Id, name: string): Promise<boolean>;
  renameChoice(fieldId: Id, choiceId: Id, name: string): Promise<boolean>;
  recolorChoice(fieldId: Id, choiceId: Id, color: Color): Promise<void>;
  /** Removes a choice: the tasks that held it are cleared. */
  removeChoice(fieldId: Id, choiceId: Id): Promise<void>;
  /** Gives a task a value, or clears it with nothing. Resolves to whether it was taken. */
  setValue(taskId: Id, fieldId: Id, value: FieldValue | null): Promise<boolean>;
}

export function useFieldCommands(projectId: Id): FieldCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const notifier = useNotifier();

  return useMemo(() => {
    const reload = () => queryClient.invalidateQueries({ queryKey: fieldsKey(projectId) });
    const reloadTasks = () => queryClient.invalidateQueries({ queryKey: tasksKey(projectId) });
    const reloadViews = () => queryClient.invalidateQueries({ queryKey: viewsKey(projectId) });

    /** Runs a request. A refusal is shown and answered with nothing. */
    const attempt = async <Result>(request: () => Promise<Result>): Promise<Result | undefined> => {
      try {
        return await request();
      } catch (failure) {
        notifier.showError(describeFailure(failure));
        return undefined;
      }
    };

    /** Runs a change of a field and reloads the fields. */
    const changeField = async (request: () => Promise<CustomField>): Promise<boolean> => {
      const field = await attempt(request);
      await reload();
      return field !== undefined;
    };

    return {
      add: async (name, kind, unit) => {
        const field = await attempt(() => backend.addField(projectId, name, kind, unit));
        await reload();
        return field;
      },
      rename: (fieldId, name) => changeField(() => backend.renameField(fieldId, name)),
      setUnit: (fieldId, unit) => changeField(() => backend.setFieldUnit(fieldId, unit)),
      move: async (fieldId, index) => {
        await attempt(() => backend.moveField(fieldId, index));
        await reload();
      },
      remove: async (fieldId) => {
        await attempt(() => backend.removeField(fieldId));
        await Promise.all([reload(), reloadTasks(), reloadViews()]);
      },
      addChoice: (fieldId, name) => changeField(() => backend.addFieldChoice(fieldId, name)),
      renameChoice: (fieldId, choiceId, name) =>
        changeField(() => backend.renameFieldChoice(fieldId, choiceId, name)),
      recolorChoice: async (fieldId, choiceId, color) => {
        await changeField(() => backend.recolorFieldChoice(fieldId, choiceId, color));
      },
      removeChoice: async (fieldId, choiceId) => {
        await attempt(() => backend.removeFieldChoice(fieldId, choiceId));
        await Promise.all([reload(), reloadTasks(), reloadViews()]);
      },
      setValue: async (taskId, fieldId, value) => {
        const task = await attempt<Task>(() =>
          value === null
            ? backend.clearFieldValue(taskId, fieldId)
            : backend.setFieldValue(taskId, fieldId, value),
        );
        if (task) {
          queryClient.setQueryData<Task[]>(tasksKey(projectId), (tasks = []) =>
            withTask(tasks, task),
          );
          return true;
        }
        await reloadTasks();
        return false;
      },
    };
  }, [backend, queryClient, notifier, projectId]);
}
