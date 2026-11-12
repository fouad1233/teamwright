import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { Id, SyncState } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { isStaleAfterChanges } from './syncModel';

/** The name under which the state of syncing is cached. */
const STATE_KEY = ['sync', 'state'] as const;

/** The name under which the notices of syncing are cached. */
const NOTICES_KEY = ['sync', 'notices'] as const;

/**
 * Where syncing stands. The backend sends the whole state after every
 * round, which takes its place here; a round can also bring notices, so
 * they are asked for again.
 */
export function useSyncState() {
  const backend = useBackend();
  const queryClient = useQueryClient();

  useEffect(
    () =>
      backend.onSyncChanged((state) => {
        queryClient.setQueryData<SyncState>(STATE_KEY, state);
        void queryClient.invalidateQueries({ queryKey: NOTICES_KEY });
      }),
    [backend, queryClient],
  );

  return useQuery({ queryKey: STATE_KEY, queryFn: () => backend.syncState() });
}

/** What syncing tells the manager, the newest first. */
export function useSyncNotices() {
  const backend = useBackend();
  return useQuery({ queryKey: NOTICES_KEY, queryFn: () => backend.syncNotices() });
}

/**
 * Asks again for whatever is shown of the projects in which another
 * computer changed something, so that the views show its work. It listens
 * once, for the whole window.
 */
export function useFreshDataFromOtherComputers(): void {
  const backend = useBackend();
  const queryClient = useQueryClient();

  useEffect(
    () =>
      backend.onDataChanged(({ projectIds }) => {
        void queryClient.invalidateQueries({
          predicate: (query) => isStaleAfterChanges(query.queryKey, projectIds),
        });
      }),
    [backend, queryClient],
  );
}

/** Everything the manager can do about syncing. A refusal is shown to the manager. */
export interface SyncCommands {
  /** Chooses the folder and names this computer. Resolves to whether it was taken. */
  chooseFolder(folder: string, computerName: string): Promise<boolean>;
  /** Lets the manager pick a folder with the window of the system. */
  pickFolder(): Promise<string | null>;
  stop(): Promise<void>;
  syncNow(): Promise<void>;
  /** Renames this computer. Resolves to whether the name was taken. */
  rename(name: string): Promise<boolean>;
  accept(computerId: Id): Promise<void>;
  ignore(computerId: Id): Promise<void>;
  dismissNotice(noticeId: number): Promise<void>;
  giveNewIdentity(): Promise<void>;
}

/** The commands of syncing. Each one shows the state it answers with. */
export function useSyncCommands(): SyncCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const notifier = useNotifier();

  return useMemo(() => {
    /** Runs a request that answers with the state, and shows it. */
    const change = async (request: () => Promise<SyncState>): Promise<boolean> => {
      try {
        queryClient.setQueryData<SyncState>(STATE_KEY, await request());
        await queryClient.invalidateQueries({ queryKey: NOTICES_KEY });
        return true;
      } catch (failure) {
        notifier.showError(describeFailure(failure));
        return false;
      }
    };

    return {
      chooseFolder: (folder, computerName) =>
        change(() => backend.chooseSyncFolder(folder, computerName)),
      pickFolder: async () => {
        try {
          return await backend.chooseFolder();
        } catch (failure) {
          notifier.showError(describeFailure(failure));
          return null;
        }
      },
      stop: async () => {
        await change(() => backend.stopSyncing());
      },
      syncNow: async () => {
        // A round that fails says why in the state, so the state is asked
        // for again whatever the answer was.
        if (!(await change(() => backend.syncNow()))) {
          await queryClient.invalidateQueries({ queryKey: STATE_KEY });
        }
      },
      rename: (name) => change(() => backend.renameThisComputer(name)),
      accept: async (computerId) => {
        await change(() => backend.acceptComputer(computerId));
      },
      ignore: async (computerId) => {
        await change(() => backend.ignoreComputer(computerId));
      },
      dismissNotice: async (noticeId) => {
        try {
          await backend.dismissSyncNotice(noticeId);
          await queryClient.invalidateQueries({ queryKey: NOTICES_KEY });
        } catch (failure) {
          notifier.showError(describeFailure(failure));
        }
      },
      giveNewIdentity: async () => {
        await change(() => backend.giveThisComputerANewIdentity());
      },
    };
  }, [backend, queryClient, notifier]);
}
