import { createContext, useContext, type ReactNode } from 'react';

import type { Id, MemoryEntry } from '@/shared/api/types';

import { useProjectMemory } from './memoryQueries';

/** The memory of the open project, while it is known. */
const MemoryContext = createContext<MemoryEntry[] | undefined>(undefined);

/**
 * Asks for the memory of a project once, for every part of the project that
 * shows some of it: the place of memory and the turns of conversations.
 */
export function MemoryProvider({ projectId, children }: { projectId: Id; children: ReactNode }) {
  const memory = useProjectMemory(projectId);
  return <MemoryContext.Provider value={memory.data}>{children}</MemoryContext.Provider>;
}

/** Every entry of the memory of the open project, the newest first, once it is known. */
export function useMemoryOfProject(): MemoryEntry[] | undefined {
  return useContext(MemoryContext);
}
