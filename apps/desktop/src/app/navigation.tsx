import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';

import type { Id } from '@/shared/api/types';

/** The ways a project can be looked at. */
export const PROJECT_VIEWS = [
  'board',
  'list',
  'gantt',
  'calendar',
  'team',
  'office',
  'advisor',
] as const;

export type ProjectView = (typeof PROJECT_VIEWS)[number];

/** Where the user is in the application. */
interface Location {
  /**
   * Whether the user looks at a project or at the inbox, which holds what
   * waits for the manager in every project.
   */
  place: 'project' | 'inbox';
  /** The project that is open, or was open before the inbox, if any. */
  projectId: Id | null;
  /** How the open project is shown. */
  view: ProjectView;
  /** The task whose details are open, if any. */
  taskId: Id | null;
  /** The specialist whose conversation with the manager is open, if any. */
  chatMemberId: Id | null;
}

/**
 * Where the user is, and the ways to go somewhere else.
 *
 * The ways are plain functions, so a component can hand one on as it is:
 * `onClose={navigation.closeTask}`.
 */
export interface Navigation extends Location {
  openProject: (projectId: Id) => void;
  /** Opens the inbox. The project that was open stays the one to come back to. */
  openInbox: () => void;
  showView: (view: ProjectView) => void;
  openTask: (taskId: Id) => void;
  /** Opens a task of any project, for example from the inbox. */
  openTaskOf: (projectId: Id, taskId: Id) => void;
  /** Opens the conversation with the Advisor of any project, for example from the inbox. */
  openAdvisorOf: (projectId: Id) => void;
  closeTask: () => void;
  /** Opens the conversation with a specialist. It takes the place of an open task. */
  openChat: (memberId: Id) => void;
  closeChat: () => void;
}

/** Where the last project and view are remembered between sessions. */
const STORAGE_KEY = 'teamwright.navigation';

const STARTING_LOCATION: Location = {
  place: 'project',
  projectId: null,
  view: 'board',
  taskId: null,
  chatMemberId: null,
};

/** Reads where the user was when the application was last closed. */
function restoreLocation(): Location {
  try {
    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? 'null') as unknown;
    if (typeof stored !== 'object' || stored === null) {
      return STARTING_LOCATION;
    }
    const { projectId, view } = stored as Partial<Location>;
    return {
      place: 'project',
      projectId: typeof projectId === 'string' ? projectId : null,
      view: PROJECT_VIEWS.find((known) => known === view) ?? STARTING_LOCATION.view,
      taskId: null,
      chatMemberId: null,
    };
  } catch {
    // Storage that is unavailable or holds something unreadable only means
    // the user starts at the beginning.
    return STARTING_LOCATION;
  }
}

/**
 * Remembers the project and the view. An open task or conversation is not
 * remembered, and neither is the inbox: the application opens on a project.
 */
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
        go({ place: 'project', projectId, taskId: null, chatMemberId: null });
      },
      openInbox: () => {
        go({ place: 'inbox', taskId: null, chatMemberId: null });
      },
      openTaskOf: (projectId, taskId) => {
        go({ place: 'project', projectId, taskId, chatMemberId: null });
      },
      openAdvisorOf: (projectId) => {
        go({ place: 'project', projectId, view: 'advisor', taskId: null, chatMemberId: null });
      },
      showView: (view) => {
        go({ view });
      },
      // The task and the conversation share one place at the side of the
      // window, so opening the one closes the other.
      openTask: (taskId) => {
        go({ taskId, chatMemberId: null });
      },
      closeTask: () => {
        go({ taskId: null });
      },
      openChat: (chatMemberId) => {
        go({ chatMemberId, taskId: null });
      },
      closeChat: () => {
        go({ chatMemberId: null });
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
