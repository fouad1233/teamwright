import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';

import type { Id } from '@/shared/api/types';

/**
 * The places of a project. Unlike its views, which show its tasks and
 * belong to the manager, they are about the project itself and are always
 * there.
 */
export const PROJECT_PLACES = ['team', 'office', 'advisor'] as const;

export type ProjectPlace = (typeof PROJECT_PLACES)[number];

/**
 * What the open project shows: one of its views, by its identifier, or one
 * of its places. A view that is nothing, or that the project does not have
 * (any more), means the first view of the project.
 */
export type Showing = { view: Id | null } | { place: ProjectPlace };

/** Where the user is in the application. */
interface Location {
  /**
   * Whether the user looks at a project, at the inbox, which holds what
   * waits for the manager in every project, or at the computers that are
   * kept in step.
   */
  place: 'project' | 'inbox' | 'computers';
  /** The project that is open, or was open before the inbox, if any. */
  projectId: Id | null;
  /**
   * The area of that project whose tasks are shown, or nothing for every
   * area. It belongs to the project: it stays while the inbox or the
   * computers are open. An area the project does not have (any more) means
   * every area.
   */
  areaId: Id | null;
  /** What the open project shows. */
  showing: Showing;
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
  /**
   * Opens a project. Coming back to it from the inbox or the computers
   * brings back the area that was open; otherwise every area is shown.
   */
  openProject: (projectId: Id) => void;
  /** Opens one area of a project: every view then shows only its tasks. */
  openArea: (projectId: Id, areaId: Id) => void;
  /** Opens the inbox. The project that was open stays the one to come back to. */
  openInbox: () => void;
  /** Opens the computers. The project that was open stays the one to come back to. */
  openComputers: () => void;
  /** Shows a view of the open project. */
  showView: (viewId: Id) => void;
  /** Shows the first view of the open project, whatever it is. */
  showFirstView: () => void;
  /** Shows one of the places of the open project: its team, its office or its Advisor. */
  showPlace: (place: ProjectPlace) => void;
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

/** Where the last project and what it showed are remembered between sessions. */
const STORAGE_KEY = 'teamwright.navigation';

const FIRST_VIEW: Showing = { view: null };

const STARTING_LOCATION: Location = {
  place: 'project',
  projectId: null,
  areaId: null,
  showing: FIRST_VIEW,
  taskId: null,
  chatMemberId: null,
};

/** Reads what a project showed from what was remembered, if it can be read. */
function restoreShowing(stored: unknown): Showing {
  if (typeof stored !== 'object' || stored === null) {
    return FIRST_VIEW;
  }
  const { view, place } = stored as { view?: unknown; place?: unknown };
  const knownPlace = PROJECT_PLACES.find((known) => known === place);
  if (knownPlace) {
    return { place: knownPlace };
  }
  return { view: typeof view === 'string' ? view : null };
}

/** Reads where the user was when the application was last closed. */
function restoreLocation(): Location {
  try {
    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? 'null') as unknown;
    if (typeof stored !== 'object' || stored === null) {
      return STARTING_LOCATION;
    }
    const { projectId, areaId, showing } = stored as {
      projectId?: unknown;
      areaId?: unknown;
      showing?: unknown;
    };
    return {
      place: 'project',
      projectId: typeof projectId === 'string' ? projectId : null,
      areaId: typeof areaId === 'string' ? areaId : null,
      showing: restoreShowing(showing),
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
 * Remembers the project, its open area and what it shows. An open task or
 * conversation is not remembered, and neither is the inbox: the application
 * opens on a project. A view and an area are remembered by their
 * identifiers; if one has been removed by the next session, the project
 * opens on its first view, or on every area.
 */
function rememberLocation(location: Location): void {
  try {
    const remembered = {
      projectId: location.projectId,
      areaId: location.areaId,
      showing: location.showing,
    };
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
    /** Goes into a project, keeping its open area only if it is the same project. */
    const enter = (projectId: Id, change: Partial<Location>) => {
      setLocation((current) => ({
        ...current,
        place: 'project',
        projectId,
        areaId: current.projectId === projectId ? current.areaId : null,
        ...change,
      }));
    };
    return {
      ...location,
      openProject: (projectId) => {
        setLocation((current) => ({
          ...current,
          place: 'project',
          projectId,
          // Back from the inbox or the computers, the project shows what it
          // showed. Pressed while it is open, it shows every area.
          areaId:
            current.place !== 'project' && current.projectId === projectId ? current.areaId : null,
          taskId: null,
          chatMemberId: null,
        }));
      },
      openArea: (projectId, areaId) => {
        go({ place: 'project', projectId, areaId, taskId: null, chatMemberId: null });
      },
      openInbox: () => {
        go({ place: 'inbox', taskId: null, chatMemberId: null });
      },
      openComputers: () => {
        go({ place: 'computers', taskId: null, chatMemberId: null });
      },
      openTaskOf: (projectId, taskId) => {
        enter(projectId, { taskId, chatMemberId: null });
      },
      openAdvisorOf: (projectId) => {
        enter(projectId, { showing: { place: 'advisor' }, taskId: null, chatMemberId: null });
      },
      showView: (viewId) => {
        go({ showing: { view: viewId } });
      },
      showFirstView: () => {
        go({ showing: FIRST_VIEW });
      },
      showPlace: (place) => {
        go({ showing: { place } });
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
