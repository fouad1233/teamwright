import { createContext, useContext, type ReactNode } from 'react';

import type { Id } from '@/shared/api/types';

const NewTaskViewContext = createContext<Id | undefined>(undefined);

interface NewTaskViewProviderProps {
  /** The view that is open and can be filtered, or nothing. */
  viewId: Id | undefined;
  children: ReactNode;
}

/**
 * Makes the view on screen known to everything inside, so that a task opened
 * anywhere in it is made "in the view": the backend then gives the task what
 * the filter of the view asks of it, such as its tag, and the task stays in
 * sight. Without it, every view would have to know of filters.
 */
export function NewTaskViewProvider({ viewId, children }: NewTaskViewProviderProps) {
  return <NewTaskViewContext value={viewId}>{children}</NewTaskViewContext>;
}

/** The view a task opened now is made in, or nothing. */
export function useViewOfNewTasks(): Id | undefined {
  return useContext(NewTaskViewContext);
}
