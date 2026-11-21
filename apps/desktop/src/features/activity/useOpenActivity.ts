import { useCallback } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Id } from '@/shared/api/types';

import { useViewCommands, useViews } from '../views/viewQueries';

/**
 * Returns a function that opens the Activity view of a project, after
 * adding one if the project has none.
 */
export function useOpenActivity(projectId: Id): {
  hasActivity: boolean;
  open: () => Promise<void>;
} {
  const navigation = useNavigation();
  const views = useViews(projectId).data;
  const commands = useViewCommands(projectId);
  const activity = views?.find((each) => each.kind === 'activity');

  const open = useCallback(async () => {
    const view = activity ?? (await commands.add('activity'));
    if (view !== undefined) {
      navigation.showView(view.id);
    }
  }, [activity, commands, navigation]);

  return { hasActivity: activity !== undefined, open };
}
