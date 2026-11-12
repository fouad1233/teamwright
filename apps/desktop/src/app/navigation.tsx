import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';

import type { Id } from '@/shared/api/types';

/** The ways a project can be looked at. */
export const PROJECT_VIEWS = ['board', 'list', 'gantt', 'calendar', 'team'] as const;

export type ProjectView = (typeof PROJECT_VIEWS)[number];

/** Where the user is in the application. */
interface Location {
  /** The project that is open, if any. */
  projectId: Id | null;
  /** How the open project is shown. */
  view: ProjectView;
  /** The task whose details are open, if any. */
  taskId: Id | null;
}

/**
 * Where the user is, and the ways to go somewhere else.
 *
 * The ways are plain functions, so a component can hand one on as it is:
 * `onClose={navigation.closeTask}`.
 */
export interface Navigation extends Location {
  openProject: (projectId: Id) => void;
  showView: (view: ProjectView) => void;
  openTask: (taskId: Id) => void;
  closeTask: () => void;
}

/** Where the last project and view are remembered between sessions. */
const STORAGE_KEY = 'teamwright.navigation';

const STARTING_LOCATION: Location = { projectId: null, view: 'board', taskId: null };

/** Reads where the user was when the application was last closed. */
function restoreLocation(): Location {
  try {
    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? 'null') as unknown;
    if (typeof stored !== 'object' || stored === null) {
      return STARTING_LOCATION;
    }
    const { projectId, view } = stored as Partial<Location>;
    return {
      projectId: typeof projectId === 'string' ? projectId : null,
      view: PROJECT_VIEWS.find((known) => known === view) ?? STARTING_LOCATION.view,
      taskId: null,
    };
  } catch {
    // Storage that is unavailable or holds something unreadable only means
    // the user starts at the beginning.
    return STARTING_LOCATION;
  }
}

/** Remembers the project and the view. An open task is not remembered. */
function rememberLocation(location: Location): void {
  try {
    const remembered = { projectId: location.projectId, view: location.view };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(remembered));
  } catch {
    // Without storage the location is simply not remembered.
  }
}

const NavigationContext = createContext<Navigation | null>(null);

/**
 * Keeps track of where the user is. A desktop application has no address
 * bar, so this small piece of state takes the place of a router.
 */
export function NavigationProvider({ children }: { children: ReactNode }) {
  const [location, setLocation] = useState(restoreLocation);

  useEffect(() => {
    rememberLocation(location);
  }, [location]);

  const navigation = useMemo<Navigation>(() => {
    const go = (change: Partial<Location>) => {
      setLocation((current) => ({ ...current, ...change }));
    };
    return {
      ...location,
      openProject: (projectId) => {
        go({ projectId, taskId: null });
      },
      showView: (view) => {
        go({ view });
      },
      openTask: (taskId) => {
        go({ taskId });
      },
      closeTask: () => {
        go({ taskId: null });
      },
    };
  }, [location]);

  return <NavigationContext value={navigation}>{children}</NavigationContext>;
}

/** Where the user is, and the ways to go somewhere else. */
export function useNavigation(): Navigation {
  const navigation = useContext(NavigationContext);
  if (!navigation) {
    throw new Error('useNavigation must be used inside a NavigationProvider');
  }
  return navigation;
}
