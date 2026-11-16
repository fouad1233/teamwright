import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';

import { useNavigation } from '@/app/navigation';
import { useBackend } from '@/shared/api/BackendProvider';
import type { NotificationChoice, NotificationKind } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { placeOfPress } from './notificationModel';

/** The name under which the choice of this computer is cached. */
const CHOICE_KEY = ['notifications', 'choice'] as const;

/** What the manager chose about notifications on this computer. */
export function useNotificationChoice() {
  const backend = useBackend();
  return useQuery({ queryKey: CHOICE_KEY, queryFn: () => backend.notificationChoice() });
}

/** Everything the manager can do about notifications. A refusal is shown to the manager. */
export interface NotificationCommands {
  allow(): Promise<void>;
  decline(): Promise<void>;
  setKind(kind: NotificationKind, on: boolean): Promise<void>;
  /** Sends one to try them, and says so. */
  sendTest(): Promise<void>;
}

/** The commands of notifications. Each one shows the choice it answers with. */
export function useNotificationCommands(): NotificationCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const notifier = useNotifier();

  return useMemo(() => {
    const change = async (request: () => Promise<NotificationChoice>) => {
      try {
        queryClient.setQueryData<NotificationChoice>(CHOICE_KEY, await request());
      } catch (failure) {
        notifier.showError(describeFailure(failure));
      }
    };
    return {
      allow: () => change(() => backend.allowNotifications()),
      decline: () => change(() => backend.declineNotifications()),
      setKind: (kind, on) => change(() => backend.setNotificationKind(kind, on)),
      sendTest: async () => {
        try {
          await backend.sendTestNotification();
          notifier.showNotice('A notification was sent. It shows in the corner of the screen.');
        } catch (failure) {
          notifier.showError(describeFailure(failure));
        }
      },
    };
  }, [backend, queryClient, notifier]);
}

/**
 * Opens what a notification the manager pressed was about. It listens
 * once, for the whole window.
 */
export function useOpenPressedNotifications(): void {
  const backend = useBackend();
  const navigation = useNavigation();

  useEffect(
    () =>
      backend.onNotificationPressed((pressed) => {
        const place = placeOfPress(pressed);
        switch (place.kind) {
          case 'inbox':
            navigation.openInbox();
            break;
          case 'task':
            navigation.openTaskOf(place.projectId, place.taskId);
            break;
          case 'advisor':
            navigation.openAdvisorOf(place.projectId);
            break;
          case 'chat':
            navigation.openChatOf(place.projectId, place.memberId);
            break;
          case 'nothing':
            break;
        }
      }),
    [backend, navigation],
  );
}
