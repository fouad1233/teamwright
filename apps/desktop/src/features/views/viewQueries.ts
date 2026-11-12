import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { Id, View, ViewFilter, ViewKind } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { withMovedView } from './viewModel';

/** The name under which the views of a project are cached. */
export function viewsKey(projectId: Id) {
  return ['views', projectId] as const;
}

/** The views of a project, in the order the manager gave them. */
export function useViews(projectId: Id) {
  const backend = useBackend();
  return useQuery({
    queryKey: viewsKey(projectId),
    queryFn: () => backend.listViews(projectId),
  });
}

/**
 * Everything the interface can do with the views of one project. Every
 * command is done in place: a refusal is shown as a message, and the views
 * are loaded again either way.
 */
export interface ViewCommands {
  /** Adds a view of a kind, named after it. Resolves to the view, or nothing if refused. */
  add(kind: ViewKind): Promise<View | undefined>;
  /** Renames a view. Resolves to whether it was renamed. */
  rename(viewId: Id, name: string): Promise<boolean>;
  setFilter(viewId: Id, filter: ViewFilter): Promise<void>;
  /** Moves a view to a place among the others. The move shows at once. */
  move(viewId: Id, index: number): Promise<void>;
  /** Removes a view. Resolves to whether it was removed. */
  remove(viewId: Id): Promise<boolean>;
}

export function useViewCommands(projectId: Id): ViewCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const notifier = useNotifier();

  return useMemo(() => {
    const reload = () => queryClient.invalidateQueries({ queryKey: viewsKey(projectId) });

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
      add: (kind) => inPlace(() => backend.addView({ projectId, kind })),
      rename: async (viewId, name) =>
        (await inPlace(() => backend.renameView(viewId, name))) !== undefined,
      setFilter: async (viewId, filter) => {
        await inPlace(() => backend.setViewFilter(viewId, filter));
      },
      move: async (viewId, index) => {
        // A tab that was dragged must not jump back while the backend answers.
        queryClient.setQueryData<View[]>(viewsKey(projectId), (views = []) =>
          withMovedView(views, viewId, index),
        );
        await inPlace(() => backend.moveView(viewId, index));
      },
      remove: async (viewId) => {
        const removed = await inPlace(async () => {
          await backend.removeView(viewId);
          return true;
        });
        return removed ?? false;
      },
    };
  }, [backend, queryClient, notifier, projectId]);
}
