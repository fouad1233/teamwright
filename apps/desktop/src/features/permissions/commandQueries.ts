import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { CommandPermissions } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

/** The name under which the switch of command permissions is cached. */
const COMMANDS_KEY = ['permissions', 'commands'] as const;

/**
 * Whether command permissions are switched on, on this computer, and what
 * holds commands back here even then (decision 0038). Off until the
 * manager switches them on.
 */
export function useCommandPermissions() {
  const backend = useBackend();
  return useQuery({ queryKey: COMMANDS_KEY, queryFn: () => backend.commandPermissions() });
}

/**
 * Whether the inbox and the team may offer to allow a command: only while
 * command permissions are switched on. Until the answer is known, they
 * offer none.
 */
export function useCommandsSwitchedOn(): boolean {
  return useCommandPermissions().data?.switchedOn ?? false;
}

/** Switches command permissions on or off. A refusal is shown to the manager. */
export function useSwitchCommandPermissions(): (on: boolean) => Promise<void> {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const notifier = useNotifier();

  return useMemo(
    () => async (on: boolean) => {
      try {
        queryClient.setQueryData<CommandPermissions>(
          COMMANDS_KEY,
          await backend.switchCommandPermissions(on),
        );
      } catch (failure) {
        notifier.showError(describeFailure(failure));
      }
    },
    [backend, queryClient, notifier],
  );
}
