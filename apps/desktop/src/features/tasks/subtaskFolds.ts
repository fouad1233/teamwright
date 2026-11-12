import { useMemo, useSyncExternalStore } from 'react';

import type { Id } from '@/shared/api/types';

/** The part of the browser's storage that the folds are remembered in. */
export type FoldStorage = Pick<Storage, 'getItem' | 'setItem'>;

/** The open tasks of a project are stored under this name and its identifier. */
const STORAGE_KEY_PREFIX = 'teamwright.openSubtasks.';

const NOTHING_OPEN: ReadonlySet<Id> = new Set();

/**
 * Remembers, for every project, which tasks show their subtasks.
 *
 * A task is closed until the manager opens it. Whether it is open says
 * something about the window and nothing about the project, so it is kept in
 * the browser's storage and never sent to the backend. Without storage it
 * lasts as long as the window does.
 */
export class SubtaskFoldStore {
  private readonly storage: FoldStorage | undefined;
  private readonly openByProject = new Map<Id, ReadonlySet<Id>>();
  private readonly listeners = new Set<() => void>();

  constructor(storage?: FoldStorage) {
    this.storage = storage;
  }

  /**
   * The tasks of a project that show their subtasks. The same set is
   * returned until one of them changes, so it can be compared by identity.
   */
  openTasks(projectId: Id): ReadonlySet<Id> {
    const known = this.openByProject.get(projectId);
    if (known) {
      return known;
    }
    const restored = this.restore(projectId);
    this.openByProject.set(projectId, restored);
    return restored;
  }

  /** Opens a task that is closed and closes one that is open. */
  toggle(projectId: Id, taskId: Id): void {
    const open = new Set(this.openTasks(projectId));
    if (!open.delete(taskId)) {
      open.add(taskId);
    }
    this.openByProject.set(projectId, open);
    this.remember(projectId, open);
    for (const listener of this.listeners) {
      listener();
    }
  }

  /** Calls `listener` after every change, until the returned function is called. */
  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  private restore(projectId: Id): ReadonlySet<Id> {
    try {
      const stored: unknown = JSON.parse(
        this.storage?.getItem(STORAGE_KEY_PREFIX + projectId) ?? 'null',
      );
      if (Array.isArray(stored)) {
        return new Set(stored.filter((entry): entry is Id => typeof entry === 'string'));
      }
    } catch {
      // Storage that is unavailable or holds something unreadable only means
      // that every task starts closed.
    }
    return NOTHING_OPEN;
  }

  private remember(projectId: Id, open: ReadonlySet<Id>): void {
    try {
      this.storage?.setItem(STORAGE_KEY_PREFIX + projectId, JSON.stringify([...open]));
    } catch {
      // Without storage the folds are simply not remembered.
    }
  }
}

/** The storage of the browser, if there is one and it may be used. */
function browserStorage(): FoldStorage | undefined {
  try {
    return typeof localStorage === 'undefined' ? undefined : localStorage;
  } catch {
    return undefined;
  }
}

/** The folds of this window. Every view of a project shares them. */
const windowFolds = new SubtaskFoldStore(browserStorage());

/** Which tasks of one project show their subtasks, and the way to change it. */
export interface SubtaskFolds {
  isOpen: (taskId: Id) => boolean;
  toggle: (taskId: Id) => void;
}

/**
 * The folds of a project. A task opened in one view is open in the others
 * too, and stays open the next time the project is looked at.
 */
export function useSubtaskFolds(projectId: Id): SubtaskFolds {
  const open = useSyncExternalStore(windowFolds.subscribe, () => windowFolds.openTasks(projectId));
  return useMemo(
    () => ({
      isOpen: (taskId) => open.has(taskId),
      toggle: (taskId) => {
        windowFolds.toggle(projectId, taskId);
      },
    }),
    [open, projectId],
  );
}
