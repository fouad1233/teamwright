import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { UpdateInstall, UpdateStatus } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

/** The name under which what is known about versions is cached. */
const STATUS_KEY = ['updates', 'status'] as const;

/** How often the window asks the program whether a look is due, in milliseconds. The program itself looks at most once a day. */
const ASK_EVERY_HOUR = 60 * 60 * 1000;

/**
 * What is known about versions. Asking also lets the program look if the
 * manager switched looking on and a day has passed; the window asks when it
 * opens and then once an hour, which the program never turns into more than
 * one look a day.
 */
export function useUpdateStatus() {
  const backend = useBackend();
  return useQuery({
    queryKey: STATUS_KEY,
    queryFn: () => backend.lookForUpdateIfDue(),
    refetchInterval: ASK_EVERY_HOUR,
    refetchOnWindowFocus: false,
  });
}

/** The name under which where installing stands is cached. */
const INSTALL_KEY = ['updates', 'install'] as const;

/** How often the window asks for the progress while an install runs, in milliseconds. */
const PROGRESS_EVERY = 400;

/**
 * Where installing an update stands (decision 0045). Asking costs nothing
 * on the network; while an install runs the window asks often, to show its
 * progress, and otherwise not at all.
 */
export function useUpdateInstall() {
  const backend = useBackend();
  return useQuery({
    queryKey: INSTALL_KEY,
    queryFn: () => backend.updateInstall(),
    refetchInterval: (query) => {
      const phase = query.state.data?.progress.phase;
      return phase === 'looking' || phase === 'downloading' ? PROGRESS_EVERY : false;
    },
    refetchOnWindowFocus: false,
  });
}

/** What the manager can do about installing. A refusal is shown to the manager. */
export interface InstallCommands {
  /** Starts installing the newer version. */
  install(): Promise<void>;
  /**
   * Restarts into the new version. While specialists work it is refused
   * unless `evenWhileWorking` says the manager agreed to stop them.
   */
  restart(evenWhileWorking: boolean): Promise<void>;
}

/** The commands of installing. Each one shows the state it answers with. */
export function useInstallCommands(): InstallCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const notifier = useNotifier();

  return useMemo(() => {
    const change = async (request: () => Promise<UpdateInstall>) => {
      try {
        queryClient.setQueryData<UpdateInstall>(INSTALL_KEY, await request());
      } catch (failure) {
        notifier.showError(describeFailure(failure));
        // What changed meanwhile, such as specialists that started, shows.
        await queryClient.invalidateQueries({ queryKey: INSTALL_KEY });
      }
    };
    return {
      install: () => change(() => backend.installUpdate()),
      restart: (evenWhileWorking) => change(() => backend.restartForUpdate(evenWhileWorking)),
    };
  }, [backend, queryClient, notifier]);
}

/** What the manager can do about versions. A refusal is shown to the manager. */
export interface UpdateCommands {
  /** Switches looking by itself on or off. */
  switchLook(on: boolean): Promise<void>;
  /** Looks now. A look with no answer is not a refusal: the status says so. */
  lookNow(): Promise<void>;
}

/** The commands of versions. Each one shows the status it answers with. */
export function useUpdateCommands(): UpdateCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const notifier = useNotifier();

  return useMemo(() => {
    const change = async (request: () => Promise<UpdateStatus>) => {
      try {
        queryClient.setQueryData<UpdateStatus>(STATUS_KEY, await request());
      } catch (failure) {
        notifier.showError(describeFailure(failure));
      }
    };
    return {
      switchLook: (on) => change(() => backend.switchUpdateLook(on)),
      lookNow: () => change(() => backend.lookForUpdateNow()),
    };
  }, [backend, queryClient, notifier]);
}
