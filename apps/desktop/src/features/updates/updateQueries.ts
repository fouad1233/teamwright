import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { UpdateStatus } from '@/shared/api/types';
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
